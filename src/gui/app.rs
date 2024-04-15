use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use iced::{keyboard, widget::scrollable, Alignment, Application, Command, Subscription};

use crate::{
    cloud::{rclone_monitor, Rclone, Remote},
    gui::{
        button,
        common::*,
        modal::{CloudModalState, Modal, ModalField, ModalInputKind},
        notification::Notification,
        screen,
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{id, Column, Container, Element, IcedParentExt, Progress, Row},
    },
    lang::TRANSLATOR,
    prelude::{app_dir, get_threads_from_env, initialize_rayon, Error, Finality, StrictPath, SyncDirection},
    resource::{
        cache::Cache,
        config::{Config, CustomGame, CustomGameKind, RootsConfig},
        manifest::{Manifest, Store},
        ResourceFile, SaveableResourceFile,
    },
    scan::{
        layout::BackupLayout, prepare_backup_target, registry_compat::RegistryItem, scan_game_for_backup, BackupId,
        Launchers, OperationStepDecision, SteamShortcuts, TitleFinder,
    },
};

pub struct Executor(tokio::runtime::Runtime);

impl iced::Executor for Executor {
    fn new() -> Result<Self, iced::futures::io::Error> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.enable_all();

        if let Some(threads) = get_threads_from_env().or_else(|| Config::load().ok().and_then(|x| x.runtime.threads)) {
            initialize_rayon(threads);
            builder.worker_threads(threads.get());
        }

