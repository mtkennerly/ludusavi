import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Mirrors api::CloudStatus.
interface CloudStatus {
  connected: boolean;
  remote_kind: string | null;
  path: string;
  synchronize: boolean;
  rclone_path: string;
  rclone_valid: boolean;
}

export function CloudSettings() {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  const [pathDraft, setPathDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function refresh() {
    invoke<CloudStatus>("cloud_status")
      .then((s) => {
        setStatus(s);
        setPathDraft(s.path);
      })
      .catch((e) => setError(String(e)));
  }

  useEffect(refresh, []);

  async function connect() {
    setBusy(true);
    setError(null);
    try {
      // Opens a browser for Google's OAuth consent screen; this call doesn't
      // return until that finishes (or fails), so it can take a while.
      await invoke("connect_google_drive");
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    setBusy(true);
    setError(null);
    try {
      await invoke("disconnect_cloud");
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function savePath() {
    setBusy(true);
    setError(null);
    try {
      await invoke("set_cloud_path", { path: pathDraft });
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function toggleSynchronize() {
    if (!status) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("set_cloud_synchronize", { enabled: !status.synchronize });
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!status) {
    return <section className="cloud-settings">Loading cloud settings...</section>;
  }

  return (
    <section className="cloud-settings">
      <h2>Cloud</h2>

      {error && <p className="error-text">{error}</p>}

      {!status.rclone_valid && (
        <p className="warning-text">
          rclone not found ({status.rclone_path || "no path set"}). Install it and make
          sure it's on your PATH, then reopen this screen.
        </p>
      )}

      {status.connected ? (
        <>
          <p>
            Connected: <strong>{status.remote_kind}</strong>
          </p>
          <label className="field">
            Cloud folder
            <input
              value={pathDraft}
              onChange={(e) => setPathDraft(e.currentTarget.value)}
              placeholder="ludusavi-backup"
            />
          </label>
          <div className="row">
            <button disabled={busy || pathDraft === status.path} onClick={savePath}>
              Save folder
            </button>
            <button disabled={busy} onClick={disconnect}>
              Disconnect
            </button>
          </div>
          <label className="checkbox-field">
            <input
              type="checkbox"
              checked={status.synchronize}
              disabled={busy}
              onChange={toggleSynchronize}
            />
            Auto-upload after every backup
          </label>
        </>
      ) : (
        <>
          <p>No cloud remote configured.</p>
          <button disabled={busy || !status.rclone_valid} onClick={connect}>
            {busy ? "Waiting for Google sign-in..." : "Connect Google Drive"}
          </button>
        </>
      )}
    </section>
  );
}
