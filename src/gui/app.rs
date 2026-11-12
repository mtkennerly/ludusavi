use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use iced::{Alignment, Length, Subscription, Task, keyboard, widget::scrollable};

use crate::{
    cloud::{Rclone, Remote, rclone_monitor},
    gui::{
        button,
        common::{
            BackupPhase, BrowseFileSubject, BrowseSubject, Flags, GameAction, GameSelection, Message, Operation,
            RestorePhase, Screen, ScrollSubject, UndoSubject, ValidatePhase,
        },
        modal::{self, CloudModalState, Modal, ModalField, ModalInputKind},
        notification::Notification,
        screen,
        shortcuts::{RootHistory, Shortcut, TextHistories, TextHistory},
        style,
        widget::{
            Column, Container, Element, IcedParentExt, Progress, Row, Stack, id, operation::container_scroll_offset,
        },
    },
    lang::TRANSLATOR,
    prelude::{
        EditAction, Error, Finality, RedirectEditActionField, Security, StrictPath, SyncDirection, app_dir,
        get_threads_from_env, initialize_rayon,
    },
    resource::{
        ResourceFile, SaveableResourceFile,
        cache::{self, Cache},
        config::{self, Config, CustomGame, CustomGameKind, Root},
        manifest::Manifest,
    },
    scan::{
        BackupId, Launchers, ScanKind, SteamShortcuts, TitleFinder, game_filter, layout::BackupLayout,
        prepare_backup_target, registry::RegistryItem, scan_game_for_backup, semantic,
    },
};