        builder.build().map(Self)
    }

    #[allow(clippy::let_underscore_future)]
    fn spawn(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        let _ = tokio::runtime::Runtime::spawn(&self.0, future);
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = tokio::runtime::Runtime::enter(&self.0);
        f()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SaveKind {
    Config,
    Cache,
    Backup(String),
}

#[derive(Default)]
pub struct App {
    config: Config,
    manifest: Manifest,
    cache: Cache,
    operation: Operation,
    screen: Screen,
    modal: Option<Modal>,
    backup_screen: screen::Backup,
    restore_screen: screen::Restore,
    operation_should_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    operation_steps: Vec<Command<Message>>,
    operation_steps_active: usize,
    progress: Progress,
    backups_to_restore: HashMap<String, BackupId>,
    updating_manifest: bool,
    notify_on_single_game_scanned: Option<(String, Screen)>,
    timed_notification: Option<Notification>,
    scroll_offsets: HashMap<ScrollSubject, scrollable::AbsoluteOffset>,
    text_histories: TextHistories,
    rclone_monitor_sender: Option<iced::futures::channel::mpsc::Sender<rclone_monitor::Input>>,
    exiting: bool,
    pending_save: HashMap<SaveKind, Instant>,
    modifiers: keyboard::Modifiers,
}

impl App {
    fn go_idle(&mut self) {
        if self.exiting {
            self.save();
            std::process::exit(0);
        }

        self.operation = Operation::Idle;
        self.operation_steps.clear();
        self.operation_steps_active = 0;
        self.modal = None;
        self.progress.reset();
        self.operation_should_cancel
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        self.notify_on_single_game_scanned = None;
    }

    fn show_modal(&mut self, modal: Modal) -> Command<Message> {
        self.modal = Some(modal);
        self.reset_scroll_position(ScrollSubject::Modal);
        self.refresh_scroll_position()
    }

    fn close_modal(&mut self) -> Command<Message> {
        if self.modal.is_some() {
            self.reset_scroll_position(ScrollSubject::Modal);
            let need_cancel_cloud = self.modal.as_ref().map(|x| x.is_cloud_active()).unwrap_or_default();
            self.modal = None;
            Command::batch([
                self.refresh_scroll_position(),
                if need_cancel_cloud {
                    self.cancel_operation()
                } else {
                    Command::none()
                },
            ])
        } else {
            Command::none()
        }
    }

    fn close_specific_modal(&mut self, modal: Modal) -> Command<Message> {
        if self.modal == Some(modal) {
            self.close_modal()
        } else {
            Command::none()
        }
    }

    fn show_error(&mut self, error: Error) -> Command<Message> {
        self.show_modal(Modal::Error { variant: error })
    }

    fn save(&mut self) {
        let threshold = Duration::from_secs(1);
        let now = Instant::now();

        self.pending_save.retain(|item, then| {
            if (now - *then) < threshold {
                return true;
            }

            match item {
                SaveKind::Config => self.config.save(),
                SaveKind::Cache => self.cache.save(),
                SaveKind::Backup(game) => self.restore_screen.log.save_layout(game),
            }

            false
        });
    }

    fn save_config(&mut self) {
        self.pending_save.insert(SaveKind::Config, Instant::now());
    }

    fn save_cache(&mut self) {
        self.pending_save.insert(SaveKind::Cache, Instant::now());
    }

    fn save_backup(&mut self, game: &str) {
        self.pending_save
            .insert(SaveKind::Backup(game.to_string()), Instant::now());
    }

    fn invalidate_path_caches(&self) {
        for x in &self.config.roots {
            x.path.invalidate_cache();
        }
        for x in &self.config.redirects {
            x.source.invalidate_cache();
            x.target.invalidate_cache();
        }
        self.config.backup.path.invalidate_cache();
        self.config.restore.path.invalidate_cache();
        self.config.backup.toggled_paths.invalidate_path_caches();
    }

    fn register_notify_on_single_game_scanned(&mut self) {
        if let Some(games) = &self.operation.games() {
            if games.len() == 1 {
                self.notify_on_single_game_scanned = Some((games[0].clone(), self.screen));
            }
        }
    }

    fn handle_notify_on_single_game_scanned(&mut self) {
        if let Some((name, screen)) = self.notify_on_single_game_scanned.as_ref() {
            let log = match self.operation {
                Operation::Backup { .. } => &self.backup_screen.log,
                Operation::Restore { .. } => &self.restore_screen.log,
                _ => return,
            };
            let found = log.entries.iter().any(|x| &x.scan_info.game_name == name);

            if *screen != Screen::CustomGames && found {
                return;
            }

            let msg = TRANSLATOR.notify_single_game_status(found);
            self.timed_notification = Some(Notification::new(msg).expires(3));
        }
    }

    fn start_sync_cloud(
        &mut self,
        local: &StrictPath,
        direction: SyncDirection,
        finality: Finality,
        games: Option<&Vec<String>>,
        standalone: bool,
    ) -> Result<(), Error> {
        let remote = crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path)?;

        let games = match games {
            Some(games) => {
                let layout = BackupLayout::new(local.clone(), self.config.backup.retention.clone());
                let games: Vec<_> = games.iter().filter_map(|x| layout.game_folder(x).leaf()).collect();
                games
            }
            None => vec![],
        };

        let rclone = Rclone::new(self.config.apps.rclone.clone(), remote);
        match rclone.sync(local, &self.config.cloud.path, direction, finality, &games) {
            Ok(process) => {
                if let Some(sender) = self.rclone_monitor_sender.as_mut() {
                    if standalone {
                        self.operation = Operation::new_cloud(direction, finality);
                    } else {
                        self.operation.update_integrated_cloud(finality);
                    }
                    self.progress.start();
                    let _ = sender.try_send(rclone_monitor::Input::Process(process));
                }
            }
            Err(e) => {
                return Err(Error::UnableToSynchronizeCloud(e));
            }
        }

        Ok(())
    }

    fn handle_backup(&mut self, phase: BackupPhase) -> Command<Message> {
        match phase {
            BackupPhase::Confirm { games } => self.show_modal(Modal::ConfirmBackup { games }),
            BackupPhase::Start { preview, repair, games } => {
                if !self.operation.idle() {
                    return Command::none();
                }

                let mut cleared_log = false;
                if games.is_none() {
                    self.backup_screen.log.clear();
                    self.backup_screen.duplicate_detector.clear();
                    self.reset_scroll_position(ScrollSubject::Backup);
                    cleared_log = true;
                }

                self.operation =
                    Operation::new_backup(if preview { Finality::Preview } else { Finality::Final }, games);
                self.operation.set_force_new_full_backups(repair);

                if !preview {
                    if let Err(e) = prepare_backup_target(&self.config.backup.path) {
                        return self.show_error(e);
                    }
                }

                Command::batch([
                    self.close_modal(),
                    if repair {
                        self.switch_screen(Screen::Backup)
                    } else {
                        Command::none()
                    },
                    self.refresh_scroll_position_on_log(cleared_log),
                    self.handle_backup(BackupPhase::CloudCheck),
                ])
            }
            BackupPhase::CloudCheck => {
                if self.operation.preview()
                    || !self.config.cloud.synchronize
                    || crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path).is_err()
                {
                    return self.handle_backup(BackupPhase::Load);
                }

                let local = self.config.backup.path.clone();
                let games = self.operation.games();

                match self.start_sync_cloud(&local, SyncDirection::Upload, Finality::Preview, games.as_ref(), false) {
                    Ok(_) => {
                        // deferring to `transition_from_cloud_step`
                        Command::none()
                    }
                    Err(e) => {
                        self.operation.push_error(e);
                        self.handle_backup(BackupPhase::Load)
                    }
                }
            }
            BackupPhase::Load => {
                self.invalidate_path_caches();
                self.timed_notification = None;

                let preview = self.operation.preview();
                let full = self.operation.full();
                let games = self.operation.games();

                if preview && full {
                    self.backup_screen.previewed_games.clear();
                }

                let all_scanned = !self.backup_screen.log.contains_unscanned_games();
                if let Some(games) = &games {
                    self.backup_screen.log.unscan_games(games);
                }
                self.progress.start();

                let mut manifest = self.manifest.clone();
                let config = self.config.clone();
                let previewed_games = self.backup_screen.previewed_games.clone();
                let should_force_new_full_backups = self.operation.should_force_new_full_backups();

                Command::perform(
                    async move {
                        manifest.incorporate_extensions(&config);
                        let subjects: Vec<_> = if let Some(games) = &games {
                            manifest.0.keys().filter(|k| games.contains(k)).cloned().collect()
                        } else if !previewed_games.is_empty() && all_scanned {
                            manifest
                                .0
                                .keys()
                                .filter(|k| previewed_games.contains(*k))
                                .cloned()
                                .collect()
                        } else {
                            manifest.processable_titles().cloned().collect()
                        };

                        let mut retention = config.backup.retention.clone();
                        retention.force_new_full = should_force_new_full_backups;

                        let roots = config.expanded_roots();
                        let layout = BackupLayout::new(config.backup.path.clone(), retention);
                        let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());
                        let steam = SteamShortcuts::scan();
                        let launchers = Launchers::scan(&roots, &manifest, &subjects, &title_finder, None);

                        (subjects, manifest, layout, steam, launchers)
                    },
                    move |(subjects, manifest, layout, steam, heroic)| {
                        Message::Backup(BackupPhase::RegisterCommands {
                            subjects,
                            manifest,
                            layout: Box::new(layout),
                            steam,
                            launchers: heroic,
                        })
                    },
                )
            }
            BackupPhase::RegisterCommands {
                subjects,
                manifest,
                layout,
                steam,
                launchers,
            } => {
                log::info!("beginning backup with {} steps", subjects.len());
                let preview = self.operation.preview();
                let full = self.operation.full();

                if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    self.go_idle();
                    return Command::none();
                }

                if subjects.is_empty() {
                    if let Some(games) = self.operation.games() {
                        for game in &games {
                            let duplicates = self.backup_screen.duplicate_detector.remove_game(game);
                            self.backup_screen.log.remove_game(
                                game,
                                &self.backup_screen.duplicate_detector,
                                &duplicates,
                                &self.config,
                                false,
                            );
                        }
                        self.cache.backup.recent_games.retain(|x| !games.contains(x));
                        self.save_cache();
                    }
                    self.go_idle();
                    return Command::none();
                }

                self.progress.set_max(subjects.len() as f32);
                self.register_notify_on_single_game_scanned();

                let config = std::sync::Arc::new(self.config.clone());
                let roots = std::sync::Arc::new(config.expanded_roots());
                let layout = std::sync::Arc::new(*layout);
                let launchers = std::sync::Arc::new(launchers);
                let filter = std::sync::Arc::new(self.config.backup.filter.clone());
                let steam_shortcuts = std::sync::Arc::new(steam);

                for key in subjects {
                    let game = manifest.0[&key].clone();
                    let config = config.clone();
                    let roots = roots.clone();
                    let launchers = launchers.clone();
                    let layout = layout.clone();
                    let filter = filter.clone();
                    let steam_shortcuts = steam_shortcuts.clone();
                    let cancel_flag = self.operation_should_cancel.clone();
                    self.operation_steps.push(Command::perform(
                        async move {
                            if key.trim().is_empty() {
                                return (None, None, OperationStepDecision::Ignored);
                            }
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(Duration::from_millis(1));
                                return (None, None, OperationStepDecision::Cancelled);
                            }

                            let previous =
                                layout.latest_backup(&key, false, &config.redirects, &config.restore.toggled_paths);

                            let scan_info = scan_game_for_backup(
                                &game,
                                &key,
                                &roots,
                                &app_dir(),
                                &launchers,
                                &filter,
                                &None,
                                // &ranking,
                                &config.backup.toggled_paths,
                                &config.backup.toggled_registry,
                                previous,
                                &config.redirects,
                                &steam_shortcuts,
                            );
                            if !config.is_game_enabled_for_backup(&key) && full {
                                return (Some(scan_info), None, OperationStepDecision::Ignored);
                            }

                            let backup_info = if !preview {
                                Some(layout.game_layout(&key).back_up(
                                    &scan_info,
                                    &chrono::Utc::now(),
                                    &config.backup.format,
                                ))
                            } else {
                                None
                            };
                            (Some(scan_info), backup_info, OperationStepDecision::Processed)
                        },
                        move |(scan_info, backup_info, decision)| {
                            Message::Backup(BackupPhase::GameScanned {
                                scan_info,
                                backup_info,
                                decision,
                            })
                        },
                    ));
                }

                self.operation_steps_active = 100.min(self.operation_steps.len());
                Command::batch(self.operation_steps.drain(..self.operation_steps_active))
            }
            BackupPhase::GameScanned {
                scan_info,
                backup_info,
                decision: _,
            } => {
                self.progress.step();
                let restoring = false;
                let full = self.operation.full();

                if let Some(scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    if scan_info.can_report_game() {
                        let duplicates = self.backup_screen.duplicate_detector.add_game(
                            &scan_info,
                            self.config
                                .is_game_enabled_for_operation(&scan_info.game_name, restoring),
                        );
                        self.backup_screen.previewed_games.insert(scan_info.game_name.clone());
                        self.backup_screen.log.update_game(
                            scan_info,
                            backup_info,
                            &self.config.backup.sort,
                            &self.backup_screen.duplicate_detector,
                            &duplicates,
                            None,
                            &self.config,
                            restoring,
                        );
                    } else if !full {
                        let duplicates = self.backup_screen.duplicate_detector.remove_game(&scan_info.game_name);
                        self.backup_screen.log.remove_game(
                            &scan_info.game_name,
                            &self.backup_screen.duplicate_detector,
                            &duplicates,
                            &self.config,
                            restoring,
                        );
                        self.cache.backup.recent_games.remove(&scan_info.game_name);
                    }
                } else {
                    log::trace!(
                        "step {} / {}, awaiting {}",
                        self.progress.current,
                        self.progress.max,
                        self.operation_steps_active
                    );
                }

                match self.operation_steps.pop() {
                    Some(step) => step,
                    None => {
                        self.operation_steps_active -= 1;
                        if self.operation_steps_active == 0 {
                            self.handle_backup(BackupPhase::CloudSync)
                        } else {
                            Command::none()
                        }
                    }
                }
            }
            BackupPhase::CloudSync => {
                if !self.operation.should_sync_cloud_after() {
                    return self.handle_backup(BackupPhase::Done);
                }

                let local = self.config.backup.path.clone();
                let games = self.operation.games();

                let changed_games: Vec<_> = self
                    .backup_screen
                    .log
                    .entries
                    .iter()
                    .filter(|x| {
                        let relevant = games
                            .as_ref()
                            .map(|games| games.contains(&x.scan_info.game_name))
                            .unwrap_or(true);
                        relevant && x.scan_info.needs_cloud_sync()
                    })
                    .map(|x| x.scan_info.game_name.clone())
                    .collect();

                if changed_games.is_empty() {
                    return self.handle_backup(BackupPhase::Done);
                }

                match self.start_sync_cloud(
                    &local,
                    SyncDirection::Upload,
                    Finality::Final,
                    Some(&changed_games),
                    false,
                ) {
                    Ok(_) => {
                        // deferring to `transition_from_cloud_step`
                        Command::none()
                    }
                    Err(e) => {
                        self.operation.push_error(e);
                        self.handle_backup(BackupPhase::Done)
                    }
                }
            }
            BackupPhase::Done => {
                log::info!("completed backup");
                let mut failed = false;
                let preview = self.operation.preview();
                let full = self.operation.full();

                self.handle_notify_on_single_game_scanned();

                if full {
                    self.cache.backup.recent_games.clear();
                }

                for entry in &self.backup_screen.log.entries {
                    self.cache.backup.recent_games.insert(entry.scan_info.game_name.clone());
                    if let Some(backup_info) = &entry.backup_info {
                        if !backup_info.successful() {
                            failed = true;
                        }
                    }
                }

                if !preview && full {
                    self.backup_screen.previewed_games.clear();
                }

                self.save_cache();

                if failed {
                    self.operation.push_error(Error::SomeEntriesFailed);
                }

                let errors = self.operation.errors().cloned();
                self.go_idle();

                if let Some(errors) = errors {
                    if !errors.is_empty() {
                        return self.show_modal(Modal::Errors { errors });
                    }
                }

                Command::none()
            }
        }
    }

    fn handle_restore(&mut self, phase: RestorePhase) -> Command<Message> {
        match phase {
            RestorePhase::Confirm { games } => self.show_modal(Modal::ConfirmRestore { games }),
            RestorePhase::Start { preview, games } => {
                if !self.operation.idle() {
                    return Command::none();
                }

                let path = self.config.restore.path.clone();
                if !path.is_dir() {
                    return self.show_modal(Modal::Error {
                        variant: Error::RestorationSourceInvalid { path },
                    });
                }

                let mut cleared_log = false;
                if games.is_none() {
                    self.restore_screen.log.clear();
                    self.restore_screen.duplicate_detector.clear();
                    self.reset_scroll_position(ScrollSubject::Restore);
                    cleared_log = true;
                }

                self.operation =
                    Operation::new_restore(if preview { Finality::Preview } else { Finality::Final }, games);

                self.invalidate_path_caches();
                self.timed_notification = None;

                Command::batch([
                    self.close_modal(),
                    self.refresh_scroll_position_on_log(cleared_log),
                    self.handle_restore(RestorePhase::CloudCheck),
                ])
            }
            RestorePhase::CloudCheck => {
                if self.operation.preview()
                    || !self.config.cloud.synchronize
                    || crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path).is_err()
                {
                    return self.handle_restore(RestorePhase::Load);
                }

                let local = self.config.restore.path.clone();
                let games = self.operation.games();

                match self.start_sync_cloud(&local, SyncDirection::Upload, Finality::Preview, games.as_ref(), false) {
                    Ok(_) => {
                        // waiting for background thread
                        Command::none()
                    }
                    Err(e) => {
                        self.operation.push_error(e);
                        self.handle_restore(RestorePhase::Load)
                    }
                }
            }
            RestorePhase::Load => {
                let restore_path = self.config.restore.path.clone();

                let config = std::sync::Arc::new(self.config.clone());

                self.progress.start();

                Command::perform(
                    async move {
                        let layout = BackupLayout::new(restore_path, config.backup.retention.clone());
                        let restorables = layout.restorable_games();
                        (layout, restorables)
                    },
                    move |(layout, restorables)| {
                        Message::Restore(RestorePhase::RegisterCommands { layout, restorables })
                    },
                )
            }
            RestorePhase::RegisterCommands {
                mut restorables,
                layout,
            } => {
                log::info!("beginning restore with {} steps", restorables.len());
                let preview = self.operation.preview();
                let full = self.operation.full();
                let games = self.operation.games();

                if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    self.go_idle();
                    return Command::none();
                }

                if let Some(games) = &games {
                    restorables.retain(|v| games.contains(v));
                    self.restore_screen.log.unscan_games(games);
                }

                if restorables.is_empty() {
                    if let Some(games) = games {
                        for game in &games {
                            let duplicates = self.restore_screen.duplicate_detector.remove_game(game);
                            self.restore_screen.log.remove_game(
                                game,
                                &self.restore_screen.duplicate_detector,
                                &duplicates,
                                &self.config,
                                true,
                            );
                        }
                        self.cache.restore.recent_games.retain(|x| !games.contains(x));
                        self.save_cache();
                    }
                    self.go_idle();
                    return Command::none();
                }

                self.progress.set_max(restorables.len() as f32);

                self.register_notify_on_single_game_scanned();

                let config = std::sync::Arc::new(self.config.clone());
                let layout = std::sync::Arc::new(layout);

                for name in restorables {
                    let config = config.clone();
                    let layout = layout.clone();
                    let cancel_flag = self.operation_should_cancel.clone();
                    let backup_id = self.backups_to_restore.get(&name).cloned().unwrap_or(BackupId::Latest);
                    self.operation_steps.push(Command::perform(
                        async move {
                            let mut layout = layout.game_layout(&name);

                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(Duration::from_millis(1));
                                return (None, None, OperationStepDecision::Cancelled, layout);
                            }

                            let scan_info = layout.scan_for_restoration(
                                &name,
                                &backup_id,
                                &config.redirects,
                                &config.restore.toggled_paths,
                                &config.restore.toggled_registry,
                            );
                            if !config.is_game_enabled_for_restore(&name) && full {
                                return (Some(scan_info), None, OperationStepDecision::Ignored, layout);
                            }

                            let backup_info = if scan_info.backup.is_some() && !preview {
                                Some(layout.restore(&scan_info, &config.restore.toggled_registry))
                            } else {
                                None
                            };
                            (Some(scan_info), backup_info, OperationStepDecision::Processed, layout)
                        },
                        move |(scan_info, backup_info, decision, game_layout)| {
                            Message::Restore(RestorePhase::GameScanned {
                                scan_info,
                                backup_info,
                                decision,
                                game_layout: Box::new(game_layout),
                            })
                        },
                    ));
                }

                self.operation_steps_active = 100.min(self.operation_steps.len());
                Command::batch(self.operation_steps.drain(..self.operation_steps_active))
            }
            RestorePhase::GameScanned {
                scan_info,
                backup_info,
                decision: _,
                game_layout,
            } => {
                self.progress.step();
                let restoring = true;
                let full = self.operation.full();

                if let Some(scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    if scan_info.can_report_game() {
                        let comment = scan_info.backup.as_ref().and_then(|x| x.comment()).map(|x| x.as_str());
                        self.text_histories.backup_comments.insert(
                            scan_info.game_name.clone(),
                            TextHistory::raw(comment.unwrap_or_default()),
                        );

                        let duplicates = self.restore_screen.duplicate_detector.add_game(
                            &scan_info,
                            self.config
                                .is_game_enabled_for_operation(&scan_info.game_name, restoring),
                        );
                        self.restore_screen.log.update_game(
                            scan_info,
                            backup_info,
                            &self.config.backup.sort,
                            &self.restore_screen.duplicate_detector,
                            &duplicates,
                            Some(*game_layout),
                            &self.config,
                            restoring,
                        );
                    } else if !full {
                        let duplicates = self.restore_screen.duplicate_detector.remove_game(&scan_info.game_name);
                        self.restore_screen.log.remove_game(
                            &scan_info.game_name,
                            &self.restore_screen.duplicate_detector,
                            &duplicates,
                            &self.config,
                            restoring,
                        );
                        self.cache.restore.recent_games.remove(&scan_info.game_name);
                    }
                } else {
                    log::trace!(
                        "step {} / {}, awaiting {}",
                        self.progress.current,
                        self.progress.max,
                        self.operation_steps_active
                    );
                }

                match self.operation_steps.pop() {
                    Some(step) => step,
                    None => {
                        self.operation_steps_active -= 1;
                        if self.operation_steps_active == 0 {
                            self.handle_restore(RestorePhase::Done)
                        } else {
                            Command::none()
                        }
                    }
                }
            }
            RestorePhase::Done => {
                log::info!("completed restore");
                let mut failed = false;
                let full = self.operation.full();

                self.handle_notify_on_single_game_scanned();

                if full {
                    self.cache.restore.recent_games.clear();
                }

                for entry in &self.restore_screen.log.entries {
                    self.cache
                        .restore
                        .recent_games
                        .insert(entry.scan_info.game_name.clone());
                    if let Some(backup_info) = &entry.backup_info {
                        if !backup_info.successful() {
                            failed = true;
                        }
                    }
                }

                self.save_cache();

                if failed {
                    self.operation.push_error(Error::SomeEntriesFailed);
                }

                let errors = self.operation.errors().cloned();
                self.go_idle();

                if let Some(errors) = errors {
                    if !errors.is_empty() {
                        return self.show_modal(Modal::Errors { errors });
                    }
                }

                Command::none()
            }
        }
    }

    fn handle_validation(&mut self, phase: ValidatePhase) -> Command<Message> {
        match phase {
            ValidatePhase::Start => {
                if !self.operation.idle() {
                    return Command::none();
                }

                let path = self.config.restore.path.clone();
                if !path.is_dir() {
                    return self.show_modal(Modal::Error {
                        variant: Error::RestorationSourceInvalid { path },
                    });
                }

                self.operation = Operation::new_validate_backups();

                self.invalidate_path_caches();
                self.timed_notification = None;

                Command::batch([self.close_modal(), self.handle_validation(ValidatePhase::Load)])
            }
            ValidatePhase::Load => {
                let restore_path = self.config.restore.path.clone();

                let config = std::sync::Arc::new(self.config.clone());

                self.progress.start();

                Command::perform(
                    async move {
                        let layout = BackupLayout::new(restore_path, config.backup.retention.clone());
                        let subjects = layout.restorable_games();
                        (layout, subjects)
                    },
                    move |(layout, subjects)| {
                        Message::ValidateBackups(ValidatePhase::RegisterCommands { layout, subjects })
                    },
                )
            }
            ValidatePhase::RegisterCommands { subjects, layout } => {
                log::info!("beginning validation with {} steps", subjects.len());

                if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    self.go_idle();
                    return Command::none();
                }

                if subjects.is_empty() {
                    self.go_idle();
                    return Command::none();
                }

                self.progress.set_max(subjects.len() as f32);

                let layout = std::sync::Arc::new(layout);

                for name in subjects {
                    let layout = layout.clone();
                    let cancel_flag = self.operation_should_cancel.clone();
                    let backup_id = self.backups_to_restore.get(&name).cloned().unwrap_or(BackupId::Latest);
                    self.operation_steps.push(Command::perform(
                        async move {
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(Duration::from_millis(1));
                                return (name, true);
                            }

                            let Some(layout) = layout.try_game_layout(&name) else {
                                return (name, false);
                            };

                            let valid = layout.validate(backup_id);
                            (name, valid)
                        },
                        move |(game, valid)| Message::ValidateBackups(ValidatePhase::GameScanned { game, valid }),
                    ));
                }

                self.operation_steps_active = 100.min(self.operation_steps.len());
                Command::batch(self.operation_steps.drain(..self.operation_steps_active))
            }
            ValidatePhase::GameScanned { game, valid } => {
                self.progress.step();
                log::trace!("step {} / {}: {}", self.progress.current, self.progress.max, &game);

                if !valid {
                    if let Operation::ValidateBackups { faulty_games, .. } = &mut self.operation {
                        faulty_games.insert(game);
                    }
                }

                match self.operation_steps.pop() {
                    Some(step) => step,
                    None => {
                        self.operation_steps_active -= 1;
                        if self.operation_steps_active == 0 {
                            self.handle_validation(ValidatePhase::Done)
                        } else {
                            Command::none()
                        }
                    }
                }
            }
            ValidatePhase::Done => {
                log::info!("completed validation");
                let faulty_games = if let Operation::ValidateBackups { faulty_games, .. } = &self.operation {
                    faulty_games.clone()
                } else {
                    Default::default()
                };
                self.go_idle();
                self.show_modal(Modal::BackupValidation { games: faulty_games })
            }
        }
    }

    fn transition_from_cloud_step(&mut self) -> Option<Command<Message>> {
        let synced = self.operation.cloud_changes() == 0;

        if self.operation.integrated_checking_cloud() {
            self.operation.transition_from_cloud_step(synced);

            match self.operation {
                Operation::Backup { .. } => Some(self.handle_backup(BackupPhase::Load)),
                Operation::Restore { .. } => Some(self.handle_restore(RestorePhase::Load)),
                Operation::Idle | Operation::ValidateBackups { .. } | Operation::Cloud { .. } => None,
            }
        } else if self.operation.integrated_syncing_cloud() {
            self.operation.transition_from_cloud_step(synced);
            match self.operation {
                Operation::Backup { .. } => Some(self.handle_backup(BackupPhase::Done)),
                Operation::Idle
                | Operation::ValidateBackups { .. }
                | Operation::Restore { .. }
                | Operation::Cloud { .. } => None,
            }
        } else {
            None
        }
    }

    fn cancel_operation(&mut self) -> Command<Message> {
        self.operation_should_cancel
            .swap(true, std::sync::atomic::Ordering::Relaxed);
        self.operation_steps.clear();
        self.operation.flag_cancel();
        if self.operation.is_cloud_active() {
            if let Some(sender) = self.rclone_monitor_sender.as_mut() {
                let _ = sender.try_send(rclone_monitor::Input::Cancel);
            }
        }
        Command::none()
    }

    fn customize_game(&mut self, name: String) -> Command<Message> {
        let game = if let Some(standard) = self.manifest.0.get(&name) {
            CustomGame {
                name: name.clone(),
                ignore: false,
                alias: standard.alias.clone(),
                prefer_alias: false,
                files: standard.files.clone().unwrap_or_default().keys().cloned().collect(),
                registry: standard.registry.clone().unwrap_or_default().keys().cloned().collect(),
            }
        } else {
            CustomGame {
                name: name.clone(),
                ignore: false,
                alias: None,
                prefer_alias: false,
                files: vec![],
                registry: vec![],
            }
        };

        self.text_histories.add_custom_game(&game);
        self.config.custom_games.push(game);
        self.save_config();

        self.switch_screen(Screen::CustomGames)
    }

    fn customize_game_as_alias(&mut self, name: String) -> Command<Message> {
        let game = CustomGame {
            name: "".to_string(),
            ignore: false,
            alias: Some(name),
            prefer_alias: true,
            files: vec![],
            registry: vec![],
        };

        self.text_histories.add_custom_game(&game);
        self.config.custom_games.push(game);
        self.save_config();

        self.switch_screen(Screen::CustomGames)
    }

    fn open_url(url: String) -> Command<Message> {
        let url2 = url.clone();
        Command::perform(async { opener::open(url) }, move |res| match res {
            Ok(_) => Message::Ignore,
            Err(e) => {
                log::error!("Unable to open URL: `{}` - {}", url2, e);
                Message::OpenUrlFailure { url: url2 }
            }
        })
    }

    fn open_wiki(game: String) -> Command<Message> {
        let url = format!("https://www.pcgamingwiki.com/wiki/{}", game.replace(' ', "_"));
        Self::open_url(url)
    }

    fn toggle_backup_comment_editor(&mut self, name: String) -> Command<Message> {
        self.restore_screen.log.toggle_backup_comment_editor(&name);
        Command::none()
    }

    fn switch_screen(&mut self, screen: Screen) -> Command<Message> {
        self.screen = screen;
        self.refresh_scroll_position()
    }

    fn scroll_subject(&self) -> ScrollSubject {
        if self.modal.is_some() {
            ScrollSubject::Modal
        } else {
            ScrollSubject::from(self.screen)
        }
    }

    fn refresh_scroll_position(&mut self) -> Command<Message> {
        let subject = self.scroll_subject();
        let offset = self.scroll_offsets.get(&subject).copied().unwrap_or_default();

        scrollable::scroll_to(subject.id(), offset)
    }

    fn refresh_scroll_position_on_log(&mut self, cleared: bool) -> Command<Message> {
        if cleared {
            self.refresh_scroll_position()
        } else {
            Command::none()
        }
    }

    fn reset_scroll_position(&mut self, subject: ScrollSubject) {
        self.scroll_offsets
            .insert(subject, scrollable::AbsoluteOffset::default());
    }

    fn configure_remote(&self, remote: Remote) -> Command<Message> {
        let rclone = self.config.apps.rclone.clone();
        let old_remote = self.config.cloud.remote.clone();
        let new_remote = remote.clone();
        Command::perform(
            async move {
                if let Some(old_remote) = old_remote {
                    _ = Rclone::new(rclone.clone(), old_remote).unconfigure_remote();
                }
                Rclone::new(rclone, new_remote).configure_remote()
            },
            move |res| match res {
                Ok(_) => Message::ConfigureCloudSuccess(remote),
                Err(e) => Message::ConfigureCloudFailure(e),
            },
        )
    }
}

