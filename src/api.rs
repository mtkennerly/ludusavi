use std::collections::{BTreeMap, BTreeSet};

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    cloud::{CloudChange, Rclone, Remote},
    prelude::{Cancel, Error, app_dir},
    report,
    resource::{SaveableResourceFile, sync_state::SyncStateFile},
    scan::{
        BackupId, DuplicateDetector, Launchers, OperationStepDecision, ScanKind, SteamShortcuts, TitleFinder,
        TitleMatch,
        layout::{BackupLayout, BackupSemantics},
        prepare_backup_target, scan_game_for_backup, semantic,
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
            cancel,
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
                                    log::error!(
                                        "Failed to resolve save conflict pre-backup with direction {direction:?}: {e:?}"
                                    );
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

        let sync_state = SyncStateFile::load_from(&backup_dir);
        let wine_env = semantic::WineEnvironment {
            config: &self.config,
            manifest: &self.manifest,
            roots: &roots,
            steam_shortcuts: &self.steam_shortcuts,
            launchers: Some(&launchers),
            cli_prefix: wine_prefix.as_ref(),
            registry: Some(&sync_state),
        };

        let step = |i, name| {
            log::trace!("step {i} / {}: {name}", games.len());
            let game = &self.manifest.0[name];

            let wine_ctx = semantic::Wine::for_game(name, &wine_env);
            let previous = self.layout.latest_backup(
                name,
                ScanKind::Backup,
                &self.config.redirects,
                self.config.restore.reverse_redirects,
                &self.config.restore.toggled_paths,
                self.config.backup.only_constructive,
                wine_ctx.as_ref(),
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
            .filter_map(|(i, name)| {
                // Cooperative cancellation: once flagged, stop scheduling further
                // per-game work. In-flight steps finish, then the scan unwinds.
                if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
                    None
                } else {
                    step(i, name)
                }
            })
            .collect();
        log::info!("completed backup");

        if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            log::info!("backup cancelled by request");
            return Ok(reporter.json_output().unwrap_or_default());
        }

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

        for (name, scan_info, backup_info, decision) in &info {
            reporter.add_game(
                name,
                scan_info,
                backup_info.as_ref(),
                decision,
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
                                    log::error!(
                                        "Failed to resolve save conflict pre-restore with direction {direction:?}: {e:?}"
                                    );
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

        // Restore needs to know where this machine's Wine prefixes are, which backup
        // never had to ask about. Scanning roots is only worth it when remapping is on.
        let roots = if self.config.scan.redirect_wine {
            self.config.expanded_roots()
        } else {
            vec![]
        };
        let sync_state = SyncStateFile::load_from(&self.config.backup.path);
        let wine_env = semantic::WineEnvironment {
            config: &self.config,
            manifest: &self.manifest,
            roots: &roots,
            steam_shortcuts: &self.steam_shortcuts,
            launchers: None,
            cli_prefix: None,
            registry: Some(&sync_state),
        };

        let step = |i, name| {
            log::trace!("step {i} / {}: {name}", games.len());
            let mut layout = self.layout.game_layout(name);
            let id = backup_id.as_ref().unwrap_or(&BackupId::Latest);

            let wine_ctx = semantic::Wine::for_game(name, &wine_env);

            // Refuse rather than silently write to the source device's literal path
            // (e.g. `/home/deck/...` on a PC): the backup demonstrably needs a Wine
            // prefix, and this machine doesn't have one for it.
            if let Some((full, diff)) = layout.find_by_id(id) {
                let backup_semantics = BackupSemantics::merged(&full.semantics, diff.map(|d| &d.semantics));
                if let Some(backup_prefix) = backup_semantics.wine_prefixes().into_iter().next()
                    && wine_ctx.as_ref().is_none_or(|w| w.prefixes.is_empty())
                {
                    log::error!("[{name}] no local Wine prefix found; backup recorded {backup_prefix}");
                    let display_title = self.config.display_name(name);
                    return Some((
                        display_title,
                        crate::scan::ScanInfo {
                            game_name: name.to_string(),
                            ..Default::default()
                        },
                        Default::default(),
                        OperationStepDecision::Ignored,
                        Some(Error::WinePrefixNotFound {
                            game: name.to_string(),
                            backup_prefix,
                        }),
                    ));
                }
            }

            let scan_info = layout.scan_for_restoration(
                name,
                id,
                &self.config.redirects,
                self.config.restore.reverse_redirects,
                &self.config.restore.toggled_paths,
                &self.config.restore.toggled_registry,
                wine_ctx.as_ref(),
            );

            let ignored = !&self.config.is_game_enabled_for_restore(name) && !games_specified && !include_disabled;
            let decision = if ignored {
                OperationStepDecision::Ignored
            } else {
                OperationStepDecision::Processed
            };

            if let Some(backup) = &backup
                && let Some(BackupId::Named(scanned_backup)) = scan_info.backup.as_ref().map(|x| x.id())
                && backup != &scanned_backup
            {
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

    /// Case-insensitive substring search over every game Ludusavi recognizes (primary
    /// manifest/custom-game titles, not aliases), for a game-picker search box.
    /// Capped at `limit` results; empty query matches everything up to that cap.
    pub fn search_games(&self, query: &str, limit: usize) -> Vec<String> {
        let query = query.to_lowercase();
        let mut matches: Vec<String> = self
            .manifest
            .primary_titles()
            .into_iter()
            .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
            .collect();
        matches.truncate(limit);
        matches
    }

    /// Enable or disable a game for cloud sync (`config.yaml`'s `sync.enabled_games`).
    pub fn set_game_enabled(&mut self, game: &str, enabled: bool) {
        if enabled {
            self.config.sync.enabled_games.insert(game.to_string());
        } else {
            self.config.sync.enabled_games.remove(game);
        }
        self.config.save();
    }

    /// Games found by a recent full scan (`config.yaml`'s `sync.discovered_games`).
    /// Persisted so a restart doesn't require re-scanning.
    pub fn discovered_games(&self) -> Vec<String> {
        self.config.sync.discovered_games.iter().cloned().collect()
    }

    /// Replace the persisted set of scan-discovered games.
    ///
    /// A successful full-library scan is authoritative for what's installed right now,
    /// so this replaces rather than unions - uninstalled games get pruned naturally.
    pub fn set_discovered_games(&mut self, names: impl IntoIterator<Item = String>) {
        self.config.sync.discovered_games = names.into_iter().collect();
        self.config.save();
    }

    /// Push a single game's local backup to the cloud.
    /// Additive on the destination - never deletes another game's cloud data.
    pub fn sync_push(
        &self,
        game: &str,
        finality: Finality,
        on_progress: Option<&mut dyn FnMut(crate::sync::SyncProgress)>,
    ) -> Result<crate::sync::SyncResult, Error> {
        let Some(game) = self.title_finder.find_one_by_name(game) else {
            return Err(Error::GameIsUnrecognized);
        };
        crate::sync::push_game(
            &self.config,
            &self.config.backup.path,
            &self.config.cloud.path,
            &game,
            finality,
            on_progress,
        )
    }

    /// Pull a single game's backup from the cloud.
    /// Additive on the destination - never deletes local data for another game.
    pub fn sync_pull(
        &self,
        game: &str,
        finality: Finality,
        on_progress: Option<&mut dyn FnMut(crate::sync::SyncProgress)>,
    ) -> Result<crate::sync::SyncResult, Error> {
        let Some(game) = self.title_finder.find_one_by_name(game) else {
            return Err(Error::GameIsUnrecognized);
        };
        crate::sync::pull_game(
            &self.config,
            &self.config.backup.path,
            &self.config.cloud.path,
            &game,
            finality,
            on_progress,
        )
    }

    /// Get the last-known cloud sync info for a game, from `settings.config`.
    pub fn sync_status(&self, game: &str) -> Option<crate::resource::sync_state::GameSyncEntry> {
        let game = self.title_finder.find_one_by_name(game)?;
        crate::sync::get_game_sync_info(&self.config.backup.path, &game)
    }

    /// Local Wine/Proton prefixes found for a game on this machine, best first.
    /// Empty on Windows, when `scan.redirect_wine` is off, or when the game isn't Wine/Proton.
    pub fn wine_prefixes_for(&self, game: &str) -> Vec<String> {
        let Some(game) = self.title_finder.find_one_by_name(game) else {
            return vec![];
        };
        if !self.config.scan.redirect_wine {
            return vec![];
        }

        let roots = self.config.expanded_roots();
        let sync_state = SyncStateFile::load_from(&self.config.backup.path);
        let wine_env = semantic::WineEnvironment {
            config: &self.config,
            manifest: &self.manifest,
            roots: &roots,
            steam_shortcuts: &self.steam_shortcuts,
            launchers: None,
            cli_prefix: None,
            registry: Some(&sync_state),
        };
        wine_env
            .prefixes_for_game(&game)
            .into_iter()
            .map(|p| p.path.render())
            .collect()
    }

    /// Wine/Proton prefixes recorded in a game's latest backup.
    ///
    /// Non-empty here with [`Self::wine_prefixes_for`] empty means a restore of this
    /// game will fail (or, with `scan.redirect_wine` off, silently write to a foreign
    /// path) - useful for a UI to warn about before the user tries.
    pub fn backup_wine_prefixes(&self, game: &str) -> Vec<String> {
        let Some(game) = self.title_finder.find_one_by_name(game) else {
            return vec![];
        };
        let layout = self.layout.game_layout(&game);
        match layout.find_by_id(&BackupId::Latest) {
            Some((full, diff)) => BackupSemantics::merged(&full.semantics, diff.map(|d| &d.semantics)).wine_prefixes(),
            None => vec![],
        }
    }

    /// The full device -> prefix registry for a game, from `settings.config`.
    /// Lets a UI show e.g. "Deck: .../4110821628/pfx, PC: .../2811670038/pfx".
    pub fn registered_prefixes(&self, game: &str) -> BTreeMap<String, String> {
        let Some(game) = self.title_finder.find_one_by_name(game) else {
            return BTreeMap::new();
        };
        let sync_state = SyncStateFile::load_from(&self.config.backup.path);
        sync_state
            .games
            .get(&game)
            .map(|entry| entry.prefixes.clone())
            .unwrap_or_default()
    }

    /// Current cloud configuration, for a settings UI to display.
    pub fn cloud_status(&self) -> CloudStatus {
        CloudStatus {
            connected: self.config.cloud.remote.is_some(),
            remote_kind: self.config.cloud.remote.as_ref().map(|r| r.slug().to_string()),
            path: self.config.cloud.path.clone(),
            synchronize: self.config.cloud.synchronize,
            rclone_path: self.config.apps.rclone.path.render(),
            rclone_valid: self.config.apps.rclone.is_valid(),
        }
    }

    /// Configure Google Drive as the cloud remote.
    ///
    /// This runs `rclone config create`, which for Google Drive drives rclone's own OAuth
    /// flow (it opens a browser and waits for the user to approve access). That's a
    /// blocking, possibly slow, network+UI operation - call this from a background
    /// thread, not directly on a UI event loop.
    pub fn set_cloud_remote_google_drive(&mut self) -> Result<(), Error> {
        self.configure_cloud(Remote::GoogleDrive {
            id: Remote::generate_id(),
        })
    }

    /// Remove the configured cloud remote, both from `rclone`'s own config and here.
    pub fn disconnect_cloud_remote(&mut self) -> Result<(), Error> {
        if let Some(old) = self.config.cloud.remote.take() {
            let _ = Rclone::new(self.config.apps.rclone.clone(), old).unconfigure_remote();
            self.config.save();
        }
        Ok(())
    }

    /// The cloud-side folder to sync into (sibling to the local backup root's contents).
    pub fn set_cloud_path(&mut self, path: String) {
        self.config.cloud.path = path;
        self.config.save();
    }

    /// Whether to auto-upload after every backup (upstream's bulk-sync feature). Separate
    /// from the fork's own `sync_push`/`sync_pull`, which are per-game and manual.
    pub fn set_cloud_synchronize(&mut self, enabled: bool) {
        self.config.cloud.synchronize = enabled;
        self.config.save();
    }

    /// Swap in a new remote, tearing down the old one first. Shared by every
    /// `set_cloud_remote_*` method; mirrors `cli.rs`'s `configure_cloud`.
    fn configure_cloud(&mut self, remote: Remote) -> Result<(), Error> {
        if let Some(old_remote) = self.config.cloud.remote.as_ref() {
            let _ = Rclone::new(self.config.apps.rclone.clone(), old_remote.clone()).unconfigure_remote();
        }

        Rclone::new(self.config.apps.rclone.clone(), remote.clone())
            .configure_remote()
            .map_err(Error::UnableToConfigureCloud)?;

        self.config.cloud.remote = Some(remote);
        self.config.save();
        Ok(())
    }
}

/// Cloud configuration snapshot for a settings UI.
#[derive(Clone, Debug, serde::Serialize)]
pub struct CloudStatus {
    pub connected: bool,
    /// e.g. `"google drive"`, `"dropbox"` - `None` when nothing is configured.
    pub remote_kind: Option<String>,
    /// Cloud-side folder name (sibling to the local backup root's contents).
    pub path: String,
    pub synchronize: bool,
    pub rclone_path: String,
    pub rclone_valid: bool,
}

pub mod parameters {
    use super::*;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
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
        /// Cooperative cancellation token checked between per-game steps.
        /// `None` (the default) runs the operation to completion.
        pub cancel: Option<Cancel>,
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
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

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct ListBackups {
        /// Which game to list. Defaults to all games.
        pub games: Vec<String>,
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
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
