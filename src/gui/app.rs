use crate::{
    config::{Config, CustomGame, RootsConfig},
    gui::{
        backup_screen::BackupScreenComponent,
        common::*,
        custom_games_editor::{CustomGamesEditorEntry, CustomGamesEditorEntryRow},
        custom_games_screen::CustomGamesScreenComponent,
        disappearing_progress::DisappearingProgress,
        game_list::GameListEntry,
        modal::ModalComponent,
        modal::ModalTheme,
        other_screen::OtherScreenComponent,
        redirect_editor::RedirectEditorRow,
        restore_screen::RestoreScreenComponent,
        root_editor::RootEditorRow,
        style,
    },
    lang::Translator,
    layout::BackupLayout,
    manifest::{Manifest, SteamMetadata, Store},
    prelude::{
        app_dir, back_up_game, prepare_backup_target, restore_game, scan_game_for_backup, scan_game_for_restoration,
        Error, OperationStepDecision, StrictPath,
    },
    shortcuts::Shortcut,
};

use iced::{
    button, executor,
    keyboard::{KeyCode, Modifiers},
    Align, Application, Button, Column, Command, Element, HorizontalAlignment, Length, Row, Subscription, Text,
};
use native_dialog::Dialog;

pub fn get_key_pressed(event: iced::keyboard::Event) -> Option<(KeyCode, Modifiers)> {
    match event {
        iced::keyboard::Event::KeyPressed { key_code, modifiers } => Some((key_code, modifiers)),
        _ => None,
    }
}

