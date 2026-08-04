# Ludusavi Sync -- Desktop App

Tauri desktop frontend for [Ludusavi Sync](../README.md)'s per-game cloud sync.
React + TypeScript frontend, Rust backend -- no subprocess hop, the Tauri
commands call straight into `api.rs::Ludusavi` in-process.

## Architecture

```
desktop/
  src/                    React frontend
    App.tsx               Sync screen + page switch (useState, no router)
    CloudSettings.tsx     Cloud config: connect/disconnect, path, auto-sync
    App.css               Styles (dark mode via prefers-color-scheme)
  src-tauri/              Rust backend
    src/lib.rs            13 Tauri commands wrapping api.rs::Ludusavi
    Cargo.toml            Depends on root crate (default-features = false)
    tauri.conf.json       Window title/size, bundle config
  scripts/
    dev.sh                WebKitGTK/Wayland workaround (see below)
```

The root crate (`ludusavi = { path = "../..", default-features = false }`) is
used as a path dependency -- the `app` feature (clap/rfd/dialoguer) is excluded
so the GUI backend has no CLI-only deps in its tree. `Ludusavi::load()` runs
once at startup into managed state; if `manifest.yaml` is missing (fresh
checkout), the window still opens and commands return a clear error instead of
panicking.

## Screens

### Sync screen (`App.tsx`)

Unified game list -- no separate "enabled" vs "search" sections.

- **Search box**: debounced `search_games` (capped at 50 results, only fires on
  non-empty input -- never dumps the ~19k-title manifest by default).
- **Scan button**: `scan_games`, a full-library `Finality::Preview` backup.
  Same cost as upstream's startup scan -- finds installed games on this machine.
- **Star toggle**: per-row, controls membership in `sync.enabled_games` and
  sorts starred games to the top. Only starred rows show action buttons.
- **Action buttons** (starred only): Backup, Push, Pull, Status.
- **Status text**: shows last push time and device, or "no cloud sync record".

### Cloud settings (`CloudSettings.tsx`)

Accessed via the gear icon in the header. Plain `useState` page switch, no
router library.

- Connect/disconnect Google Drive (opens browser for OAuth).
- Cloud folder path (the rclone remote subfolder).
- Auto-upload-after-backup toggle (upstream's bulk-sync feature, separate from
  the fork's manual per-game push/pull).

## Tauri Commands

All commands bind to `api.rs::Ludusavi` -- the same surface the CLI's
`sync push|pull|status` subcommand uses. Extend `api.rs` rather than reaching
into `sync.rs`/`cloud.rs` from here.

| Command | Description |
|---------|-------------|
| `sync_push` | Push one game's local backup to cloud (additive, no delete) |
| `sync_pull` | Pull one game's backup from cloud (additive, no delete) |
| `sync_status` | Last-known cloud sync info for a game (`settings.config`) |
| `enabled_games` | List games in `sync.enabled_games` |
| `search_games` | Fuzzy search the full manifest (capped at 50) |
| `set_game_enabled` | Add/remove a game from `sync.enabled_games` |
| `backup_game` | Take a local backup of one game (scan roots, copy saves) |
| `scan_games` | Full-library preview backup -- find all installed games |
| `cloud_status` | Current cloud remote, path, rclone validity |
| `connect_google_drive` | Start rclone OAuth flow (opens browser) |
| `disconnect_cloud` | Tear down cloud remote config |
| `set_cloud_path` | Set the cloud-side folder name |
| `set_cloud_synchronize` | Toggle auto-upload after backup |

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [pnpm](https://pnpm.io/)
- [rclone](https://rclone.org/) on your PATH
- Linux: `gcc libxcb-composite0-dev libgtk-3-dev libwebkit2gtk-4.1-dev`

### Running in dev mode

```bash
cd desktop
pnpm install
pnpm run tauri:dev
```

This uses `scripts/dev.sh`, which sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` to
work around a WebKitGTK/Wayland crash on some compositor/GPU combos. If it
still fails, the script retries with `GDK_BACKEND=x11` (XWayland fallback).

You can also run `pnpm tauri dev` directly if Wayland works fine on your setup,
but the dev.sh wrapper is the safe default.

### Building a release binary

```bash
cd desktop
pnpm install
pnpm run tauri build
```

Produces a portable AppImage in `src-tauri/target/release/bundle/appimage/`.
Single file, no install required -- just `chmod +x` and run.

### CI builds (GitHub Actions)

The `desktop-build.yaml` workflow builds an AppImage automatically when you push
a version tag and uploads it to the GitHub release:

```bash
git tag v0.31.0
git push origin v0.31.0
```

The workflow runs in an Arch Linux container, installs all Tauri system
dependencies, derives the version from the tag via [dunamai](https://github.com/mtkennerly/dunamai),
injects it into `tauri.conf.json`, and uploads the resulting `.AppImage` to the
release. The version in `tauri.conf.json` is a placeholder (`0.1.0`) -- CI
overwrites it at build time.

## Key Files to Read

- `../src/api.rs` -- the `Ludusavi` struct and all its methods (the integration
  surface for this frontend).
- `src-tauri/src/lib.rs` -- Tauri command handlers and app setup.
- `src/App.tsx` -- the sync screen UI and game list logic.
- `src/CloudSettings.tsx` -- cloud configuration screen.
- `../AGENTS.md` -- fork design notes, phase checklist, and the Wine/Proton
  remap docs.