#[cfg(windows)]
use super::background;

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

    fn block_on<T>(&self, future: impl std::prelude::rust_2024::Future<Output = T>) -> T {
        self.0.block_on(future)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SaveKind {
    Config,
    Cache,
    Backup(String),
}

#[derive(Clone, Debug, Default)]
pub struct LoadedManifest {
    pub primary: Manifest,
    pub extended: Manifest,
}

pub struct OperationStep {
    title: String,
    task: Task<Message>,
}

#[derive(Default)]
pub struct App {
    flags: Flags,
    config: Config,
    manifest: LoadedManifest,
    cache: Cache,
    operation: Operation,
    screen: Screen,
    modals: Vec<Modal>,
    backup_screen: screen::Backup,
    restore_screen: screen::Restore,
    custom_games_screen: screen::CustomGames,
    operation_should_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    operation_steps: Vec<OperationStep>,
    operation_steps_active: usize,
    progress: Progress,
    backups_to_restore: HashMap<String, BackupId>,
    updating_manifest: bool,
    notify_on_single_game_scanned: Option<(String, Screen)>,
    manifest_notification: Option<Notification>,
    timed_notification: Option<Notification>,
    scroll_offsets: HashMap<ScrollSubject, scrollable::AbsoluteOffset>,
    text_histories: TextHistories,
    active_text_input: Option<(UndoSubject, usize)>,
    rclone_monitor_sender: Option<iced::futures::channel::mpsc::Sender<rclone_monitor::Input>>,
    exiting: bool,
    pending_save: HashMap<SaveKind, Instant>,
    #[cfg(windows)]
    background_config: Option<background::Config>,
    #[cfg(windows)]
    background_instance: Option<background::Instance>,
    #[cfg(windows)]
    background_settings: background::Settings,
    #[cfg(windows)]
    background_revision: u64,
    #[cfg(windows)]
    background: Option<background::Background>,
    #[cfg(windows)]
    background_backup: Option<background::BackupRun>,
    #[cfg(windows)]
    background_last_poll: Option<Instant>,
    #[cfg(windows)]
    window: Option<iced::window::Id>,
    #[cfg(windows)]
    background_settings_window: Option<iced::window::Id>,
    #[cfg(windows)]
    hidden: bool,
    modifiers: keyboard::Modifiers,
    jump_to_game_after_scan: Option<String>,
}

impl App {
    fn go_idle(&mut self) {
        #[cfg(windows)]
        self.finish_background_backup();
        self.operation = Operation::Idle;
        if self.exiting {
            #[cfg(windows)]
            if !self.save_pending(true) {
                self.exiting = false;
                self.close_specific_modal_alt(modal::Kind::Exiting);
                let _ = self.show_error(Error::SomeEntriesFailed);
            } else {
                std::process::exit(0);
            }
            #[cfg(not(windows))]
            {
                self.save();
                std::process::exit(0);
            }
        }

        self.operation_steps.clear();
        self.operation_steps_active = 0;
        self.close_specific_modal_alt(modal::Kind::ActiveScanGames);
        self.progress.reset();
        self.operation_should_cancel
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        self.notify_on_single_game_scanned = None;
    }

    #[cfg(windows)]
    fn finish_background_backup(&mut self) -> Option<background::BackupNotice> {
        let run = self.background_backup.take()?;
        let notice = run.finished(
            self.operation.errors().map(Vec::as_slice).unwrap_or_default(),
            self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed),
        );
        if notice.failed {
            log::warn!("{}: {}", notice.title, notice.body);
        } else {
            log::info!("{}: {}", notice.title, notice.body);
        }
        if let Some(background) = &self.background {
            match background.notify_backup(&notice) {
                Ok(()) => log::info!("Windows backup result notification submitted: {}", notice.title),
                Err(why) => log::warn!("{why}"),
            }
        }
        Some(notice)
    }

    #[cfg(windows)]
    fn fail_background_backup(&mut self, error: &Error) {
        if let Some(run) = self.background_backup.as_mut() {
            run.result = background::BackupResult::Failed(TRANSLATOR.handle_error(error));
        }
        self.finish_background_backup();
    }

    fn show_modal(&mut self, modal: Modal) -> Task<Message> {
        let replace = self
            .modals
            .last()
            .map(|last| last.kind() == modal.kind() && !modal.stackable())
            .unwrap_or(false);

        if replace {
            self.modals.pop();
        }

        self.modals.push(modal);
        self.reset_scroll_position(ScrollSubject::Modal);
        self.refresh_scroll_position()
    }

    fn close_modal(&mut self) -> Task<Message> {
        if let Some(modal) = self.modals.pop() {
            self.reset_scroll_position(ScrollSubject::Modal);
            let need_cancel_cloud = modal.is_cloud_active();
            Task::batch([
                self.refresh_scroll_position(),
                if need_cancel_cloud {
                    self.cancel_operation()
                } else {
                    Task::none()
                },
            ])
        } else {
            Task::none()
        }
    }

    fn close_specific_modal(&mut self, kind: modal::Kind) -> Task<Message> {
        self.modals.retain(|modal| modal.kind() != kind);
        self.refresh_scroll_position()
    }

    fn close_specific_modal_alt(&mut self, kind: modal::Kind) {
        self.modals.retain(|modal| modal.kind() != kind);
    }

    fn show_error(&mut self, error: Error) -> Task<Message> {
        self.show_modal(Modal::Error { variant: error })
    }

    #[cfg(windows)]
    fn show_window(&mut self) -> Task<Message> {
        self.hidden = false;
        self.window
            .map(|id| {
                Task::batch([
                    iced::window::minimize(id, false),
                    iced::window::set_mode(id, iced::window::Mode::Windowed)
                        .chain(iced::window::gain_focus(id))
                        .chain(iced::window::request_user_attention(
                            id,
                            Some(iced::window::UserAttention::Informational),
                        )),
                ])
            })
            .unwrap_or_else(Task::none)
    }

    #[cfg(windows)]
    fn hide_window(&self, id: iced::window::Id) -> Task<Message> {
        if self.background.as_ref().is_some_and(background::Background::has_tray) {
            iced::window::set_mode(id, iced::window::Mode::Hidden)
        } else {
            iced::window::minimize(id, true)
        }
    }

    #[cfg(windows)]
    fn background_tick(&mut self) -> Task<Message> {
        let Some(background) = self.background.as_ref() else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        for action in background.actions() {
            match action {
                background::Action::Show => tasks.push(self.show_window()),
                background::Action::OpenCustomGame(name) => {
                    if let Some(game) = self.config.custom_games.iter_mut().find(|game| game.name == name) {
                        game.expanded = true;
                        tasks.push(self.switch_screen(Screen::CustomGames));
                        tasks.push(self.update(Message::ShowCustomGame { name }));
                    } else {
                        tasks.push(self.customize_game(name));
                    }
                    tasks.push(self.show_window());
                }
                background::Action::Pause(paused) => {
                    if let Some(background) = self.background.as_mut() {
                        background.set_paused(paused);
                    }
                }
                background::Action::Configure => {
                    tasks.push(self.open_background_settings());
                }
                background::Action::Quit => return self.update(Message::Exit { user: true }),
            }
        }
        if self.hidden
            && self
                .modals
                .iter()
                .any(|m| matches!(m.kind(), modal::Kind::Error | modal::Kind::Errors))
        {
            tasks.push(self.show_window());
        }
        let background = self.background.as_mut().unwrap();
        if !self.exiting
            && !background.paused
            && !background.polling
            && self
                .background_last_poll
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(1))
        {
            background.polling = true;
            self.background_last_poll = Some(Instant::now());
            let revision = self.background_revision;
            let processes = background.processes.clone();
            let confirm = background
                .monitor
                .confirm_discovery(self.operation.idle() && self.modals.is_empty());
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        let config = background::Config::load().map_err(|e| e.to_string())?;
                        config.validate()?;
                        let running = processes.lock().map_err(|e| e.to_string())?.running_games(
                            &config,
                            Instant::now(),
                            confirm,
                        )?;
                        Ok((config, running))
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                move |result| Message::BackgroundPolled(revision, result),
            ));
        }
        Task::batch(tasks)
    }

    #[cfg(windows)]
    fn edit_background(&mut self, edit: background::Edit) -> Task<Message> {
        use background::Edit;
        let mut config = self.background_settings.config.clone();
        match edit {
            Edit::Search(value) => {
                self.background_settings.search = value;
                return Task::none();
            }
            Edit::ChooseGame(name) => {
                if self.backup_screen.log.find_game(&name).is_none() {
                    return Task::none();
                }
                return self.update(Message::BrowseFile(BrowseFileSubject::BackgroundExecutable(name)));
            }
            Edit::Pause(paused) => {
                if let Some(background) = self.background.as_mut() {
                    background.set_paused(paused);
                }
                return Task::none();
            }
            Edit::Enable(enabled) => config.enabled = enabled,
            Edit::Delay(seconds) => config.delay_seconds = seconds.clamp(0, 3600) as u64,
            Edit::Poll(seconds) => config.poll_seconds = seconds.clamp(1, 60) as u64,
            Edit::GameEnabled(i, enabled) => {
                if let Some(game) = config.games.get_mut(i) {
                    game.enabled = enabled;
                }
            }
            Edit::Remove(i) => {
                if i < config.games.len() {
                    config.games.remove(i);
                }
            }
            Edit::RemoveExecutable(i, j) => {
                if let Some(game) = config.games.get_mut(i)
                    && j < game.executables.len()
                {
                    game.executables.remove(j);
                    if game.executables.is_empty() {
                        config.games.remove(i);
                    }
                }
            }
            Edit::AddExecutable(name, executable) => {
                if self.backup_screen.log.find_game(&name).is_none()
                    && !config.games.iter().any(|game| game.name == name)
                {
                    return self.show_error(Error::ConfigInvalid {
                        why: TRANSLATOR.automatic_backup_text("automatic-backup-choose-discovered"),
                    });
                }
                if let Err(why) = config.add_executable(name, executable) {
                    return self.show_error(Error::ConfigInvalid { why });
                }
            }
        }
        // Create the tray before persisting an enabled state, so a tray failure stays disabled.
        let new_background = if config.enabled && self.background.is_none() {
            match background::Background::new(config.clone(), None) {
                Ok(background) => Some(background),
                Err(why) => return self.show_error(Error::ConfigInvalid { why }),
            }
        } else {
            None
        };
        if let Err(why) = config.persist() {
            return self.show_error(Error::ConfigInvalid { why });
        }
        if let Some(background) = new_background {
            self.background = Some(background);
        }
        if let Some(background) = self.background.as_mut() {
            background.monitor.reconfigure(&background.config, &config);
            background.config = config.clone();
            background.polling = false;
        }
        self.background_revision += 1;
        self.background_settings.config = config;
        if !self.background_settings.config.enabled {
            self.background = None;
            if self.hidden {
                return self.show_window();
            }
        }
        Task::none()
    }

    #[cfg(windows)]
    fn open_background_settings(&mut self) -> Task<Message> {
        if let Some(id) = self.background_settings_window {
            return iced::window::gain_focus(id);
        }
        let (id, open) = iced::window::open(super::window_settings());
        self.background_settings_window = Some(id);
        open.map(|_| Message::Ignore)
    }

    #[cfg(windows)]
    fn save(&mut self) {
        self.save_pending(false);
    }

    #[cfg(windows)]
    fn save_pending(&mut self, force: bool) -> bool {
        let threshold = Duration::from_secs(1);
        let now = Instant::now();
        let mut metadata_save_error = None;
        let backing_up = matches!(self.operation, Operation::Backup { .. });

        self.pending_save.retain(|item, then| {
            if !force && (now - *then) < threshold {
                return true;
            }

            match item {
                SaveKind::Config => self.config.save(),
                SaveKind::Cache => self.cache.save(),
                SaveKind::Backup(game) => {
                    if backing_up {
                        return true;
                    }
                    if let Err(error) = self.restore_screen.log.save_layout(game) {
                        metadata_save_error = Some((game.clone(), error.to_string()));
                        return true;
                    }
                }
            }

            false
        });
        let saved = metadata_save_error.is_none();
        if let Some((game, why)) = metadata_save_error {
            self.timed_notification = Some(
                Notification::new(TRANSLATOR.automatic_backup_error(
                    "automatic-backup-metadata-error",
                    Some(&game),
                    &why,
                ))
                .action(
                    &TRANSLATOR.automatic_backup_text("automatic-backup-discard-metadata"),
                    Message::DiscardBackupMetadata(game),
                ),
            );
        }
        saved
    }

    #[cfg(not(windows))]
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
            x.path().invalidate_cache();
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
        if let Some(GameSelection::Single { game }) = self.operation.games() {
            self.notify_on_single_game_scanned = Some((game.clone(), self.screen));
        }
    }

    fn handle_notify_on_single_game_scanned(&mut self) -> bool {
        if let Some((name, screen)) = self.notify_on_single_game_scanned.as_ref() {
            let log = match self.operation {
                Operation::Backup { .. } => &self.backup_screen.log,
                Operation::Restore { .. } => &self.restore_screen.log,
                _ => return false,
            };
            let found = log.entries.iter().any(|x| &x.scan_info.game_name == name);

            if *screen != Screen::CustomGames && found {
                return found;
            }

            let msg = TRANSLATOR.notify_single_game_status(found);
            self.timed_notification = Some(Notification::new(msg).expires(3));
            return found;
        }

        false
    }

    fn start_sync_cloud(
        &mut self,
        local: &StrictPath,
        direction: SyncDirection,
        finality: Finality,
        games: Option<GameSelection>,
        standalone: bool,
    ) -> Result<(), Error> {
        let remote = crate::cloud::validate_cloud_config(&self.config, &self.config.cloud.path)?;

        let games = match games {
            Some(games) => {
                let layout = BackupLayout::new(local.clone());
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

    fn handle_backup(&mut self, phase: BackupPhase) -> Task<Message> {
        const SCAN_KIND: ScanKind = ScanKind::Backup;

        match phase {
            BackupPhase::Confirm { games } => self.show_modal(Modal::ConfirmBackup { games }),
            BackupPhase::Start {
                preview,
                repair,
                jump,
                mut games,
            } => {
                if !self.operation.idle() {
                    return Task::none();
                }
                #[cfg(windows)]
                if !self.save_pending(true) {
                    self.fail_background_backup(&Error::SomeEntriesFailed);
                    return self.show_error(Error::SomeEntriesFailed);
                }

                let mut cleared_log = false;
                if games.is_none() {
                    if self.backup_screen.log.is_filtered() {
                        games = Some(GameSelection::group(self.backup_screen.log.visible_games(
                            SCAN_KIND,
                            &self.config,
                            &self.manifest.extended,
                            &self.backup_screen.duplicate_detector,
                        )));
                    } else {
                        self.backup_screen.log.clear();
                        self.backup_screen.duplicate_detector.clear();
                        self.reset_scroll_position(ScrollSubject::Backup);
                        cleared_log = true;
                    }
                }

                if jump && let Some(GameSelection::Single { game }) = &games {
                    self.jump_to_game_after_scan = Some(game.clone());
                }

                self.operation =
                    Operation::new_backup(if preview { Finality::Preview } else { Finality::Final }, games);
                self.operation.set_force_new_full_backups(repair);

                if !preview && let Err(e) = prepare_backup_target(&self.config.backup.path) {
                    #[cfg(windows)]
                    self.fail_background_backup(&e);
                    self.go_idle();
                    return self.show_error(e);
                }

                Task::batch([
                    self.close_modal(),
                    if repair {
                        self.switch_screen(Screen::Backup)
                    } else {
                        Task::none()
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

                match self.start_sync_cloud(&local, SyncDirection::Upload, Finality::Preview, games.cloned(), false) {
                    Ok(_) => {
                        // deferring to `transition_from_cloud_step`
                        Task::none()
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
                let games = self.operation.games().cloned();

                if preview && full {
                    self.backup_screen.previewed_games.clear();
                }

                let all_scanned = !self.backup_screen.log.contains_unscanned_games();
                if let Some(games) = &games {
                    self.backup_screen.log.unscan_games(games);
                }
                self.progress.start();

                let mut manifest = self.manifest.primary.clone();
                let config = self.config.clone();
                let previewed_games = self.backup_screen.previewed_games.clone();

                Task::perform(
                    async move {
                        manifest.incorporate_extensions(&config);
                        let subjects: HashSet<_> = if let Some(games) = &games {
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

                        // HashSet -> Vec because randomized order looks nicer in the GUI.
                        let subjects: Vec<_> = subjects.into_iter().collect();

                        let roots = config.expanded_roots();
                        let layout = BackupLayout::new(config.backup.path.clone());
                        let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());
                        let steam = SteamShortcuts::scan(&title_finder);
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
                let single = self.operation.games().is_some_and(|x| x.is_single());

                if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    self.go_idle();
                    return Task::none();
                }

                if subjects.is_empty() {
                    if let Some(games) = self.operation.games() {
                        for game in games.iter() {
                            let duplicates = self.backup_screen.duplicate_detector.remove_game(game);
                            self.backup_screen.log.remove_game(
                                game,
                                &self.backup_screen.duplicate_detector,
                                &duplicates,
                                &self.config,
                                SCAN_KIND,
                            );
                        }
                        self.cache.backup.recent_games.retain(|x| !games.contains(x));
                        self.save_cache();
                    }
                    self.go_idle();
                    return Task::none();
                }

                self.progress.set_max(subjects.len() as f32);
                self.register_notify_on_single_game_scanned();

                let config = std::sync::Arc::new(self.config.clone());
                let roots = std::sync::Arc::new(config.expanded_roots());
                let layout = std::sync::Arc::new(*layout);
                let launchers = std::sync::Arc::new(launchers);
                let filter = std::sync::Arc::new(self.config.backup.filter.clone());
                let steam_shortcuts = std::sync::Arc::new(steam);
                let games_specified = self.operation.games_specified();
                let retention = config
                    .backup
                    .retention
                    .with_force_new_full(self.operation.should_force_new_full_backups());

                for key in subjects {
                    let game = manifest.0[&key].clone();
                    let config = config.clone();
                    let roots = roots.clone();
                    let launchers = launchers.clone();
                    let layout = layout.clone();
                    let filter = filter.clone();
                    let steam_shortcuts = steam_shortcuts.clone();
                    let cancel_flag = self.operation_should_cancel.clone();
                    self.operation_steps.push(OperationStep {
                        title: key.clone(),
                        task: Task::perform(
                            async move {
                                if key.trim().is_empty() {
                                    return (None, None);
                                }
                                if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                    return (None, None);
                                }

                                let wine_ctx = semantic::Wine::for_game(&key, &config);
                                let previous = layout.latest_backup(
                                    &key,
                                    SCAN_KIND,
                                    &config.redirects,
                                    config.restore.reverse_redirects,
                                    &config.restore.toggled_paths,
                                    config.backup.only_constructive,
                                    wine_ctx.as_ref(),
                                );

                                if filter.excludes(games_specified, previous.is_some(), &game.cloud) {
                                    log::trace!("[{key}] excluded by backup filter");
                                    return (None, None);
                                }

                                let scan_info = scan_game_for_backup(
                                    &game,
                                    &key,
                                    &roots,
                                    &app_dir(),
                                    &launchers,
                                    &filter,
                                    None,
                                    &config.backup.toggled_paths,
                                    &config.backup.toggled_registry,
                                    previous.as_ref(),
                                    &config.redirects,
                                    config.restore.reverse_redirects,
                                    &steam_shortcuts,
                                    config.backup.only_constructive,
                                );
                                if !config.is_game_enabled_for_backup(&key) && !single {
                                    return (Some(scan_info), None);
                                }

                                let backup_info = if !preview {
                                    layout.game_layout(&key).back_up(
                                        &scan_info,
                                        &chrono::Utc::now(),
                                        &config.backup.format,
                                        retention,
                                        config.backup.only_constructive,
                                    )
                                } else {
                                    None
                                };
                                (Some(scan_info), backup_info)
                            },
                            move |(scan_info, backup_info)| {
                                Message::Backup(BackupPhase::GameScanned { scan_info, backup_info })
                            },
                        ),
                    });
                }

                self.operation_steps_active = 100.min(self.operation_steps.len());

                let mut tasks = vec![];
                for step in self.operation_steps.drain(..self.operation_steps_active) {
                    self.operation.add_active_game(step.title);
                    tasks.push(step.task);
                }
                Task::batch(tasks)
            }
            BackupPhase::GameScanned { scan_info, backup_info } => {
                self.progress.step();
                let full = self.operation.full();

                #[cfg(windows)]
                if let (Some(run), Some(scan)) = (&mut self.background_backup, &scan_info) {
                    run.scanned(scan, backup_info.as_ref());
                }

                if let Some(mut scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    self.operation.remove_active_game(&scan_info.game_name);
                    if scan_info.can_report_game() {
                        if let Some(backup_info) = backup_info.as_ref() {
                            if scan_info.needs_cloud_sync() {
                                self.operation.add_syncable_game(scan_info.game_name.clone());
                            }
                            scan_info.clear_processed_changes(backup_info, SCAN_KIND);
                        }

                        let duplicates = self.backup_screen.duplicate_detector.add_game(
                            &scan_info,
                            self.config
                                .is_game_enabled_for_operation(&scan_info.game_name, SCAN_KIND),
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
                            SCAN_KIND,
                        );
                    } else if !full {
                        let duplicates = self.backup_screen.duplicate_detector.remove_game(&scan_info.game_name);
                        self.backup_screen.log.remove_game(
                            &scan_info.game_name,
                            &self.backup_screen.duplicate_detector,
                            &duplicates,
                            &self.config,
                            SCAN_KIND,
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
                    Some(step) => {
                        self.operation.add_active_game(step.title);
                        step.task
                    }
                    None => {
                        self.operation_steps_active -= 1;
                        if self.operation_steps_active == 0 {
                            self.handle_backup(BackupPhase::CloudSync)
                        } else {
                            Task::none()
                        }
                    }
                }
            }
            BackupPhase::CloudSync => {
                if !self.operation.should_sync_cloud_after() {
                    return self.handle_backup(BackupPhase::Done);
                }

                let local = self.config.backup.path.clone();
                let games = GameSelection::group(self.operation.syncable_games().cloned().unwrap_or_default());

                if games.is_empty() {
                    return self.handle_backup(BackupPhase::Done);
                }

                match self.start_sync_cloud(&local, SyncDirection::Upload, Finality::Final, Some(games), false) {
                    Ok(_) => {
                        // deferring to `transition_from_cloud_step`
                        Task::none()
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

                let found_single = self.handle_notify_on_single_game_scanned();

                if full {
                    self.cache.backup.recent_games.clear();
                }

                for entry in &self.backup_screen.log.entries {
                    self.cache.backup.recent_games.insert(entry.scan_info.game_name.clone());
                    if let Some(backup_info) = &entry.backup_info
                        && !backup_info.successful()
                    {
                        failed = true;
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

                if let Some(errors) = errors
                    && !errors.is_empty()
                {
                    return self.show_modal(Modal::Errors { errors });
                }

                if let Some(jump) = self.jump_to_game_after_scan.take()
                    && found_single
                {
                    use crate::gui::widget::operation::container_scroll_offset;

                    self.backup_screen.log.expand_game(
                        &jump,
                        &self.backup_screen.duplicate_detector,
                        &self.config,
                        SCAN_KIND,
                    );

                    return self
                        .switch_screen(Screen::Backup)
                        .chain(container_scroll_offset(jump.into()).map(move |offset| match offset {
                            Some(position) => Message::Scroll {
                                subject: ScrollSubject::Backup,
                                position,
                            },
                            None => Message::Ignore,
                        }));
                }

                Task::none()
            }
        }
    }

    fn handle_restore(&mut self, phase: RestorePhase) -> Task<Message> {
        const SCAN_KIND: ScanKind = ScanKind::Restore;

        match phase {
            RestorePhase::Confirm { games } => self.show_modal(Modal::ConfirmRestore { games }),
            RestorePhase::Start { preview, mut games } => {
                if !self.operation.idle() {
                    return Task::none();
                }
                #[cfg(windows)]
                if !self.save_pending(true) {
                    return self.show_error(Error::SomeEntriesFailed);
                }

                let path = self.config.restore.path.clone();
                if !path.is_dir() {
                    return self.show_modal(Modal::Error {
                        variant: Error::RestorationSourceInvalid { path },
                    });
                }

                let mut cleared_log = false;
                if games.is_none() {
                    if self.restore_screen.log.is_filtered() {
                        games = Some(GameSelection::group(self.restore_screen.log.visible_games(
                            SCAN_KIND,
                            &self.config,
                            &self.manifest.extended,
                            &self.restore_screen.duplicate_detector,
                        )));
                    } else {
                        self.restore_screen.log.clear();
                        self.restore_screen.duplicate_detector.clear();
                        self.reset_scroll_position(ScrollSubject::Restore);
                        cleared_log = true;
                    }
                }

                self.operation =
                    Operation::new_restore(if preview { Finality::Preview } else { Finality::Final }, games);

                self.invalidate_path_caches();
                self.timed_notification = None;

                Task::batch([
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

                match self.start_sync_cloud(&local, SyncDirection::Upload, Finality::Preview, games.cloned(), false) {
                    Ok(_) => {
                        // waiting for background thread
                        Task::none()
                    }
                    Err(e) => {
                        self.operation.push_error(e);
                        self.handle_restore(RestorePhase::Load)
                    }
                }
            }
            RestorePhase::Load => {
                let restore_path = self.config.restore.path.clone();

                self.progress.start();

                Task::perform(
                    async move {
                        let layout = BackupLayout::new(restore_path);
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
                let games = self.operation.games();
                let single = games.is_some_and(|x| x.is_single());

                if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    self.go_idle();
                    return Task::none();
                }

                if let Some(games) = &games {
                    restorables.retain(|v| games.contains(v));
                    self.restore_screen.log.unscan_games(games);
                }

                if restorables.is_empty() {
                    if let Some(games) = games {
                        for game in games.iter() {
                            let duplicates = self.restore_screen.duplicate_detector.remove_game(game);
                            self.restore_screen.log.remove_game(
                                game,
                                &self.restore_screen.duplicate_detector,
                                &duplicates,
                                &self.config,
                                SCAN_KIND,
                            );
                        }
                        self.cache.restore.recent_games.retain(|x| !games.contains(x));
                        self.save_cache();
                    }
                    self.go_idle();
                    return Task::none();
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
                    self.operation_steps.push(OperationStep {
                        title: name.clone(),
                        task: Task::perform(
                            async move {
                                let mut layout = layout.game_layout(&name);

                                if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                    return (None, None, layout);
                                }

                                let wine_ctx = semantic::Wine::for_game(&name, &config);
                                let scan_info = layout.scan_for_restoration(
                                    &name,
                                    &backup_id,
                                    &config.redirects,
                                    config.restore.reverse_redirects,
                                    &config.restore.toggled_paths,
                                    &config.restore.toggled_registry,
                                    wine_ctx.as_ref(),
                                );

                                if !config.is_game_enabled_for_restore(&name) && !single {
                                    return (Some(scan_info), None, layout);
                                }

                                let backup_info = if scan_info.backup.is_some() && !preview {
                                    Some(layout.restore(&scan_info, &config.restore.toggled_registry))
                                } else {
                                    None
                                };
                                (Some(scan_info), backup_info, layout)
                            },
                            move |(scan_info, backup_info, game_layout)| {
                                Message::Restore(RestorePhase::GameScanned {
                                    scan_info,
                                    backup_info,
                                    game_layout: Box::new(game_layout),
                                })
                            },
                        ),
                    });
                }

                self.operation_steps_active = 100.min(self.operation_steps.len());

                let mut tasks = vec![];
                for step in self.operation_steps.drain(..self.operation_steps_active) {
                    self.operation.add_active_game(step.title);
                    tasks.push(step.task);
                }
                Task::batch(tasks)
            }
            RestorePhase::GameScanned {
                scan_info,
                backup_info,
                game_layout,
            } => {
                self.progress.step();
                let full = self.operation.full();

                if let Some(mut scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    self.operation.remove_active_game(&scan_info.game_name);

                    if scan_info.can_report_game() {
                        if let Some(backup_info) = backup_info.as_ref() {
                            scan_info.clear_processed_changes(backup_info, SCAN_KIND);
                        }

                        let comment = scan_info.backup.as_ref().and_then(|x| x.comment()).map(|x| x.as_str());
                        self.text_histories.backup_comments.insert(
                            scan_info.game_name.clone(),
                            TextHistory::raw(comment.unwrap_or_default()),
                        );

                        let duplicates = self.restore_screen.duplicate_detector.add_game(
                            &scan_info,
                            self.config
                                .is_game_enabled_for_operation(&scan_info.game_name, SCAN_KIND),
                        );
                        self.restore_screen.log.update_game(
                            scan_info,
                            backup_info,
                            &self.config.backup.sort,
                            &self.restore_screen.duplicate_detector,
                            &duplicates,
                            Some(*game_layout),
                            &self.config,
                            SCAN_KIND,
                        );
                    } else if !full {
                        let duplicates = self.restore_screen.duplicate_detector.remove_game(&scan_info.game_name);
                        self.restore_screen.log.remove_game(
                            &scan_info.game_name,
                            &self.restore_screen.duplicate_detector,
                            &duplicates,
                            &self.config,
                            SCAN_KIND,
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
                    Some(step) => {
                        self.operation.add_active_game(step.title);
                        step.task
                    }
                    None => {
                        self.operation_steps_active -= 1;
                        if self.operation_steps_active == 0 {
                            self.handle_restore(RestorePhase::Done)
                        } else {
                            Task::none()
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
                    if let Some(backup_info) = &entry.backup_info
                        && !backup_info.successful()
                    {
                        failed = true;
                    }
                }

                self.save_cache();

                if failed {
                    self.operation.push_error(Error::SomeEntriesFailed);
                }

                let errors = self.operation.errors().cloned();
                self.go_idle();

                if let Some(errors) = errors
                    && !errors.is_empty()
                {
                    return self.show_modal(Modal::Errors { errors });
                }

                Task::none()
            }
        }
    }

    fn handle_validation(&mut self, phase: ValidatePhase) -> Task<Message> {
        match phase {
            ValidatePhase::Start => {
                if !self.operation.idle() {
                    return Task::none();
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

                Task::batch([self.close_modal(), self.handle_validation(ValidatePhase::Load)])
            }
            ValidatePhase::Load => {
                let restore_path = self.config.restore.path.clone();

                self.progress.start();

                Task::perform(
                    async move {
                        let layout = BackupLayout::new(restore_path);
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
                    return Task::none();
                }

                if subjects.is_empty() {
                    self.go_idle();
                    return Task::none();
                }

                self.progress.set_max(subjects.len() as f32);

                let layout = std::sync::Arc::new(layout);

                for name in subjects {
                    let layout = layout.clone();
                    let cancel_flag = self.operation_should_cancel.clone();
                    let backup_id = self.backups_to_restore.get(&name).cloned().unwrap_or(BackupId::Latest);
                    self.operation_steps.push(OperationStep {
                        title: name.clone(),
                        task: Task::perform(
                            async move {
                                if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                    return (name, true);
                                }

                                let Some(layout) = layout.try_game_layout(&name) else {
                                    return (name, false);
                                };

                                // TODO: Add an option to validate all backups at once.
                                let valid = layout.validate(backup_id);
                                (name, valid)
                            },
                            move |(game, valid)| Message::ValidateBackups(ValidatePhase::GameScanned { game, valid }),
                        ),
                    });
                }

                self.operation_steps_active = 100.min(self.operation_steps.len());

                let mut tasks = vec![];
                for step in self.operation_steps.drain(..self.operation_steps_active) {
                    self.operation.add_active_game(step.title);
                    tasks.push(step.task);
                }
                Task::batch(tasks)
            }
            ValidatePhase::GameScanned { game, valid } => {
                self.progress.step();
                log::trace!("step {} / {}: {}", self.progress.current, self.progress.max, game);
                self.operation.remove_active_game(&game);

                if !valid && let Operation::ValidateBackups { faulty_games, .. } = &mut self.operation {
                    faulty_games.insert(game);
                }

                match self.operation_steps.pop() {
                    Some(step) => {
                        self.operation.add_active_game(step.title);
                        step.task
                    }
                    None => {
                        self.operation_steps_active -= 1;
                        if self.operation_steps_active == 0 {
                            self.handle_validation(ValidatePhase::Done)
                        } else {
                            Task::none()
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

    fn transition_from_cloud_step(&mut self) -> Option<Task<Message>> {
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

    fn cancel_operation(&mut self) -> Task<Message> {
        self.operation_should_cancel
            .swap(true, std::sync::atomic::Ordering::Relaxed);
        self.operation_steps.clear();
        self.operation.flag_cancel();
        if self.operation.is_cloud_active()
            && let Some(sender) = self.rclone_monitor_sender.as_mut()
        {
            let _ = sender.try_send(rclone_monitor::Input::Cancel);
        }
        Task::none()
    }

    fn make_custom_game(name: String, manifest: &LoadedManifest) -> CustomGame {
        if let Some(standard) = manifest.extended.0.get(&name) {
            CustomGame {
                name: name.clone(),
                ignore: false,
                integration: config::Integration::Override,
                alias: standard.alias.clone(),
                prefer_alias: false,
                files: standard.files.keys().cloned().collect(),
                registry: standard.registry.keys().cloned().collect(),
                install_dir: standard.install_dir.keys().filter(|x| *x != &name).cloned().collect(),
                wine_prefix: vec![],
                expanded: true,
            }
        } else {
            CustomGame {
                name: name.clone(),
                ignore: false,
                integration: config::Integration::Override,
                alias: None,
                prefer_alias: false,
                files: vec![],
                registry: vec![],
                install_dir: vec![],
                wine_prefix: vec![],
                expanded: true,
            }
        }
    }

    fn customize_game(&mut self, name: String) -> Task<Message> {
        let game = Self::make_custom_game(name, &self.manifest);

        self.text_histories.add_custom_game(&game);
        self.config.custom_games.push(game);
        self.save_config();

        self.scroll_offsets.insert(
            ScrollSubject::CustomGames,
            scrollable::AbsoluteOffset { x: 0.0, y: f32::MAX },
        );
        self.switch_screen(Screen::CustomGames)
    }

    fn customize_game_as_alias(&mut self, name: String) -> Task<Message> {
        let game = CustomGame {
            name: "".to_string(),
            ignore: false,
            integration: config::Integration::Override,
            alias: Some(name),
            prefer_alias: true,
            files: vec![],
            registry: vec![],
            install_dir: vec![],
            wine_prefix: vec![],
            expanded: true,
        };

        self.text_histories.add_custom_game(&game);
        self.config.custom_games.push(game);
        self.save_config();

        self.scroll_offsets.insert(
            ScrollSubject::CustomGames,
            scrollable::AbsoluteOffset { x: 0.0, y: f32::MAX },
        );
        self.switch_screen(Screen::CustomGames)
    }

    fn update_manifest(
        config: config::ManifestConfig,
        cache: cache::Manifests,
        force: bool,
        network_security: Security,
    ) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || Manifest::update(config, cache, force, network_security)).await
            },
            |join| match join {
                Ok(x) => Message::ManifestUpdated(x),
                Err(_) => Message::Ignore,
            },
        )
    }

    fn open_url(url: String) -> Task<Message> {
        let url2 = url.clone();
        Task::future(async move {
            let result = async { opener::open(url) }.await;

            match result {
                Ok(_) => Message::Ignore,
                Err(e) => {
                    log::error!("Unable to open URL: `{}` - {}", url2, e);
                    Message::OpenUrlFailure { url: url2 }
                }
            }
        })
    }

    fn open_wiki(game: String) -> Task<Message> {
        let url = format!("https://www.pcgamingwiki.com/wiki/{}", game.replace(' ', "_"));
        Self::open_url(url)
    }

    fn toggle_backup_comment_editor(&mut self, name: String) -> Task<Message> {
        self.restore_screen.log.toggle_backup_comment_editor(&name);
        Task::none()
    }

    fn switch_screen(&mut self, screen: Screen) -> Task<Message> {
        self.screen = screen;
        self.refresh_scroll_position()
    }

    fn scroll_subject(&self) -> ScrollSubject {
        if !self.modals.is_empty() {
            ScrollSubject::Modal
        } else {
            ScrollSubject::from(self.screen)
        }
    }

    fn refresh_scroll_position(&mut self) -> Task<Message> {
        let subject = self.scroll_subject();
        let offset = self.scroll_offsets.get(&subject).copied().unwrap_or_default();

        iced::widget::operation::scroll_to(subject.id(), offset)
    }

    fn refresh_scroll_position_on_log(&mut self, cleared: bool) -> Task<Message> {
        if cleared {
            self.refresh_scroll_position()
        } else {
            Task::none()
        }
    }

    fn reset_scroll_position(&mut self, subject: ScrollSubject) {
        self.scroll_offsets
            .insert(subject, scrollable::AbsoluteOffset::default());
    }

    fn configure_remote(&self, remote: Remote) -> Task<Message> {
        let rclone = self.config.apps.rclone.clone();
        let old_remote = self.config.cloud.remote.clone();
        let new_remote = remote.clone();
        Task::future(async move {
            let result = async {
                if let Some(old_remote) = old_remote {
                    _ = Rclone::new(rclone.clone(), old_remote).unconfigure_remote();
                }
                Rclone::new(rclone, new_remote).configure_remote()
            }
            .await;

            match result {
                Ok(_) => Message::ConfigureCloudSuccess(remote),
                Err(e) => Message::ConfigureCloudFailure(e),
            }
        })
    }

    pub fn new(
        flags: Flags,
        #[cfg(windows)] background_config: Result<background::Config, String>,
        #[cfg(windows)] instance: Option<background::Instance>,
    ) -> (Self, Task<Message>) {
        #[cfg(windows)]
        let (window, open) = iced::window::open(super::window_settings());
        let mut errors = vec![];
        let mut commands = vec![
            #[cfg(windows)]
            open.map(|_| Message::Ignore),
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::TEXT_DATA)).map(|_| Message::Ignore),
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::ICONS_DATA)).map(|_| Message::Ignore),
            iced::window::oldest().and_then(iced::window::gain_focus),
        ];

        let mut screen = Screen::default();
        let mut modals: Vec<Modal> = vec![];
        let mut pending_save = HashMap::new();

        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                errors.push(x);
                let _ = Config::archive_invalid();
                Config::default()
            }
        };
        #[cfg(windows)]
        let background_settings = match background_config {
            Ok(config) => background::Settings {
                config,
                ..Default::default()
            },
            Err(why) => {
                errors.push(Error::ConfigInvalid { why });
                background::Settings::default()
            }
        };
        #[cfg(windows)]
        let background_config = background_settings
            .config
            .enabled
            .then(|| background_settings.config.clone());
        let mut cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
        TRANSLATOR.set_language(config.language);

        let manifest = if Manifest::path().exists() {
            match Manifest::load() {
                Ok(y) => LoadedManifest {
                    primary: y.clone(),
                    extended: y.with_extensions(&config),
                },
                Err(e) => {
                    errors.push(e);
                    LoadedManifest::default()
                }
            }
        } else {
            if flags.update_manifest {
                modals.push(Modal::UpdatingManifest);
            }
            LoadedManifest::default()
        };

        if let Some(custom_game) = flags.custom_game.as_ref() {
            screen = Screen::CustomGames;

            if let Some(entry) = config.custom_games.iter_mut().find(|entry| &entry.name == custom_game) {
                entry.expanded = true;
            } else {
                let game = Self::make_custom_game(custom_game.clone(), &manifest);
                config.custom_games.push(game);
                pending_save.insert(SaveKind::Config, Instant::now());
            }

            commands.push(
                container_scroll_offset(custom_game.clone().into()).map(move |offset| match offset {
                    Some(position) => Message::Scroll {
                        subject: ScrollSubject::CustomGames,
                        position,
                    },
                    None => Message::Ignore,
                }),
            );
        }

        if !errors.is_empty() {
            modals.push(Modal::Errors { errors });
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
                modals.push(Modal::ConfirmAddMissingRoots(missing));
            }
        }

        let text_histories = TextHistories::new(&config);

        log::debug!("Config on startup: {config:?}");

        if flags.update_manifest {
            commands.push(Self::update_manifest(
                config.manifest.clone(),
                cache.manifests.clone(),
                false,
                config.runtime.network_security,
            ));
        }

        if config.release.check && cache.should_check_app_update() {
            commands.push(Task::future(async move {
                let result = crate::metadata::Release::fetch(config.runtime.network_security).await;

                Message::AppReleaseChecked(result.map_err(|x| x.to_string()))
            }))
        }

        (
            Self {
                backup_screen: screen::Backup::new(&config, &cache),
                restore_screen: screen::Restore::new(&config, &cache),
                config,
                manifest,
                cache,
                modals,
                updating_manifest: flags.update_manifest,
                text_histories,
                flags,
                screen,
                pending_save,
                #[cfg(windows)]
                background_config,
                #[cfg(windows)]
                background_instance: instance,
                #[cfg(windows)]
                background_settings,
                #[cfg(windows)]
                window: Some(window),
                ..Self::default()
            },
            Task::batch(commands),
        )
    }

    pub fn title(&self, #[cfg(windows)] window: iced::window::Id) -> String {
        #[cfg(windows)]
        if self.background_settings_window == Some(window) {
            return TRANSLATOR.automatic_backup_text("automatic-backup-window-title");
        }
        TRANSLATOR.window_title()
    }

    pub fn theme(&self, #[cfg(windows)] _window: iced::window::Id) -> crate::gui::style::Theme {
        crate::gui::style::Theme::from(self.config.theme)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ignore => Task::none(),
            #[cfg(windows)]
            Message::WindowOpened(id) => {
                if self.background_settings_window == Some(id) {
                    return Task::none();
                }
                self.window = Some(id);
                if let Some(config) = self.background_config.take() {
                    match background::Background::new(config, self.background_instance.take()) {
                        Ok(background) => {
                            self.background = Some(background);
                        }
                        Err(why) => return self.show_error(Error::ConfigInvalid { why }),
                    }
                }
                Task::none()
            }
            #[cfg(windows)]
            Message::CloseWindow(id) => {
                if self.background_settings_window == Some(id) {
                    self.background_settings_window = None;
                    return iced::window::close(id);
                }
                if self.background.is_some() && !self.exiting {
                    if !self.save_pending(true) {
                        return self.show_error(Error::SomeEntriesFailed);
                    }
                    self.hidden = true;
                    self.hide_window(id)
                } else {
                    self.update(Message::Exit { user: true })
                }
            }
            #[cfg(windows)]
            Message::BackgroundTick => self.background_tick(),
            #[cfg(windows)]
            Message::BackgroundEdit(edit) => self.edit_background(edit),
            #[cfg(windows)]
            Message::OpenBackgroundSettings => self.open_background_settings(),
            #[cfg(not(windows))]
            Message::OpenBackgroundSettings => Task::none(),
            #[cfg(windows)]
            Message::BackgroundPolled(revision, result) => {
                if revision != self.background_revision {
                    return Task::none();
                }
                let Some(background) = self.background.as_mut() else {
                    return Task::none();
                };
                background.polling = false;
                if background.paused || self.exiting {
                    return Task::none();
                }
                let (config, running) = match result {
                    Ok(result) => result,
                    Err(why) => {
                        background.set_paused(true);
                        return self.show_error(Error::ConfigInvalid { why });
                    }
                };
                self.background_settings.config = config.clone();
                if !config.enabled {
                    self.background = None;
                    log::info!("Background monitoring disabled");
                    return if self.hidden { self.show_window() } else { Task::none() };
                }
                if background.config != config {
                    background.monitor.reconfigure(&background.config, &config);
                    background.config = config;
                    log::info!("Background configuration reloaded");
                }
                background
                    .monitor
                    .observe_games(&background.config, &running, Instant::now());
                if self.operation.idle()
                    && self.modals.is_empty()
                    && !self.updating_manifest
                    && self.rclone_monitor_sender.is_some()
                    && let Some(game) = background.monitor.take_ready()
                {
                    self.background_backup = Some(background::BackupRun::new(game.clone()));
                    if !self.manifest.extended.0.contains_key(&game) {
                        self.fail_background_backup(&Error::GameIsUnrecognized);
                        return self.show_error(Error::CliUnrecognizedGames { games: vec![game] });
                    }
                    log::info!("Backing up {game} after its processes closed");
                    match background.notify_backup_start(&game) {
                        Ok(()) => log::info!("Windows backup notification submitted for {game}"),
                        Err(why) => log::warn!("{why}: {game}"),
                    }
                    self.handle_backup(BackupPhase::Start {
                        preview: false,
                        repair: false,
                        jump: false,
                        games: Some(GameSelection::Single { game }),
                    })
                } else {
                    Task::none()
                }
            }
            Message::CloseModal => self.close_modal(),
            Message::Exit { user } => {
                #[cfg(windows)]
                if !user && !self.exiting {
                    return Task::none();
                }
                if self.operation.idle() || (user && self.exiting) {
                    #[cfg(windows)]
                    if !self.save_pending(true)
                        || self.pending_save.keys().any(|item| matches!(item, SaveKind::Backup(_)))
                    {
                        self.exiting = false;
                        self.close_specific_modal_alt(modal::Kind::Exiting);
                        return self.show_error(Error::SomeEntriesFailed);
                    }
                    #[cfg(not(windows))]
                    self.save();
                    std::process::exit(0)
                } else {
                    self.exiting = true;
                    Task::batch([self.show_modal(Modal::Exiting), self.cancel_operation()])
                }
            }
            Message::Save => {
                self.save();
                Task::none()
            }
            #[cfg(windows)]
            Message::DiscardBackupMetadata(game) => {
                self.restore_screen.log.discard_metadata_edits(&game);
                self.pending_save.remove(&SaveKind::Backup(game.clone()));
                self.timed_notification = None;
                self.handle_restore(RestorePhase::Start {
                    preview: true,
                    games: Some(GameSelection::single(game)),
                })
            }
            Message::UpdateTime => {
                self.progress.update_time();
                Task::none()
            }
            Message::PruneNotifications => {
                if let Some(notification) = &self.timed_notification
                    && notification.expired()
                {
                    self.timed_notification = None;
                }
                Task::none()
            }
            Message::TextInputCursorChanged { subject, position } => {
                self.active_text_input = Some((subject, position));
                Task::none()
            }
            Message::InsertPlaceholder { subject, placeholder } => {
                let (game_index, file_index) = match &subject {
                    UndoSubject::CustomGameFile(game_index, file_index) => (*game_index, *file_index),
                    _ => return Task::none(),
                };
                let Some(current) = self
                    .config
                    .custom_games
                    .get(game_index)
                    .and_then(|game| game.files.get(file_index))
                else {
                    return Task::none();
                };

                let value = iced::widget::text_input::Value::new(current);
                let position = self
                    .active_text_input
                    .as_ref()
                    .filter(|(active, _)| active == &subject)
                    .map(|(_, position)| *position)
                    .unwrap_or_else(|| value.len())
                    .min(value.len());
                let updated = format!(
                    "{}{}{}",
                    value.until(position),
                    placeholder,
                    value.select(position, value.len()),
                );
                let cursor_position = position + iced::widget::text_input::Value::new(&placeholder).len();

                self.text_histories.custom_games[game_index].files[file_index].push(&updated);
                self.config.custom_games[game_index].files[file_index] = updated;
                self.active_text_input = Some((subject.clone(), cursor_position));
                self.save_config();

                iced::widget::operation::move_cursor_to(subject.input_id(), cursor_position)
            }
            Message::Config { event } => {
                let mut task = None;

                match event {
                    config::Event::Theme(theme) => {
                        self.config.theme = theme;
                    }
                    config::Event::Language(language) => {
                        TRANSLATOR.set_language(language);
                        self.config.language = language;
                    }
                    config::Event::CheckRelease(enabled) => {
                        self.config.release.check = enabled;
                    }
                    config::Event::BackupTarget(text) => {
                        self.text_histories.backup_target.push(&text);
                        self.config.backup.path.reset(text);
                    }
                    config::Event::RestoreSource(text) => {
                        self.text_histories.restore_source.push(&text);
                        self.config.restore.path.reset(text);
                    }
                    config::Event::Root(action) => match action {
                        EditAction::Add => {
                            self.text_histories.roots.push(Default::default());
                            self.config.roots.push(Root::default());
                        }
                        EditAction::Change(index, value) => {
                            self.text_histories.roots[index].path.push(&value);
                            self.config.roots[index].path_mut().reset(value);
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
                    },
                    config::Event::RootLutrisDatabase(index, value) => {
                        self.text_histories.roots[index].lutris_database.push(&value);
                        if let Root::Lutris(root) = &mut self.config.roots[index] {
                            root.database = if value.is_empty() { None } else { Some(value.into()) };
                        }
                    }
                    config::Event::SecondaryManifest(action) => match action {
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
                    },
                    config::Event::RootStore(index, store) => {
                        self.text_histories.roots[index].clear_secondary();
                        self.config.roots[index].set_store(store);
                    }
                    config::Event::RedirectKind(index, kind) => {
                        self.config.redirects[index].kind = kind;
                    }
                    config::Event::SecondaryManifestKind(index, kind) => {
                        self.config.manifest.secondary[index].convert(kind);
                    }
                    config::Event::CustomGameKind(index, kind) => {
                        self.config.custom_games[index].convert(kind);
                        match kind {
                            CustomGameKind::Game => {
                                self.text_histories.custom_games[index].alias.clear();
                            }
                            CustomGameKind::Alias => {}
                        }
                    }
                    config::Event::CustomGameIntegration(index, integration) => {
                        self.config.custom_games[index].integration = integration;
                    }
                    config::Event::Redirect(action, field) => {
                        // TODO: Automatically refresh redirected paths in the game list.
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
                    }
                    config::Event::ReverseRedirectsOnRestore(enabled) => {
                        self.config.restore.reverse_redirects = enabled;
                    }
                    config::Event::CustomGame(action) => {
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
                        if snap {
                            self.scroll_offsets.insert(
                                ScrollSubject::CustomGames,
                                scrollable::AbsoluteOffset { x: 0.0, y: f32::MAX },
                            );
                            task = Some(self.refresh_scroll_position());
                        }
                    }
                    config::Event::CustomGameAlias(index, value) => {
                        self.text_histories.custom_games[index].alias.push(&value);
                        self.config.custom_games[index].alias = Some(value);
                    }
                    config::Event::CustomGaleAliasDisplay(index, value) => {
                        self.config.custom_games[index].prefer_alias = value;
                    }
                    config::Event::CustomGameFile(game_index, action) => match action {
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
                    },
                    config::Event::CustomGameRegistry(game_index, action) => match action {
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
                    },
                    config::Event::CustomGameInstallDir(game_index, action) => match action {
                        EditAction::Add => {
                            self.text_histories.custom_games[game_index]
                                .install_dir
                                .push(Default::default());
                            self.config.custom_games[game_index].install_dir.push("".to_string());
                        }
                        EditAction::Change(index, value) => {
                            self.text_histories.custom_games[game_index].install_dir[index].push(&value);
                            self.config.custom_games[game_index].install_dir[index] = value;
                        }
                        EditAction::Remove(index) => {
                            self.text_histories.custom_games[game_index].install_dir.remove(index);
                            self.config.custom_games[game_index].install_dir.remove(index);
                        }
                        EditAction::Move(index, direction) => {
                            let offset = direction.shift(index);
                            self.text_histories.custom_games[game_index]
                                .install_dir
                                .swap(index, offset);
                            self.config.custom_games[game_index].install_dir.swap(index, offset);
                        }
                    },
                    config::Event::CustomGameWinePrefix(game_index, action) => match action {
                        EditAction::Add => {
                            self.text_histories.custom_games[game_index]
                                .wine_prefix
                                .push(Default::default());
                            self.config.custom_games[game_index].wine_prefix.push("".to_string());
                        }
                        EditAction::Change(index, value) => {
                            self.text_histories.custom_games[game_index].wine_prefix[index].push(&value);
                            self.config.custom_games[game_index].wine_prefix[index] = value;
                        }
                        EditAction::Remove(index) => {
                            self.text_histories.custom_games[game_index].wine_prefix.remove(index);
                            self.config.custom_games[game_index].wine_prefix.remove(index);
                        }
                        EditAction::Move(index, direction) => {
                            let offset = direction.shift(index);
                            self.text_histories.custom_games[game_index]
                                .wine_prefix
                                .swap(index, offset);
                            self.config.custom_games[game_index].wine_prefix.swap(index, offset);
                        }
                    },
                    config::Event::ExcludeStoreScreenshots(enabled) => {
                        self.config.backup.filter.exclude_store_screenshots = enabled;
                    }
                    config::Event::CloudFilter(filter) => {
                        self.config.backup.filter.cloud = filter;
                    }
                    config::Event::BackupFilterIgnoredPath(action) => {
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
                    }
                    config::Event::BackupFilterIgnoredRegistry(action) => match action {
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
                    },
                    config::Event::GameListEntryEnabled {
                        name,
                        enabled,
                        scan_kind,
                    } => {
                        match (scan_kind, enabled) {
                            (ScanKind::Backup, false) => self.config.disable_game_for_backup(&name),
                            (ScanKind::Backup, true) => self.config.enable_game_for_backup(&name),
                            (ScanKind::Restore, false) => self.config.disable_game_for_restore(&name),
                            (ScanKind::Restore, true) => self.config.enable_game_for_restore(&name),
                        };

                        match scan_kind {
                            ScanKind::Backup => {
                                self.backup_screen.log.refresh_game_tree(
                                    &name,
                                    &self.config,
                                    &mut self.backup_screen.duplicate_detector,
                                    scan_kind,
                                );
                            }
                            ScanKind::Restore => {
                                self.restore_screen.log.refresh_game_tree(
                                    &name,
                                    &self.config,
                                    &mut self.restore_screen.duplicate_detector,
                                    scan_kind,
                                );
                            }
                        }
                    }
                    config::Event::CustomGameEnabled { index, enabled } => {
                        if enabled {
                            self.config.enable_custom_game(index);
                        } else {
                            self.config.disable_custom_game(index);
                        }
                    }
                    config::Event::PrimaryManifestEnabled { enabled } => {
                        self.config.manifest.enable = enabled;
                    }
                    config::Event::SecondaryManifestEnabled { index, enabled } => {
                        self.config.manifest.secondary[index].enable(enabled);
                    }
                    config::Event::ToggleSpecificGamePathIgnored { name, path, scan_kind } => match scan_kind {
                        ScanKind::Backup => {
                            self.config.backup.toggled_paths.toggle(&name, &path);
                            self.backup_screen.log.refresh_game_tree(
                                &name,
                                &self.config,
                                &mut self.backup_screen.duplicate_detector,
                                scan_kind,
                            );
                        }
                        ScanKind::Restore => {
                            self.config.restore.toggled_paths.toggle(&name, &path);
                            self.restore_screen.log.refresh_game_tree(
                                &name,
                                &self.config,
                                &mut self.restore_screen.duplicate_detector,
                                scan_kind,
                            );
                        }
                    },
                    config::Event::ToggleSpecificGameRegistryIgnored {
                        name,
                        path,
                        value,
                        scan_kind,
                    } => match scan_kind {
                        ScanKind::Backup => {
                            self.config.backup.toggled_registry.toggle_owned(&name, &path, value);
                            self.backup_screen.log.refresh_game_tree(
                                &name,
                                &self.config,
                                &mut self.backup_screen.duplicate_detector,
                                scan_kind,
                            );
                        }
                        ScanKind::Restore => {
                            self.config.restore.toggled_registry.toggle_owned(&name, &path, value);
                            self.restore_screen.log.refresh_game_tree(
                                &name,
                                &self.config,
                                &mut self.restore_screen.duplicate_detector,
                                scan_kind,
                            );
                        }
                    },
                    config::Event::SortKey(value) => match self.screen {
                        Screen::Backup => {
                            self.config.backup.sort.key = value;
                            self.backup_screen.log.sort(&self.config.backup.sort, &self.config);
                        }
                        Screen::Restore => {
                            self.config.restore.sort.key = value;
                            self.restore_screen.log.sort(&self.config.restore.sort, &self.config);
                        }
                        _ => {}
                    },
                    config::Event::SortReversed(value) => match self.screen {
                        Screen::Backup => {
                            self.config.backup.sort.reversed = value;
                            self.backup_screen.log.sort(&self.config.backup.sort, &self.config);
                        }
                        Screen::Restore => {
                            self.config.restore.sort.reversed = value;
                            self.restore_screen.log.sort(&self.config.restore.sort, &self.config);
                        }
                        _ => {}
                    },
                    config::Event::FullRetention(value) => {
                        self.config.backup.retention.full = value;
                    }
                    config::Event::DiffRetention(value) => {
                        self.config.backup.retention.differential = value;
                    }
                    config::Event::BackupFormat(format) => {
                        self.config.backup.format.chosen = format;
                    }
                    config::Event::BackupCompression(compression) => {
                        self.config.backup.format.zip.compression = compression;
                    }
                    config::Event::CompressionLevel(value) => {
                        self.config.backup.format.set_level(value);
                    }
                    config::Event::ToggleCloudSynchronize => {
                        self.config.cloud.synchronize = !self.config.cloud.synchronize;
                    }
                    config::Event::ShowDeselectedGames(value) => {
                        self.config.scan.show_deselected_games = value;
                    }
                    config::Event::ShowUnchangedGames(value) => {
                        self.config.scan.show_unchanged_games = value;
                    }
                    config::Event::ShowUnscannedGames(value) => {
                        self.config.scan.show_unscanned_games = value;
                    }
                    config::Event::OverrideMaxThreads(overridden) => {
                        self.config.override_threads(overridden);
                    }
                    config::Event::MaxThreads(threads) => {
                        self.config.set_threads(threads);
                    }
                    config::Event::RcloneExecutable(text) => {
                        self.text_histories.rclone_executable.push(&text);
                        self.config.apps.rclone.path.reset(text);
                    }
                    config::Event::RcloneArguments(text) => {
                        self.text_histories.rclone_arguments.push(&text);
                        self.config.apps.rclone.arguments = text;
                    }
                    config::Event::CloudRemoteId(text) => {
                        self.text_histories.cloud_remote_id.push(&text);
                        if let Some(Remote::Custom { id }) = &mut self.config.cloud.remote {
                            *id = text;
                        }
                    }
                    config::Event::CloudPath(text) => {
                        self.text_histories.cloud_path.push(&text);
                        self.config.cloud.path = text;
                    }
                    config::Event::SortCustomGames => {
                        self.config.custom_games.sort_by(|x, y| x.name.cmp(&y.name));
                        self.text_histories.custom_games.sort_by_key(|x| x.name.current());
                    }
                    config::Event::OnlyConstructiveBackups(value) => {
                        self.config.backup.only_constructive = value;
                        for entry in &mut self.backup_screen.log.entries {
                            entry.scan_info.only_constructive_backups = value;
                        }
                    }
                }

                self.save_config();
                task.unwrap_or_else(Task::none)
            }
            Message::CheckAppRelease => {
                if !self.cache.should_check_app_update() {
                    return Task::none();
                }

                let security = self.config.runtime.network_security;

                Task::future(async move {
                    let result = crate::metadata::Release::fetch(security).await;

                    Message::AppReleaseChecked(result.map_err(|x| x.to_string()))
                })
            }
            Message::AppReleaseChecked(outcome) => {
                self.save_cache();
                self.cache.release.checked = chrono::offset::Utc::now();

                match outcome {
                    Ok(release) => {
                        let previous_latest = self.cache.release.latest.clone();
                        self.cache.release.latest = Some(release.version.clone());

                        if previous_latest.as_ref() != Some(&release.version) {
                            // The latest available version has changed (or this is our first time checking)
                            if release.is_update() {
                                return self.show_modal(Modal::AppUpdate { release });
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("App update check failed: {e:?}");
                    }
                }

                Task::none()
            }
            Message::UpdateManifest { force } => {
                if self.updating_manifest {
                    return Task::none();
                }

                self.updating_manifest = true;
                self.manifest_notification = Some(Notification::new(TRANSLATOR.updating_manifest()));
                Self::update_manifest(
                    self.config.manifest.clone(),
                    self.cache.manifests.clone(),
                    force,
                    self.config.runtime.network_security,
                )
            }
            Message::ManifestUpdated(updates) => {
                self.updating_manifest = false;
                self.manifest_notification = None;
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
                        self.manifest = LoadedManifest {
                            primary: x.clone(),
                            extended: x.with_extensions(&self.config),
                        };
                    }
                    Err(e) => {
                        errors.push(e);
                    }
                }

                let mut tasks = vec![self.close_specific_modal(modal::Kind::UpdatingManifest)];
                if !errors.is_empty() {
                    tasks.push(self.show_modal(Modal::Errors { errors }));
                }

                Task::batch(tasks)
            }
            Message::Backup(phase) => self.handle_backup(phase),
            Message::Restore(phase) => self.handle_restore(phase),
            Message::ValidateBackups(phase) => self.handle_validation(phase),
            Message::CancelOperation => self.cancel_operation(),
            Message::ShowGameNotes { game, notes } => self.show_modal(Modal::GameNotes { game, notes }),
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
                    let path = root.path().render();
                    let lutris_database = root.lutris_database().map(|x| x.render()).unwrap_or_default();

                    if let Some(updated) = self.config.merge_root(&root) {
                        self.text_histories.roots[updated].path.push(&path);
                        self.text_histories.roots[updated]
                            .lutris_database
                            .push(&lutris_database);
                    } else {
                        self.text_histories.roots.push(RootHistory {
                            path: TextHistory::raw(&path),
                            lutris_database: TextHistory::raw(&lutris_database),
                        });
                        self.config.roots.push(root);
                    }
                }
                self.save_config();
                self.close_specific_modal(modal::Kind::ConfirmAddMissingRoots)
            }
            Message::SwitchScreen(screen) => self.switch_screen(screen),
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        self.backup_screen.log.toggle_game_expanded(
                            &name,
                            &self.backup_screen.duplicate_detector,
                            &self.config,
                            ScanKind::Backup,
                        );
                    }
                    Screen::Restore => {
                        self.restore_screen.log.toggle_game_expanded(
                            &name,
                            &self.restore_screen.duplicate_detector,
                            &self.config,
                            ScanKind::Restore,
                        );
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::ToggleGameListEntryTreeExpanded { name, keys } => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &mut self.backup_screen.log.entries {
                            if entry.scan_info.game_name == name
                                && let Some(tree) = entry.tree.as_mut()
                            {
                                tree.expand_or_collapse_keys(&keys);
                            }
                        }
                    }
                    Screen::Restore => {
                        for entry in &mut self.restore_screen.log.entries {
                            if entry.scan_info.game_name == name
                                && let Some(tree) = entry.tree.as_mut()
                            {
                                tree.expand_or_collapse_keys(&keys);
                            }
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::ToggleCustomGameExpanded { index, expanded } => {
                self.config.custom_games[index].expanded = expanded;
                self.save_config();
                Task::none()
            }
            Message::Filter { event } => {
                let mut task = None;

                match event {
                    game_filter::Event::Toggled => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.show = !self.backup_screen.log.search.show;
                            task = Some(iced::widget::operation::focus(id::backup_search()));
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.show = !self.restore_screen.log.search.show;
                            task = Some(iced::widget::operation::focus(id::restore_search()));
                        }
                        Screen::CustomGames => {
                            self.custom_games_screen.filter.enabled = !self.custom_games_screen.filter.enabled;
                            task = Some(iced::widget::operation::focus(id::custom_games_search()));
                        }
                        Screen::Other => {}
                    },
                    game_filter::Event::ToggledFilter { filter, enabled } => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.toggle_filter(filter, enabled);
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.toggle_filter(filter, enabled);
                        }
                        Screen::CustomGames => {}
                        Screen::Other => {}
                    },
                    game_filter::Event::EditedGameName(value) => match self.screen {
                        Screen::Backup => {
                            self.text_histories.backup_search_game_name.push(&value);
                            self.backup_screen.log.search.game_name = value;
                        }
                        Screen::Restore => {
                            self.text_histories.restore_search_game_name.push(&value);
                            self.restore_screen.log.search.game_name = value;
                        }
                        Screen::CustomGames => {
                            self.text_histories.custom_games_search_game_name.push(&value);
                            self.custom_games_screen.filter.name = value;
                        }
                        Screen::Other => {}
                    },
                    game_filter::Event::Reset => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.reset();
                            self.text_histories.backup_search_game_name.push("");
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.reset();
                            self.text_histories.restore_search_game_name.push("");
                        }
                        Screen::CustomGames => {
                            self.custom_games_screen.filter.reset();
                            self.text_histories.custom_games_search_game_name.push("");
                        }
                        Screen::Other => {}
                    },
                    game_filter::Event::EditedFilterUniqueness(value) => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.uniqueness.choice = value;
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.uniqueness.choice = value;
                        }
                        Screen::CustomGames => {}
                        Screen::Other => {}
                    },
                    game_filter::Event::EditedFilterCompleteness(value) => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.completeness.choice = value;
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.completeness.choice = value;
                        }
                        Screen::CustomGames => {}
                        Screen::Other => {}
                    },
                    game_filter::Event::EditedFilterEnablement(value) => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.enablement.choice = value;
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.enablement.choice = value;
                        }
                        Screen::CustomGames => {}
                        Screen::Other => {}
                    },
                    game_filter::Event::EditedFilterChange(value) => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.change.choice = value;
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.change.choice = value;
                        }
                        Screen::CustomGames => {}
                        Screen::Other => {}
                    },
                    game_filter::Event::EditedFilterManifest(value) => match self.screen {
                        Screen::Backup => {
                            self.backup_screen.log.search.manifest.choice = value;
                        }
                        Screen::Restore => {
                            self.restore_screen.log.search.manifest.choice = value;
                        }
                        Screen::CustomGames => {}
                        Screen::Other => {}
                    },
                }

                task.unwrap_or_else(Task::none)
            }
            Message::BrowseDir(subject) => Task::future(async move {
                let choice = async move { rfd::AsyncFileDialog::new().pick_folder().await }.await;

                Message::browsed_dir(subject, choice.map(|x| x.path().to_path_buf()))
            }),
            Message::BrowseFile(subject) => Task::future(async move {
                let dialog = rfd::AsyncFileDialog::new();
                #[cfg(windows)]
                let dialog = if let BrowseFileSubject::BackgroundExecutable(name) = &subject {
                    dialog
                        .set_title(TRANSLATOR.automatic_backup_game("automatic-backup-choose-executable-for", name))
                        .add_filter("Game executable", &["exe"])
                } else {
                    dialog
                };
                let choice = dialog.pick_file().await;

                Message::browsed_file(subject, choice.map(|x| x.path().to_path_buf()))
            }),
            Message::SelectedFile(subject, path) => {
                match subject {
                    #[cfg(windows)]
                    BrowseFileSubject::BackgroundExecutable(name) => {
                        return self.edit_background(background::Edit::AddExecutable(name, path));
                    }
                    BrowseFileSubject::RcloneExecutable => {
                        self.text_histories.rclone_executable.push(path.raw());
                        self.config.apps.rclone.path = path;
                    }
                    BrowseFileSubject::RootLutrisDatabase(i) => {
                        self.text_histories.roots[i].lutris_database.push(path.raw());
                        if let Root::Lutris(root) = &mut self.config.roots[i] {
                            root.database = Some(path);
                        }
                    }
                    BrowseFileSubject::SecondaryManifest(i) => {
                        self.text_histories.secondary_manifests[i].push(path.raw());
                        self.config.manifest.secondary[i].set(path.raw().into());
                    }
                }
                self.save_config();
                Task::none()
            }
            Message::SelectAllGames => {
                match self.screen {
                    Screen::Backup => {
                        for name in self.backup_screen.log.visible_games(
                            ScanKind::Backup,
                            &self.config,
                            &self.manifest.extended,
                            &self.backup_screen.duplicate_detector,
                        ) {
                            self.config.enable_game_for_backup(&name);
                        }
                    }
                    Screen::Restore => {
                        for name in self.restore_screen.log.visible_games(
                            ScanKind::Restore,
                            &self.config,
                            &self.manifest.extended,
                            &self.restore_screen.duplicate_detector,
                        ) {
                            self.config.enable_game_for_restore(&name);
                        }
                    }
                    Screen::CustomGames => {
                        for i in self.custom_games_screen.visible_games(&self.config) {
                            self.config.enable_custom_game(i);
                        }
                    }
                    _ => {}
                }
                self.save_config();
                Task::none()
            }
            Message::DeselectAllGames => {
                match self.screen {
                    Screen::Backup => {
                        for name in self.backup_screen.log.visible_games(
                            ScanKind::Backup,
                            &self.config,
                            &self.manifest.extended,
                            &self.backup_screen.duplicate_detector,
                        ) {
                            self.config.disable_game_for_backup(&name);
                        }
                    }
                    Screen::Restore => {
                        for name in self.restore_screen.log.visible_games(
                            ScanKind::Restore,
                            &self.config,
                            &self.manifest.extended,
                            &self.restore_screen.duplicate_detector,
                        ) {
                            self.config.disable_game_for_restore(&name);
                        }
                    }
                    Screen::CustomGames => {
                        for i in self.custom_games_screen.visible_games(&self.config) {
                            self.config.disable_custom_game(i);
                        }
                    }
                    _ => {}
                }
                self.save_config();
                Task::none()
            }
            Message::OpenDir { path } => {
                let path2 = path.clone();
                Task::future(async move {
                    let result = async { opener::open(path.resolve()) }.await;

                    match result {
                        Ok(_) => Message::Ignore,
                        Err(e) => {
                            log::error!("Unable to open directory: `{}` - {:?}", path2.resolve(), e);
                            Message::OpenDirFailure { path: path2 }
                        }
                    }
                })
            }
            Message::OpenDirSubject(subject) => {
                let path = match subject {
                    BrowseSubject::BackupTarget => self.config.backup.path.clone(),
                    BrowseSubject::RestoreSource => self.config.restore.path.clone(),
                    BrowseSubject::Root(i) => self.config.roots[i].path().clone(),
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
                    #[cfg(windows)]
                    BrowseFileSubject::BackgroundExecutable(name) => {
                        let Some(path) = self
                            .background_settings
                            .config
                            .games
                            .iter()
                            .find(|game| game.name == name)
                            .and_then(|game| game.executables.first())
                        else {
                            return Task::none();
                        };
                        path.clone()
                    }
                    BrowseFileSubject::RcloneExecutable => self.config.apps.rclone.path.clone(),
                    BrowseFileSubject::RootLutrisDatabase(i) => {
                        let Root::Lutris(root) = &self.config.roots[i] else {
                            return Task::none();
                        };
                        let Some(database) = root.database.as_ref() else {
                            return Task::none();
                        };
                        database.clone()
                    }
                    BrowseFileSubject::SecondaryManifest(i) => {
                        let Some(path) = self.config.manifest.secondary[i].path() else {
                            return Task::none();
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
                            iced::widget::operation::focus_previous()
                        } else {
                            iced::widget::operation::focus_next()
                        }
                    }
                    _ => Task::none(),
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
                    UndoSubject::CustomGamesSearchGameName => shortcut.apply_to_string_field(
                        &mut self.custom_games_screen.filter.name,
                        &mut self.text_histories.custom_games_search_game_name,
                    ),
                    UndoSubject::RootPath(i) => shortcut.apply_to_strict_path_field(
                        self.config.roots[i].path_mut(),
                        &mut self.text_histories.roots[i].path,
                    ),
                    UndoSubject::RootLutrisDatabase(i) => {
                        if let Root::Lutris(root) = &mut self.config.roots[i] {
                            shortcut.apply_to_option_strict_path_field(
                                &mut root.database,
                                &mut self.text_histories.roots[i].lutris_database,
                            )
                        }
                    }
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
                    UndoSubject::CustomGameInstallDir(i, j) => shortcut.apply_to_string_field(
                        &mut self.config.custom_games[i].install_dir[j],
                        &mut self.text_histories.custom_games[i].install_dir[j],
                    ),
                    UndoSubject::CustomGameWinePrefix(i, j) => shortcut.apply_to_string_field(
                        &mut self.config.custom_games[i].wine_prefix[j],
                        &mut self.text_histories.custom_games[i].wine_prefix[j],
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
                        return Task::none();
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
                Task::none()
            }
            Message::SelectedBackupToRestore { game, backup } => {
                self.backups_to_restore.insert(game.clone(), backup.id());
                self.handle_restore(RestorePhase::Start {
                    preview: true,
                    games: Some(GameSelection::single(game)),
                })
            }
            Message::GameAction { action, game } => match action {
                GameAction::PreviewBackup => self.handle_backup(BackupPhase::Start {
                    preview: true,
                    repair: false,
                    jump: false,
                    games: Some(GameSelection::single(game)),
                }),
                GameAction::Backup { confirm } => {
                    if confirm {
                        self.handle_backup(BackupPhase::Confirm {
                            games: Some(GameSelection::single(game)),
                        })
                    } else {
                        self.handle_backup(BackupPhase::Start {
                            preview: false,
                            repair: false,
                            jump: false,
                            games: Some(GameSelection::single(game)),
                        })
                    }
                }
                GameAction::PreviewRestore => self.handle_restore(RestorePhase::Start {
                    preview: true,
                    games: Some(GameSelection::single(game)),
                }),
                GameAction::Restore { confirm } => {
                    if confirm {
                        self.handle_restore(RestorePhase::Confirm {
                            games: Some(GameSelection::single(game)),
                        })
                    } else {
                        self.handle_restore(RestorePhase::Start {
                            preview: false,
                            games: Some(GameSelection::single(game)),
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
                    Task::none()
                }
                GameAction::MakeAlias => self.customize_game_as_alias(game),
            },
            Message::Scrolled { subject, position } => {
                self.scroll_offsets.insert(subject, position);
                Task::none()
            }
            Message::Scroll { subject, position } => {
                self.scroll_offsets.insert(subject, position);
                iced::widget::operation::scroll_to(subject.id(), position)
            }
            Message::EditedBackupComment { game, action } => {
                if let Some(comment) = self.restore_screen.log.apply_comment_action(&game, action) {
                    self.save_backup(&game);
                    if let Some(info) = self.text_histories.backup_comments.get_mut(&game) {
                        info.push(&comment);
                    }
                }

                Task::none()
            }
            Message::FilterDuplicates { scan_kind, game } => {
                let log = match scan_kind {
                    ScanKind::Backup => &mut self.backup_screen.log,
                    ScanKind::Restore => &mut self.restore_screen.log,
                };
                log.filter_duplicates_of = game;
                Task::none()
            }
            Message::OpenUrl(url) => Self::open_url(url),
            Message::OpenUrlAndCloseModal(url) => Task::batch([Self::open_url(url), self.close_modal()]),
            Message::EditedCloudRemote(choice) => {
                if let Ok(remote) = Remote::try_from(choice) {
                    match &remote {
                        Remote::Custom { id } => {
                            self.text_histories.cloud_remote_id.push(id);
                            self.config.cloud.remote = Some(remote);
                            self.save_config();
                            Task::none()
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
                    Task::none()
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
                                    if let Some(modal) = self.modals.last_mut() {
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

                        if let Some(modal) = self.modals.last_mut() {
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
                        return Task::batch([
                            self.close_specific_modal(modal::Kind::ConfirmCloudSync),
                            self.show_error(Error::UnableToSynchronizeCloud(e)),
                        ]);
                    }
                    rclone_monitor::Event::Cancelled => {
                        #[cfg(windows)]
                        if self.background_backup.is_some() {
                            self.operation_should_cancel
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        self.go_idle();
                        return self.close_specific_modal(modal::Kind::ConfirmCloudSync);
                    }
                }
                Task::none()
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
                        if let Some(Modal::ConfigureWebDavRemote { provider }) = self.modals.last_mut() {
                            *provider = new;
                        }
                    }
                }
                Task::none()
            }
            Message::FinalizeRemote(remote) => self.configure_remote(remote),
            Message::ModalChangePage(page) => {
                if let Some(modal) = self.modals.last_mut() {
                    modal.set_page(page);
                }
                Task::none()
            }
            Message::ShowCustomGame { name } => {
                use crate::gui::widget::operation::container_scroll_offset;

                let subject = ScrollSubject::CustomGames;

                self.scroll_offsets.remove(&subject);
                self.screen = Screen::CustomGames;

                container_scroll_offset(name.clone().into()).map(move |offset| match offset {
                    Some(position) => Message::Scroll { subject, position },
                    None => Message::Ignore,
                })
            }
            Message::ShowScanActiveGames => self.show_modal(Modal::ActiveScanGames),
            Message::CopyText(text) => iced::clipboard::write(text),
            #[cfg_attr(not(windows), allow(unused))]
            Message::OpenRegistry(item) => {
                #[cfg(windows)]
                {
                    use windows::{
                        Win32::UI::{
                            Shell::{SHELLEXECUTEINFOA, ShellExecuteExA},
                            WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL},
                        },
                        core::s,
                    };

                    let mut system = sysinfo::System::new_all();
                    system.refresh_all();
                    if system.processes_by_exact_name("regedit.exe".as_ref()).next().is_some() {
                        let mut info = SHELLEXECUTEINFOA {
                            cbSize: size_of::<SHELLEXECUTEINFOA>() as u32,
                            lpVerb: s!("runas"),
                            lpFile: s!("taskkill.exe"),
                            lpParameters: s!("/im regedit.exe"),
                            nShow: SW_HIDE.0,
                            ..Default::default()
                        };
                        unsafe {
                            if let Err(e) = ShellExecuteExA(&mut info) {
                                log::error!("Failed to close Regedit: {e:?}");
                                return Task::none();
                            }
                        }

                        // When already running as admin (i.e., no UAC prompts),
                        // this is needed or else Regedit won't reopen.
                        // Maybe `taskkill` returns while the process is still shutting down?
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }

                    let hive = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
                    let Ok(key) = hive.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Applets\Regedit")
                    else {
                        return Task::none();
                    };
                    if let Err(e) = key.0.set_value("LastKey", &format!("Computer\\{}", item.interpret())) {
                        log::error!("Failed to edit Regedit last key: {e:?}");
                        return Task::none();
                    }

                    let mut info = SHELLEXECUTEINFOA {
                        cbSize: size_of::<SHELLEXECUTEINFOA>() as u32,
                        lpVerb: s!("runas"),
                        lpFile: s!("regedit.exe"),
                        nShow: SW_SHOWNORMAL.0,
                        ..Default::default()
                    };
                    unsafe {
                        if let Err(e) = ShellExecuteExA(&mut info) {
                            log::error!("Failed to open Regedit: {e:?}");
                            return Task::none();
                        }
                    }
                }
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Keyboard(event) => Some(Message::KeyboardEvent(event)),
                #[cfg(windows)]
                iced::Event::Window(iced::window::Event::Opened { .. }) => Some(Message::WindowOpened(_window)),
                #[cfg(windows)]
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::CloseWindow(_window)),
                #[cfg(not(windows))]
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::Exit { user: true }),
                _ => None,
            }),
            rclone_monitor::run().map(Message::RcloneMonitor),
        ];

        #[cfg(windows)]
        if self.background.is_some() {
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| Message::BackgroundTick));
        }

        if self.timed_notification.is_some() {
            subscriptions.push(iced::time::every(Duration::from_millis(250)).map(|_| Message::PruneNotifications));
        }

        if self.progress.visible() {
            subscriptions.push(iced::time::every(Duration::from_millis(100)).map(|_| Message::UpdateTime));
        }

        if !self.pending_save.is_empty() {
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| Message::Save));
        }

        if self.flags.update_manifest {
            subscriptions.push(
                iced::time::every(Duration::from_secs(60 * 60 * 24)).map(|_| Message::UpdateManifest { force: false }),
            );
        }

        if self.config.release.check {
            subscriptions.push(iced::time::every(Duration::from_secs(60 * 60 * 24)).map(|_| Message::CheckAppRelease));
        }

        if self.exiting {
            subscriptions.push(iced::time::every(Duration::from_millis(50)).map(|_| Message::Exit { user: false }));
        }

        iced::Subscription::batch(subscriptions)
    }

    pub fn view(&self, #[cfg(windows)] window: iced::window::Id) -> Element {
        #[cfg(windows)]
        if self.background_settings_window == Some(window) {
            let content = self.background_settings.view(
                &self.backup_screen.log,
                self.background.as_ref().is_some_and(|background| background.paused),
            );
            return Stack::new()
                .push(
                    Container::new(super::widget::Scrollable::new(Container::new(content).padding(15)))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .class(style::Container::Primary),
                )
                .push(
                    self.modals
                        .last()
                        .filter(|modal| matches!(modal.kind(), modal::Kind::Error | modal::Kind::Errors))
                        .map(|modal| modal.view(&self.config, &self.text_histories, &self.operation)),
                )
                .into();
        }
        let content = Column::new()
            .align_x(Alignment::Center)
            .push(
                Row::new()
                    .padding([10, 20])
                    .spacing(20)
                    .push(button::nav(Screen::Backup, self.screen))
                    .push(button::nav(Screen::Restore, self.screen))
                    .push(button::nav(Screen::CustomGames, self.screen))
                    .push(button::nav(Screen::Other, self.screen)),
            )
            .push(match self.screen {
                Screen::Backup => self.backup_screen.view(
                    &self.config,
                    &self.manifest.extended,
                    &self.operation,
                    &self.text_histories,
                    &self.modifiers,
                ),
                Screen::Restore => self.restore_screen.view(
                    &self.config,
                    &self.manifest.extended,
                    &self.operation,
                    &self.text_histories,
                    &self.modifiers,
                ),
                Screen::CustomGames => self.custom_games_screen.view(
                    &self.config,
                    &self.manifest.extended,
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
            .push(self.timed_notification.as_ref().map(|x| x.view()))
            .push(self.manifest_notification.as_ref().map(|x| x.view()));

        let stack = Stack::new()
            .push(Container::new(content).class(style::Container::Primary))
            .push(
                self.modals
                    .last()
                    .map(|modal| modal.view(&self.config, &self.text_histories, &self.operation)),
            );

        Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .push(stack)
            .push_if(self.progress.visible(), || self.progress.view(&self.operation))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::game_list::GameListEntry,
        scan::{Duplication, ScanChange, ScanInfo, ScannedFile},
    };

    #[test]
    fn clearing_duplicates_filter_restores_visible_games_after_disabling_all_matches() {
        for scan_kind in [ScanKind::Backup, ScanKind::Restore] {
            let mut app = App::default();
            app.config.scan.show_deselected_games = false;

            let (log, detector) = match scan_kind {
                ScanKind::Backup => (&mut app.backup_screen.log, &mut app.backup_screen.duplicate_detector),
                ScanKind::Restore => (&mut app.restore_screen.log, &mut app.restore_screen.duplicate_detector),
            };
            for (name, path) in [
                ("duplicate1", "shared.sav"),
                ("duplicate2", "shared.sav"),
                ("other", "other.sav"),
            ] {
                let scan_info = ScanInfo {
                    game_name: name.to_owned(),
                    found_files: HashMap::from([(
                        StrictPath::from(path),
                        ScannedFile {
                            original_path: (scan_kind == ScanKind::Restore).then(|| StrictPath::from(path)),
                            change: ScanChange::New,
                            ..Default::default()
                        },
                    )]),
                    backup: (scan_kind == ScanKind::Restore)
                        .then(|| crate::scan::layout::Backup::Full(Default::default())),
                    has_backups: scan_kind == ScanKind::Restore,
                    ..Default::default()
                };
                detector.add_game(&scan_info, true);
                log.entries.push(GameListEntry {
                    scan_info,
                    scanned: true,
                    ..Default::default()
                });
            }

            let visible_games = |app: &App| {
                let (log, detector) = match scan_kind {
                    ScanKind::Backup => (&app.backup_screen.log, &app.backup_screen.duplicate_detector),
                    ScanKind::Restore => (&app.restore_screen.log, &app.restore_screen.duplicate_detector),
                };
                log.visible_games(scan_kind, &app.config, &app.manifest.extended, detector)
            };
            let _ = app.update(Message::FilterDuplicates {
                scan_kind,
                game: Some("duplicate1".to_owned()),
            });
            assert_eq!(
                visible_games(&app),
                HashSet::from(["duplicate1".to_owned(), "duplicate2".to_owned()])
            );

            for name in ["duplicate1", "duplicate2"] {
                let _ = app.update(Message::Config {
                    event: config::Event::GameListEntryEnabled {
                        name: name.to_owned(),
                        enabled: false,
                        scan_kind,
                    },
                });
            }
            assert!(visible_games(&app).is_empty());
            let detector = match scan_kind {
                ScanKind::Backup => &app.backup_screen.duplicate_detector,
                ScanKind::Restore => &app.restore_screen.duplicate_detector,
            };
            assert_eq!(detector.overall(), Duplication::Resolved);

            let _ = app.update(Message::FilterDuplicates { scan_kind, game: None });
            assert_eq!(visible_games(&app), HashSet::from(["other".to_owned()]));
            assert!(!app.config.scan.show_deselected_games);
            let log = match scan_kind {
                ScanKind::Backup => &app.backup_screen.log,
                ScanKind::Restore => &app.restore_screen.log,
            };
            assert!(!log.is_filtered());
        }
    }
    #[cfg(windows)]
    mod automatic_backup_tests {
        use super::*;
        use crate::{
            gui::game_list::GameListEntry,
            scan::{ScanInfo, layout::GameLayout},
        };

        #[test]
        fn automatic_backup_uses_backup_scan_results() {
            let mut app = App::default();
            app.operation = Operation::new_backup(Finality::Final, Some(GameSelection::single("Example".into())));
            app.operation_steps_active = 2;
            app.background_backup = Some(background::BackupRun::new("Example".into()));
            let scan = ScanInfo {
                game_name: "Example".into(),
                found_files: HashMap::from([(
                    StrictPath::new("save.dat"),
                    crate::scan::ScannedFile {
                        change: crate::scan::ScanChange::New,
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            };
            let _ = app.handle_backup(BackupPhase::GameScanned {
                scan_info: Some(scan),
                backup_info: Some(crate::scan::BackupInfo::default()),
            });
            assert!(matches!(
                app.background_backup.as_ref().unwrap().result,
                background::BackupResult::Saved
            ));
        }

        #[test]
        fn automatic_backup_finishes_once_and_manual_backups_do_not_notify() {
            let mut app = App::default();
            assert!(app.finish_background_backup().is_none());
            app.background_backup = Some(background::BackupRun::new("Example".into()));
            app.background_backup.as_mut().unwrap().result = background::BackupResult::Saved;
            let notice = app.finish_background_backup().unwrap();
            assert_eq!(notice.title, "Automatic backup complete");
            assert!(app.finish_background_backup().is_none());
        }

        fn note_editor() -> (tempfile::TempDir, App, StrictPath) {
            let temp = tempfile::tempdir().unwrap();
            let root = StrictPath::new(temp.path().to_string_lossy().into_owned());
            let file = root.joined("game3/mapping.yaml");
            file.create_parent_dir().unwrap();
            file.write_with_content(include_str!("../../tests/backup/game3-renamed/mapping.yaml"))
                .unwrap();
            root.joined("game3/registry.yaml")
                .write_with_content(include_str!("../../tests/backup/game3-renamed/registry.yaml"))
                .unwrap();
            let layout = GameLayout::load(root.joined("game3")).unwrap();
            let backup = layout.restorable_backups_flattened().pop().unwrap();
            let mut app = App {
                screen: Screen::Restore,
                ..Default::default()
            };
            app.config.backup.path = root.clone();
            app.config.restore.path = root;
            app.restore_screen.log.entries.push(GameListEntry {
                scan_info: ScanInfo {
                    game_name: "game3".into(),
                    backup: Some(backup),
                    ..Default::default()
                },
                game_layout: Some(layout),
                comment_editor: Some(Default::default()),
                ..Default::default()
            });
            (temp, app, file)
        }

        #[test]
        fn leaving_restore_does_not_flush_notes_or_start_cloud_sync() {
            let (_temp, mut app, file) = note_editor();
            app.config.cloud.synchronize = true;
            assert!(app.restore_screen.log.set_comment("game3", "keep this backup".into()));
            app.save_backup("game3");
            assert!(!file.try_read().unwrap().contains("keep this backup"));
            let _ = app.switch_screen(Screen::Backup);
            assert!(app.operation.idle());
            assert!(app.modals.is_empty());
            assert!(app.pending_save.contains_key(&SaveKind::Backup("game3".into())));
            assert!(!file.try_read().unwrap().contains("keep this backup"));
            assert!(app.save_pending(true));
            assert!(file.try_read().unwrap().contains("keep this backup"));
            assert!(app.pending_save.is_empty());
        }

        #[test]
        fn pending_note_does_not_add_cloud_errors_to_another_games_backup() {
            let (_temp, mut app, file) = note_editor();
            app.config.cloud.synchronize = true;
            app.config.cloud.remote = None;
            assert!(app.restore_screen.log.set_comment("game3", "local note".into()));
            app.save_backup("game3");
            let games = GameSelection::single("another game".into());

            let _ = app.handle_backup(BackupPhase::Start {
                preview: false,
                repair: false,
                jump: false,
                games: Some(games.clone()),
            });

            assert!(matches!(app.operation, Operation::Backup { .. }));
            assert_eq!(app.operation.games(), Some(&games));
            assert!(app.operation.errors().unwrap().is_empty());
            assert!(file.try_read().unwrap().contains("local note"));
        }

        #[test]
        fn local_lock_edits_still_save_without_cloud_sync() {
            let (_temp, mut app, file) = note_editor();
            app.config.cloud.synchronize = true;
            let _ = app.update(Message::GameAction {
                action: GameAction::Lock,
                game: "game3".into(),
            });
            assert!(app.operation.idle());
            assert!(app.save_pending(true));
            let layout = GameLayout::load(file.popped()).unwrap();
            assert!(layout.restorable_backups_flattened().last().unwrap().locked());
        }

        #[test]
        fn metadata_edits_wait_until_backup_finishes() {
            let (_temp, mut app, file) = note_editor();
            assert!(app.restore_screen.log.set_comment("game3", "pending note".into()));
            app.save_backup("game3");
            app.operation = Operation::new_backup(Finality::Final, Some(GameSelection::single("game3".into())));
            assert!(app.save_pending(true));
            assert!(app.pending_save.contains_key(&SaveKind::Backup("game3".into())));
            assert!(!file.try_read().unwrap().contains("pending note"));
            app.go_idle();
            assert!(app.save_pending(true));
            assert!(file.try_read().unwrap().contains("pending note"));
        }

        #[test]
        fn metadata_edits_preserve_newer_backups() {
            let (_temp, mut app, file) = note_editor();
            app.config.cloud.synchronize = false;
            assert!(app.restore_screen.log.set_comment("game3", "keep this note".into()));
            app.save_backup("game3");
            let newer = format!(
                "{}\n  - name: newer-backup\n    when: \"2026-11-01T00:00:00Z\"\n    files: {{}}\n    registry: {{hash: null}}\n    children: []\n",
                file.try_read().unwrap()
            );
            file.write_with_content(&newer).unwrap();
            assert!(app.save_pending(true));
            let saved = file.try_read().unwrap();
            assert!(saved.contains("newer-backup"));
            assert!(saved.contains("keep this note"));
        }

        #[test]
        fn failed_metadata_save_while_exiting_unblocks_the_window() {
            let (_temp, mut app, file) = note_editor();
            assert!(app.restore_screen.log.set_comment("game3", "unsaved note".into()));
            app.save_backup("game3");
            file.write_with_content("name: game3\ndrives: {}\nbackups: []\n")
                .unwrap();
            app.exiting = true;
            app.modals.push(Modal::Exiting);
            app.go_idle();
            assert!(!app.exiting);
            assert!(app.pending_save.contains_key(&SaveKind::Backup("game3".into())));
            assert!(matches!(app.modals.last(), Some(Modal::Error { .. })));
            assert!(!app.modals.iter().any(|modal| modal.kind() == modal::Kind::Exiting));
            assert!(app.timed_notification.is_some());
        }

        #[test]
        fn stale_metadata_can_be_discarded_before_refreshing() {
            let (_temp, mut app, file) = note_editor();
            app.config.cloud.synchronize = false;
            assert!(app.restore_screen.log.set_comment("game3", "unsaved note".into()));
            app.save_backup("game3");
            let replaced = "name: game3\ndrives: {}\nbackups: []\n";
            file.write_with_content(replaced).unwrap();
            assert!(!app.save_pending(true));
            assert!(app.pending_save.contains_key(&SaveKind::Backup("game3".into())));
            assert_eq!(file.try_read().unwrap(), replaced);
            let _ = app.update(Message::DiscardBackupMetadata("game3".into()));
            assert!(app.pending_save.is_empty());
            assert!(matches!(app.operation, Operation::Restore { .. }));
            assert_eq!(file.try_read().unwrap(), replaced);
        }

        #[test]
        fn background_selection_rejects_games_not_discovered_in_backup() {
            let mut app = App::default();
            let _ = app.update(Message::BackgroundEdit(background::Edit::Search("Example game".into())));
            let _ = app.update(Message::SelectedFile(
                BrowseFileSubject::BackgroundExecutable("Example game".into()),
                StrictPath::new("C:/games/example.exe"),
            ));
            assert_eq!(app.background_settings.search, "Example game");
            assert!(app.background_settings.config.games.is_empty());
            assert!(!app.modals.is_empty());
        }

        #[test]
        fn executable_picker_keeps_the_chosen_game_and_cancel_changes_nothing() {
            let path = std::env::current_exe().unwrap();
            let chosen = "First game".to_string();
            let message = Message::browsed_file(
                BrowseFileSubject::BackgroundExecutable(chosen.clone()),
                Some(path.clone()),
            );
            assert!(matches!(message, Message::SelectedFile(
                BrowseFileSubject::BackgroundExecutable(name), executable
            ) if name == chosen && executable == StrictPath::from(path)));
            let mut app = App::default();
            let _ = app.update(Message::browsed_file(
                BrowseFileSubject::BackgroundExecutable(chosen),
                None,
            ));
            assert!(app.background_settings.config.games.is_empty());
            assert!(app.modals.is_empty());
        }
        #[test]
        fn settings_window_closes_without_closing_the_main_window() {
            let main = iced::window::Id::unique();
            let settings = iced::window::Id::unique();
            let mut app = App {
                window: Some(main),
                background_settings_window: Some(settings),
                ..Default::default()
            };
            let _ = app.update(Message::WindowOpened(settings));
            assert_eq!(app.window, Some(main));
            let _ = app.update(Message::CloseWindow(settings));
            assert_eq!(app.window, Some(main));
            assert!(app.background_settings_window.is_none());
            assert!(!app.exiting);
        }
    }
}
