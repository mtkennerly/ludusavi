// Tauri backend for the fork's sync frontend.
//
// Commands bind straight to `api.rs::Ludusavi` (the same surface the CLI's
// `sync push|pull|status` subcommand uses) - no subprocess, no JSON-stdio
// hop. See AGENTS.md "Frontend Pivot" and CLAUDE.md for why.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ludusavi::{
    api::{CloudStatus, Ludusavi, parameters},
    prelude::{Cancel, Finality},
    report::ApiGame,
    resource::sync_state::GameSyncEntry,
    sync::SyncProgress,
};
use serde::Serialize;
use tauri::Emitter;

/// `Ludusavi::load()` needs `manifest.yaml` to already exist (via `ludusavi manifest
/// update` or a prior GUI/CLI run), so on a fresh checkout it can fail. Hold that as
/// `None` rather than crashing app startup; commands surface a clear error instead.
struct AppState {
    ludusavi: Mutex<Option<Ludusavi>>,
    /// Cooperative cancel flag for the in-flight `scan_games`, flipped by `cancel_scan`.
    /// Kept outside the `Ludusavi` mutex so a cancel is delivered even while a scan
    /// is holding that lock.
    scan_cancel: Arc<AtomicBool>,
}

fn load_ludusavi() -> Option<Ludusavi> {
    match Ludusavi::load() {
        Ok(l) => Some(l),
        Err(e) => {
            eprintln!("Failed to load Ludusavi state (run `ludusavi manifest update` first?): {e:?}");
            None
        }
    }
}

fn with_ludusavi<T>(
    state: &tauri::State<AppState>,
    f: impl FnOnce(&Ludusavi) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state.ludusavi.lock().map_err(|e| e.to_string())?;
    let ludusavi = guard
        .as_ref()
        .ok_or_else(|| "Config/manifest not loaded - run `ludusavi manifest update`, then restart".to_string())?;
    f(ludusavi)
}

fn with_ludusavi_mut<T>(
    state: &tauri::State<AppState>,
    f: impl FnOnce(&mut Ludusavi) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state.ludusavi.lock().map_err(|e| e.to_string())?;
    let ludusavi = guard
        .as_mut()
        .ok_or_else(|| "Config/manifest not loaded - run `ludusavi manifest update`, then restart".to_string())?;
    f(ludusavi)
}

/// Live progress of a `sync_push`/`sync_pull`, streamed to the webview as
/// `"sync-progress"` events so the UI can show a per-game progress bar.
#[derive(Clone, Serialize)]
struct SyncProgressEvent {
    game: String,
    current: f32,
    total: f32,
}

