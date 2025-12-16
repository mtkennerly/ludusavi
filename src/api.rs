use std::collections::{BTreeMap, BTreeSet};

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    cloud::{CloudChange, Rclone},
    prelude::{app_dir, Error},
    report,
    resource::ResourceFile,
    scan::{
        layout::BackupLayout, prepare_backup_target, scan_game_for_backup, BackupId, DuplicateDetector, Launchers,
        OperationStepDecision, ScanKind, SteamShortcuts, TitleFinder, TitleMatch,
    },
};

pub use crate::{
    path::StrictPath,
    prelude::{Finality, SyncDirection},
    report::ApiOutput,
    resource::{config::Config, manifest::Manifest},
    scan::TitleQuery,
};

/// Unlike the CLI, this always uses the config's backup path, never the restore path.
pub struct Ludusavi {
    pub config: Config,
    pub manifest: Manifest,
    layout: BackupLayout,
    title_finder: TitleFinder,
    steam_shortcuts: SteamShortcuts,
}

impl Ludusavi {
    pub fn new(config: Config, manifest: Manifest) -> Self {
        let (layout, title_finder, steam_shortcuts) = Self::make_state(&config, &manifest);

        Self {
            config,
            manifest,
            layout,
            title_finder,
            steam_shortcuts,
        }
    }

    pub fn load() -> Result<Self, Error> {
        let config = Config::load()?;
        let manifest = Manifest::load()?;

        Ok(Self::new(config, manifest))
    }

    fn make_state(config: &Config, manifest: &Manifest) -> (BackupLayout, TitleFinder, SteamShortcuts) {
        let layout = BackupLayout::new(config.backup.path.clone());

        let title_finder = TitleFinder::new(config, manifest, layout.restorable_game_set());

        let steam_shortcuts = SteamShortcuts::scan(&title_finder);

        (layout, title_finder, steam_shortcuts)
    }

    /// Update internal state after a change to config, manifest, or backups.
    pub fn refresh(&mut self) {
        let (layout, title_finder, steam_shortcuts) = Self::make_state(&self.config, &self.manifest);

        self.layout = layout;
        self.title_finder = title_finder;
        self.steam_shortcuts = steam_shortcuts;
    }

    fn target(&self) -> &StrictPath {
        &self.config.backup.path
    }

    fn sync_cloud(&self, sync: SyncDirection, finality: Finality, games: &[String]) -> Result<Vec<CloudChange>, Error> {
        match finality {
            Finality::Preview => log::info!("checking cloud sync"),
            Finality::Final => log::info!("performing cloud sync"),
        }

        let remote = crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path)?;

        let games = if !games.is_empty() {
            games.iter().filter_map(|x| self.layout.game_folder(x).leaf()).collect()
        } else {
            vec![]
        };

