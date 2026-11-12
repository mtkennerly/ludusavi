#!/usr/bin/env python3
"""Exercise background backups with temporary configuration and a test process."""

import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time


def wait_for(check, timeout=30, description="background backup"):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if check():
            return
        time.sleep(0.2)
    raise AssertionError(f"Timed out waiting for {description}")


def wait_for_owner(app, configuration):
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as client:
        # The app handles reopen requests on its 200 ms background tick and can be
        # busy during startup, so wait for that tick instead of queuing more requests.
        client.settimeout(5)

        def ready():
            assert app.poll() is None, f"Background app exited during startup: {app.returncode}"
            try:
                endpoint = (configuration / "background.port").read_text(encoding="utf-8").splitlines()
                if len(endpoint) != 2 or int(endpoint[1]) != app.pid:
                    return False
                address = ("127.0.0.1", int(endpoint[0]))
                client.sendto(b"ludusavi:open:null", address)
                reply, sender = client.recvfrom(32)
                return sender == address and reply == b"ludusavi:opened"
            except (OSError, ValueError, OverflowError):
                return False

        wait_for(ready, description="the background app's reopen handshake")


def visible_windows(process):
    import ctypes
    from ctypes import wintypes
    user = ctypes.WinDLL("user32", use_last_error=True)
    callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user.EnumWindows.argtypes = [callback, wintypes.LPARAM]
    user.IsWindowVisible.argtypes = [wintypes.HWND]
    user.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    windows = []
    @callback
    def visit(window, _):
        owner = wintypes.DWORD()
        user.GetWindowThreadProcessId(window, ctypes.byref(owner))
        kind = ctypes.create_unicode_buffer(128)
        user.GetClassNameW(window, kind, len(kind))
        if owner.value == process and kind.value == "Window Class" and user.IsWindowVisible(window):
            windows.append(window)
        return True
    assert user.EnumWindows(visit, 0), "Unable to inspect the test app's windows"
    return windows


