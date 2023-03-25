use iced::{executor, Alignment, Application, Command, Subscription};

use crate::{
    gui::{
        button,
        common::*,
        modal::{ModalComponent, ModalTheme},
        notification::Notification,
        screen,
        shortcuts::{Shortcut, TextHistory},
        style,
        widget::{Column, Container, Element, ProgressBar, Row},
    },
    lang::TRANSLATOR,
    prelude::{app_dir, Error, StrictPath},
    resource::{
        cache::Cache,
        config::{Config, CustomGame, RootsConfig},
        manifest::{Manifest, Store},
        ResourceFile, SaveableResourceFile,
    },
    scan::{
        back_up_game, heroic::HeroicGames, layout::BackupLayout, prepare_backup_target, registry_compat::RegistryItem,
        scan_game_for_backup, scan_game_for_restoration, BackupId, InstallDirRanking, OperationStepDecision,
        SteamShortcuts, TitleFinder,
    },
};

#[derive(Default)]
struct Progress {
    pub max: f32,
    pub current: f32,
}

#[derive(Default)]
pub struct App {
    config: Config,
    manifest: Manifest,
    cache: Cache,
    operation: Option<OngoingOperation>,
    screen: Screen,
    modal_theme: Option<ModalTheme>,
    modal: ModalComponent,
    backup_screen: screen::Backup,
    restore_screen: screen::Restore,
    operation_should_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    operation_steps: Vec<Command<Message>>,
    operation_steps_active: usize,
    progress: Progress,
    backups_to_restore: std::collections::HashMap<String, BackupId>,
    updating_manifest: bool,
    notify_on_single_game_scanned: Option<(String, Screen)>,
    timed_notification: Option<Notification>,
    scroll_offsets: std::collections::HashMap<ScrollSubject, iced_native::widget::scrollable::RelativeOffset>,
    text_histories: TextHistories,
}

impl App {
    fn go_idle(&mut self) {
        self.operation = None;
        self.operation_steps.clear();
        self.operation_steps_active = 0;
        self.modal_theme = None;
        self.progress.current = 0.0;
        self.progress.max = 0.0;
        self.operation_should_cancel
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        self.notify_on_single_game_scanned = None;
    }

    fn show_error(&mut self, error: Error) {
        self.modal_theme = Some(ModalTheme::Error { variant: error });
    }

    fn confirm_backup_start(&mut self, games: Option<Vec<String>>) -> Command<Message> {
        self.modal_theme = Some(ModalTheme::ConfirmBackup { games });
        Command::none()
    }