        let rclone = Rclone::new(self.config.apps.rclone.clone(), remote);
        let mut process = match rclone.sync(self.target(), &self.config.cloud.path, sync, finality, &games) {
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
                Some(Ok(_)) => {
                    return Ok(changes);
                }
                Some(Err(e)) => {
                    return Err(Error::UnableToSynchronizeCloud(e));
                }
                None => (),
            }
        }
    }

    /// Back up games.
    pub fn back_up(
        &mut self,
        parameters::BackUp {
            games,
            finality,
            resolve_cloud_conflict,
            wine_prefix,
            include_disabled,
            skip_downgrade,
        }: parameters::BackUp,
    ) -> Result<ApiOutput, Error> {
        let mut reporter = report::Reporter::json();

        let roots = self.config.expanded_roots();
        let backup_dir = self.target().clone();

        if !finality.preview() {
            prepare_backup_target(&backup_dir)?;
        }

        let retention = self.config.backup.retention;

        let games_specified = !games.is_empty();
        let games = evaluate_games(self.manifest.primary_titles(), games, &self.title_finder)?;

        let mut duplicate_detector = DuplicateDetector::default();
        let launchers = Launchers::scan(&roots, &self.manifest, &games, &self.title_finder, None);

        let cloud_sync = self.config.cloud.synchronize
            && !finality.preview()
            && crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path).is_ok();
        let mut should_sync_cloud_after = cloud_sync && !finality.preview();
        let mut should_sync_cloud_after_even_if_unchanged = false;
        if cloud_sync {
            let changes = self.sync_cloud(
                SyncDirection::Upload,
                Finality::Preview,
                if games_specified { &games } else { &[] },
            );
            match changes {
                Ok(changes) => {
                    if !changes.is_empty() {
                        match resolve_cloud_conflict {
                            Some(direction @ SyncDirection::Download) => {
                                // We need to download before the new backup
                                // to keep mapping.yaml in a coherent state.
                                if let Err(e) = self.sync_cloud(
                                    direction,
                                    Finality::Final,
                                    if games_specified { &games } else { &[] },
                                ) {
                                    log::error!("Failed to resolve save conflict pre-backup with direction {direction:?}: {e:?}");
                                    should_sync_cloud_after = false;
                                    reporter.trip_cloud_sync_failed();
                                }
                            }
                            Some(SyncDirection::Upload) => {
                                // We'll make the new backup first and then sync after.
                                should_sync_cloud_after_even_if_unchanged = true;
                            }
                            None => {
                                should_sync_cloud_after = false;
                                reporter.trip_cloud_conflict();
                            }
                        }
                    }
                }
                Err(_) => {
                    should_sync_cloud_after = false;
                    reporter.trip_cloud_sync_failed();
                }
            }
        }

        let step = |i, name| {
            log::trace!("step {i} / {}: {name}", games.len());
            let game = &self.manifest.0[name];

            let previous = self.layout.latest_backup(
                name,
                ScanKind::Backup,
                &self.config.redirects,
                self.config.restore.reverse_redirects,
                &self.config.restore.toggled_paths,
                self.config.backup.only_constructive,
            );

            if self
                .config
                .backup
                .filter
                .excludes(games_specified, previous.is_some(), &game.cloud)
            {
                log::trace!("[{name}] excluded by backup filter");
                return None;
            }

            let scan_info = scan_game_for_backup(
                game,
                name,
                &roots,
                &app_dir(),
                &launchers,
                &self.config.backup.filter,
                wine_prefix.as_ref(),
                &self.config.backup.toggled_paths,
                &self.config.backup.toggled_registry,
                previous.as_ref(),
                &self.config.redirects,
                self.config.restore.reverse_redirects,
                &self.steam_shortcuts,
                self.config.backup.only_constructive,
            );
            let ignored = !&self.config.is_game_enabled_for_backup(name) && !games_specified && !include_disabled;
            let decision = if ignored {
                OperationStepDecision::Ignored
            } else {
                OperationStepDecision::Processed
            };
            let backup_info = if finality.preview()
                || ignored
                || (skip_downgrade && previous.is_some_and(|x| scan_info.is_downgraded_backup(x.when)))
            {
                None
            } else {
                self.layout.game_layout(name).back_up(
                    &scan_info,
                    &chrono::Utc::now(),
                    &self.config.backup.format,
                    retention,
                    self.config.backup.only_constructive,
                )
            };
            log::trace!("step {i} completed");
            if !scan_info.can_report_game() {
                None
            } else {
                let display_title = self.config.display_name(name);
                Some((display_title, scan_info, backup_info, decision))
            }
        };

        log::info!("beginning backup with {} steps", games.len());

        let info: Vec<_> = games
            .par_iter()
            .enumerate()
            .filter_map(|(i, name)| step(i, name))
            .collect();
        log::info!("completed backup");

        if should_sync_cloud_after {
            let changed_games: Vec<_> = info
                .iter()
                .filter(|(_, scan_info, backup_info, _)| scan_info.needs_cloud_sync() && backup_info.is_some())
                .map(|(_, scan_info, _, _)| scan_info.game_name.clone())
                .collect();
            if !changed_games.is_empty() || should_sync_cloud_after_even_if_unchanged {
                let sync_result = self.sync_cloud(SyncDirection::Upload, Finality::Final, &changed_games);
                if sync_result.is_err() {
                    reporter.trip_cloud_sync_failed();
                }
            }
        }

        for (_, scan_info, _, _) in info.iter() {
            duplicate_detector.add_game(
                scan_info,
                self.config
                    .is_game_enabled_for_operation(&scan_info.game_name, ScanKind::Backup),
            );
        }

        for (name, scan_info, backup_info, decision) in info {
            reporter.add_game(
                name,
                &scan_info,
                backup_info.as_ref(),
                &decision,
                &duplicate_detector,
                false,
            );
        }

        self.refresh();
        reporter.json_output().ok_or(Error::SomeEntriesFailed)
    }

    /// Restore backups.
    pub fn restore(
        &mut self,
        parameters::Restore {
            games,
            finality,
            backup,
            resolve_cloud_conflict,
            include_disabled,
            skip_downgrade,
        }: parameters::Restore,
    ) -> Result<ApiOutput, Error> {
        let mut reporter = report::Reporter::json();

        if backup.is_some() && games.len() != 1 {
            return Err(Error::CliBackupIdWithMultipleGames);
        }
        let backup_id = backup.as_ref().map(|x| BackupId::Named(x.clone()));

        let games_specified = !games.is_empty();
        let games = evaluate_games(self.manifest.primary_titles(), games, &self.title_finder)?;

        let mut duplicate_detector = DuplicateDetector::default();

        let cloud_sync = self.config.cloud.synchronize
            && !finality.preview()
            && crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path).is_ok();
        if cloud_sync {
            let changes = self.sync_cloud(
                SyncDirection::Upload,
                Finality::Preview,
                if games_specified { &games } else { &[] },
            );
            match changes {
                Ok(changes) => {
                    if !changes.is_empty() {
                        match resolve_cloud_conflict {
                            Some(direction) => {
                                if let Err(e) = self.sync_cloud(
                                    direction,
                                    Finality::Final,
                                    if games_specified { &games } else { &[] },
                                ) {
                                    log::error!("Failed to resolve save conflict pre-restore with direction {direction:?}: {e:?}");
                                    reporter.trip_cloud_sync_failed();
                                }
                            }
                            None => {
                                reporter.trip_cloud_conflict();
                            }
                        }
                    }
                }
                Err(_) => {
                    reporter.trip_cloud_sync_failed();
                }
            }
        }

        let step = |i, name| {
            log::trace!("step {i} / {}: {name}", games.len());
            let mut layout = self.layout.game_layout(name);
            let scan_info = layout.scan_for_restoration(
                name,
                backup_id.as_ref().unwrap_or(&BackupId::Latest),
                &self.config.redirects,
                self.config.restore.reverse_redirects,
                &self.config.restore.toggled_paths,
                &self.config.restore.toggled_registry,
            );
            let ignored = !&self.config.is_game_enabled_for_restore(name) && !games_specified && !include_disabled;
            let decision = if ignored {
                OperationStepDecision::Ignored
            } else {
                OperationStepDecision::Processed
            };

            if let Some(backup) = &backup {
                if let Some(BackupId::Named(scanned_backup)) = scan_info.backup.as_ref().map(|x| x.id()) {
                    if backup != &scanned_backup {
                        log::trace!("step {i} completed (backup mismatch)");
                        let display_title = self.config.display_name(name);
                        return Some((
                            display_title,
                            scan_info,
                            Default::default(),
                            decision,
                            Some(Error::CliInvalidBackupId),
                        ));
                    }
                }
            }

            let restore_info = if scan_info.backup.is_none()
                || finality.preview()
                || ignored
                || (skip_downgrade && scan_info.is_downgraded_restore())
            {
                None
            } else {
                Some(layout.restore(&scan_info, &self.config.restore.toggled_registry))
            };
            log::trace!("step {i} completed");
            if !scan_info.can_report_game() {
                None
            } else {
                let display_title = self.config.display_name(name);
                Some((display_title, scan_info, restore_info, decision, None))
            }
        };

        log::info!("beginning restore with {} steps", games.len());

        let info: Vec<_> = games
            .par_iter()
            .enumerate()
            .filter_map(|(i, name)| step(i, name))
            .collect();
        log::info!("completed restore");

        for (_, scan_info, _, _, failure) in info.iter() {
            if let Some(failure) = failure {
                return Err(failure.clone());
            }
            duplicate_detector.add_game(
                scan_info,
                self.config
                    .is_game_enabled_for_operation(&scan_info.game_name, ScanKind::Restore),
            );
        }

        for (name, scan_info, backup_info, decision, _) in info {
            reporter.add_game(
                name,
                &scan_info,
                backup_info.as_ref(),
                &decision,
                &duplicate_detector,
                false,
            );
        }

        reporter.json_output().ok_or(Error::SomeEntriesFailed)
    }

    /// List backups.
    pub fn list_backups(&self, parameters::ListBackups { games }: parameters::ListBackups) -> Result<ApiOutput, Error> {
        let mut reporter = report::Reporter::json();
        reporter.suppress_overall();

        let games = evaluate_games(self.layout.restorable_game_set(), games, &self.title_finder)?;

        let info: Vec<_> = games
            .par_iter()
            .map(|name| {
                let mut layout = self.layout.game_layout(name);
                let backups = layout.get_backups();
                let display_title = self.config.display_name(name);
                let backup_dir = layout.path;
                (name, display_title, backup_dir, backups)
            })
            .collect();

        for (name, display_title, backup_dir, backups) in info {
            reporter.add_backups(name, display_title, backup_dir, &backups);
        }

        reporter.json_output().ok_or(Error::SomeEntriesFailed)
    }

    /// Edit a backup.
    ///
    /// These changes are not automatically synced with the cloud.
    pub fn edit_backup(
        &mut self,
        parameters::EditBackup {
            game,
            backup,
            locked,
            comment,
        }: parameters::EditBackup,
    ) -> Result<(), Error> {
        let backup = backup.map(BackupId::Named).unwrap_or(BackupId::Latest);

        let Some(game) = self.title_finder.find_one_by_name(&game) else {
            return Err(Error::GameIsUnrecognized);
        };

        let mut layout = self.layout.game_layout(&game);
        layout.validate_id(&backup)?;

        if let Some(locked) = locked {
            layout.set_backup_locked(&backup, locked);
        }
        if let Some(comment) = comment {
            layout.set_backup_comment(&backup, &comment);
        }
        layout.save();

        self.refresh();
        Ok(())
    }

    /// Look up games based on certain criteria.
    ///
    /// Only returns one result when querying for exact titles or store IDs.
    /// Precedence: Steam ID -> GOG ID -> exact title -> normalized title.
    ///
    /// Otherwise, returns all results that match the query.
    pub fn find_title(&self, query: TitleQuery) -> BTreeMap<String, TitleMatch> {
        self.title_finder.find(query)
    }
}

