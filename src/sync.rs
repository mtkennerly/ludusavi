use std::time::Duration;

use crate::{
    cloud::{self, CloudChange, Rclone, RcloneProcessEvent},
    prelude::{Error, Finality, StrictPath, SyncDirection},
    resource::{
        config::Config,
        sync_state::{GameSyncEntry, SyncStateFile},
    },
    scan::layout::BackupLayout,
};

/// Result of a push or pull operation for a single game.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub game: String,
    pub success: bool,
    pub changes: Vec<CloudChange>,
    pub error: Option<Error>,
}

/// Wait for an rclone process to complete, collecting changes.
/// Includes a small sleep to avoid CPU spin.
fn wait_for_rclone(process: &mut crate::cloud::RcloneProcess) -> Result<Vec<CloudChange>, Error> {
    let mut changes = vec![];
    loop {
        let events = process.events();
        for event in events {
            match event {
                RcloneProcessEvent::Progress { .. } => {}
                RcloneProcessEvent::Change(change) => {
                    changes.push(change);
                }
            }
        }
        match process.succeeded() {
            Some(Ok(_)) => return Ok(changes),
            Some(Err(e)) => return Err(Error::UnableToSynchronizeCloud(e)),
            None => {}
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Upload settings.config to the cloud after a push.
fn upload_sync_state(
    rclone: &Rclone,
    backup_dir: &StrictPath,
    cloud_path: &str,
    finality: Finality,
) {
    if finality.preview() {
        return;
    }
    let mut process = match rclone.copy(
        backup_dir,
        cloud_path,
        SyncDirection::Upload,
        finality,
        &["settings.config".to_string()],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to start upload of settings.config: {:?}", e);
            return;
        }
    };
    if let Err(e) = wait_for_rclone(&mut process) {
        log::error!("Failed to upload settings.config: {:?}", e);
    }
}

/// Download settings.config from the cloud before a pull.
fn download_sync_state(
    rclone: &Rclone,
    backup_dir: &StrictPath,
    cloud_path: &str,
    finality: Finality,
) {
    if finality.preview() {
        return;
    }
    let mut process = match rclone.copy(
        backup_dir,
        cloud_path,
        SyncDirection::Download,
        finality,
        &["settings.config".to_string()],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to start download of settings.config: {:?}", e);
            return;
        }
    };
    if let Err(e) = wait_for_rclone(&mut process) {
        log::error!("Failed to download settings.config: {:?}", e);
    }
}

/// High-level push operation for a single game.
/// 1. Validates cloud config
/// 2. Locates the game's backup folder
/// 3. Uploads via rclone copy (additive - no deletes)
/// 4. Uploads settings.config to cloud
pub fn push_game(
    config: &Config,
    backup_dir: &StrictPath,
    cloud_path: &str,
    game_name: &str,
    finality: Finality,
) -> Result<SyncResult, Error> {
    log::info!("pushing game: {}", game_name);

    let remote = cloud::validate_cloud_config(config, cloud_path)?;
    let layout = BackupLayout::new(backup_dir.clone());

    let game_folder = layout.game_folder(game_name);
    let game_dir_name = game_folder
        .leaf()
        .ok_or(Error::GameIsUnrecognized)?;

    let rclone = Rclone::new(config.apps.rclone.clone(), remote);

    // Upload game files
    let mut process = match rclone.copy(backup_dir, cloud_path, SyncDirection::Upload, finality, &[game_dir_name]) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let changes = wait_for_rclone(&mut process)?;

    // Update local settings.config and upload to cloud
    if !finality.preview() {
        let mut sync_state = SyncStateFile::load_from(backup_dir);
        sync_state.merge_game(game_name, SyncStateFile::push_entry(game_name));
        if let Err(e) = sync_state.save_to(backup_dir) {
            log::error!("Failed to save sync state locally: {:?}", e);
        }
        upload_sync_state(&rclone, backup_dir, cloud_path, finality);
    }

    Ok(SyncResult {
        game: game_name.to_string(),
        success: true,
        changes,
        error: None,
    })
}

/// High-level pull operation for a single game.
/// 1. Validates cloud config
/// 2. Downloads settings.config from cloud
/// 3. Downloads the game's backup folder via rclone copy (additive)
/// 4. Merges cloud metadata into local state
pub fn pull_game(
    config: &Config,
    backup_dir: &StrictPath,
    cloud_path: &str,
    game_name: &str,
    finality: Finality,
) -> Result<SyncResult, Error> {
    log::info!("pulling game: {}", game_name);

    let remote = cloud::validate_cloud_config(config, cloud_path)?;
    let layout = BackupLayout::new(backup_dir.clone());

    let game_folder = layout.game_folder(game_name);
    let game_dir_name = game_folder
        .leaf()
        .ok_or(Error::GameIsUnrecognized)?;

    let rclone = Rclone::new(config.apps.rclone.clone(), remote);

    // Download settings.config from cloud first
    download_sync_state(&rclone, backup_dir, cloud_path, finality);

    // Download game files
    let mut process = match rclone.copy(backup_dir, cloud_path, SyncDirection::Download, finality, &[game_dir_name]) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let changes = wait_for_rclone(&mut process)?;

    // Merge cloud metadata into local state
    if !finality.preview() {
        let local_state = SyncStateFile::load_from(backup_dir);
        if let Some(info) = local_state.game_info(game_name).cloned() {
            log::info!(
                "Cloud has game {} last pushed by {} at {}",
                game_name,
                info.device,
                info.last_push
            );
        }
    }

    Ok(SyncResult {
        game: game_name.to_string(),
        success: true,
        changes,
        error: None,
    })
}

/// Get sync info for a game from settings.config.
pub fn get_game_sync_info(
    backup_dir: &StrictPath,
    game_name: &str,
) -> Option<GameSyncEntry> {
    let state = SyncStateFile::load_from(backup_dir);
    state.game_info(game_name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_result_creation() {
        let result = SyncResult {
            game: "Test Game".to_string(),
            success: true,
            changes: vec![],
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.game, "Test Game");
    }
}
