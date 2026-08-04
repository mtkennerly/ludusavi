# Ludusavi - Cross-Device Save Sync

## Architecture Summary

Ludusavi is a Rust game save backup tool being modified into a **per-game cloud sync tool** for Steam Deck and PC.

### Current State (post-GUI-strip)
- **Framework**: Clap CLI only. The Phase 1-3 iced GUI (`src/gui/`) was deliberately deleted — see "Frontend Pivot" below.
- **Cloud**: `rclone copy` (additive only - never deletes from cloud) for per-game `sync push`/`sync pull`; `rclone sync` (mirroring, can delete) still backs the older bulk `cloud upload`/`cloud download` commands.
- **Backup**: Bulk backup/restore of all games via `BackupLayout` / `GameLayout`, plus per-game push/pull via `sync.rs`.
- **Config**: YAML-based (`config.yaml`) with roots, redirects, custom games, cloud settings, plus fork's `SyncConfig { enabled_games }`.
- **Sync metadata**: JSON `settings.config` at the backup root, per-game `{ last_push, device, mapping_path }` (`resource/sync_state.rs`).

### Frontend Pivot (current phase)
The original plan below (Phases 1-3) built a Rust-native iced GUI with game cards. That GUI has since been **removed**. Decision: no more Rust-native GUI toolkits — the next frontend will be **Tauri** (JS/HTML/React frontend, Rust backend). Rationale: Rust GUI ecosystem (iced included) was judged not worth fighting; Tauri gets a non-Rust frontend without Electron's Chromium+Node bundle weight, which matters on Steam Deck, and its commands call straight into `api.rs`'s `Ludusavi` struct in-process (no subprocess/JSON-stdio hop).