#[derive(Default)]
pub struct App {
    config: Config,
    manifest: Manifest,
    translator: Translator,
    operation: Option<OngoingOperation>,
    screen: Screen,
    modal_theme: Option<ModalTheme>,
    modal: ModalComponent,
    nav_to_backup_button: button::State,
    nav_to_restore_button: button::State,
    nav_to_custom_games_button: button::State,
    nav_to_other_button: button::State,
    backup_screen: BackupScreenComponent,
    restore_screen: RestoreScreenComponent,
    custom_games_screen: CustomGamesScreenComponent,
    other_screen: OtherScreenComponent,
    operation_should_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    progress: DisappearingProgress,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let translator = Translator::default();
        let mut modal_theme: Option<ModalTheme> = None;
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(ModalTheme::Error { variant: x });
                Config::default()
            }
        };
        let manifest = match Manifest::load(&mut config, true) {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(ModalTheme::Error { variant: x });
                match Manifest::load(&mut config, false) {
                    Ok(y) => y,
                    Err(_) => Manifest::default(),
                }
            }
        };

        (
            Self {
                backup_screen: BackupScreenComponent::new(&config),
                restore_screen: RestoreScreenComponent::new(&config),
                custom_games_screen: CustomGamesScreenComponent::new(&config),
                translator,
                config,
                manifest,
                modal_theme,
                ..Self::default()
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        self.translator.window_title()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Idle => {
                self.operation = None;
                self.modal_theme = None;
                self.progress.current = 0.0;
                self.progress.max = 0.0;
                self.operation_should_cancel
                    .swap(false, std::sync::atomic::Ordering::Relaxed);
                Command::none()
            }
            Message::Ignore => Command::none(),
            Message::ConfirmBackupStart => {
                self.modal_theme = Some(ModalTheme::ConfirmBackup);
                Command::none()
            }
            Message::ConfirmRestoreStart => {
                self.modal_theme = Some(ModalTheme::ConfirmRestore);
                Command::none()
            }
            Message::BackupStart { preview } => {
                if self.operation.is_some() {
                    return Command::none();
                }

                let backup_path = &self.config.backup.path;
                if !preview {
                    if let Err(e) = prepare_backup_target(&backup_path, self.config.backup.merge) {
                        self.modal_theme = Some(ModalTheme::Error { variant: e });
                        return Command::none();
                    }
                }

                let mut all_games = self.manifest.clone();
                for custom_game in &self.config.custom_games {
                    all_games.add_custom_game(custom_game.clone());
                }

                if self.backup_screen.only_scan_recent_found_games {
                    all_games.0.retain(|k, _| {
                        self.backup_screen.recent_found_games.contains(k)
                            || self.config.custom_games.iter().any(|x| &x.name == k)
                    });
                }

                self.backup_screen.status.clear();
                self.backup_screen.log.entries.clear();
                self.modal_theme = None;
                self.progress.current = 0.0;
                self.progress.max = all_games.0.len() as f32;
                self.backup_screen.duplicate_detector.clear();

                self.operation = Some(if preview {
                    OngoingOperation::PreviewBackup
                } else {
                    OngoingOperation::Backup
                });

                let layout = std::sync::Arc::new(BackupLayout::new(backup_path.clone()));
                let filter = std::sync::Arc::new(self.config.backup.filter.clone());

                let mut commands: Vec<Command<Message>> = vec![];
                for key in all_games.0.iter().map(|(k, _)| k.clone()) {
                    let game = all_games.0[&key].clone();
                    let roots = self.config.roots.clone();
                    let layout2 = layout.clone();
                    let filter2 = filter.clone();
                    let steam_id = game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;
                    let cancel_flag = self.operation_should_cancel.clone();
                    let ignored = !self.config.is_game_enabled_for_backup(&key);
                    let merge = self.config.backup.merge;
                    commands.push(Command::perform(
                        async move {
                            if key.trim().is_empty() {
                                return (None, None, OperationStepDecision::Ignored);
                            }
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(std::time::Duration::from_millis(1));
                                return (None, None, OperationStepDecision::Cancelled);
                            }

                            let scan_info = scan_game_for_backup(
                                &game,
                                &key,
                                &roots,
                                &StrictPath::from_std_path_buf(&app_dir()),
                                &steam_id,
                                &filter2,
                                &None,
                            );
                            if ignored {
                                return (Some(scan_info), None, OperationStepDecision::Ignored);
                            }

                            let backup_info = if !preview {
                                Some(back_up_game(&scan_info, &key, &layout2, merge))
                            } else {
                                None
                            };
                            (Some(scan_info), backup_info, OperationStepDecision::Processed)
                        },
                        move |(scan_info, backup_info, decision)| Message::BackupStep {
                            scan_info,
                            backup_info,
                            decision,
                        },
                    ));
                }

                Command::batch(commands)
            }
            Message::RestoreStart { preview } => {
                if self.operation.is_some() {
                    return Command::none();
                }

                let restore_path = &self.config.restore.path;
                if !restore_path.is_dir() {
                    self.modal_theme = Some(ModalTheme::Error {
                        variant: Error::RestorationSourceInvalid {
                            path: restore_path.clone(),
                        },
                    });
                    return Command::none();
                }

                let layout = std::sync::Arc::new(BackupLayout::new(restore_path.clone()));
                let restorables: Vec<_> = layout.mapping.games.keys().cloned().collect();

                self.restore_screen.status.clear();
                self.restore_screen.log.entries.clear();
                self.modal_theme = None;
                self.restore_screen.duplicate_detector.clear();

                if restorables.is_empty() {
                    return Command::none();
                }

                self.operation = Some(if preview {
                    OngoingOperation::PreviewRestore
                } else {
                    OngoingOperation::Restore
                });
                self.progress.current = 0.0;
                self.progress.max = restorables.len() as f32;

                let mut commands: Vec<Command<Message>> = vec![];
                for name in restorables {
                    let redirects = self.config.get_redirects();
                    let layout2 = layout.clone();
                    let cancel_flag = self.operation_should_cancel.clone();
                    let ignored = !self.config.is_game_enabled_for_restore(&name);
                    commands.push(Command::perform(
                        async move {
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(std::time::Duration::from_millis(1));
                                return (None, None, OperationStepDecision::Cancelled);
                            }

                            let scan_info = scan_game_for_restoration(&name, &layout2);
                            if ignored {
                                return (Some(scan_info), None, OperationStepDecision::Ignored);
                            }

                            let backup_info = if !preview {
                                Some(restore_game(&scan_info, &redirects))
                            } else {
                                None
                            };
                            (Some(scan_info), backup_info, OperationStepDecision::Processed)
                        },
                        move |(scan_info, backup_info, decision)| Message::RestoreStep {
                            scan_info,
                            backup_info,
                            decision,
                        },
                    ));
                }

                Command::batch(commands)
            }
            Message::BackupStep {
                scan_info,
                backup_info,
                decision,
            } => {
                self.progress.current += 1.0;
                if let Some(scan_info) = scan_info {
                    if scan_info.found_anything() {
                        self.backup_screen.duplicate_detector.add_game(&scan_info);
                        self.backup_screen
                            .recent_found_games
                            .insert(scan_info.game_name.clone());
                        self.backup_screen.status.add_game(
                            &scan_info,
                            &backup_info,
                            decision == OperationStepDecision::Processed,
                        );
                        self.backup_screen.log.entries.push(GameListEntry {
                            scan_info,
                            backup_info,
                            ..Default::default()
                        });
                    }
                }
                if self.progress.complete() {
                    Command::perform(async move {}, move |_| Message::BackupComplete)
                } else {
                    Command::none()
                }
            }
            Message::RestoreStep {
                scan_info,
                backup_info,
                decision,
            } => {
                self.progress.current += 1.0;
                if let Some(scan_info) = scan_info {
                    if scan_info.found_anything() {
                        self.restore_screen.duplicate_detector.add_game(&scan_info);
                        self.restore_screen.status.add_game(
                            &scan_info,
                            &backup_info,
                            decision == OperationStepDecision::Processed,
                        );
                        self.restore_screen.log.entries.push(GameListEntry {
                            scan_info,
                            backup_info,
                            ..Default::default()
                        });
                    }
                }
                if self.progress.complete() {
                    Command::perform(async move {}, move |_| Message::RestoreComplete)
                } else {
                    Command::none()
                }
            }
            Message::CancelOperation => {
                self.operation_should_cancel
                    .swap(true, std::sync::atomic::Ordering::Relaxed);
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
            Message::BackupComplete => {
                for entry in &self.backup_screen.log.entries {
                    if let Some(backup_info) = &entry.backup_info {
                        if !backup_info.successful() {
                            self.modal_theme = Some(ModalTheme::Error {
                                variant: Error::SomeEntriesFailed,
                            });
                            return Command::none();
                        }
                    }
                }
                self.backup_screen.only_scan_recent_found_games = !self.backup_screen.recent_found_games.is_empty();
                Command::perform(async move {}, move |_| Message::Idle)
            }
            Message::RestoreComplete => {
                for entry in &self.restore_screen.log.entries {
                    if let Some(backup_info) = &entry.backup_info {
                        if !backup_info.successful() {
                            self.modal_theme = Some(ModalTheme::Error {
                                variant: Error::SomeEntriesFailed,
                            });
                            return Command::none();
                        }
                    }
                }
                Command::perform(async move {}, move |_| Message::Idle)
            }
            Message::EditedBackupTarget(text) => {
                self.backup_screen.backup_target_history.push(&text);
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
                self.restore_screen.restore_source_history.push(&text);
                self.config.restore.path.reset(text);
                self.config.save();
                Command::none()
            }
            Message::EditedRoot(action) => {
                match action {
                    EditAction::Add => {
                        self.backup_screen.root_editor.rows.push(RootEditorRow::default());
                        self.config.roots.push(RootsConfig {
                            path: StrictPath::default(),
                            store: Store::Other,
                        });
                    }
                    EditAction::Change(index, value) => {
                        self.backup_screen.root_editor.rows[index].text_history.push(&value);
                        self.config.roots[index].path.reset(value);
                    }
                    EditAction::Remove(index) => {
                        self.backup_screen.root_editor.rows.remove(index);
                        self.config.roots.remove(index);
                    }
                }
                self.config.save();
                self.backup_screen.only_scan_recent_found_games = false;
                Command::none()
            }
            Message::SelectedRootStore(index, store) => {
                self.config.roots[index].store = store;
                self.config.save();
                self.backup_screen.only_scan_recent_found_games = false;
                Command::none()
            }
            Message::EditedRedirect(action, field) => {
                match action {
                    EditAction::Add => {
                        self.restore_screen
                            .redirect_editor
                            .rows
                            .push(RedirectEditorRow::default());
                        self.config.add_redirect(&StrictPath::default(), &StrictPath::default());
                    }
                    EditAction::Change(index, value) => match field {
                        Some(RedirectEditActionField::Source) => {
                            self.restore_screen.redirect_editor.rows[index]
                                .source_text_history
                                .push(&value);
                            self.config.restore.redirects[index].source.reset(value);
                        }
                        Some(RedirectEditActionField::Target) => {
                            self.restore_screen.redirect_editor.rows[index]
                                .target_text_history
                                .push(&value);
                            self.config.restore.redirects[index].target.reset(value);
                        }
                        _ => {}
                    },
                    EditAction::Remove(index) => {
                        self.restore_screen.redirect_editor.rows.remove(index);
                        self.config.restore.redirects.remove(index);
                    }
                }
                self.config.save();
                for item in self.restore_screen.log.entries.iter_mut() {
                    item.tree_should_reload = true;
                }
                Command::none()
            }
            Message::EditedCustomGame(action) => {
                match action {
                    EditAction::Add => {
                        self.custom_games_screen
                            .games_editor
                            .entries
                            .push(CustomGamesEditorEntry::default());
                        self.config.add_custom_game();
                    }
                    EditAction::Change(index, value) => {
                        self.custom_games_screen.games_editor.entries[index]
                            .text_history
                            .push(&value);
                        self.config.custom_games[index].name = value;
                    }
                    EditAction::Remove(index) => {
                        self.custom_games_screen.games_editor.entries.remove(index);
                        self.config.custom_games.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedCustomGameFile(game_index, action) => {
                match action {
                    EditAction::Add => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .files
                            .push(CustomGamesEditorEntryRow::default());
                        self.config.custom_games[game_index].files.push("".to_string());
                    }
                    EditAction::Change(index, value) => {
                        self.custom_games_screen.games_editor.entries[game_index].files[index]
                            .text_history
                            .push(&value);
                        self.config.custom_games[game_index].files[index] = value;
                    }
                    EditAction::Remove(index) => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .files
                            .remove(index);
                        self.config.custom_games[game_index].files.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedCustomGameRegistry(game_index, action) => {
                match action {
                    EditAction::Add => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .registry
                            .push(CustomGamesEditorEntryRow::default());
                        self.config.custom_games[game_index].registry.push("".to_string());
                    }
                    EditAction::Change(index, value) => {
                        self.custom_games_screen.games_editor.entries[game_index].registry[index]
                            .text_history
                            .push(&value);
                        self.config.custom_games[game_index].registry[index] = value;
                    }
                    EditAction::Remove(index) => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .registry
                            .remove(index);
                        self.config.custom_games[game_index].registry.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedExcludeOtherOsData(enabled) => {
                self.config.backup.filter.exclude_other_os_data = enabled;
                self.config.save();
                self.backup_screen.only_scan_recent_found_games = false;
                Command::none()
            }
            Message::EditedExcludeStoreScreenshots(enabled) => {
                self.config.backup.filter.exclude_store_screenshots = enabled;
                self.config.save();
                self.backup_screen.only_scan_recent_found_games = false;
                Command::none()
            }
            Message::SwitchScreen(screen) => {
                self.screen = screen;
                Command::none()
            }
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &mut self.backup_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                entry.expanded = !entry.expanded;
                            }
                        }
                    }
                    Screen::Restore => {
                        for entry in &mut self.restore_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                entry.expanded = !entry.expanded;
                            }
                        }
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
            Message::EditedSearchGameName { screen, value } => {
                match screen {
                    Screen::Backup => {
                        self.backup_screen.log.search.game_name_history.push(&value);
                        self.backup_screen.log.search.game_name = value;
                    }
                    Screen::Restore => {
                        self.restore_screen.log.search.game_name_history.push(&value);
                        self.restore_screen.log.search.game_name = value;
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::BrowseDir(subject) => Command::perform(
                async move { native_dialog::OpenSingleDir { dir: None }.show() },
                move |choice| match choice {
                    Ok(Some(path)) => match subject {
                        BrowseSubject::BackupTarget => Message::EditedBackupTarget(crate::path::render_pathbuf(&path)),
                        BrowseSubject::RestoreSource => {
                            Message::EditedRestoreSource(crate::path::render_pathbuf(&path))
                        }
                        BrowseSubject::Root(i) => {
                            Message::EditedRoot(EditAction::Change(i, crate::path::render_pathbuf(&path)))
                        }
                        BrowseSubject::RedirectSource(i) => Message::EditedRedirect(
                            EditAction::Change(i, crate::path::render_pathbuf(&path)),
                            Some(RedirectEditActionField::Source),
                        ),
                        BrowseSubject::RedirectTarget(i) => Message::EditedRedirect(
                            EditAction::Change(i, crate::path::render_pathbuf(&path)),
                            Some(RedirectEditActionField::Target),
                        ),
                        BrowseSubject::CustomGameFile(i, j) => {
                            Message::EditedCustomGameFile(i, EditAction::Change(j, crate::path::render_pathbuf(&path)))
                        }
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
                    _ => {}
                }
                self.config.save();
                Command::none()
            }
            Message::CustomizeGame { name } => {
                let game = if let Some(standard) = self.manifest.0.get(&name) {
                    CustomGame {
                        name: name.clone(),
                        files: standard.files.clone().unwrap_or_default().keys().cloned().collect(),
                        registry: standard.registry.clone().unwrap_or_default().keys().cloned().collect(),
                    }
                } else {
                    CustomGame {
                        name: name.clone(),
                        files: vec![],
                        registry: vec![],
                    }
                };

                let mut gui_entry = CustomGamesEditorEntry::new(&name);
                for item in game.files.iter() {
                    gui_entry.files.push(CustomGamesEditorEntryRow::new(&item));
                }
                for item in game.registry.iter() {
                    gui_entry.registry.push(CustomGamesEditorEntryRow::new(&item));
                }
                self.custom_games_screen.games_editor.entries.push(gui_entry);

                self.config.custom_games.push(game);
                self.config.save();

                self.screen = Screen::CustomGames;
                Command::none()
            }
            Message::OpenDir { path } => {
                let path2 = path.clone();
                match std::thread::spawn(move || opener::open(&path.interpret())).join() {
                    Ok(Ok(_)) => Command::none(),
                    _ => Command::perform(async {}, move |_| Message::OpenDirFailure { path: path2.clone() }),
                }
            }
            Message::OpenDirFailure { path } => {
                self.modal_theme = Some(ModalTheme::Error {
                    variant: Error::UnableToOpenDir(path),
                });
                Command::none()
            }
            Message::OpenWiki { game } => {
                let url = format!("https://www.pcgamingwiki.com/wiki/{}", game.replace(" ", "_"));
                let url2 = url.clone();
                match std::thread::spawn(move || opener::open(&url)).join() {
                    Ok(Ok(_)) => Command::none(),
                    _ => Command::perform(async {}, move |_| Message::OpenUrlFailure { url: url2.clone() }),
                }
            }
            Message::OpenUrlFailure { url } => {
                self.modal_theme = Some(ModalTheme::Error {
                    variant: Error::UnableToOpenUrl(url),
                });
                Command::none()
            }
            Message::SubscribedEvent(event) => {
                if let iced_native::Event::Keyboard(key) = event {
                    if let Some((key_code, modifiers)) = get_key_pressed(key) {
                        let activated = if cfg!(target_os = "mac") {
                            modifiers.logo || modifiers.control
                        } else {
                            modifiers.control
                        };
                        let shortcut = match (key_code, activated, modifiers.shift) {
                            (KeyCode::Z, true, false) => Some(Shortcut::Undo),
                            (KeyCode::Y, true, false) | (KeyCode::Z, true, true) => Some(Shortcut::Redo),
                            (KeyCode::C, true, false) => Some(Shortcut::ClipboardCopy),
                            (KeyCode::X, true, false) => Some(Shortcut::ClipboardCut),
                            _ => None,
                        };

                        if let Some(shortcut) = shortcut {
                            let mut matched = false;

                            if self.backup_screen.backup_target_input.is_focused() {
                                apply_shortcut_to_strict_path_field(
                                    &shortcut,
                                    &mut self.config.backup.path,
                                    &self.backup_screen.backup_target_input,
                                    &mut self.backup_screen.backup_target_history,
                                );
                                matched = true;
                            } else if self.restore_screen.restore_source_input.is_focused() {
                                apply_shortcut_to_strict_path_field(
                                    &shortcut,
                                    &mut self.config.restore.path,
                                    &self.restore_screen.restore_source_input,
                                    &mut self.restore_screen.restore_source_history,
                                );
                                matched = true;
                            } else if self.backup_screen.log.search.game_name_input.is_focused() {
                                apply_shortcut_to_string_field(
                                    &shortcut,
                                    &mut self.backup_screen.log.search.game_name,
                                    &self.backup_screen.log.search.game_name_input,
                                    &mut self.backup_screen.log.search.game_name_history,
                                );
                                matched = true;
                            } else if self.restore_screen.log.search.game_name_input.is_focused() {
                                apply_shortcut_to_string_field(
                                    &shortcut,
                                    &mut self.restore_screen.log.search.game_name,
                                    &self.restore_screen.log.search.game_name_input,
                                    &mut self.restore_screen.log.search.game_name_history,
                                );
                                matched = true;
                            } else {
                                for (i, root) in self.backup_screen.root_editor.rows.iter_mut().enumerate() {
                                    if root.text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.roots[i].path,
                                            &root.text_state,
                                            &mut root.text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                }
                                for (i, redirect) in self.restore_screen.redirect_editor.rows.iter_mut().enumerate() {
                                    if redirect.source_text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.restore.redirects[i].source,
                                            &redirect.source_text_state,
                                            &mut redirect.source_text_history,
                                        );
                                        for item in self.restore_screen.log.entries.iter_mut() {
                                            item.tree_should_reload = true;
                                        }
                                        matched = true;
                                        break;
                                    }
                                    if redirect.target_text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.restore.redirects[i].target,
                                            &redirect.target_text_state,
                                            &mut redirect.target_text_history,
                                        );
                                        for item in self.restore_screen.log.entries.iter_mut() {
                                            item.tree_should_reload = true;
                                        }
                                        matched = true;
                                        break;
                                    }
                                }
                                for (i, game) in self.custom_games_screen.games_editor.entries.iter_mut().enumerate() {
                                    if matched {
                                        break;
                                    }
                                    if game.text_state.is_focused() {
                                        apply_shortcut_to_string_field(
                                            &shortcut,
                                            &mut self.config.custom_games[i].name,
                                            &game.text_state,
                                            &mut game.text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                    for (j, file_row) in game.files.iter_mut().enumerate() {
                                        if file_row.text_state.is_focused() {
                                            apply_shortcut_to_string_field(
                                                &shortcut,
                                                &mut self.config.custom_games[i].files[j],
                                                &file_row.text_state,
                                                &mut file_row.text_history,
                                            );
                                            matched = true;
                                            break;
                                        }
                                    }
                                    for (j, registry_row) in game.registry.iter_mut().enumerate() {
                                        if registry_row.text_state.is_focused() {
                                            apply_shortcut_to_string_field(
                                                &shortcut,
                                                &mut self.config.custom_games[i].registry[j],
                                                &registry_row.text_state,
                                                &mut registry_row.text_history,
                                            );
                                            matched = true;
                                            break;
                                        }
                                    }
                                }
                            }

                            if matched {
                                self.config.save();
                            }
                        }
                    }
                };
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events_with(|event, _| Some(event)).map(Message::SubscribedEvent)
    }

    fn view(&mut self) -> Element<Message> {
        if let Some(m) = &self.modal_theme {
            return self.modal.view(m, &self.config, &self.translator).into();
        }

        Column::new()
            .align_items(Align::Center)
            .push(
                Row::new()
                    .spacing(20)
                    .push(
                        Button::new(
                            &mut self.nav_to_backup_button,
                            Text::new(self.translator.nav_backup_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::Backup))
                        .width(Length::Units(175))
                        .style(match self.screen {
                            Screen::Backup => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    )
                    .push(
                        Button::new(
                            &mut self.nav_to_restore_button,
                            Text::new(self.translator.nav_restore_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::Restore))
                        .width(Length::Units(175))
                        .style(match self.screen {
                            Screen::Restore => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    )
                    .push(
                        Button::new(
                            &mut self.nav_to_custom_games_button,
                            Text::new(self.translator.nav_custom_games_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::CustomGames))
                        .width(Length::Units(175))
                        .style(match self.screen {
                            Screen::CustomGames => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    )
                    .push(
                        Button::new(
                            &mut self.nav_to_other_button,
                            Text::new(self.translator.nav_other_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::Other))
                        .width(Length::Units(175))
                        .style(match self.screen {
                            Screen::Other => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    ),
            )
            .push(
                match self.screen {
                    Screen::Backup => {
                        self.backup_screen
                            .view(&self.config, &self.manifest, &self.translator, &self.operation)
                    }
                    Screen::Restore => {
                        self.restore_screen
                            .view(&self.config, &self.manifest, &self.translator, &self.operation)
                    }
                    Screen::CustomGames => {
                        self.custom_games_screen
                            .view(&self.config, &self.translator, &self.operation)
                    }
                    Screen::Other => self.other_screen.view(&self.config, &self.translator),
                }
                .height(Length::FillPortion(10_000)),
            )
            .push(self.progress.view())
            .into()
    }
}
