use crate::{
    cloud::{self, CloudChange, Rclone},
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

/// High-level push operation for a single game.
/// 1. Validates cloud config
/// 2. Locates the game's backup folder
/// 3. Uploads via rclone copy (additive - no deletes)
/// 4. Updates settings.config
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
    let mut process = match rclone.copy(backup_dir, cloud_path, SyncDirection::Upload, finality, &[game_dir_name]) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let mut changes = vec![];
    loop {
        let events = process.events();
        for event in events {
            match event {
                crate::cloud::RcloneProcessEvent::Progress { .. } => {}
                crate::cloud::RcloneProcessEvent::Change(change) => {
                    changes.push(change);
                }
            }
        }
        match process.succeeded() {
            Some(Ok(_)) => break,
            Some(Err(e)) => return Err(Error::UnableToSynchronizeCloud(e)),
            None => {}
        }
    }

    // Update settings.config if this was a real push (not preview)
    if !finality.preview() {
        let mut sync_state = SyncStateFile::load_from(backup_dir);
        sync_state.merge_game(game_name, SyncStateFile::create_entry(game_name));
        if let Err(e) = sync_state.save_to(backup_dir) {
            log::error!("Failed to save sync state: {:?}", e);
        }
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
/// 2. Downloads the game's backup folder via rclone copy (additive)
/// 3. The local BackupLayout now has both old and new backups
/// 4. Returns the latest backup info for the game
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
    let mut process = match rclone.copy(backup_dir, cloud_path, SyncDirection::Download, finality, &[game_dir_name]) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let mut changes = vec![];
    loop {
        let events = process.events();
        for event in events {
            match event {
                crate::cloud::RcloneProcessEvent::Progress { .. } => {}
                crate::cloud::RcloneProcessEvent::Change(change) => {
                    changes.push(change);
                }
            }
        }
        match process.succeeded() {
            Some(Ok(_)) => break,
            Some(Err(e)) => return Err(Error::UnableToSynchronizeCloud(e)),
            None => {}
        }
    }

    // Read settings.config from cloud (if we downloaded it)
    // and merge with local state
    if !finality.preview() {
        let cloud_sync_state = SyncStateFile::load_from(backup_dir);
        if cloud_sync_state.has_game(game_name) {
            log::info!(
                "Cloud has game {} last pushed by {} at {}",
                game_name,
                cloud_sync_state.game_info(game_name).unwrap().device,
                cloud_sync_state.game_info(game_name).unwrap().last_push
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