    fn confirm_restore_start(&mut self, games: Option<Vec<String>>) -> Command<Message> {
        self.modal_theme = Some(ModalTheme::ConfirmRestore { games });
        Command::none()
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

    fn restoring(&self) -> bool {
        match self.operation {
            Some(
                OngoingOperation::Restore
                | OngoingOperation::CancelRestore
                | OngoingOperation::PreviewRestore
                | OngoingOperation::CancelPreviewRestore,
            ) => true,
            None
            | Some(
                OngoingOperation::Backup
                | OngoingOperation::CancelBackup
                | OngoingOperation::PreviewBackup
                | OngoingOperation::CancelPreviewBackup,
            ) => false,
        }
    }

    fn register_notify_on_single_game_scanned(&mut self, games: &Option<Vec<String>>) {
        if let Some(games) = &games {
            if games.len() == 1 {
                self.notify_on_single_game_scanned = Some((games[0].clone(), self.screen));
            }
        }
    }

    fn handle_notify_on_single_game_scanned(&mut self) {
        if let Some((name, screen)) = self.notify_on_single_game_scanned.as_ref() {
            let log = if self.restoring() {
                &self.restore_screen.log
            } else {
                &self.backup_screen.log
            };
            let found = log.entries.iter().any(|x| &x.scan_info.game_name == name);

            if *screen != Screen::CustomGames && found {
                return;
            }

            let msg = TRANSLATOR.notify_single_game_status(found);
            self.timed_notification = Some(Notification::new(msg).expires(3));
        }
    }

    fn start_backup(&mut self, preview: bool, games: Option<Vec<String>>) -> Command<Message> {
        if self.operation.is_some() {
            return Command::none();
        }
        self.invalidate_path_caches();
        self.timed_notification = None;

        let full = games.is_none();

        let mut all_games = self.manifest.clone();
        for custom_game in &self.config.custom_games {
            if custom_game.ignore {
                continue;
            }
            all_games.add_custom_game(custom_game.clone());
        }

        if preview && full {
            self.backup_screen.previewed_games.clear();
        }

        let layout = std::sync::Arc::new(BackupLayout::new(
            self.config.backup.path.clone(),
            self.config.backup.retention.clone(),
        ));
        let title_finder = TitleFinder::new(&all_games, &layout);

        if let Some(games) = &games {
            all_games.0.retain(|k, _| games.contains(k));
        } else if !self.backup_screen.previewed_games.is_empty() && !self.backup_screen.log.contains_unscanned_games() {
            all_games
                .0
                .retain(|k, _| self.backup_screen.previewed_games.contains(k));
        }

        let subjects: Vec<_> = all_games.0.keys().cloned().collect();
        if subjects.is_empty() {
            if let Some(games) = &games {
                for game in games {
                    let duplicates = self.backup_screen.duplicate_detector.remove_game(game);
                    self.backup_screen.log.remove_game(
                        game,
                        &self.config,
                        &self.backup_screen.duplicate_detector,
                        &duplicates,
                    );
                }
                self.cache.backup.recent_games.retain(|x| !games.contains(x));
                self.cache.save();
            }
            return Command::none();
        }

        if let Some(games) = &games {
            self.backup_screen.log.unscan_games(games);
        } else {
            self.backup_screen.log.clear();
            self.backup_screen.duplicate_detector.clear();
        }
        self.modal_theme = None;
        self.progress.current = 0.0;
        self.progress.max = all_games.0.len() as f32;

        self.operation = Some(if preview {
            OngoingOperation::PreviewBackup
        } else {
            OngoingOperation::Backup
        });

        log::info!("beginning backup with {} steps", self.progress.max);

        self.register_notify_on_single_game_scanned(&games);

        let config = std::sync::Arc::new(self.config.clone());
        let roots = std::sync::Arc::new(config.expanded_roots());
        let heroic_games = std::sync::Arc::new(HeroicGames::scan(&roots, &title_finder, None));
        let filter = std::sync::Arc::new(self.config.backup.filter.clone());
        let ranking = std::sync::Arc::new(InstallDirRanking::scan(&roots, &all_games, &subjects));
        let steam_shortcuts = std::sync::Arc::new(SteamShortcuts::scan());

        for key in subjects {
            let game = all_games.0[&key].clone();
            let config = config.clone();
            let roots = roots.clone();
            let heroic_games = heroic_games.clone();
            let layout = layout.clone();
            let filter = filter.clone();
            let ranking = ranking.clone();
            let steam_shortcuts = steam_shortcuts.clone();
            let steam_id = game.steam.as_ref().and_then(|x| x.id);
            let cancel_flag = self.operation_should_cancel.clone();
            let merge = self.config.backup.merge;
            self.operation_steps.push(Command::perform(
                async move {
                    if key.trim().is_empty() {
                        return (None, None, OperationStepDecision::Ignored);
                    }
                    if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        // TODO: https://github.com/hecrj/iced/issues/436
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        return (None, None, OperationStepDecision::Cancelled);
                    }

                    let previous = layout.latest_backup(&key, false, &config.redirects);

                    let scan_info = scan_game_for_backup(
                        &game,
                        &key,
                        &roots,
                        &StrictPath::from_std_path_buf(&app_dir()),
                        &heroic_games,
                        &steam_id,
                        &filter,
                        &None,
                        &ranking,
                        &config.backup.toggled_paths,
                        &config.backup.toggled_registry,
                        previous,
                        &config.redirects,
                        &steam_shortcuts,
                    );
                    if !config.is_game_enabled_for_backup(&key) {
                        return (Some(scan_info), None, OperationStepDecision::Ignored);
                    }

                    let backup_info = if !preview {
                        Some(back_up_game(
                            &scan_info,
                            layout.game_layout(&key),
                            merge,
                            &chrono::Utc::now(),
                            &config.backup.format,
                        ))
                    } else {
                        None
                    };
                    (Some(scan_info), backup_info, OperationStepDecision::Processed)
                },
                move |(scan_info, backup_info, decision)| Message::BackupStep {
                    scan_info,
                    backup_info,
                    decision,
                    preview,
                    full,
                },
            ));
        }

        self.operation_steps_active = 100.min(self.operation_steps.len());
        Command::batch(self.operation_steps.drain(..self.operation_steps_active))
    }

