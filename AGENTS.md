# Ludusavi - Cross-Device Save Sync

## Architecture Summary

Ludusavi is a Rust game save backup tool being modified into a **per-game cloud sync tool** for Steam Deck and PC.

### Current State
- **Framework**: Iced 0.14 GUI + Clap CLI
- **Cloud**: rclone-based sync (currently `rclone sync` which mirrors - deletes non-included files)
- **Backup**: Bulk backup/restore of all games via `BackupLayout` / `GameLayout`
- **Config**: YAML-based (`config.yaml`) with roots, redirects, custom games, cloud settings

### Target State
- **UI**: Single "Sync" screen with game cards (Steam Deck optimized: 48px+ buttons, larger fonts)
- **Cloud**: `rclone copy` (additive only - never deletes from cloud)
- **Backup**: Per-game push/pull with merge on pull
- **Config**: JSON `settings.config` at backup root with per-game sync metadata

### Key Changes
1. Replace `rclone sync` with `rclone copy` for additive-only cloud operations
2. Replace Backup/Restore screens with game card UI
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
- [ ] Create `src/gui/game_card.rs` - card data model + Steam Deck rendering
- [ ] Modify `src/gui/icon.rs` - add larger up/down arrow icons for Steam Deck
- [ ] Modify `src/gui/button.rs` - add push/pull arrow buttons (48x48px min)
- [ ] Modify `src/gui/screen.rs` - replace Backup/Restore with Sync screen
- [ ] Modify `src/gui/common.rs` - new Screen, Message, Operation variants
- [ ] Modify `src/gui/app.rs` - new sync state machine, remove old backup/restore
- [ ] Modify `src/gui/modal.rs` - add sync confirmation modals
- [ ] **Commit Phase 2**

### Phase 3: Integration & Polish
- [ ] Wire up async progress for rclone operations
- [ ] Batch sync ("Sync All Enabled") with progress bar
- [ ] **Commit Phase 3**

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
| `src/gui/game_card.rs` | NEW | Game card model + rendering |
| `src/gui/icon.rs` | MODIFY | Add larger arrow icons |
| `src/gui/button.rs` | MODIFY | Add push/pull buttons |
| `src/gui/screen.rs` | MODIFY | Replace with Sync screen |
| `src/gui/common.rs` | MODIFY | New types |
| `src/gui/app.rs` | MODIFY | New state machine |
| `src/gui/modal.rs` | MODIFY | Sync confirmation modals |