/// Push a single game's local backup to the cloud (additive - never deletes other games).
#[tauri::command]
async fn sync_push(
    app: tauri::AppHandle,
    game: String,
    preview: bool,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    with_ludusavi(&state, |l| {
        let finality = if preview { Finality::Preview } else { Finality::Final };
        let mut on_progress = |p: SyncProgress| {
            let _ = app.emit(
                "sync-progress",
                SyncProgressEvent {
                    game: game.clone(),
                    current: p.current,
                    total: p.max,
                },
            );
        };
        l.sync_push(&game, finality, Some(&mut on_progress))
            .map(|r| r.changes.len())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Pull a single game's backup from the cloud (additive - never deletes local data).
#[tauri::command]
async fn sync_pull(
    app: tauri::AppHandle,
    game: String,
    preview: bool,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    with_ludusavi(&state, |l| {
        let finality = if preview { Finality::Preview } else { Finality::Final };
        let mut on_progress = |p: SyncProgress| {
            let _ = app.emit(
                "sync-progress",
                SyncProgressEvent {
                    game: game.clone(),
                    current: p.current,
                    total: p.max,
                },
            );
        };
        l.sync_pull(&game, finality, Some(&mut on_progress))
            .map(|r| r.changes.len())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Last-known cloud sync info for a game, from `settings.config`.
#[tauri::command]
async fn sync_status(game: String, state: tauri::State<'_, AppState>) -> Result<Option<GameSyncEntry>, String> {
    with_ludusavi(&state, |l| Ok(l.sync_status(&game)))
}

/// Games currently enabled for sync (`config.yaml`'s `sync.enabled_games`).
#[tauri::command]
async fn enabled_games(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    with_ludusavi(&state, |l| Ok(l.config.sync.enabled_games.iter().cloned().collect()))
}

/// Search every game Ludusavi recognizes, for the "add a game" search box.
#[tauri::command]
async fn search_games(query: String, state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    with_ludusavi(&state, |l| Ok(l.search_games(&query, 50)))
}

/// Enable or disable a game for cloud sync.
#[tauri::command]
async fn set_game_enabled(game: String, enabled: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    with_ludusavi_mut(&state, |l| {
        l.set_game_enabled(&game, enabled);
        Ok(())
    })
}

/// Take a real local backup of one game (scans its roots, copies changed saves into the
/// backup dir) - the step that has to happen before there's anything to push.
#[tauri::command]
async fn backup_game(game: String, state: tauri::State<'_, AppState>) -> Result<usize, String> {
    with_ludusavi_mut(&state, |l| {
        let output = l
            .back_up(parameters::BackUp {
                games: vec![game.clone()],
                finality: Finality::Final,
                ..Default::default()
            })
            .map_err(|e| format!("{e:?}"))?;

        Ok(match output.games.get(&game) {
            Some(ApiGame::Operative { files, registry, .. }) => files.len() + registry.len(),
            _ => 0,
        })
    })
}

/// One game found by [`scan_games`]: it has actual local save data on this machine,
/// whether or not it's currently enabled for sync.
#[derive(Serialize)]
struct ScanResult {
    name: String,
    file_count: usize,
    registry_count: usize,
    /// Debug-formatted `ScanChange` (e.g. "New", "Different", "Same").
    change: String,
}

/// Scan every game Ludusavi knows about against this machine's configured roots - the
/// same full-library preview upstream ludusavi does on startup. Slow-ish (walks
/// thousands of possible games), which is inherent to the operation, not this UI.
///
/// Runs as an async command so the webview stays responsive throughout.
#[tauri::command]
async fn scan_games(state: tauri::State<'_, AppState>) -> Result<Vec<ScanResult>, String> {
    state.scan_cancel.store(false, Ordering::Relaxed);
    let cancel = Cancel::from_flag(state.scan_cancel.clone());

    let output = with_ludusavi_mut(&state, |l| {
        l.back_up(parameters::BackUp {
            finality: Finality::Preview,
            cancel: Some(cancel.clone()),
            ..Default::default()
        })
        .map_err(|e| format!("{e:?}"))
    })?;

    // Cancelled: the partial preview is meaningless, and we must not persist a
    // half-scanned library as the discovered set. Return nothing; the UI has
    // already torn down the spinner.
    if cancel.is_cancelled() {
        return Ok(vec![]);
    }

    let mut results: Vec<ScanResult> = output
        .games
        .into_iter()
        .filter_map(|(name, game)| match game {
            ApiGame::Operative {
                change,
                files,
                registry,
                ..
            } => Some(ScanResult {
                name,
                file_count: files.len(),
                registry_count: registry.len(),
                change: format!("{change:?}"),
            }),
            _ => None,
        })
        .collect();
    results.sort_by(|a, b| a.name.cmp(&b.name));

    // Persist the discovered names so a restart doesn't require re-scanning.
    // A successful full scan replaces the set, pruning games no longer installed.
    with_ludusavi_mut(&state, |l| {
        l.set_discovered_games(results.iter().map(|r| r.name.clone()));
        Ok(())
    })?;

    Ok(results)
}

/// Cancel the in-flight [`scan_games`], if any. Cheap and immediate: it only flips
/// the shared flag; the scan's per-game steps notice and unwind.
#[tauri::command]
async fn cancel_scan(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.scan_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

/// Games found by a previous [`scan_games`], persisted so a restart doesn't
/// require re-scanning (`config.yaml`'s `sync.discovered_games`).
#[tauri::command]
async fn discovered_games(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    with_ludusavi(&state, |l| Ok(l.discovered_games()))
}

/// Current cloud remote/path/rclone status, for the settings screen.
#[tauri::command]
async fn cloud_status(state: tauri::State<'_, AppState>) -> Result<CloudStatus, String> {
    with_ludusavi(&state, |l| Ok(l.cloud_status()))
}

/// Configure Google Drive as the cloud remote.
///
/// This drives rclone's own OAuth flow (opens a browser, waits for approval) and can
/// take a while. Runs as an async command so the webview stays responsive.
#[tauri::command]
async fn connect_google_drive(state: tauri::State<'_, AppState>) -> Result<(), String> {
    with_ludusavi_mut(&state, |l| {
        l.set_cloud_remote_google_drive().map_err(|e| format!("{e:?}"))
    })
}

/// Tear down the configured cloud remote (both rclone's own config and ours).
#[tauri::command]
async fn disconnect_cloud(state: tauri::State<'_, AppState>) -> Result<(), String> {
    with_ludusavi_mut(&state, |l| l.disconnect_cloud_remote().map_err(|e| format!("{e:?}")))
}

/// Cloud-side folder name to sync into.
#[tauri::command]
async fn set_cloud_path(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    with_ludusavi_mut(&state, |l| {
        l.set_cloud_path(path);
        Ok(())
    })
}

/// Toggle upstream's auto-upload-after-backup. Separate from this fork's manual
/// `sync_push`/`sync_pull`.
#[tauri::command]
async fn set_cloud_synchronize(enabled: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    with_ludusavi_mut(&state, |l| {
        l.set_cloud_synchronize(enabled);
        Ok(())
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            ludusavi: Mutex::new(load_ludusavi()),
            scan_cancel: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            sync_push,
            sync_pull,
            sync_status,
            enabled_games,
            discovered_games,
            search_games,
            set_game_enabled,
            backup_game,
            scan_games,
            cancel_scan,
            cloud_status,
            connect_google_drive,
            disconnect_cloud,
            set_cloud_path,
            set_cloud_synchronize
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