    fn start_restore(&mut self, preview: bool, games: Option<Vec<String>>) -> Command<Message> {
        if self.operation.is_some() {
            return Command::none();
        }
        self.invalidate_path_caches();
        self.timed_notification = None;

        let full = games.is_none();

        let restore_path = &self.config.restore.path;
        if !restore_path.is_dir() {
            self.modal_theme = Some(ModalTheme::Error {
                variant: Error::RestorationSourceInvalid {
                    path: restore_path.clone(),
                },
            });
            return Command::none();
        }

        let config = std::sync::Arc::new(self.config.clone());
        let layout = std::sync::Arc::new(BackupLayout::new(restore_path.clone(), config.backup.retention.clone()));
        let mut restorables = layout.restorable_games();

        if let Some(games) = &games {
            restorables.retain(|v| games.contains(v));
            self.restore_screen.log.unscan_games(games);
        } else {
            self.restore_screen.log.clear();
            self.restore_screen.duplicate_detector.clear();
        }
        self.modal_theme = None;

        if restorables.is_empty() {
            if let Some(games) = &games {
                for game in games {
                    let duplicates = self.restore_screen.duplicate_detector.remove_game(game);
                    self.restore_screen.log.remove_game(
                        game,
                        &self.config,
                        &self.restore_screen.duplicate_detector,
                        &duplicates,
                    );
                }
                self.cache.restore.recent_games.retain(|x| !games.contains(x));
                self.cache.save();
            }
            return Command::none();
        }

        self.operation = Some(if preview {
            OngoingOperation::PreviewRestore
        } else {
            OngoingOperation::Restore
        });
        self.progress.current = 0.0;
        self.progress.max = restorables.len() as f32;

        log::info!("beginning restore with {} steps", self.progress.max);

        self.register_notify_on_single_game_scanned(&games);

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
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        return (None, None, OperationStepDecision::Cancelled, layout);
                    }

                    let scan_info = scan_game_for_restoration(&name, &backup_id, &mut layout, &config.redirects);
                    if !config.is_game_enabled_for_restore(&name) {
                        return (Some(scan_info), None, OperationStepDecision::Ignored, layout);
                    }

