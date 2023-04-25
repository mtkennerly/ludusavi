use std::collections::HashMap;

use iced::{Alignment, Application, Command, Subscription};

use crate::{
    cloud::{rclone_monitor, Rclone, Remote},
    gui::{
        button,
        common::*,
        modal::Modal,
        notification::Notification,
        screen,
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{Column, Container, Element, IcedParentExt, ProgressBar, Row, Text},
    },
    lang::TRANSLATOR,
    prelude::{app_dir, get_threads_from_env, initialize_rayon, Error, Finality, StrictPath},
    resource::{
        cache::Cache,
        config::{Config, CustomGame, RootsConfig},
        manifest::{Manifest, Store},
        ResourceFile, SaveableResourceFile,
    },
    scan::{
        heroic::HeroicGames, layout::BackupLayout, prepare_backup_target, registry_compat::RegistryItem,
        scan_game_for_backup, BackupId, InstallDirRanking, OperationStepDecision, SteamShortcuts, TitleFinder,
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

#[derive(Default)]
struct Progress {
    pub max: f32,
    pub current: f32,
    prepared: bool,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Progress {
    pub fn visible(&self) -> bool {
        self.max > 1.0
    }

    pub fn reset(&mut self) {
        self.max = 0.0;
        self.current = 0.0;
        self.prepared = false;
        self.start_time = None;
    }

    pub fn start(&mut self) {
        self.max = 100.0;
        self.current = 0.0;
        self.prepared = false;
        self.start_time = Some(chrono::Utc::now());
    }

    pub fn step(&mut self) {
        self.current += 1.0;
        self.prepared = true;
    }

    pub fn set(&mut self, current: f32, max: f32) {
        self.current = current;
        self.max = max;
        self.prepared = true;
    }

    pub fn set_max(&mut self, max: f32) {
        self.max = max;
        self.prepared = true;
    }

    pub fn view(&self, operation: &Option<OngoingOperation>) -> Element {
        use OngoingOperation as Op;

        let label = operation.map(|op| match op {
            Op::Backup
            | Op::CancelBackup
            | Op::PreviewBackup
            | Op::CancelPreviewBackup
            | Op::Restore
            | Op::CancelRestore
            | Op::PreviewRestore
            | Op::CancelPreviewRestore => TRANSLATOR.scan_label(),
            Op::CloudSync { .. } | Op::CancelCloudSync { .. } => TRANSLATOR.cloud_label(),
        });

        let elapsed = self.start_time.as_ref().map(|start| {
            let elapsed = chrono::Utc::now().time() - start.time();
            format!(
                "({:0>2}:{:0>2}:{:0>2})",
                elapsed.num_hours(),
                elapsed.num_minutes(),
                elapsed.num_seconds()
            )
        });

        let count = if !self.prepared {
            None
        } else {
            operation.map(|op| match op {
                Op::Backup
                | Op::CancelBackup
                | Op::PreviewBackup
                | Op::CancelPreviewBackup
                | Op::Restore
                | Op::CancelRestore
                | Op::PreviewRestore
                | Op::CancelPreviewRestore => format!("{} / {} {}", self.current, self.max, TRANSLATOR.games_unit()),
                Op::CloudSync { .. } | Op::CancelCloudSync { .. } => {
                    TRANSLATOR.cloud_progress(self.current as u64, self.max as u64)
                }
            })
        };

        Container::new(
            Row::new()
                .spacing(5)
                .padding([0, 5, 0, 5])
                .align_items(Alignment::Center)
                .push_some(|| label.map(|x| Text::new(x).size(15)))
                .push_some(|| elapsed.map(|x| Text::new(x).size(15)))
                .push(ProgressBar::new(0.0..=self.max, self.current).height(15))
                .push_some(|| count.map(|x| Text::new(x).size(15))),
        )
        .style(style::Container::ModalBackground)
        .into()
    }
}

#[derive(Default)]
pub struct App {
    config: Config,
    manifest: Manifest,
    cache: Cache,
    operation: Option<OngoingOperation>,
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
    scroll_offsets: HashMap<ScrollSubject, iced_native::widget::scrollable::RelativeOffset>,
    text_histories: TextHistories,
    rclone_monitor_sender: Option<iced_native::futures::channel::mpsc::Sender<rclone_monitor::Input>>,
}

impl App {
    fn go_idle(&mut self) {
        self.operation = None;
        self.operation_steps.clear();
        self.operation_steps_active = 0;
        self.modal = None;
        self.progress.reset();
        self.operation_should_cancel
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        self.notify_on_single_game_scanned = None;
    }

    fn show_error(&mut self, error: Error) {
        self.modal = Some(Modal::Error { variant: error });
    }

    fn confirm_backup_start(&mut self, games: Option<Vec<String>>) -> Command<Message> {
        self.modal = Some(Modal::ConfirmBackup { games });
        Command::none()
    }

    fn confirm_restore_start(&mut self, games: Option<Vec<String>>) -> Command<Message> {
        self.modal = Some(Modal::ConfirmRestore { games });
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
                | OngoingOperation::CancelPreviewBackup
                | OngoingOperation::CloudSync { .. }
                | OngoingOperation::CancelCloudSync { .. },
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

        if preview && full {
            self.backup_screen.previewed_games.clear();
        }

        let all_scanned = !self.backup_screen.log.contains_unscanned_games();
        if let Some(games) = &games {
            self.backup_screen.log.unscan_games(games);
        } else {
            self.backup_screen.log.clear();
            self.backup_screen.duplicate_detector.clear();
        }
        self.modal = None;
        self.progress.start();

        self.operation = Some(if preview {
            OngoingOperation::PreviewBackup
        } else {
            OngoingOperation::Backup
        });

        let mut all_games = self.manifest.clone();
        let config = self.config.clone();
        let previewed_games = self.backup_screen.previewed_games.clone();

        Command::perform(
            async move {
                all_games.incorporate_extensions(&config.roots, &config.custom_games);
                if let Some(games) = &games {
                    all_games.0.retain(|k, _| games.contains(k));
                } else if !previewed_games.is_empty() && all_scanned {
                    all_games.0.retain(|k, _| previewed_games.contains(k));
                }
                let subjects: Vec<_> = all_games.0.keys().cloned().collect();

                let roots = config.expanded_roots();
                let layout = BackupLayout::new(config.backup.path.clone(), config.backup.retention.clone());
                let title_finder = TitleFinder::new(&all_games, &layout);
                let ranking = InstallDirRanking::scan(&roots, &all_games, &subjects);
                let steam = SteamShortcuts::scan();
                let heroic = HeroicGames::scan(&roots, &title_finder, None);

                (games, subjects, all_games, layout, ranking, steam, heroic)
            },
            move |(games, subjects, all_games, layout, ranking, steam, heroic)| Message::BackupPerform {
                preview,
                full,
                games,
                subjects,
                all_games,
                layout,
                ranking,
                steam,
                heroic,
            },
        )
    }

    fn perform_backup(
        &mut self,
        preview: bool,
        full: bool,
        games: Option<Vec<String>>,
        subjects: Vec<String>,
        all_games: Manifest,
        layout: BackupLayout,
        ranking: InstallDirRanking,
        steam: SteamShortcuts,
        heroic: HeroicGames,
    ) -> Command<Message> {
        log::info!("beginning backup with {} steps", self.progress.max);

        if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
            self.go_idle();
            return Command::none();
        }

        if subjects.is_empty() {
            if let Some(games) = &games {
                for game in games {
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
                self.cache.save();
            }
            self.go_idle();
            return Command::none();
        }

        self.progress.set_max(all_games.0.len() as f32);
        self.register_notify_on_single_game_scanned(&games);

        let config = std::sync::Arc::new(self.config.clone());
        let roots = std::sync::Arc::new(config.expanded_roots());
        let layout = std::sync::Arc::new(layout);
        let heroic_games = std::sync::Arc::new(heroic);
        let filter = std::sync::Arc::new(self.config.backup.filter.clone());
        let ranking = std::sync::Arc::new(ranking);
        let steam_shortcuts = std::sync::Arc::new(steam);

        for key in subjects {
            let game = all_games.0[&key].clone();
            let config = config.clone();
            let roots = roots.clone();
            let heroic_games = heroic_games.clone();
            let layout = layout.clone();
            let filter = filter.clone();
            let ranking = ranking.clone();
            let steam_shortcuts = steam_shortcuts.clone();
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
                        &filter,
                        &None,
                        &ranking,
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

        let restore_path = self.config.restore.path.clone();
        if !restore_path.is_dir() {
            self.modal = Some(Modal::Error {
                variant: Error::RestorationSourceInvalid { path: restore_path },
            });
            return Command::none();
        }

        let config = std::sync::Arc::new(self.config.clone());

        self.modal = None;

        self.operation = Some(if preview {
            OngoingOperation::PreviewRestore
        } else {
            OngoingOperation::Restore
        });
        self.progress.start();

        Command::perform(
            async move {
                let layout = BackupLayout::new(restore_path.clone(), config.backup.retention.clone());
                let restorables = layout.restorable_games();
                (layout, restorables)
            },
            move |(layout, restorables)| Message::RestorePerform {
                preview,
                full,
                games,
                layout,
                restorables,
            },
        )
    }

    fn perform_restore(
        &mut self,
        preview: bool,
        full: bool,
        games: Option<Vec<String>>,
        layout: BackupLayout,
        mut restorables: Vec<String>,
    ) -> Command<Message> {
        log::info!("beginning restore with {} steps", self.progress.max);

        if self.operation_should_cancel.load(std::sync::atomic::Ordering::Relaxed) {
            self.go_idle();
            return Command::none();
        }

        if let Some(games) = &games {
            restorables.retain(|v| games.contains(v));
            self.restore_screen.log.unscan_games(games);
        } else {
            self.restore_screen.log.clear();
            self.restore_screen.duplicate_detector.clear();
        }

        if restorables.is_empty() {
            if let Some(games) = &games {
                for game in games {
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
                self.cache.save();
            }
            self.go_idle();
            return Command::none();
        }

        self.progress.set_max(restorables.len() as f32);

        self.register_notify_on_single_game_scanned(&games);

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
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        return (None, None, OperationStepDecision::Cancelled, layout);
                    }

                    let scan_info = layout.scan_for_restoration(&name, &backup_id, &config.redirects);
                    if !config.is_game_enabled_for_restore(&name) && full {
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
            self.modal = Some(Modal::Error {
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
            self.modal = Some(Modal::Error {
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

        self.text_histories.add_custom_game(&game);
        self.config.custom_games.push(game);
        self.config.save();

        self.switch_screen(Screen::CustomGames)
    }

    fn open_url(url: String) -> Command<Message> {
        let url2 = url.clone();
        Command::perform(async { opener::open(url) }, move |res| match res {
            Ok(_) => Message::Ignore,
            Err(_) => Message::OpenUrlFailure { url: url2 },
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
        let subject = ScrollSubject::from(screen);

        if let Some(offset) = self.scroll_offsets.get(&subject) {
            iced::widget::scrollable::snap_to(subject.id(), *offset)
        } else {
            Command::none()
        }
    }

    fn configure_remote(&self, remote: Remote) -> Command<Message> {
        let rclone = self.config.apps.rclone.clone();
        let remote2 = remote.clone();
        Command::perform(
            async move { Rclone::new(rclone, remote2).configure_remote() },
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
    type Flags = ();
    type Theme = crate::gui::style::Theme;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut modal_theme: Option<Modal> = None;
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(Modal::Error { variant: x });
                let _ = Config::archive_invalid();
                Config::default()
            }
        };
        TRANSLATOR.set_language(config.language);
        let mut cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
        let manifest = match Manifest::load() {
            Ok(y) => y,
            Err(_) => {
                modal_theme = Some(Modal::UpdatingManifest);
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
            modal_theme = Some(Modal::ConfirmAddMissingRoots(missing));
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
                modal: modal_theme,
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
                if matches!(self.modal, Some(Modal::ConfirmCloudSync { .. })) {
                    if let Some(sender) = self.rclone_monitor_sender.as_mut() {
                        let _ = sender.try_send(rclone_monitor::Input::Cancel);
                    }
                }
                self.modal = None;
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

                if self.modal == Some(Modal::UpdatingManifest) {
                    self.modal = None;
                }

                self.cache.update_manifest(updated);
                self.cache.save();

                match Manifest::load() {
                    Ok(x) => {
                        self.manifest = x;
                    }
                    Err(variant) => {
                        self.modal = Some(Modal::Error { variant });
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

                self.modal = Some(Modal::PreparingBackupDir);

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
            Message::BackupPerform {
                preview,
                full,
                games,
                subjects,
                all_games,
                layout,
                ranking,
                steam,
                heroic,
            } => self.perform_backup(
                preview, full, games, subjects, all_games, layout, ranking, steam, heroic,
            ),
            Message::RestoreStart { preview, games } => self.start_restore(preview, games),
            Message::RestorePerform {
                preview,
                full,
                games,
                restorables,
                layout,
            } => self.perform_restore(preview, full, games, layout, restorables),
            Message::BackupStep {
                scan_info,
                backup_info,
                decision: _,
                preview,
                full,
            } => {
                self.progress.step();
                let restoring = false;

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
                self.progress.step();
                let restoring = true;

                if let Some(scan_info) = scan_info {
                    log::trace!(
                        "step {} / {}: {}",
                        self.progress.current,
                        self.progress.max,
                        scan_info.game_name
                    );
                    if scan_info.can_report_game() {
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
                            Some(game_layout),
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
                    Some(OngoingOperation::CloudSync { direction, .. }) => {
                        self.operation = Some(OngoingOperation::CancelCloudSync { direction });
                        if let Some(sender) = self.rclone_monitor_sender.as_mut() {
                            let _ = sender.try_send(rclone_monitor::Input::Cancel);
                        }
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
                    self.modal = Some(Modal::NoMissingRoots);
                } else {
                    self.cache.add_roots(&missing);
                    self.cache.save();
                    self.modal = Some(Modal::ConfirmAddMissingRoots(missing));
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
                self.config.save();

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
                self.backup_screen.log.refresh_game_tree(
                    &name,
                    &self.config,
                    &mut self.backup_screen.duplicate_detector,
                    false,
                );
                Command::none()
            }
            Message::ToggleSpecificBackupRegistryIgnored { name, path, value, .. } => {
                self.config.backup.toggled_registry.toggle_owned(&name, &path, value);
                self.config.save();
                self.backup_screen.log.refresh_game_tree(
                    &name,
                    &self.config,
                    &mut self.backup_screen.duplicate_detector,
                    false,
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
            Message::BrowseFile(subject) => Command::perform(
                async move { native_dialog::FileDialog::new().show_open_single_file() },
                move |choice| match choice {
                    Ok(Some(path)) => Message::SelectedFile(subject, StrictPath::from(path)),
                    Ok(None) => Message::Ignore,
                    Err(_) => Message::BrowseDirFailure,
                },
            ),
            Message::BrowseDirFailure => {
                self.modal = Some(Modal::Error {
                    variant: Error::UnableToBrowseFileSystem,
                });
                Command::none()
            }
            Message::SelectedFile(subject, path) => {
                match subject {
                    BrowseFileSubject::RcloneExecutable => {
                        self.text_histories.rclone_executable.push(&path.raw());
                        self.config.apps.rclone.path = path;
                    }
                }
                self.config.save();
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
                self.modal = Some(Modal::Error {
                    variant: Error::UnableToOpenDir(path),
                });
                Command::none()
            }
            Message::OpenUrlFailure { url } => {
                self.modal = Some(Modal::Error {
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
                    UndoSubject::CloudRemoteName => {
                        if let Some(Remote::Custom { name }) = &mut self.config.cloud.remote {
                            shortcut.apply_to_string_field(name, &mut self.text_histories.cloud_remote_name)
                        }
                    }
                    UndoSubject::CloudPath => {
                        shortcut.apply_to_string_field(&mut self.config.cloud.path, &mut self.text_histories.cloud_path)
                    }
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
            Message::SetShowDeselectedGames(value) => {
                self.config.scan.show_deselected_games = value;
                self.config.save();
                Command::none()
            }
            Message::SetShowUnchangedGames(value) => {
                self.config.scan.show_unchanged_games = value;
                self.config.save();
                Command::none()
            }
            Message::SetShowUnscannedGames(value) => {
                self.config.scan.show_unscanned_games = value;
                self.config.save();
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
                self.config.save();
                Command::none()
            }
            Message::EditedMaxThreads(threads) => {
                self.config.set_threads(threads);
                self.config.save();
                Command::none()
            }
            Message::EditedRcloneExecutable(text) => {
                self.text_histories.rclone_executable.push(&text);
                self.config.apps.rclone.path.reset(text);
                self.config.save();
                Command::none()
            }
            Message::EditedRcloneArguments(text) => {
                self.text_histories.rclone_arguments.push(&text);
                self.config.apps.rclone.arguments = text;
                self.config.save();
                Command::none()
            }
            Message::EditedCloudRemoteName(text) => {
                self.text_histories.cloud_remote_name.push(&text);
                if let Some(Remote::Custom { name }) = &mut self.config.cloud.remote {
                    *name = text;
                }
                self.config.save();
                Command::none()
            }
            Message::EditedCloudPath(text) => {
                self.text_histories.cloud_path.push(&text);
                self.config.cloud.path = text;
                self.config.save();
                Command::none()
            }
            Message::OpenUrl(url) => Self::open_url(url),
            Message::EditedCloudRemote(choice) => {
                if let Ok(remote) = Remote::try_from(choice) {
                    match &remote {
                        Remote::Custom { name } => {
                            self.text_histories.cloud_remote_name.push(name);
                            self.config.cloud.remote = Some(remote);
                            self.config.save();
                            Command::none()
                        }
                        Remote::Ftp {
                            host,
                            port,
                            username,
                            password,
                        } => {
                            self.modal = Some(Modal::ConfigureFtpRemote {
                                host: host.clone(),
                                port: port.to_string(),
                                username: username.clone(),
                                password: password.clone(),
                            });
                            Command::none()
                        }
                        Remote::Smb {
                            host,
                            port,
                            username,
                            password,
                        } => {
                            self.modal = Some(Modal::ConfigureSmbRemote {
                                host: host.clone(),
                                port: port.to_string(),
                                username: username.clone(),
                                password: password.clone(),
                            });
                            Command::none()
                        }
                        Remote::WebDav {
                            url,
                            username,
                            password,
                            provider,
                        } => {
                            self.modal = Some(Modal::ConfigureWebDavRemote {
                                url: url.clone(),
                                username: username.clone(),
                                password: password.clone(),
                                provider: *provider,
                            });
                            Command::none()
                        }
                        Remote::Box | Remote::Dropbox | Remote::GoogleDrive | Remote::OneDrive => {
                            self.configure_remote(remote)
                        }
                    }
                } else {
                    self.config.cloud.remote = None;
                    self.config.save();
                    Command::none()
                }
            }
            Message::ConfigureCloudSuccess(remote) => {
                self.config.cloud.remote = Some(remote);
                self.config.save();
                self.modal = None;
                Command::none()
            }
            Message::ConfigureCloudFailure(error) => {
                self.show_error(Error::UnableToConfigureCloud(error));
                self.config.cloud.remote = None;
                self.config.save();
                Command::none()
            }
            Message::ConfirmSynchronizeCloud { direction } => {
                self.modal = Some(Modal::ConfirmCloudSync {
                    local: self.config.backup.path.render(),
                    cloud: self.config.cloud.path.clone(),
                    direction,
                    changes: vec![],
                    done: false,
                    page: 0,
                });

                if let Some(remote) = self.config.cloud.remote.as_ref() {
                    let rclone = Rclone::new(self.config.apps.rclone.clone(), remote.clone());
                    match rclone.sync(
                        &self.config.backup.path,
                        &self.config.cloud.path,
                        direction,
                        Finality::Preview,
                        &[],
                    ) {
                        Ok(process) => {
                            if let Some(sender) = self.rclone_monitor_sender.as_mut() {
                                self.operation = Some(OngoingOperation::CloudSync {
                                    direction,
                                    finality: Finality::Preview,
                                });
                                self.progress.start();
                                let _ = sender.try_send(rclone_monitor::Input::Process(process));
                            }
                        }
                        Err(e) => self.show_error(Error::UnableToSynchronizeCloud(e)),
                    }
                }

                Command::none()
            }
            Message::SynchronizeCloud { direction } => {
                self.modal = None;

                if let Some(remote) = self.config.cloud.remote.as_ref() {
                    let rclone = Rclone::new(self.config.apps.rclone.clone(), remote.clone());
                    match rclone.sync(
                        &self.config.backup.path,
                        &self.config.cloud.path,
                        direction,
                        Finality::Final,
                        &[],
                    ) {
                        Ok(process) => {
                            if let Some(sender) = self.rclone_monitor_sender.as_mut() {
                                self.operation = Some(OngoingOperation::CloudSync {
                                    direction,
                                    finality: Finality::Final,
                                });
                                self.progress.start();
                                let _ = sender.try_send(rclone_monitor::Input::Process(process));
                            }
                        }
                        Err(e) => self.show_error(Error::UnableToSynchronizeCloud(e)),
                    }
                }
                Command::none()
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
                                    if let Some(modal) = self.modal.as_mut() {
                                        modal.add_cloud_change(change);
                                    }
                                }
                            }
                        }
                    }
                    rclone_monitor::Event::Succeeded => {
                        if let Some(modal) = self.modal.as_mut() {
                            self.operation = None;
                            self.progress.reset();
                            modal.finish_cloud_scan();
                        } else {
                            self.go_idle();
                        }
                    }
                    rclone_monitor::Event::Failed(e) => {
                        self.go_idle();
                        self.show_error(Error::UnableToSynchronizeCloud(e));
                    }
                    rclone_monitor::Event::Cancelled => {
                        self.go_idle();
                    }
                }
                Command::none()
            }
            Message::EditedModalField(field) => {
                if let Some(modal) = self.modal.as_mut() {
                    modal.edit(field);
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
            rclone_monitor::run().map(Message::RcloneMonitor),
        ])
    }

    fn view(&self) -> Element {
        if let Some(m) = &self.modal {
            return Column::new()
                .push(m.view(&self.config).style(style::Container::Primary))
                .push_if(|| self.progress.visible(), || self.progress.view(&self.operation))
                .into();
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
                Screen::Other => screen::other(
                    self.updating_manifest,
                    &self.config,
                    &self.cache,
                    &self.operation,
                    &self.text_histories,
                ),
            })
            .push_some(|| self.timed_notification.as_ref().map(|x| x.view()))
            .push_if(
                || self.updating_manifest,
                || Notification::new(TRANSLATOR.updating_manifest()).view(),
            )
            .push_if(|| self.progress.visible(), || self.progress.view(&self.operation));

        Container::new(content).style(style::Container::Primary).into()
    }
}
