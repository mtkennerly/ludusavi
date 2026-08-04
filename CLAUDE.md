# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

A fork of [ludusavi](https://github.com/mtkennerly/ludusavi) (game save backup tool, Rust + Iced) being converted into a **per-game cloud sync tool** aimed at Steam Deck + PC. Upstream code (backup/restore/scan/CLI) is largely intact; the sync work is layered on top.

`AGENTS.md` holds the fork's design notes and phase checklist. Read it before making sync-related changes — it records the intended target state and history.

Upstream files (`CHANGELOG.md`, `docs/`, `lang/*.ftl`, `tasks.py` release tasks) describe the original project, not the fork.

**The GUI is gone (deliberately).** The original iced-based GUI (`src/gui/`, `src/gui.rs`) was stripped in favor of CLI-only, then rebuilt as a Tauri app instead of another Rust-native GUI toolkit. Don't resurrect `iced`/`rfd` dialogs or an `mod gui` in this crate — the CLI and `api.rs` are the integration points for the root crate; new frontend work belongs in `desktop/`.

`desktop/` is a separate Tauri project (React+TS, `pnpm`) that depends on this crate as a path dependency with `default-features = false` (see `desktop/src-tauri/Cargo.toml`) — no `app` feature, so no clap/rfd/dialoguer pulled into the GUI backend. Its `#[tauri::command]` handlers (`desktop/src-tauri/src/lib.rs`) wrap `api.rs::Ludusavi` directly, the same struct the CLI's `sync` subcommand uses — extend that struct's methods rather than reaching into `sync.rs`/`cloud.rs` from Tauri code. Run it with `cd desktop && pnpm install && pnpm tauri dev`. See `AGENTS.md`'s "Frontend Pivot" section for the full sequencing/rationale.

## Commands

```bash
cargo run -- --help                          # CLI-only binary; no args prints help (no GUI to launch)
cargo run -- backup --preview                # CLI mode
cargo test                                   # all tests
cargo test sync_state_file_roundtrip         # single test by name
cargo test --lib resource::sync_state        # one module
cargo fmt --all -- --check                   # CI formatting gate (currently already failing on master — see caveat below)
cargo clippy --workspace -- --deny warnings  # CI lint gate (currently already failing on master — see caveat below)
```

**Caveat:** as of the GUI strip, `cargo fmt --check` and `cargo clippy --deny warnings` both fail on unrelated pre-existing code (`sync.rs` formatting, `scan.rs` redundant-`&`-in-`format!` lints) — confirmed via `git stash` that this predates the strip. Don't assume a red CI run here is something you broke; check `git blame`/`git stash` before chasing it.

One-time test setup (some tests need a symlink fixture):

```bash
cd tests/root3/game5 && ln -s data data-symlink   # Windows: mklink /J data-symlink data + reg import tests/ludusavi.reg
```

Optional: `pip install --user pre-commit && pre-commit install` runs fmt + clippy on commit.

Linux build deps: `gcc libxcb-composite0-dev libgtk-3-dev`.

## Crate layout

Dual target: `src/lib.rs` (library, no `app` feature needed) and `src/main.rs` (binary, `required-features = ["app"]`). The `app` feature gates clap/rfd/dialoguer/indicatif/flexi_logger/signal-hook — all CLI-only now, no GUI toolkit in the dependency tree.

Key modules:

- `path.rs` — `StrictPath`, the path type used everywhere instead of `PathBuf`. Handles Windows/Linux differences, globs, interior-mutability caching (see `clippy.toml`).
- `prelude.rs` — `Error`, `CommandError`, `Finality` (Preview vs Final), `SyncDirection` (Upload/Download), `app_dir()`.
- `resource/` — YAML files under the app dir via the `ResourceFile`/`SaveableResourceFile` traits: `config.rs` (`config.yaml`, 3k lines, includes fork's `SyncConfig { enabled_games }`), `manifest.rs`, `cache.rs`. Exception: `sync_state.rs` is JSON at the *backup root*, not the app dir.
- `scan/` — game discovery and backup storage. `scan/layout.rs` (`BackupLayout`, `GameLayout`, `escape_folder_name`) owns the on-disk backup structure; `game_folder(name)` maps a game title to its directory.
- `cloud.rs` — rclone wrapper. `RcloneProcess` spawns rclone with `--use-json-log` and parses events.
- `sync.rs` — fork's per-game push/pull/merge logic (`push_game`, `pull_game`, `get_game_sync_info`). Callers: `cli.rs`'s `sync push|pull|status` subcommand and `api.rs`'s `Ludusavi::sync_push`/`sync_pull`/`sync_status` wrappers.
- `api.rs` — `Ludusavi` struct, the stable library entry point (`back_up`, `restore`, `list_backups`, `edit_backup`, `find_title`, plus the fork's `sync_push`/`sync_pull`/`sync_status`). This is the surface a future Tauri backend should bind against directly, rather than reaching into `cli.rs` or `sync.rs`.
- `report.rs`, `cli/` — CLI output and JSON API surface. `cli/parse.rs` defines the clap `Subcommand` enum (incl. the fork's `Sync` variant); `cli.rs::run()` dispatches it.

## Sync flow (the fork's core)

Cloud layout mirrors the local backup root, so a game's cloud folder is `<cloud path>/<escaped game folder>/`.

1. `Rclone::copy()` (not `sync()`) — additive, never deletes on the destination, so pushing game A cannot remove game B. It builds `--include=/<escaped dir>/**` filters (`escape_rclone_glob` handles `[]{}*?` in titles) and *always* also includes `/settings.config`. `Rclone::sync()` still exists too and backs the upstream `cloud upload`/`cloud download` subcommands (bulk, mirroring, can delete on the destination) — don't conflate the two; `sync push`/`sync pull` (singular game) always goes through `copy()`.
2. `resource/sync_state.rs` — `SyncStateFile` (`settings.config`, JSON, at the backup root) maps game name → `{ last_push, device, mapping_path }`. Written atomically (temp + rename). It rides along in the same rclone transfer as the game files.
3. `sync.rs::push_game` updates `settings.config` locally *before* spawning rclone so the metadata is part of that transfer; `pull_game` downloads `settings.config` first, then the game files.
4. `cli.rs::resolve_single_game` resolves a user-typed game name (fuzzy/case-insensitive) via `TitleFinder` before any push/pull/status — the CLI and `api.rs` both expect canonical titles internally.

## Conventions

- Rust edition 2024, `max_width = 120`, tabs forbidden, clippy warnings are CI errors (see the fmt/clippy caveat above for current known-broken state).
- Upstream UI strings go through Fluent: add the key to `lang/en-US.ftl` and a method on `TRANSLATOR` in `src/lang.rs` (other `lang/*.ftl` files are Crowdin-managed — don't hand-edit). The fork's own CLI additions (`sync push`/`pull`/`status` messages) hardcode English rather than going through Fluent, matching how the old game-card UI did it — follow the surrounding file's choice rather than mixing both in one place.
- New config fields need a `#[serde(default)]`-friendly shape plus a `skip_serializing_if` guard if they should stay out of `config.yaml` when unused (see `is_sync_config_default`).
- `docs/schema/*.yaml` are generated (`invoke docs-schema`); don't edit by hand. They don't yet cover the fork's `sync` subcommand or `Ludusavi::sync_*` methods — regenerate/extend if you formalize that surface for the JSON `api` command.