                    let backup_info = if scan_info.backup.is_some() && !preview {
                        Some(layout.restore(&scan_info))
                    } else {
                        None
                    };
                    (Some(scan_info), backup_info, OperationStepDecision::Processed, layout)
                },
                move |(scan_info, backup_info, decision, game_layout)| Message::RestoreStep {
                    scan_info,
                    backup_info,
                    decision,
                    full,
                    game_layout,
                },
            ));
        }

        self.operation_steps_active = 100.min(self.operation_steps.len());
        Command::batch(self.operation_steps.drain(..self.operation_steps_active))
    }

    fn complete_backup(&mut self, preview: bool, full: bool) {
        log::info!("completed backup");
        let mut failed = false;

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

        self.cache.save();

        self.go_idle();

        if failed {
            self.modal_theme = Some(ModalTheme::Error {
                variant: Error::SomeEntriesFailed,
            });
        }
    }

    fn complete_restore(&mut self, full: bool) {
        log::info!("completed restore");
        let mut failed = false;

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

        self.cache.save();

        self.go_idle();

        if failed {
            self.modal_theme = Some(ModalTheme::Error {
                variant: Error::SomeEntriesFailed,
            });
        }
    }

    fn customize_game(&mut self, name: String) -> Command<Message> {
        let game = if let Some(standard) = self.manifest.0.get(&name) {
            CustomGame {
                name: name.clone(),
                ignore: false,
                files: standard.files.clone().unwrap_or_default().keys().cloned().collect(),
                registry: standard.registry.clone().unwrap_or_default().keys().cloned().collect(),
            }
        } else {
            CustomGame {
                name: name.clone(),
                ignore: false,
                files: vec![],
                registry: vec![],
            }
        };

        self.config.custom_games.push(game);
        self.config.save();

        self.switch_screen(Screen::CustomGames)
    }

    fn open_wiki(game: String) -> Command<Message> {
        let url = format!("https://www.pcgamingwiki.com/wiki/{}", game.replace(' ', "_"));
        let url2 = url.clone();
        Command::perform(async { opener::open(url) }, move |res| match res {
            Ok(_) => Message::Ignore,
            Err(_) => Message::OpenUrlFailure { url: url2 },
        })
    }

    fn toggle_backup_comment_editor(&mut self, name: String) -> Command<Message> {
        self.restore_screen.log.toggle_backup_comment_editor(&name);
        Command::none()
    }

    fn switch_screen(&mut self, screen: Screen) -> Command<Message> {
        self.screen = screen;
        let subject = ScrollSubject::from(screen);

        if let Some(offset) = self.scroll_offsets.get(&subject) {
            iced::widget::scrollable::snap_to(subject.id(), *offset)
        } else {
            Command::none()
        }
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = crate::gui::style::Theme;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut modal_theme: Option<ModalTheme> = None;
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(ModalTheme::Error { variant: x });
                let _ = Config::archive_invalid();
                Config::default()
            }
        };
        TRANSLATOR.set_language(config.language);
        let mut cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
        let manifest = match Manifest::load() {
            Ok(y) => y,
            Err(_) => {
                modal_theme = Some(ModalTheme::UpdatingManifest);
                Manifest::default()
            }
        };

        let missing: Vec<_> = config
            .find_missing_roots()
            .iter()
            .filter(|x| !cache.has_root(x))
            .cloned()
            .collect();
        if !missing.is_empty() {
            cache.add_roots(&missing);
            cache.save();
            modal_theme = Some(ModalTheme::ConfirmAddMissingRoots(missing));
        }

        let manifest_config = config.manifest.clone();
        let manifest_cache = cache.manifests.clone();
        let text_histories = TextHistories::new(&config);

        log::debug!("Config on startup: {config:?}");

        (
            Self {
                backup_screen: screen::Backup::new(&config, &cache),
                restore_screen: screen::Restore::new(&config, &cache),
                config,
                manifest,
                cache,
                modal_theme,
                updating_manifest: true,
                text_histories,
                ..Self::default()
            },
            Command::perform(
                async move {
                    tokio::task::spawn_blocking(move || Manifest::update(manifest_config, manifest_cache, false)).await
                },
                |join| match join {
                    Ok(x) => Message::ManifestUpdated(x),
                    Err(_) => Message::Ignore,
                },
            ),
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
            Message::Error(error) => {
                self.show_error(error);
                Command::none()
            }
            Message::CloseModal => {
                self.modal_theme = None;
                Command::none()
            }
            Message::Exit => std::process::exit(0),
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
            Message::ManifestUpdated(updated) => {
                self.updating_manifest = false;

                let updated = match updated {
                    Ok(Some(updated)) => updated,
                    Ok(None) => return Command::none(),
                    Err(e) => {
                        self.show_error(e);
                        return Command::none();
                    }
                };

                if self.modal_theme == Some(ModalTheme::UpdatingManifest) {
                    self.modal_theme = None;
                }

                self.cache.update_manifest(updated);
                self.cache.save();

                match Manifest::load() {
                    Ok(x) => {
                        self.manifest = x;
                    }
                    Err(variant) => {
                        self.modal_theme = Some(ModalTheme::Error { variant });
                    }
                }
                Command::none()
            }
            Message::ConfirmBackupStart { games } => self.confirm_backup_start(games),
            Message::ConfirmRestoreStart { games } => self.confirm_restore_start(games),
            Message::BackupPrep { preview, games } => {
                if self.operation.is_some() {
                    return Command::none();
                }

                if preview {
                    return self.start_backup(preview, games);
                }

                self.modal_theme = Some(ModalTheme::PreparingBackupDir);

                let backup_path = self.config.backup.path.clone();
                let merge = if games.is_some() {
                    true
                } else {
                    self.config.backup.merge
                };

                Command::perform(
                    async move { prepare_backup_target(&backup_path, merge) },
                    move |result| match result {
                        Ok(_) => Message::BackupStart { preview, games },
                        Err(e) => Message::Error(e),
                    },
                )
            }
            Message::BackupStart { preview, games } => self.start_backup(preview, games),
            Message::RestoreStart { preview, games } => self.start_restore(preview, games),
            Message::BackupStep {
                scan_info,
                backup_info,
                decision: _,
                preview,
                full,
            } => {
                self.progress.current += 1.0;

                if let Some(scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    if scan_info.found_anything() {
                        let duplicates = self.backup_screen.duplicate_detector.add_game(&scan_info);
                        self.backup_screen.previewed_games.insert(scan_info.game_name.clone());
                        self.backup_screen.log.update_game(
                            scan_info,
                            backup_info,
                            &self.config.backup.sort,
                            &self.config,
                            &self.backup_screen.duplicate_detector,
                            &duplicates,
                            None,
                        );
                    } else if !full {
                        let duplicates = self.backup_screen.duplicate_detector.remove_game(&scan_info.game_name);
                        self.backup_screen.log.remove_game(
                            &scan_info.game_name,
                            &self.config,
                            &self.backup_screen.duplicate_detector,
                            &duplicates,
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
                            self.complete_backup(preview, full);
                        }
                        Command::none()
                    }
                }
            }
            Message::RestoreStep {
                scan_info,
                backup_info,
                decision: _,
                full,
                game_layout,
            } => {
                self.progress.current += 1.0;

                if let Some(scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    if scan_info.found_anything() {
                        let duplicates = self.restore_screen.duplicate_detector.add_game(&scan_info);
                        self.restore_screen.log.update_game(
                            scan_info,
                            backup_info,
                            &self.config.backup.sort,
                            &self.config,
                            &self.restore_screen.duplicate_detector,
                            &duplicates,
                            Some(game_layout),
                        );
                    } else if !full {
                        let duplicates = self.restore_screen.duplicate_detector.remove_game(&scan_info.game_name);
                        self.restore_screen.log.remove_game(
                            &scan_info.game_name,
                            &self.config,
                            &self.restore_screen.duplicate_detector,
                            &duplicates,
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
                            self.complete_restore(full);
                        }
                        Command::none()
                    }
                }
            }
            Message::CancelOperation => {
                self.operation_should_cancel
                    .swap(true, std::sync::atomic::Ordering::Relaxed);
                self.operation_steps.clear();
                match self.operation {
                    Some(OngoingOperation::Backup) => {
                        self.operation = Some(OngoingOperation::CancelBackup);
                    }
                    Some(OngoingOperation::PreviewBackup) => {
                        self.operation = Some(OngoingOperation::CancelPreviewBackup);
                    }
                    Some(OngoingOperation::Restore) => {
                        self.operation = Some(OngoingOperation::CancelRestore);
                    }
                    Some(OngoingOperation::PreviewRestore) => {
                        self.operation = Some(OngoingOperation::CancelPreviewRestore);
                    }
                    _ => {}
                };
                Command::none()
            }
            Message::EditedBackupTarget(text) => {
                self.text_histories.backup_target.push(&text);
                self.config.backup.path.reset(text);
                self.config.save();
                Command::none()
            }
            Message::EditedBackupMerge(enabled) => {
                self.config.backup.merge = enabled;
                self.config.save();
                Command::none()
            }
            Message::EditedRestoreSource(text) => {
                self.text_histories.restore_source.push(&text);
                self.config.restore.path.reset(text);
                self.config.save();
                Command::none()
            }
            Message::FindRoots => {
                let missing = self.config.find_missing_roots();
                if missing.is_empty() {
                    self.modal_theme = Some(ModalTheme::NoMissingRoots);
                } else {
                    self.cache.add_roots(&missing);
                    self.cache.save();
                    self.modal_theme = Some(ModalTheme::ConfirmAddMissingRoots(missing));
                }
                Command::none()
            }
            Message::ConfirmAddMissingRoots(missing) => {
                for root in missing {
                    self.text_histories.roots.push(TextHistory::raw(&root.path.render()));
                    self.config.roots.push(root);
                }
                self.config.save();
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
                self.config.save();
                Command::none()
            }
            Message::SelectedRootStore(index, store) => {
                self.config.roots[index].store = store;
                self.config.save();
                Command::none()
            }
            Message::SelectedRedirectKind(index, kind) => {
                self.config.redirects[index].kind = kind;
                self.config.save();
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
                self.config.save();
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
                self.config.save();
                if snap {
                    self.scroll_offsets.insert(
                        ScrollSubject::CustomGames,
                        iced_native::widget::scrollable::RelativeOffset::END,
                    );
                    iced::widget::scrollable::snap_to(
                        crate::gui::widget::id::custom_games_scroll(),
                        iced::widget::scrollable::RelativeOffset::END,
                    )
                } else {
                    Command::none()
                }
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
                self.config.save();
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
                self.config.save();
                Command::none()
            }
            Message::EditedExcludeStoreScreenshots(enabled) => {
                self.config.backup.filter.exclude_store_screenshots = enabled;
                self.config.save();
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
                self.config.save();
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
                self.config.save();
                Command::none()
            }
            Message::SwitchScreen(screen) => self.switch_screen(screen),
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        self.backup_screen.log.toggle_game_expanded(
                            &name,
                            &self.config,
                            &self.backup_screen.duplicate_detector,
                        );
                    }
                    Screen::Restore => {
                        self.restore_screen.log.toggle_game_expanded(
                            &name,
                            &self.config,
                            &self.restore_screen.duplicate_detector,
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
                                entry.tree.expand_or_collapse_keys(&keys);
                            }
                        }
                    }
                    Screen::Restore => {
                        for entry in &mut self.restore_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                entry.tree.expand_or_collapse_keys(&keys);
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
                self.config.save();
                Command::none()
            }
            Message::ToggleCustomGameEnabled { index, enabled } => {
                if enabled {
                    self.config.enable_custom_game(index);
                } else {
                    self.config.disable_custom_game(index);
                }
                self.config.save();
                Command::none()
            }
            Message::ToggleSearch { screen } => {
                match screen {
                    Screen::Backup => {
                        self.backup_screen.log.search.show = !self.backup_screen.log.search.show;
                    }
                    Screen::Restore => {
                        self.restore_screen.log.search.show = !self.restore_screen.log.search.show;
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::ToggleSpecificBackupPathIgnored { name, path, .. } => {
                self.config.backup.toggled_paths.toggle(&name, &path);
                self.config.save();
                self.backup_screen.log.update_ignored(
                    &name,
                    &self.config.backup.toggled_paths,
                    &self.config.backup.toggled_registry,
                );
                Command::none()
            }
            Message::ToggleSpecificBackupRegistryIgnored { name, path, value, .. } => {
                self.config.backup.toggled_registry.toggle_owned(&name, &path, value);
                self.config.save();
                self.backup_screen.log.update_ignored(
                    &name,
                    &self.config.backup.toggled_paths,
                    &self.config.backup.toggled_registry,
                );
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
            Message::EditedSortKey { screen, value } => {
                match screen {
                    Screen::Backup => {
                        self.config.backup.sort.key = value;
                        self.backup_screen.log.sort(&self.config.backup.sort);
                    }
                    Screen::Restore => {
                        self.config.restore.sort.key = value;
                        self.restore_screen.log.sort(&self.config.restore.sort);
                    }
                    _ => {}
                }
                self.config.save();
                Command::none()
            }
            Message::EditedSortReversed { screen, value } => {
                match screen {
                    Screen::Backup => {
                        self.config.backup.sort.reversed = value;
                        self.backup_screen.log.sort(&self.config.backup.sort);
                    }
                    Screen::Restore => {
                        self.config.restore.sort.reversed = value;
                        self.restore_screen.log.sort(&self.config.restore.sort);
                    }
                    _ => {}
                }
                self.config.save();
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
            Message::BrowseDirFailure => {
                self.modal_theme = Some(ModalTheme::Error {
                    variant: Error::UnableToBrowseFileSystem,
                });
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
                self.config.save();
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
                self.config.save();
                Command::none()
            }
            Message::OpenDir { path } => {
                let path2 = path.clone();
                Command::perform(async move { opener::open(path.interpret()) }, move |res| match res {
                    Ok(_) => Message::Ignore,
                    Err(_) => Message::OpenDirFailure { path: path2 },
                })
            }
            Message::OpenDirFailure { path } => {
                self.modal_theme = Some(ModalTheme::Error {
                    variant: Error::UnableToOpenDir(path),
                });
                Command::none()
            }
            Message::OpenUrlFailure { url } => {
                self.modal_theme = Some(ModalTheme::Error {
                    variant: Error::UnableToOpenUrl(url),
                });
                Command::none()
            }
            Message::KeyboardEvent(event) => {
                if let iced::keyboard::Event::ModifiersChanged(modifiers) = event {
                    self.backup_screen.log.modifiers = modifiers;
                    self.restore_screen.log.modifiers = modifiers;
                }
                Command::none()
            }
            Message::UndoRedo(action, subject) => {
                let shortcut = Shortcut::from(action);
                match subject {
                    UndoSubject::BackupTarget => apply_shortcut_to_strict_path_field(
                        &shortcut,
                        &mut self.config.backup.path,
                        &mut self.text_histories.backup_target,
                    ),
                    UndoSubject::RestoreSource => apply_shortcut_to_strict_path_field(
                        &shortcut,
                        &mut self.config.restore.path,
                        &mut self.text_histories.restore_source,
                    ),
                    UndoSubject::BackupSearchGameName => apply_shortcut_to_string_field(
                        &shortcut,
                        &mut self.backup_screen.log.search.game_name,
                        &mut self.text_histories.backup_search_game_name,
                    ),
                    UndoSubject::RestoreSearchGameName => apply_shortcut_to_string_field(
                        &shortcut,
                        &mut self.restore_screen.log.search.game_name,
                        &mut self.text_histories.restore_search_game_name,
                    ),
                    UndoSubject::Root(i) => apply_shortcut_to_strict_path_field(
                        &shortcut,
                        &mut self.config.roots[i].path,
                        &mut self.text_histories.roots[i],
                    ),
                    UndoSubject::RedirectSource(i) => apply_shortcut_to_strict_path_field(
                        &shortcut,
                        &mut self.config.redirects[i].source,
                        &mut self.text_histories.redirects[i].source,
                    ),
                    UndoSubject::RedirectTarget(i) => apply_shortcut_to_strict_path_field(
                        &shortcut,
                        &mut self.config.redirects[i].target,
                        &mut self.text_histories.redirects[i].target,
                    ),
                    UndoSubject::CustomGameName(i) => apply_shortcut_to_string_field(
                        &shortcut,
                        &mut self.config.custom_games[i].name,
                        &mut self.text_histories.custom_games[i].name,
                    ),
                    UndoSubject::CustomGameFile(i, j) => apply_shortcut_to_string_field(
                        &shortcut,
                        &mut self.config.custom_games[i].files[j],
                        &mut self.text_histories.custom_games[i].files[j],
                    ),
                    UndoSubject::CustomGameRegistry(i, j) => apply_shortcut_to_string_field(
                        &shortcut,
                        &mut self.config.custom_games[i].registry[j],
                        &mut self.text_histories.custom_games[i].registry[j],
                    ),
                    UndoSubject::BackupFilterIgnoredPath(i) => apply_shortcut_to_strict_path_field(
                        &shortcut,
                        &mut self.config.backup.filter.ignored_paths[i],
                        &mut self.text_histories.backup_filter_ignored_paths[i],
                    ),
                    UndoSubject::BackupFilterIgnoredRegistry(i) => apply_shortcut_to_registry_path_field(
                        &shortcut,
                        &mut self.config.backup.filter.ignored_registry[i],
                        &mut self.text_histories.backup_filter_ignored_registry[i],
                    ),
                }
                self.config.save();
                Command::none()
            }
            Message::EditedFullRetention(value) => {
                self.config.backup.retention.full = value;
                self.config.save();
                Command::none()
            }
            Message::EditedDiffRetention(value) => {
                self.config.backup.retention.differential = value;
                self.config.save();
                Command::none()
            }
            Message::SelectedBackupToRestore { game, backup } => {
                self.backups_to_restore.insert(game.clone(), backup.id());
                self.start_restore(true, Some(vec![game]))
            }
            Message::SelectedLanguage(language) => {
                TRANSLATOR.set_language(language);
                self.config.language = language;
                self.config.save();
                Command::none()
            }
            Message::SelectedTheme(theme) => {
                self.config.theme = theme;
                self.config.save();
                Command::none()
            }
            Message::SelectedBackupFormat(format) => {
                self.config.backup.format.chosen = format;
                self.config.save();
                Command::none()
            }
            Message::SelectedBackupCompression(compression) => {
                self.config.backup.format.zip.compression = compression;
                self.config.save();
                Command::none()
            }
            Message::EditedCompressionLevel(value) => {
                self.config.backup.format.set_level(value);
                self.config.save();
                Command::none()
            }
            Message::ToggleBackupSettings => {
                self.backup_screen.show_settings = !self.backup_screen.show_settings;
                Command::none()
            }
            Message::GameAction { action, game } => match action {
                GameAction::PreviewBackup => self.start_backup(true, Some(vec![game])),
                GameAction::Backup { confirm } => {
                    if confirm {
                        self.confirm_backup_start(Some(vec![game]))
                    } else {
                        self.start_backup(false, Some(vec![game]))
                    }
                }
                GameAction::PreviewRestore => self.start_restore(true, Some(vec![game])),
                GameAction::Restore { confirm } => {
                    if confirm {
                        self.confirm_restore_start(Some(vec![game]))
                    } else {
                        self.start_restore(false, Some(vec![game]))
                    }
                }
                GameAction::Customize => self.customize_game(game),
                GameAction::Wiki => Self::open_wiki(game),
                GameAction::Comment => self.toggle_backup_comment_editor(game),
            },
            Message::Scroll { subject, position } => {
                self.scroll_offsets.insert(subject, position);
                Command::none()
            }
            Message::EditedBackupComment { game, comment } => {
                self.restore_screen.log.set_comment(&game, comment);
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::Subscription::batch(vec![
            iced_native::subscription::events_with(|event, _| match event {
                iced_native::Event::Keyboard(event) => Some(event),
                _ => None,
            })
            .map(Message::KeyboardEvent),
            match self.timed_notification {
                Some(_) => {
                    iced::time::every(std::time::Duration::from_millis(250)).map(|_| Message::PruneNotifications)
                }
                None => iced_native::subscription::Subscription::none(),
            },
        ])
    }

    fn view(&self) -> Element {
        if let Some(m) = &self.modal_theme {
            return self.modal.view(m, &self.config).style(style::Container::Primary).into();
        }

        let content = Column::new()
            .align_items(Alignment::Center)
            .push(
                Row::new()
                    .padding([2, 20, 25, 20])
                    .spacing(20)
                    .push(button::nav(Screen::Backup, self.screen))
                    .push(button::nav(Screen::Restore, self.screen))
                    .push(button::nav(Screen::CustomGames, self.screen))
                    .push(button::nav(Screen::Other, self.screen)),
            )
            .push(match self.screen {
                Screen::Backup => {
                    self.backup_screen
                        .view(&self.config, &self.manifest, &self.operation, &self.text_histories)
                }
                Screen::Restore => {
                    self.restore_screen
                        .view(&self.config, &self.manifest, &self.operation, &self.text_histories)
                }
                Screen::CustomGames => {
                    screen::custom_games(&self.config, self.operation.is_some(), &self.text_histories)
                }
                Screen::Other => screen::other(self.updating_manifest, &self.config, &self.cache, &self.text_histories),
            })
            .push_some(|| self.timed_notification.as_ref().map(|x| x.view()))
            .push_if(
                || self.updating_manifest,
                || Notification::new(TRANSLATOR.updating_manifest()).view(),
            )
            .push_if(
                || self.progress.max > 1.0,
                || ProgressBar::new(0.0..=self.progress.max, self.progress.current).height(5),
            );

        Container::new(content).style(style::Container::Primary).into()
    }
}
