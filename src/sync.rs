use std::time::Duration;

use crate::{
    cloud::{self, CloudChange, Rclone, RcloneProcessEvent},
    prelude::{Error, Finality, StrictPath, SyncDirection},
    resource::{
        config::Config,
        sync_state::{self, GameSyncEntry, SyncStateFile},
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
fn upload_sync_state(rclone: &Rclone, backup_dir: &StrictPath, cloud_path: &str, finality: Finality) {
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

/// Scratch directory used to stage the cloud's settings.config before merging it.
/// Lives under the backup root so it lands on the same filesystem.
const CLOUD_STATE_STAGING_DIR: &str = ".ludusavi-cloud-state";

/// Fetch the cloud's settings.config without touching the local one.
///
/// Returns `None` if the transfer failed or the cloud has no state file yet.
fn fetch_cloud_sync_state(
    rclone: &Rclone,
    backup_dir: &StrictPath,
    cloud_path: &str,
    finality: Finality,
) -> Option<SyncStateFile> {
    if finality.preview() {
        return None;
    }

    let staging = backup_dir.joined(CLOUD_STATE_STAGING_DIR);
    let _ = staging.remove();
    if let Err(e) = staging.create_dirs() {
        log::error!("Failed to create staging dir for settings.config: {:?}", e);
        return None;
    }

    let result = (|| {
        let mut process = rclone
            .copy(
                &staging,
                cloud_path,
                SyncDirection::Download,
                finality,
                &[SyncStateFile::FILE_NAME.to_string()],
            )
            .map_err(|e| format!("failed to start download: {e:?}"))?;
        wait_for_rclone(&mut process).map_err(|e| format!("download failed: {e:?}"))?;

        if !staging.joined(SyncStateFile::FILE_NAME).is_file() {
            // Cloud has no state file yet; nothing to merge.
            return Ok(None);
        }
        Ok::<_, String>(Some(SyncStateFile::load_from(&staging)))
    })();

    let _ = staging.remove();

    match result {
        Ok(state) => state,
        Err(e) => {
            log::error!("Unable to fetch cloud {}: {}", SyncStateFile::FILE_NAME, e);
            None
        }
    }
}

/// Merge the cloud's settings.config into the local one and save.
///
/// This must never be a plain overwrite: each device is the only authority for its own
/// entry in [`GameSyncEntry::prefixes`], and games only known locally would otherwise be
/// dropped on every pull.
fn merge_cloud_sync_state(
    rclone: &Rclone,
    backup_dir: &StrictPath,
    cloud_path: &str,
    finality: Finality,
) -> SyncStateFile {
    let mut local = SyncStateFile::load_from(backup_dir);

    if let Some(cloud) = fetch_cloud_sync_state(rclone, backup_dir, cloud_path, finality) {
        local.merge_from(&cloud);
        if let Err(e) = local.save_to(backup_dir) {
            log::error!("Failed to save merged sync state: {:?}", e);
        }
    }

    local
}

/// The Wine/Proton prefix this machine's latest backup of a game came from.
///
/// Recorded by the backup scan in the mapping's `semantics`. Returns `None` for games
/// that aren't inside a prefix (native builds, Windows hosts), which is the common case.
fn local_wine_prefix(layout: &BackupLayout, game_name: &str) -> Option<String> {
    let game_layout = layout.game_layout(game_name);
    let (full, diff) = game_layout.find_by_id(&crate::scan::BackupId::Latest)?;
    let semantics = crate::scan::layout::BackupSemantics::merged(&full.semantics, diff.map(|d| &d.semantics));

    let mut prefixes = semantics.wine_prefixes();
    if prefixes.len() > 1 {
        // Ambiguous; recording an arbitrary one would send another device to the wrong
        // place. Better to record nothing and let it fall back to local discovery.
        log::warn!("Game {game_name} has multiple Wine prefixes; not recording one for sync");
        return None;
    }
    prefixes.pop()
}

/// High-level push operation for a single game.
/// 1. Validates cloud config
/// 2. Locates the game's backup folder
/// 3. Uploads via rclone copy (additive - no deletes)
/// 4. Merges + uploads settings.config, recording this device's Wine prefix
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
    let game_dir_name = game_folder.leaf().ok_or(Error::GameIsUnrecognized)?;

    let rclone = Rclone::new(config.apps.rclone.clone(), remote);

    // Upload game files
    let mut process = match rclone.copy(
        backup_dir,
        cloud_path,
        SyncDirection::Upload,
        finality,
        &[game_dir_name],
    ) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let changes = wait_for_rclone(&mut process)?;

    // Update local settings.config and upload to cloud.
    if !finality.preview() {
        // Merge the cloud's state in first, so pushing can't drop entries (or other
        // devices' recorded prefixes) that this device has never seen.
        let mut sync_state = merge_cloud_sync_state(&rclone, backup_dir, cloud_path, finality);

        sync_state.merge_game(game_name, SyncStateFile::push_entry(game_name));

        // Record where this game's saves live on this machine, so a device pulling this
        // backup knows where to put them. Read from the backup we just made rather than
        // rescanning: the prefix is whatever the scan already resolved.
        if let Some(prefix) = local_wine_prefix(&layout, game_name) {
            sync_state.set_prefix(game_name, &sync_state::current_device(), &prefix);
        }

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
    let game_dir_name = game_folder.leaf().ok_or(Error::GameIsUnrecognized)?;

    let rclone = Rclone::new(config.apps.rclone.clone(), remote);

    // Merge the cloud's settings.config into the local one first, so the game's
    // per-device prefix registry is available to the restore that follows.
    let sync_state = merge_cloud_sync_state(&rclone, backup_dir, cloud_path, finality);

    if !finality.preview() && !config.scan.redirect_wine {
        log::warn!(
            "scan.redirect_wine is disabled: restoring {game_name} will write to the \
             source device's literal paths instead of this machine's Wine prefix"
        );
    }

    // Download game files
    let mut process = match rclone.copy(
        backup_dir,
        cloud_path,
        SyncDirection::Download,
        finality,
        &[game_dir_name],
    ) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let changes = wait_for_rclone(&mut process)?;

    if !finality.preview()
        && let Some(info) = sync_state.game_info(game_name)
    {
        log::info!(
            "Cloud has game {} last pushed by {} at {}",
            game_name,
            info.device,
            info.last_push
        );

        let device = sync_state::current_device();
        match info.prefixes.get(&device) {
            Some(prefix) => log::info!("Will restore {game_name} into this device's prefix: {prefix}"),
            None if !info.prefixes.is_empty() => log::info!(
                "No Wine prefix recorded for this device ({device}); \
                 restore will fall back to detecting one locally"
            ),
            None => {}
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
pub fn get_game_sync_info(backup_dir: &StrictPath, game_name: &str) -> Option<GameSyncEntry> {
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