impl Application for App {
    type Executor = Executor;
    type Message = Message;
    type Flags = Flags;
    type Theme = crate::gui::style::Theme;

    fn new(flags: Flags) -> (Self, Command<Message>) {
        let mut errors = vec![];

        let mut modal: Option<Modal> = None;
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                errors.push(x);
                let _ = Config::archive_invalid();
                Config::default()
            }
        };
        let mut cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
        TRANSLATOR.set_language(config.language);
        let manifest = if Manifest::path().exists() {
            match Manifest::load() {
                Ok(y) => y,
                Err(e) => {
                    errors.push(e);
                    Manifest::default()
                }
            }
        } else {
            if flags.update_manifest {
                modal = Some(Modal::UpdatingManifest);
            }
            Manifest::default()
        };

        if !errors.is_empty() {
            modal = Some(Modal::Errors { errors });
        } else {
            let missing: Vec<_> = config
                .find_missing_roots()
                .iter()
                .filter(|x| !cache.has_root(x))
                .cloned()
                .collect();
            if !missing.is_empty() {
                cache.add_roots(&missing);
                cache.save();
                modal = Some(Modal::ConfirmAddMissingRoots(missing));
            }
        }

        let manifest_config = config.manifest.clone();
        let manifest_cache = cache.manifests.clone();
        let text_histories = TextHistories::new(&config);

        log::debug!("Config on startup: {config:?}");

        let mut commands = vec![
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::TEXT_DATA)).map(|_| Message::Ignore),
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::ICONS_DATA)).map(|_| Message::Ignore),
        ];
        if flags.update_manifest {
            commands.push(Command::perform(
                async move {
                    tokio::task::spawn_blocking(move || Manifest::update(manifest_config, manifest_cache, false)).await
                },
                |join| match join {
                    Ok(x) => Message::ManifestUpdated(x),
                    Err(_) => Message::Ignore,
                },
            ));
        }

        (
            Self {
                backup_screen: screen::Backup::new(&config, &cache),
                restore_screen: screen::Restore::new(&config, &cache),
                config,
                manifest,
                cache,
                modal,
                updating_manifest: flags.update_manifest,
                text_histories,
                ..Self::default()
            },
            Command::batch(commands),
        )
    }

    fn title(&self) -> String {
        TRANSLATOR.window_title()
    }

    fn theme(&self) -> Self::Theme {
        crate::gui::style::Theme::from(self.config.theme)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Ignore => Command::none(),
            Message::CloseModal => self.close_modal(),
            Message::Exit { user } => {
                if self.operation.idle() || (user && self.exiting) {
                    self.save();
                    std::process::exit(0)
                } else {
                    self.exiting = true;
                    Command::batch([self.show_modal(Modal::Exiting), self.cancel_operation()])
                }
            }
            Message::Save => {
                self.save();
                Command::none()
            }
            Message::UpdateTime => {
                self.progress.update_time();
                Command::none()
            }
            Message::PruneNotifications => {
                if let Some(notification) = &self.timed_notification {
                    if notification.expired() {
                        self.timed_notification = None;
                    }
                }
                Command::none()
            }
            Message::UpdateManifest => {
                self.updating_manifest = true;
                let manifest_config = self.config.manifest.clone();
                let manifest_cache = self.cache.manifests.clone();
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || Manifest::update(manifest_config, manifest_cache, true))
                            .await
                    },
                    |join| match join {
                        Ok(x) => Message::ManifestUpdated(x),
                        Err(_) => Message::Ignore,
                    },
                )
            }
            Message::ManifestUpdated(updates) => {
                self.updating_manifest = false;
                let mut errors = vec![];

                for update in updates {
                    match update {
                        Ok(Some(update)) => {
                            self.cache.update_manifest(update);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            errors.push(e);
                        }
                    }
                }

                self.save_cache();

                match Manifest::load() {
                    Ok(x) => {
                        self.manifest = x;
                    }
                    Err(e) => {
                        errors.push(e);
                    }
                }

                if errors.is_empty() {
                    self.close_specific_modal(Modal::UpdatingManifest)
                } else {
                    self.show_modal(Modal::Errors { errors })
                }
            }
            Message::Backup(phase) => self.handle_backup(phase),
            Message::Restore(phase) => self.handle_restore(phase),
            Message::ValidateBackups(phase) => self.handle_validation(phase),
            Message::CancelOperation => self.cancel_operation(),
            Message::EditedBackupTarget(text) => {
                self.text_histories.backup_target.push(&text);
                self.config.backup.path.reset(text);
                self.save_config();
                Command::none()
            }
            Message::EditedRestoreSource(text) => {
                self.text_histories.restore_source.push(&text);
                self.config.restore.path.reset(text);
                self.save_config();
                Command::none()
            }
            Message::FindRoots => {
                let missing = self.config.find_missing_roots();
                if missing.is_empty() {
                    self.show_modal(Modal::NoMissingRoots)
                } else {
                    self.cache.add_roots(&missing);
                    self.save_cache();
                    self.show_modal(Modal::ConfirmAddMissingRoots(missing))
                }
            }
            Message::ConfirmAddMissingRoots(missing) => {
                for root in missing {
                    self.text_histories.roots.push(TextHistory::raw(&root.path.render()));
                    self.config.roots.push(root);
                }
                self.save_config();
                self.go_idle();
                Command::none()
            }
            Message::EditedRoot(action) => {
                match action {
                    EditAction::Add => {
                        self.text_histories.roots.push(Default::default());
                        self.config.roots.push(RootsConfig {
                            path: StrictPath::default(),
                            store: Store::Other,
                        });
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.roots[index].push(&value);
                        self.config.roots[index].path.reset(value);
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.roots.remove(index);
                        self.config.roots.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.roots.swap(index, offset);
                        self.config.roots.swap(index, offset);
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::EditedSecondaryManifest(action) => {
                match action {
                    EditAction::Add => {
                        self.text_histories.secondary_manifests.push(Default::default());
                        self.config.manifest.secondary.push(Default::default());
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.secondary_manifests[index].push(&value);
                        self.config.manifest.secondary[index].set(value);
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.secondary_manifests.remove(index);
                        self.config.manifest.secondary.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.secondary_manifests.swap(index, offset);
                        self.config.manifest.secondary.swap(index, offset);
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::SelectedRootStore(index, store) => {
                self.config.roots[index].store = store;
                self.save_config();
                Command::none()
            }
            Message::SelectedRedirectKind(index, kind) => {
                self.config.redirects[index].kind = kind;
                self.save_config();
                Command::none()
            }
            Message::SelectedSecondaryManifestKind(index, kind) => {
                self.config.manifest.secondary[index].convert(kind);
                self.save_config();
                Command::none()
            }
            Message::SelectedCustomGameKind(index, kind) => {
                self.config.custom_games[index].convert(kind);
                match kind {
                    CustomGameKind::Game => {
                        self.text_histories.custom_games[index].alias.clear();
                    }
                    CustomGameKind::Alias => {}
                }
                self.save_config();
                Command::none()
            }
            Message::EditedRedirect(action, field) => {
                match action {
                    EditAction::Add => {
                        self.text_histories.redirects.push(Default::default());
                        self.config.add_redirect(&StrictPath::default(), &StrictPath::default());
                    }
                    EditAction::Change(index, value) => match field {
                        Some(RedirectEditActionField::Source) => {
                            self.text_histories.redirects[index].source.push(&value);
                            self.config.redirects[index].source.reset(value);
                        }
                        Some(RedirectEditActionField::Target) => {
                            self.text_histories.redirects[index].target.push(&value);
                            self.config.redirects[index].target.reset(value);
                        }
                        _ => {}
                    },
                    EditAction::Remove(index) => {
                        self.text_histories.redirects.remove(index);
                        self.config.redirects.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.redirects.swap(index, offset);
                        self.config.redirects.swap(index, offset);
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::EditedCustomGame(action) => {
                let mut snap = false;
                match action {
                    EditAction::Add => {
                        self.text_histories.custom_games.push(Default::default());
                        self.config.add_custom_game();
                        snap = true;
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.custom_games[index].name.push(&value);
                        self.config.custom_games[index].name = value;
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.custom_games.remove(index);
                        self.config.custom_games.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.custom_games.swap(index, offset);
                        self.config.custom_games.swap(index, offset);
                    }
                }
                self.save_config();
                if snap {
                    self.scroll_offsets.insert(
                        ScrollSubject::CustomGames,
                        scrollable::AbsoluteOffset { x: 0.0, y: f32::MAX },
                    );
                    self.refresh_scroll_position()
                } else {
                    Command::none()
                }
            }
            Message::EditedCustomGameAlias(index, value) => {
                self.text_histories.custom_games[index].alias.push(&value);
                self.config.custom_games[index].alias = Some(value);

                self.save_config();
                Command::none()
            }
            Message::EditedCustomGaleAliasDisplay(index, value) => {
                self.config.custom_games[index].prefer_alias = value;

                self.save_config();
                Command::none()
            }
            Message::EditedCustomGameFile(game_index, action) => {
                match action {
                    EditAction::Add => {
                        self.text_histories.custom_games[game_index]
                            .files
                            .push(Default::default());
                        self.config.custom_games[game_index].files.push("".to_string());
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.custom_games[game_index].files[index].push(&value);
                        self.config.custom_games[game_index].files[index] = value;
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.custom_games[game_index].files.remove(index);
                        self.config.custom_games[game_index].files.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.custom_games[game_index].files.swap(index, offset);
                        self.config.custom_games[game_index].files.swap(index, offset);
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::EditedCustomGameRegistry(game_index, action) => {
                match action {
                    EditAction::Add => {
                        self.text_histories.custom_games[game_index]
                            .registry
                            .push(Default::default());
                        self.config.custom_games[game_index].registry.push("".to_string());
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.custom_games[game_index].registry[index].push(&value);
                        self.config.custom_games[game_index].registry[index] = value;
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.custom_games[game_index].registry.remove(index);
                        self.config.custom_games[game_index].registry.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.custom_games[game_index]
                            .registry
                            .swap(index, offset);
                        self.config.custom_games[game_index].registry.swap(index, offset);
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::EditedExcludeStoreScreenshots(enabled) => {
                self.config.backup.filter.exclude_store_screenshots = enabled;
                self.save_config();
                Command::none()
            }
            Message::EditedBackupFilterIgnoredPath(action) => {
                match action {
                    EditAction::Add => {
                        self.text_histories.backup_filter_ignored_paths.push(Default::default());
                        self.config
                            .backup
                            .filter
                            .ignored_paths
                            .push(StrictPath::new("".to_string()));
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.backup_filter_ignored_paths[index].push(&value);
                        self.config.backup.filter.ignored_paths[index] = StrictPath::new(value);
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.backup_filter_ignored_paths.remove(index);
                        self.config.backup.filter.ignored_paths.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.backup_filter_ignored_paths.swap(index, offset);
                        self.config.backup.filter.ignored_paths.swap(index, offset);
                    }
                }
                self.config.backup.filter.build_globs();
                self.save_config();
                Command::none()
            }
            Message::EditedBackupFilterIgnoredRegistry(action) => {
                match action {
                    EditAction::Add => {
                        self.text_histories
                            .backup_filter_ignored_registry
                            .push(Default::default());
                        self.config
                            .backup
                            .filter
                            .ignored_registry
                            .push(RegistryItem::new("".to_string()));
                    }
                    EditAction::Change(index, value) => {
                        self.text_histories.backup_filter_ignored_registry[index].push(&value);
                        self.config.backup.filter.ignored_registry[index] = RegistryItem::new(value);
                    }
                    EditAction::Remove(index) => {
                        self.text_histories.backup_filter_ignored_registry.remove(index);
                        self.config.backup.filter.ignored_registry.remove(index);
                    }
                    EditAction::Move(index, direction) => {
                        let offset = direction.shift(index);
                        self.text_histories.backup_filter_ignored_registry.swap(index, offset);
                        self.config.backup.filter.ignored_registry.swap(index, offset);
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::SwitchScreen(screen) => self.switch_screen(screen),
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        self.backup_screen.log.toggle_game_expanded(
                            &name,
                            &self.backup_screen.duplicate_detector,
                            &self.config,
                            false,
                        );
                    }
                    Screen::Restore => {
                        self.restore_screen.log.toggle_game_expanded(
                            &name,
                            &self.restore_screen.duplicate_detector,
                            &self.config,
                            true,
                        );
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::ToggleGameListEntryTreeExpanded { name, keys } => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &mut self.backup_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                if let Some(tree) = entry.tree.as_mut() {
                                    tree.expand_or_collapse_keys(&keys);
                                }
                            }
                        }
                    }
                    Screen::Restore => {
                        for entry in &mut self.restore_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                if let Some(tree) = entry.tree.as_mut() {
                                    tree.expand_or_collapse_keys(&keys);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::ToggleGameListEntryEnabled {
                name,
                enabled,
                restoring,
            } => {
                match (restoring, enabled) {
                    (false, false) => self.config.disable_game_for_backup(&name),
                    (false, true) => self.config.enable_game_for_backup(&name),
                    (true, false) => self.config.disable_game_for_restore(&name),
                    (true, true) => self.config.enable_game_for_restore(&name),
                };
                self.save_config();

                if restoring {
                    self.restore_screen.log.refresh_game_tree(
                        &name,
                        &self.config,
                        &mut self.restore_screen.duplicate_detector,
                        restoring,
                    );
                } else {
                    self.backup_screen.log.refresh_game_tree(
                        &name,
                        &self.config,
                        &mut self.backup_screen.duplicate_detector,
                        restoring,
                    );
                }

                Command::none()
            }
            Message::ToggleCustomGameEnabled { index, enabled } => {
                if enabled {
                    self.config.enable_custom_game(index);
                } else {
                    self.config.disable_custom_game(index);
                }
                self.save_config();
                Command::none()
            }
            Message::ToggleSearch { screen } => match screen {
                Screen::Backup => {
                    self.backup_screen.log.search.show = !self.backup_screen.log.search.show;
                    iced::widget::text_input::focus(id::backup_search())
                }
                Screen::Restore => {
                    self.restore_screen.log.search.show = !self.restore_screen.log.search.show;
                    iced::widget::text_input::focus(id::restore_search())
                }
                _ => Command::none(),
            },
            Message::ToggleSpecificGamePathIgnored {
                name,
                path,
                enabled: _,
                restoring,
            } => {
                if restoring {
                    self.config.restore.toggled_paths.toggle(&name, &path);
                    self.restore_screen.log.refresh_game_tree(
                        &name,
                        &self.config,
                        &mut self.restore_screen.duplicate_detector,
                        restoring,
                    );
                } else {
                    self.config.backup.toggled_paths.toggle(&name, &path);
                    self.backup_screen.log.refresh_game_tree(
                        &name,
                        &self.config,
                        &mut self.backup_screen.duplicate_detector,
                        restoring,
                    );
                }
                self.save_config();
                Command::none()
            }
            Message::ToggleSpecificGameRegistryIgnored {
                name,
                path,
                value,
                enabled: _,
                restoring,
            } => {
                if restoring {
                    self.config.restore.toggled_registry.toggle_owned(&name, &path, value);
                    self.restore_screen.log.refresh_game_tree(
                        &name,
                        &self.config,
                        &mut self.restore_screen.duplicate_detector,
                        restoring,
                    );
                } else {
                    self.config.backup.toggled_registry.toggle_owned(&name, &path, value);
                    self.backup_screen.log.refresh_game_tree(
                        &name,
                        &self.config,
                        &mut self.backup_screen.duplicate_detector,
                        restoring,
                    );
                }
                self.save_config();
                Command::none()
            }
            Message::EditedSearchGameName { screen, value } => {
                match screen {
                    Screen::Backup => {
                        self.text_histories.backup_search_game_name.push(&value);
                        self.backup_screen.log.search.game_name = value;
                    }
                    Screen::Restore => {
                        self.text_histories.restore_search_game_name.push(&value);
                        self.restore_screen.log.search.game_name = value;
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::ToggledSearchFilter { filter, enabled } => {
                let search = if self.screen == Screen::Backup {
                    &mut self.backup_screen.log.search
                } else {
                    &mut self.restore_screen.log.search
                };
                search.toggle_filter(filter, enabled);
                Command::none()
            }
            Message::EditedSearchFilterUniqueness(filter) => {
                let search = if self.screen == Screen::Backup {
                    &mut self.backup_screen.log.search
                } else {
                    &mut self.restore_screen.log.search
                };
                search.uniqueness.choice = filter;
                Command::none()
            }
            Message::EditedSearchFilterCompleteness(filter) => {
                let search = if self.screen == Screen::Backup {
                    &mut self.backup_screen.log.search
                } else {
                    &mut self.restore_screen.log.search
                };
                search.completeness.choice = filter;
                Command::none()
            }
            Message::EditedSearchFilterEnablement(filter) => {
                let search = if self.screen == Screen::Backup {
                    &mut self.backup_screen.log.search
                } else {
                    &mut self.restore_screen.log.search
                };
                search.enablement.choice = filter;
                Command::none()
            }
            Message::EditedSearchFilterChange(filter) => {
                let search = if self.screen == Screen::Backup {
                    &mut self.backup_screen.log.search
                } else {
                    &mut self.restore_screen.log.search
                };
                search.change.choice = filter;
                Command::none()
            }
            Message::EditedSortKey { screen, value } => {
                match screen {
                    Screen::Backup => {
                        self.config.backup.sort.key = value;
                        self.backup_screen.log.sort(&self.config.backup.sort, &self.config);
                    }
                    Screen::Restore => {
                        self.config.restore.sort.key = value;
                        self.restore_screen.log.sort(&self.config.restore.sort, &self.config);
                    }
                    _ => {}
                }
                self.save_config();
                Command::none()
            }
            Message::EditedSortReversed { screen, value } => {
                match screen {
                    Screen::Backup => {
                        self.config.backup.sort.reversed = value;
                        self.backup_screen.log.sort(&self.config.backup.sort, &self.config);
                    }
                    Screen::Restore => {
                        self.config.restore.sort.reversed = value;
                        self.restore_screen.log.sort(&self.config.restore.sort, &self.config);
                    }
                    _ => {}
                }
                self.save_config();
                Command::none()
            }
            Message::BrowseDir(subject) => Command::perform(
                async move { native_dialog::FileDialog::new().show_open_single_dir() },
                move |choice| match choice {
                    Ok(Some(path)) => match subject {
                        BrowseSubject::BackupTarget => Message::EditedBackupTarget(crate::path::render_pathbuf(&path)),
                        BrowseSubject::RestoreSource => {
                            Message::EditedRestoreSource(crate::path::render_pathbuf(&path))
                        }
                        BrowseSubject::Root(i) => Message::EditedRoot(EditAction::Change(
                            i,
                            globetter::Pattern::escape(&crate::path::render_pathbuf(&path)),
                        )),
                        BrowseSubject::RedirectSource(i) => Message::EditedRedirect(
                            EditAction::Change(i, crate::path::render_pathbuf(&path)),
                            Some(RedirectEditActionField::Source),
                        ),
                        BrowseSubject::RedirectTarget(i) => Message::EditedRedirect(
                            EditAction::Change(i, crate::path::render_pathbuf(&path)),
                            Some(RedirectEditActionField::Target),
                        ),
                        BrowseSubject::CustomGameFile(i, j) => Message::EditedCustomGameFile(
                            i,
                            EditAction::Change(j, globetter::Pattern::escape(&crate::path::render_pathbuf(&path))),
                        ),
                        BrowseSubject::BackupFilterIgnoredPath(i) => Message::EditedBackupFilterIgnoredPath(
                            EditAction::Change(i, crate::path::render_pathbuf(&path)),
                        ),
                    },
                    Ok(None) => Message::Ignore,
                    Err(_) => Message::BrowseDirFailure,
                },
            ),
            Message::BrowseFile(subject) => Command::perform(
                async move { native_dialog::FileDialog::new().show_open_single_file() },
                move |choice| match choice {
                    Ok(Some(path)) => Message::SelectedFile(subject, StrictPath::from(path)),
                    Ok(None) => Message::Ignore,
                    Err(_) => Message::BrowseDirFailure,
                },
            ),
            Message::BrowseDirFailure => self.show_modal(Modal::Error {
                variant: Error::UnableToBrowseFileSystem,
            }),
            Message::SelectedFile(subject, path) => {
                match subject {
                    BrowseFileSubject::RcloneExecutable => {
                        self.text_histories.rclone_executable.push(&path.raw());
                        self.config.apps.rclone.path = path;
                    }
                    BrowseFileSubject::SecondaryManifest(i) => {
                        self.text_histories.secondary_manifests[i].push(&path.raw());
                        self.config.manifest.secondary[i].set(path.raw());
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::SelectAllGames => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &self.backup_screen.log.entries {
                            self.config.enable_game_for_backup(&entry.scan_info.game_name);
                        }
                    }
                    Screen::Restore => {
                        for entry in &self.restore_screen.log.entries {
                            self.config.enable_game_for_restore(&entry.scan_info.game_name);
                        }
                    }
                    Screen::CustomGames => {
                        for i in 0..self.config.custom_games.len() {
                            self.config.enable_custom_game(i);
                        }
                    }
                    _ => {}
                }
                self.save_config();
                Command::none()
            }
            Message::DeselectAllGames => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &self.backup_screen.log.entries {
                            self.config.disable_game_for_backup(&entry.scan_info.game_name);
                        }
                    }
                    Screen::Restore => {
                        for entry in &self.restore_screen.log.entries {
                            self.config.disable_game_for_restore(&entry.scan_info.game_name);
                        }
                    }
                    Screen::CustomGames => {
                        for i in 0..self.config.custom_games.len() {
                            self.config.disable_custom_game(i);
                        }
                    }
                    _ => {}
                }
                self.save_config();
                Command::none()
            }
            Message::OpenDir { path } => {
                let path2 = path.clone();
                Command::perform(async move { opener::open(path.resolve()) }, move |res| match res {
                    Ok(_) => Message::Ignore,
                    Err(e) => {
                        log::error!("Unable to open directory: `{}` - {:?}", path2.resolve(), e);
                        Message::OpenDirFailure { path: path2 }
                    }
                })
            }
            Message::OpenDirSubject(subject) => {
                let path = match subject {
                    BrowseSubject::BackupTarget => self.config.backup.path.clone(),
                    BrowseSubject::RestoreSource => self.config.restore.path.clone(),
                    BrowseSubject::Root(i) => self.config.roots[i].path.clone(),
                    BrowseSubject::RedirectSource(i) => self.config.redirects[i].source.clone(),
                    BrowseSubject::RedirectTarget(i) => self.config.redirects[i].target.clone(),
                    BrowseSubject::CustomGameFile(i, j) => {
                        StrictPath::new(self.config.custom_games[i].files[j].clone())
                    }
                    BrowseSubject::BackupFilterIgnoredPath(i) => self.config.backup.filter.ignored_paths[i].clone(),
                };

                match path.parent_if_file() {
                    Ok(path) => self.update(Message::OpenDir { path }),
                    Err(_) => self.show_error(Error::UnableToOpenDir(path)),
                }
            }
            Message::OpenFileSubject(subject) => {
                let path = match subject {
                    BrowseFileSubject::RcloneExecutable => self.config.apps.rclone.path.clone(),
                    BrowseFileSubject::SecondaryManifest(i) => {
                        let Some(path) = self.config.manifest.secondary[i].path() else {
                            return Command::none();
                        };
                        path.clone()
                    }
                };

                match path.parent_if_file() {
                    Ok(path) => self.update(Message::OpenDir { path }),
                    Err(_) => self.show_error(Error::UnableToOpenDir(path)),
                }
            }
            Message::OpenDirFailure { path } => self.show_modal(Modal::Error {
                variant: Error::UnableToOpenDir(path),
            }),
            Message::OpenUrlFailure { url } => self.show_modal(Modal::Error {
                variant: Error::UnableToOpenUrl(url),
            }),
            Message::KeyboardEvent(event) => {
                if let iced::keyboard::Event::ModifiersChanged(modifiers) = event {
                    self.modifiers = modifiers;
                }
                match event {
                    iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
                        modifiers,
                        ..
                    } => {
                        if modifiers.shift() {
                            iced::widget::focus_previous()
                        } else {
                            iced::widget::focus_next()
                        }
                    }
                    _ => Command::none(),
                }
            }
            Message::UndoRedo(action, subject) => {
                let shortcut = Shortcut::from(action);
                match subject {
                    UndoSubject::BackupTarget => shortcut.apply_to_strict_path_field(
                        &mut self.config.backup.path,
                        &mut self.text_histories.backup_target,
                    ),
                    UndoSubject::RestoreSource => shortcut.apply_to_strict_path_field(
                        &mut self.config.restore.path,
                        &mut self.text_histories.restore_source,
                    ),
                    UndoSubject::BackupSearchGameName => shortcut.apply_to_string_field(
                        &mut self.backup_screen.log.search.game_name,
                        &mut self.text_histories.backup_search_game_name,
                    ),
                    UndoSubject::RestoreSearchGameName => shortcut.apply_to_string_field(
                        &mut self.restore_screen.log.search.game_name,
                        &mut self.text_histories.restore_search_game_name,
                    ),
                    UndoSubject::Root(i) => shortcut
                        .apply_to_strict_path_field(&mut self.config.roots[i].path, &mut self.text_histories.roots[i]),
                    UndoSubject::SecondaryManifest(i) => {
                        let history = &mut self.text_histories.secondary_manifests[i];
                        match shortcut {
                            Shortcut::Undo => {
                                self.config.manifest.secondary[i].set(history.undo());
                            }
                            Shortcut::Redo => {
                                self.config.manifest.secondary[i].set(history.redo());
                            }
                        }
                    }
                    UndoSubject::RedirectSource(i) => shortcut.apply_to_strict_path_field(
                        &mut self.config.redirects[i].source,
                        &mut self.text_histories.redirects[i].source,
                    ),
                    UndoSubject::RedirectTarget(i) => shortcut.apply_to_strict_path_field(
                        &mut self.config.redirects[i].target,
                        &mut self.text_histories.redirects[i].target,
                    ),
                    UndoSubject::CustomGameName(i) => shortcut.apply_to_string_field(
                        &mut self.config.custom_games[i].name,
                        &mut self.text_histories.custom_games[i].name,
                    ),
                    UndoSubject::CustomGameAlias(i) => {
                        if let Some(alias) = self.config.custom_games[i].alias.as_mut() {
                            shortcut.apply_to_string_field(alias, &mut self.text_histories.custom_games[i].alias)
                        }
                    }
                    UndoSubject::CustomGameFile(i, j) => shortcut.apply_to_string_field(
                        &mut self.config.custom_games[i].files[j],
                        &mut self.text_histories.custom_games[i].files[j],
                    ),
                    UndoSubject::CustomGameRegistry(i, j) => shortcut.apply_to_string_field(
                        &mut self.config.custom_games[i].registry[j],
                        &mut self.text_histories.custom_games[i].registry[j],
                    ),
                    UndoSubject::BackupFilterIgnoredPath(i) => shortcut.apply_to_strict_path_field(
                        &mut self.config.backup.filter.ignored_paths[i],
                        &mut self.text_histories.backup_filter_ignored_paths[i],
                    ),
                    UndoSubject::BackupFilterIgnoredRegistry(i) => shortcut.apply_to_registry_path_field(
                        &mut self.config.backup.filter.ignored_registry[i],
                        &mut self.text_histories.backup_filter_ignored_registry[i],
                    ),
                    UndoSubject::RcloneExecutable => shortcut.apply_to_strict_path_field(
                        &mut self.config.apps.rclone.path,
                        &mut self.text_histories.rclone_executable,
                    ),
                    UndoSubject::RcloneArguments => shortcut.apply_to_string_field(
                        &mut self.config.apps.rclone.arguments,
                        &mut self.text_histories.rclone_arguments,
                    ),
                    UndoSubject::CloudRemoteId => {
                        if let Some(Remote::Custom { id }) = &mut self.config.cloud.remote {
                            shortcut.apply_to_string_field(id, &mut self.text_histories.cloud_remote_id)
                        }
                    }
                    UndoSubject::CloudPath => {
                        shortcut.apply_to_string_field(&mut self.config.cloud.path, &mut self.text_histories.cloud_path)
                    }
                    UndoSubject::ModalField(field) => {
                        match field {
                            ModalInputKind::Url => self.text_histories.modal.url.apply(shortcut),
                            ModalInputKind::Host => self.text_histories.modal.host.apply(shortcut),
                            ModalInputKind::Port => self.text_histories.modal.port.apply(shortcut),
                            ModalInputKind::Username => self.text_histories.modal.username.apply(shortcut),
                            ModalInputKind::Password => self.text_histories.modal.password.apply(shortcut),
                        }
                        return Command::none();
                    }
                    UndoSubject::BackupComment(game) => {
                        if let Some(info) = self.text_histories.backup_comments.get_mut(&game) {
                            let comment = match shortcut {
                                Shortcut::Undo => info.undo(),
                                Shortcut::Redo => info.redo(),
                            };

                            let updated = self.restore_screen.log.set_comment(&game, comment);
                            if updated {
                                self.save_backup(&game);
                            }
                        }
                    }
                }
                self.save_config();
                Command::none()
            }
            Message::EditedFullRetention(value) => {
                self.config.backup.retention.full = value;
                self.save_config();
                Command::none()
            }
            Message::EditedDiffRetention(value) => {
                self.config.backup.retention.differential = value;
                self.save_config();
                Command::none()
            }
            Message::SelectedBackupToRestore { game, backup } => {
                self.backups_to_restore.insert(game.clone(), backup.id());
                self.handle_restore(RestorePhase::Start {
                    preview: true,
                    games: Some(vec![game]),
                })
            }
            Message::SelectedLanguage(language) => {
                TRANSLATOR.set_language(language);
                self.config.language = language;
                self.save_config();
                Command::none()
            }
            Message::SelectedTheme(theme) => {
                self.config.theme = theme;
                self.save_config();
                Command::none()
            }
            Message::SelectedBackupFormat(format) => {
                self.config.backup.format.chosen = format;
                self.save_config();
                Command::none()
            }
            Message::SelectedBackupCompression(compression) => {
                self.config.backup.format.zip.compression = compression;
                self.save_config();
                Command::none()
            }
            Message::EditedCompressionLevel(value) => {
                self.config.backup.format.set_level(value);
                self.save_config();
                Command::none()
            }
            Message::ToggleBackupSettings => {
                self.backup_screen.show_settings = !self.backup_screen.show_settings;
                Command::none()
            }
            Message::ToggleCloudSynchronize => {
                self.config.cloud.synchronize = !self.config.cloud.synchronize;
                self.save_config();
                Command::none()
            }
            Message::GameAction { action, game } => match action {
                GameAction::PreviewBackup => self.handle_backup(BackupPhase::Start {
                    preview: true,
                    repair: false,
                    games: Some(vec![game]),
                }),
                GameAction::Backup { confirm } => {
                    if confirm {
                        self.handle_backup(BackupPhase::Confirm {
                            games: Some(vec![game]),
                        })
                    } else {
                        self.handle_backup(BackupPhase::Start {
                            preview: false,
                            repair: false,
                            games: Some(vec![game]),
                        })
                    }
                }
                GameAction::PreviewRestore => self.handle_restore(RestorePhase::Start {
                    preview: true,
                    games: Some(vec![game]),
                }),
                GameAction::Restore { confirm } => {
                    if confirm {
                        self.handle_restore(RestorePhase::Confirm {
                            games: Some(vec![game]),
                        })
                    } else {
                        self.handle_restore(RestorePhase::Start {
                            preview: false,
                            games: Some(vec![game]),
                        })
                    }
                }
                GameAction::Customize => self.customize_game(game),
                GameAction::Wiki => Self::open_wiki(game),
                GameAction::Comment => self.toggle_backup_comment_editor(game),
                GameAction::Lock | GameAction::Unlock => {
                    let updated = self.restore_screen.log.toggle_locked(&game);
                    if updated {
                        self.save_backup(&game);
                    }
                    Command::none()
                }
                GameAction::MakeAlias => self.customize_game_as_alias(game),
            },
            Message::Scrolled { subject, position } => {
                self.scroll_offsets.insert(subject, position);
                Command::none()
            }
            Message::Scroll { subject, position } => {
                self.scroll_offsets.insert(subject, position);
                scrollable::scroll_to(subject.id(), position)
            }
            Message::EditedBackupComment { game, comment } => {
                if let Some(info) = self.text_histories.backup_comments.get_mut(&game) {
                    info.push(&comment);
                }

                let updated = self.restore_screen.log.set_comment(&game, comment);
                if updated {
                    self.save_backup(&game);
                }

                Command::none()
            }
            Message::SetShowDeselectedGames(value) => {
                self.config.scan.show_deselected_games = value;
                self.save_config();
                Command::none()
            }
            Message::SetShowUnchangedGames(value) => {
                self.config.scan.show_unchanged_games = value;
                self.save_config();
                Command::none()
            }
            Message::SetShowUnscannedGames(value) => {
                self.config.scan.show_unscanned_games = value;
                self.save_config();
                Command::none()
            }
            Message::FilterDuplicates { restoring, game } => {
                let log = if restoring {
                    &mut self.restore_screen.log
                } else {
                    &mut self.backup_screen.log
                };
                log.filter_duplicates_of = game;
                Command::none()
            }
            Message::OverrideMaxThreads(overridden) => {
                self.config.override_threads(overridden);
                self.save_config();
                Command::none()
            }
            Message::EditedMaxThreads(threads) => {
                self.config.set_threads(threads);
                self.save_config();
                Command::none()
            }
            Message::EditedRcloneExecutable(text) => {
                self.text_histories.rclone_executable.push(&text);
                self.config.apps.rclone.path.reset(text);
                self.save_config();
                Command::none()
            }
            Message::EditedRcloneArguments(text) => {
                self.text_histories.rclone_arguments.push(&text);
                self.config.apps.rclone.arguments = text;
                self.save_config();
                Command::none()
            }
            Message::EditedCloudRemoteId(text) => {
                self.text_histories.cloud_remote_id.push(&text);
                if let Some(Remote::Custom { id }) = &mut self.config.cloud.remote {
                    *id = text;
                }
                self.save_config();
                Command::none()
            }
            Message::EditedCloudPath(text) => {
                self.text_histories.cloud_path.push(&text);
                self.config.cloud.path = text;
                self.save_config();
                Command::none()
            }
            Message::OpenUrl(url) => Self::open_url(url),
            Message::EditedCloudRemote(choice) => {
                if let Ok(remote) = Remote::try_from(choice) {
                    match &remote {
                        Remote::Custom { id } => {
                            self.text_histories.cloud_remote_id.push(id);
                            self.config.cloud.remote = Some(remote);
                            self.save_config();
                            Command::none()
                        }
                        Remote::Ftp {
                            id: _,
                            host,
                            port,
                            username,
                            password,
                        } => {
                            self.text_histories.modal.host.initialize(host.clone());
                            self.text_histories.modal.port.initialize(port.to_string());
                            self.text_histories.modal.username.initialize(username.clone());
                            self.text_histories.modal.password.initialize(password.clone());

                            self.show_modal(Modal::ConfigureFtpRemote)
                        }
                        Remote::Smb {
                            id: _,
                            host,
                            port,
                            username,
                            password,
                        } => {
                            self.text_histories.modal.host.initialize(host.clone());
                            self.text_histories.modal.port.initialize(port.to_string());
                            self.text_histories.modal.username.initialize(username.clone());
                            self.text_histories.modal.password.initialize(password.clone());

                            self.show_modal(Modal::ConfigureSmbRemote)
                        }
                        Remote::WebDav {
                            id: _,
                            url,
                            username,
                            password,
                            provider,
                        } => {
                            self.text_histories.modal.url.initialize(url.clone());
                            self.text_histories.modal.username.initialize(username.clone());
                            self.text_histories.modal.password.initialize(password.clone());

                            self.show_modal(Modal::ConfigureWebDavRemote { provider: *provider })
                        }
                        Remote::Box { .. }
                        | Remote::Dropbox { .. }
                        | Remote::GoogleDrive { .. }
                        | Remote::OneDrive { .. } => self.configure_remote(remote),
                    }
                } else {
                    self.config.cloud.remote = None;
                    self.save_config();
                    Command::none()
                }
            }
            Message::ConfigureCloudSuccess(remote) => {
                self.text_histories.clear_modal_fields();

                self.config.cloud.remote = Some(remote);
                self.save_config();
                self.close_modal()
            }
            Message::ConfigureCloudFailure(error) => {
                self.text_histories.clear_modal_fields();

                self.config.cloud.remote = None;
                self.save_config();
                self.show_error(Error::UnableToConfigureCloud(error))
            }
            Message::ConfirmSynchronizeCloud { direction } => {
                let local = self.config.backup.path.clone();

                self.show_modal(Modal::ConfirmCloudSync {
                    local: local.render(),
                    cloud: self.config.cloud.path.clone(),
                    direction,
                    changes: vec![],
                    page: 0,
                    state: CloudModalState::Initial,
                })
            }
            Message::SynchronizeCloud { direction, finality } => {
                let local = self.config.backup.path.clone();

                if let Err(e) = self.start_sync_cloud(&local, direction, finality, None, true) {
                    return self.show_error(e);
                }

                self.show_modal(Modal::ConfirmCloudSync {
                    local: local.render(),
                    cloud: self.config.cloud.path.clone(),
                    direction,
                    changes: vec![],
                    page: 0,
                    state: match finality {
                        Finality::Preview => CloudModalState::Previewing,
                        Finality::Final => CloudModalState::Syncing,
                    },
                })
            }
            Message::RcloneMonitor(event) => {
                match event {
                    rclone_monitor::Event::Ready(sender) => {
                        self.rclone_monitor_sender = Some(sender);
                    }
                    rclone_monitor::Event::Data(events) => {
                        for event in events {
                            match event {
                                crate::cloud::RcloneProcessEvent::Progress { current, max } => {
                                    self.progress.set(current, max);
                                }
                                crate::cloud::RcloneProcessEvent::Change(change) => {
                                    self.operation.add_cloud_change();
                                    if let Some(modal) = self.modal.as_mut() {
                                        modal.add_cloud_change(change);
                                    }
                                }
                            }
                        }
                    }
                    rclone_monitor::Event::Succeeded => {
                        if let Some(cmd) = self.transition_from_cloud_step() {
                            return cmd;
                        }

                        if let Some(modal) = self.modal.as_mut() {
                            self.operation = Operation::Idle;
                            self.progress.reset();
                            modal.finish_cloud_scan();
                        } else {
                            self.go_idle();
                        }
                    }
                    rclone_monitor::Event::Failed(e) => {
                        self.operation.push_error(Error::UnableToSynchronizeCloud(e.clone()));
                        if let Some(cmd) = self.transition_from_cloud_step() {
                            return cmd;
                        }

                        self.go_idle();
                        return self.show_error(Error::UnableToSynchronizeCloud(e));
                    }
                    rclone_monitor::Event::Cancelled => {
                        self.go_idle();
                    }
                }
                Command::none()
            }
            Message::EditedModalField(field) => {
                match field {
                    ModalField::Url(new) => {
                        self.text_histories.modal.url.push(&new);
                    }
                    ModalField::Host(new) => {
                        self.text_histories.modal.host.push(&new);
                    }
                    ModalField::Port(new) => {
                        self.text_histories.modal.port.push(&new);
                    }
                    ModalField::Username(new) => {
                        self.text_histories.modal.username.push(&new);
                    }
                    ModalField::Password(new) => {
                        self.text_histories.modal.password.push(&new);
                    }
                    ModalField::WebDavProvider(new) => {
                        if let Some(Modal::ConfigureWebDavRemote { provider }) = self.modal.as_mut() {
                            *provider = new;
                        }
                    }
                }
                Command::none()
            }
            Message::FinalizeRemote(remote) => self.configure_remote(remote),
            Message::ModalChangePage(page) => {
                if let Some(modal) = self.modal.as_mut() {
                    modal.set_page(page);
                }
                Command::none()
            }
            Message::ShowCustomGame { name } => {
                use crate::gui::widget::operation::container_scroll_offset;
                use iced::widget::container;

                let subject = ScrollSubject::CustomGames;

                self.scroll_offsets.remove(&subject);
                self.screen = Screen::CustomGames;

                container_scroll_offset(container::Id::new(name.clone())).map(move |offset| match offset {
                    Some(position) => Message::Scroll { subject, position },
                    None => Message::Ignore,
                })
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            iced::event::listen_with(|event, _| match event {
                iced::Event::Keyboard(event) => Some(Message::KeyboardEvent(event)),
                iced::Event::Window(iced::window::Id::MAIN, iced::window::Event::CloseRequested) => {
                    Some(Message::Exit { user: true })
                }
                _ => None,
            }),
            rclone_monitor::run().map(Message::RcloneMonitor),
        ];

        if self.timed_notification.is_some() {
            subscriptions.push(iced::time::every(Duration::from_millis(250)).map(|_| Message::PruneNotifications));
        }

        if self.progress.visible() {
            subscriptions.push(iced::time::every(Duration::from_millis(100)).map(|_| Message::UpdateTime));
        }

        if !self.pending_save.is_empty() {
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| Message::Save));
        }

        if self.exiting {
            subscriptions.push(iced::time::every(Duration::from_millis(50)).map(|_| Message::Exit { user: false }));
        }

        iced::subscription::Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element {
        if let Some(m) = &self.modal {
            return Column::new()
                .push(
                    m.view(&self.config, &self.text_histories)
                        .style(style::Container::Primary),
                )
                .push_if(self.progress.visible(), || self.progress.view(&self.operation))
                .into();
        }

        let content = Column::new()
            .align_items(Alignment::Center)
            .push(
                Row::new()
                    .padding([10, 20, 15, 20])
                    .spacing(20)
                    .push(button::nav(Screen::Backup, self.screen))
                    .push(button::nav(Screen::Restore, self.screen))
                    .push(button::nav(Screen::CustomGames, self.screen))
                    .push(button::nav(Screen::Other, self.screen)),
            )
            .push(match self.screen {
                Screen::Backup => self.backup_screen.view(
                    &self.config,
                    &self.manifest,
                    &self.operation,
                    &self.text_histories,
                    &self.modifiers,
                ),
                Screen::Restore => self.restore_screen.view(
                    &self.config,
                    &self.manifest,
                    &self.operation,
                    &self.text_histories,
                    &self.modifiers,
                ),
                Screen::CustomGames => screen::custom_games(
                    &self.config,
                    !self.operation.idle(),
                    &self.text_histories,
                    &self.modifiers,
                ),
                Screen::Other => screen::other(
                    self.updating_manifest,
                    &self.config,
                    &self.cache,
                    &self.operation,
                    &self.text_histories,
                    &self.modifiers,
                ),
            })
            .push_maybe(self.timed_notification.as_ref().map(|x| x.view()))
            .push_if(self.updating_manifest, || {
                Notification::new(TRANSLATOR.updating_manifest()).view()
            })
            .push_if(self.progress.visible(), || self.progress.view(&self.operation));

        Container::new(content).style(style::Container::Primary).into()
    }
}