Sequencing:
1. **Done** — strip iced GUI, go CLI-only, give `sync.rs` real callers (`cli.rs`'s `sync push|pull|status` subcommand, `api.rs`'s `Ludusavi::sync_push`/`sync_pull`/`sync_status`).
2. **Done** — Tauri app scaffolded at `desktop/` (React+TS frontend, `pnpm`). `desktop/src-tauri` depends on the root crate as a path dependency (`ludusavi = { path = "../..", default-features = false }` — no `app` feature, so no clap/rfd/dialoguer in the Tauri backend's dep tree). `Ludusavi::load()` happens once at startup into managed state (`Option<Mutex<Ludusavi>>`, `None` if `manifest.yaml` isn't present yet, so a fresh checkout doesn't crash the window on launch).
3. **Done** — real sync screen, replacing the placeholder list. `App.tsx`: one unified, deduplicated game list (no separate "enabled" vs "search" sections) — a debounced search box (`search_games`, capped at 50, only queries non-empty input) and a "Scan" button (`scan_games`, full-library preview backup, same cost as upstream's startup scan) both feed candidates into it; a star toggle per row (`set_game_enabled`) controls both `sync.enabled_games` membership and sort-to-top, and gates whether Backup/Push/Pull/Status buttons show. `CloudSettings.tsx` is a separate page (gear icon, no router lib — plain `useState` page switch in `App.tsx`): connect/disconnect Google Drive, cloud folder path, auto-sync-after-backup toggle.
4. **Next** — surface the Wine/Proton remap diagnostics (`api.rs::wine_prefixes_for`/`backup_wine_prefixes`/`registered_prefixes`, see "Cross-device Wine/Proton remap" below) in the UI: warn on a game row when a pulled backup's semantics won't resolve locally, before the user hits the `WinePrefixNotFound` refusal on an actual restore. Batch "sync all starred" action and live rclone progress (currently `sync_push`/`sync_pull` block until done with no progress feedback) are also still open.

When picking up GUI work again: build the new Tauri frontend against `api.rs`, not against `sync.rs`/`cloud.rs` directly, and don't recreate `src/gui/`. Run it with `cd desktop && pnpm install && pnpm run tauri:dev` (works around a WebKitGTK/Wayland crash some compositors hit with plain `pnpm tauri dev` — see `desktop/scripts/dev.sh`). Building a real binary for daily use (CLI and/or the Tauri app, for this machine or a Steam Deck): see root `README.md`'s "Building" section.

## Cross-device Wine/Proton remap

Separate feature from the frontend pivot above, landed the same session. The bug: Baldur's Gate 3 (or any Steam Proton game) backed up on two machines produces two divergent absolute-path trees in the same cloud game folder — `/home/deck/.../compatdata/4110821628/pfx/...` vs `/home/kookoo/.../compatdata/2811670038/pfx/...` — because both the Linux username and the Proton compatdata app ID differ per machine (the app ID even for the *same* game, since a non-Steam-shortcut ID is generated per device). Pulling one onto the other and restoring wrote to the literal foreign path. Full design doc: `/home/kookoo/.claude/plans/delegated-wobbling-mochi.md`.

Fixed via two mechanisms kept deliberately separate — **decompose** (which absolute prefix in a given backup was a Wine prefix — `layout.rs::BackupSemantics`, per backup) and **recompose** (which prefix *this* device should write into — `GameSyncEntry.prefixes: BTreeMap<device, path>` in `settings.config`, per device, cross-device). See CLAUDE.md's "Cross-device Wine/Proton remap" section for the file-by-file mechanics; summary:

- `scan.rs::wine_prefixes_with_included_files` now includes Steam compatdata (`steamapps/compatdata/<id>/pfx`) as a candidate, not just custom-game/launcher prefixes — this is why a Proton backup's `mapping.yaml` now gets a `semantics` block at all.
- `sync.rs::push_game` records this device's resolved prefix into `settings.config`'s registry at push time; `scan/semantic/discovery.rs::WineEnvironment::prefixes_for_game` reads it back first at restore, falling back to re-discovery (Steam shortcut ID → manifest Steam ID → launcher prefix) for a device that's never pushed this game.
- `SyncStateFile::merge_from` (`resource/sync_state.rs`) replaced a straight overwrite on pull — each device only knows its own registry key, so a naive overwrite would erase every other device's entry on the next sync.
- `config.scan.redirect_wine` defaults `true` now (was `false`); restore refuses (`Error::WinePrefixNotFound`) rather than silently writing a foreign path when no local prefix can be found.

Verified end-to-end against the real bug report's data (`~/ludusavi-backup/Baldur's Gate 3/mapping.yaml` gained the expected `semantics` block; `settings.config` gained the expected `prefixes` entry after a real `sync push`).

### Original Target State (superseded by the pivot above, kept for history)
- **UI**: Single "Sync" screen with game cards (Steam Deck optimized: 48px+ buttons, larger fonts) — this was the iced GUI's design; the same card-based UX is still the goal, just rebuilt in the Tauri frontend instead.
- **Cloud**: `rclone copy` (additive only - never deletes from cloud) — done, still current.
- **Backup**: Per-game push/pull with merge on pull — done via `sync.rs`, now wired to the CLI/API.
- **Config**: JSON `settings.config` at backup root with per-game sync metadata — done, still current.

### Key Changes (Phases 1-3, historical)
1. Replace `rclone sync` with `rclone copy` for additive-only cloud operations
2. Replace Backup/Restore screens with game card UI *(superseded: screens were removed, not replaced; card UI is now a Tauri-frontend goal)*
3. Add `SyncConfig` to config for enabled games tracking
4. Add `SyncStateFile` for cloud metadata (settings.config)
5. New `sync.rs` module for push/pull/merge logic

---

## Development Todo

### Phase 1: Foundation (No UI)
- [x] Create `AGENTS.md` (this file)
- [x] Create `src/resource/sync_state.rs` - settings.config JSON read/write
- [x] Modify `src/resource/config.rs` - add `SyncConfig` with `enabled_games`
- [x] Modify `src/cloud.rs` - add `Rclone::copy()` method (additive, no delete)
- [x] Create `src/sync.rs` - `push_game()`, `pull_game()`, merge logic
- [x] Update `src/lib.rs` - add `pub mod sync`
- [x] **Commit Phase 1**

### Phase 2: GUI Redesign
- [x] Create `src/gui/game_card.rs` - card data model + Steam Deck rendering
- [x] Modify `src/gui/button.rs` - nav button for Sync screen
- [x] Modify `src/gui/screen.rs` - Sync screen with view()
- [x] Modify `src/gui/common.rs` - Screen::Sync, sync Message variants
- [x] Modify `src/gui/app.rs` - game_cards population, sync handlers
- [x] Modify `src/gui/style.rs` - Container::Card, Container::Badge
- [x] Modify `src/gui/widget.rs` - SYNC_SCROLL id
- [x] Modify `src/main.rs` - add sync to use ludusavi import
- [x] **Commit Phase 2**

### Phase 3: Integration & Polish
- [x] Wire up async progress for rclone operations (via rclone_monitor subscription)
- [x] Batch sync ("Sync All Enabled") with progress bar
- [x] Compute SyncState from local vs cloud comparison (NotSynced, LocalOnly, Synced, CloudNewer)
- [x] **Commit Phase 3**

---

## Steam Deck UI Requirements
- Card height: minimum 80px
- Buttons: minimum 48x48px touch targets
- Icons: 24px+ (larger than default 20px)
- Font sizes: +2-4px over defaults
- Padding: extra breathing room between elements

---

## File Change Summary

| File | Change | Description |
|------|--------|-------------|
| `AGENTS.md` | NEW | Architecture docs and todo |
| `src/resource/sync_state.rs` | NEW | settings.config JSON read/write |
| `src/resource/config.rs` | MODIFY | Add SyncConfig |
| `src/cloud.rs` | MODIFY | Add Rclone::copy() method |
| `src/sync.rs` | NEW | Push/pull/merge logic |
| `src/lib.rs` | MODIFY | Add pub mod sync |
| `src/gui/game_card.rs` | NEW | Game card model + Steam Deck rendering |
| `src/gui/icon.rs` | MODIFY | Add larger arrow icons |
| `src/gui/button.rs` | MODIFY | Add push/pull buttons |
| `src/gui/screen.rs` | MODIFY | Replace with Sync screen |
| `src/gui/common.rs` | MODIFY | New types |
| `src/gui/app.rs` | MODIFY | New state machine |
| `src/gui/modal.rs` | MODIFY | Sync confirmation modals |
