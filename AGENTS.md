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
2. **Next** — scaffold a Tauri app; frontend in React; Tauri `#[tauri::command]` handlers wrap `api.rs::Ludusavi` methods (the same ones the CLI now uses), not a reimplementation.

When picking up GUI work again: build the new Tauri frontend against `api.rs`, not against `sync.rs`/`cloud.rs` directly, and don't recreate `src/gui/`.

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
