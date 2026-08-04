// Tauri backend for the fork's sync frontend.
//
// Commands bind straight to `api.rs::Ludusavi` (the same surface the CLI's
// `sync push|pull|status` subcommand uses) - no subprocess, no JSON-stdio
// hop. See AGENTS.md "Frontend Pivot" and CLAUDE.md for why.

use std::sync::Mutex;

use ludusavi::{api::Ludusavi, prelude::Finality, resource::sync_state::GameSyncEntry};

/// `Ludusavi::load()` needs `manifest.yaml` to already exist (via `ludusavi manifest
/// update` or a prior GUI/CLI run), so on a fresh checkout it can fail. Hold that as
/// `None` rather than crashing app startup; commands surface a clear error instead.
struct AppState(Mutex<Option<Ludusavi>>);

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
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let ludusavi = guard
        .as_ref()
        .ok_or_else(|| "Config/manifest not loaded - run `ludusavi manifest update`, then restart".to_string())?;
    f(ludusavi)
}

/// Push a single game's local backup to the cloud (additive - never deletes other games).
#[tauri::command]
fn sync_push(game: String, preview: bool, state: tauri::State<AppState>) -> Result<usize, String> {
    with_ludusavi(&state, |l| {
        let finality = if preview { Finality::Preview } else { Finality::Final };
        l.sync_push(&game, finality)
            .map(|r| r.changes.len())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Pull a single game's backup from the cloud (additive - never deletes local data).
#[tauri::command]
fn sync_pull(game: String, preview: bool, state: tauri::State<AppState>) -> Result<usize, String> {
    with_ludusavi(&state, |l| {
        let finality = if preview { Finality::Preview } else { Finality::Final };
        l.sync_pull(&game, finality)
            .map(|r| r.changes.len())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Last-known cloud sync info for a game, from `settings.config`.
#[tauri::command]
fn sync_status(game: String, state: tauri::State<AppState>) -> Result<Option<GameSyncEntry>, String> {
    with_ludusavi(&state, |l| Ok(l.sync_status(&game)))
}

/// Games currently enabled for sync (`config.yaml`'s `sync.enabled_games`).
#[tauri::command]
fn enabled_games(state: tauri::State<AppState>) -> Result<Vec<String>, String> {
    with_ludusavi(&state, |l| Ok(l.config.sync.enabled_games.iter().cloned().collect()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState(Mutex::new(load_ludusavi())))
        .invoke_handler(tauri::generate_handler![
            sync_push,
            sync_pull,
            sync_status,
            enabled_games
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