def run(binary, game):
    with tempfile.TemporaryDirectory(prefix="ludusavi-background-test-") as temporary:
        root = Path(temporary)
        configuration = root / "config"
        configuration.mkdir()
        command = [str(binary), "--config", str(configuration), "--no-manifest-update"]
        config = json.loads(subprocess.check_output(command + ["config", "show", "--api"], text=True))
        known_roots = config["roots"]
        save = root / "save.txt"
        save.write_text("first save", encoding="utf-8")
        config.update({
            "roots": [], "release": {"check": False},
            "backup": {"path": str(root / "backups")},
            "restore": {"path": str(root / "backups")},
            "cloud": {"synchronize": False},
            "customGames": [{"name": "Background Test", "files": [str(save)]}],
        })
        (configuration / "config.yaml").write_text(json.dumps(config), encoding="utf-8")
        (configuration / "cache.yaml").write_text(json.dumps({"roots": known_roots}), encoding="utf-8")
        (configuration / "manifest.yaml").write_text("{}", encoding="utf-8")
        background = {"enabled": True, "pollSeconds": 1, "delaySeconds": 2, "games": [
            {"name": "Background Test", "executables": [str(game)], "enabled": True}
        ]}
        (configuration / "background.yaml").write_text(json.dumps(background), encoding="utf-8")
        options = {"creationflags": subprocess.CREATE_NO_WINDOW} if os.name == "nt" else {}
        app = subprocess.Popen(command + ["gui"],
                               env={**os.environ, "RUST_LOG": "ludusavi=info"}, **options)
        def log_count(text):
            return sum(path.read_text(encoding="utf-8", errors="replace").count(text)
                       for path in configuration.glob("*.log"))
        detected = "Game process detected: Background Test"
        children = []

        def start_game():
            before = log_count(detected)
            child = subprocess.Popen([str(game)], **options)
            children.append(child)
            wait_for(lambda: log_count(detected) > before,
                     description="the app to detect the started game")
            return child
        try:
            wait_for_owner(app, configuration)
            if os.name == "nt":
                import ctypes
                from ctypes import wintypes
                user = ctypes.WinDLL("user32", use_last_error=True)
                user.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
                windows = visible_windows(app.pid)
                assert windows, "The test app's main window did not open"
                for window in windows:
                    assert user.PostMessageW(window, 0x0010, 0, 0), "Unable to close the test window"
                wait_for(lambda: not visible_windows(app.pid))
            for arguments in [[], ["gui"], ["gui", "--custom-game", "Reopen Test"], ["gui", "--custom-game", "Reopen Test"]]:
                reopened = subprocess.run(command + arguments, timeout=8, **options)
                assert reopened.returncode == 0, "Second launch failed"
                assert app.poll() is None, "Second launch stopped the running app"
                if os.name == "nt":
                    wait_for(lambda: visible_windows(app.pid))
            wait_for(lambda: log_count("Reopening Ludusavi after another launch") >= 5)
            def custom_game_count():
                current = json.loads(subprocess.check_output(command + ["config", "show", "--api"], text=True))
                return sum(game["name"] == "Reopen Test" for game in current["customGames"])
            wait_for(lambda: custom_game_count() == 1)
            print("Second launches reopen the owner; custom-game requests do not duplicate games")
            assert not list((root / "backups").glob("*/mapping.yaml")), "Backed up without an observed game exit"
            first = start_game()
            second = start_game()
            first.terminate()
            first.wait(timeout=5)
            time.sleep(3)
            assert not list((root / "backups").glob("*/mapping.yaml")), "Backed up while another game process was running"
            if os.name == "nt":
                assert log_count("Windows backup notification submitted for Background Test") == 0, \
                    "Notified before the game closed and the delay elapsed"
            second.terminate()
            second.wait(timeout=5)
            wait_for(lambda: bool(list((root / "backups").glob("*/mapping.yaml"))))
            if os.name == "nt":
                wait_for(lambda: log_count("Windows backup notification submitted for Background Test") == 1)
                wait_for(lambda: log_count("Windows backup result notification submitted: Automatic backup complete") == 1)
            # Restore with the ordinary CLI to verify the automatic backup's data.
            save.write_text("changed after backup", encoding="utf-8")
            subprocess.run(command + ["restore", "--force", "Background Test"], check=True, capture_output=True)
            assert save.read_text(encoding="utf-8") == "first save"
            # Editing background.yaml takes effect without restarting the app.
            background["games"][0]["enabled"] = False
            reloaded = log_count("Background configuration reloaded")
            (configuration / "background.yaml").write_text(json.dumps(background), encoding="utf-8")
            wait_for(lambda: log_count("Background configuration reloaded") > reloaded,
                     description="the app to reload background.yaml")
            save.write_text("must not be backed up", encoding="utf-8")
            third = subprocess.Popen([str(game)], **options)
            children.append(third)
            time.sleep(2)
            third.terminate()
            third.wait(timeout=5)
            time.sleep(4)
            subprocess.run(command + ["restore", "--force", "Background Test"], check=True, capture_output=True)
            assert save.read_text(encoding="utf-8") == "first save"
            background["games"][0]["enabled"] = True
            background["enabled"] = False
            disabled = log_count("Background monitoring disabled")
            (configuration / "background.yaml").write_text(json.dumps(background), encoding="utf-8")
            wait_for(lambda: log_count("Background monitoring disabled") > disabled,
                     description="the app to stop background monitoring")
            save.write_text("globally disabled", encoding="utf-8")
            fourth = subprocess.Popen([str(game)], **options)
            children.append(fourth)
            time.sleep(2)
            save.write_text("globally disabled", encoding="utf-8")
            fourth.terminate()
            fourth.wait(timeout=5)
            time.sleep(4)
            subprocess.run(command + ["restore", "--force", "Background Test"], check=True, capture_output=True)
            assert save.read_text(encoding="utf-8") == "first save"
            assert app.poll() is None, "Background app stopped after its backup"
            if os.name == "nt":
                assert log_count("Windows backup notification submitted for Background Test") == 1, \
                    "Disabled games must not send backup notifications"
                assert log_count("Windows backup result notification submitted:") == 1
            print("Backup, restore, disabled games, and success notification checks passed")
        except Exception:
            print(f"Background owner exit status: {app.poll()}", flush=True)
            for log in sorted(configuration.glob("*.log")):
                print(f"--- {log.name} ---", flush=True)
                print("\n".join(log.read_text(encoding="utf-8", errors="replace").splitlines()[-80:]), flush=True)
            raise
        finally:
            for child in children:
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=5)
            if app.poll() is None:
                app.terminate()
                app.wait(timeout=10)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("game", type=Path, help="A harmless test executable that stays running until terminated")
    args = parser.parse_args()
    run(args.binary.resolve(strict=True), args.game.resolve(strict=True))