pub mod parameters {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct BackUp {
        /// Which game to process. Defaults to all games.
        pub games: Vec<String>,
        /// Whether to actually perform the operation or just preview the results.
        pub finality: Finality,
        /// Automatically resolve cloud conflicts by performing an upload or download.
        pub resolve_cloud_conflict: Option<SyncDirection>,
        /// Extra Wine/Proton prefix to check for saves.
        /// This should be a folder with an immediate child folder named "drive_c" (or another letter).
        pub wine_prefix: Option<StrictPath>,
        /// Process disabled games.
        pub include_disabled: bool,
        /// Skip a game when its backup is newer than the live data.
        /// Currently, this only considers file-based saves, not the Windows registry.
        ///
        /// You might want to use this if you force a backup on game exit,
        /// but you sometimes restore an older save temporarily to check something,
        /// and you don't want to accidentally back up that old save again.
        /// (If the save file gets updated during play, it will be considered newer.)
        pub skip_downgrade: bool,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Restore {
        /// Which game to process. Defaults to all games.
        pub games: Vec<String>,
        /// Whether to actually perform the operation or just preview the results.
        pub finality: Finality,
        /// Restore a specific backup, using an ID returned by the `backups` command.
        /// This is only valid when restoring a single game.
        pub backup: Option<String>,
        /// Automatically resolve cloud conflicts by performing an upload or download.
        pub resolve_cloud_conflict: Option<SyncDirection>,
        /// Process disabled games.
        pub include_disabled: bool,
        /// Skip a game when its backup is newer than the live data.
        /// Currently, this only considers file-based saves, not the Windows registry.
        ///
        /// You might want to use this if you force a backup on game exit,
        /// but you sometimes restore an older save temporarily to check something,
        /// and you don't want to accidentally back up that old save again.
        /// (If the save file gets updated during play, it will be considered newer.)
        pub skip_downgrade: bool,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ListBackups {
        /// Which game to list. Defaults to all games.
        pub games: Vec<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct EditBackup {
        /// Which game to edit.
        pub game: String,
        /// Edit a specific backup, using an ID returned by the `backups` command.
        /// When not specified, this defaults to the latest backup.
        pub backup: Option<String>,
        pub locked: Option<bool>,
        pub comment: Option<String>,
    }
}

fn evaluate_games(
    default: BTreeSet<String>,
    requested: Vec<String>,
    title_finder: &TitleFinder,
) -> Result<Vec<String>, Error> {
    if requested.is_empty() {
        return Ok(default.into_iter().collect());
    }

    let mut valid = BTreeSet::new();
    let mut invalid = BTreeSet::new();

    for game in requested {
        match title_finder.find_one_by_name(&game) {
            Some(found) => {
                valid.insert(found);
            }
            None => {
                invalid.insert(game);
            }
        }
    }

    if !invalid.is_empty() {
        return Err(Error::CliUnrecognizedGames {
            games: invalid.into_iter().collect(),
        });
    }

    Ok(valid.into_iter().collect())
}
