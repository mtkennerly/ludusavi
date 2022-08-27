use crate::{
    config::{Config, CustomGame, RootsConfig, Theme},
    gui::{
        backup_screen::BackupScreenComponent,
        common::*,
        custom_games_editor::{CustomGamesEditorEntry, CustomGamesEditorEntryRow},
        custom_games_screen::CustomGamesScreenComponent,
        disappearing_progress::DisappearingProgress,
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
    manifest::{Manifest, Store},
    prelude::{
        app_dir, back_up_game, prepare_backup_target, scan_game_for_backup, scan_game_for_restoration, BackupId, Error,
        InstallDirRanking, OperationStepDecision, StrictPath,
    },
    registry_compat::RegistryItem,
    shortcuts::Shortcut,
};

use iced::{
    alignment::Horizontal as HorizontalAlignment,
    button, executor,
    keyboard::{KeyCode, Modifiers},
    Alignment, Application, Button, Column, Command, Container, Element, Length, Row, Subscription, Text,
};

pub fn get_key_pressed(event: iced::keyboard::Event) -> Option<(KeyCode, Modifiers)> {
    match event {
        iced::keyboard::Event::KeyPressed { key_code, modifiers } => Some((key_code, modifiers)),
        _ => None,
    }
}

fn make_nav_button(
    state: &mut button::State,
    text: String,
    screen: Screen,
    current_screen: Screen,
    theme: Theme,
) -> Button<Message> {
    Button::new(
        state,
        Text::new(text)
            .size(16)
            .horizontal_alignment(HorizontalAlignment::Center),
    )
    .on_press(Message::SwitchScreen(screen))
    .padding([5, 20, 5, 20])
    .style(if current_screen == screen {
        style::NavButton::Active(theme)
    } else {
        style::NavButton::Inactive(theme)
    })
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
    operation_steps: Vec<Command<Message>>,
    operation_steps_active: usize,
    progress: DisappearingProgress,
    backups_to_restore: std::collections::HashMap<String, BackupId>,
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
    }

    fn start_backup(&mut self, preview: bool, games: Option<Vec<String>>) -> Command<Message> {
        if self.operation.is_some() {
            return Command::none();
        }

        let full = games.is_none();

        let backup_path = &self.config.backup.path;

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
                self.backup_screen.log.remove_games(games);
                self.config.backup.recent_games.retain(|x| !games.contains(x));
                self.config.save();
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

        let config = std::sync::Arc::new(self.config.clone());
        let roots = std::sync::Arc::new(config.expanded_roots());
        let layout = std::sync::Arc::new(BackupLayout::new(backup_path.clone(), config.backup.retention.clone()));
        let filter = std::sync::Arc::new(self.config.backup.filter.clone());
        let ranking = std::sync::Arc::new(InstallDirRanking::scan(&roots, &all_games, &subjects));

        for key in subjects {
            let game = all_games.0[&key].clone();
            let config = config.clone();
            let roots = roots.clone();
            let layout = layout.clone();
            let filter = filter.clone();
            let ranking = ranking.clone();
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

                    let scan_info = scan_game_for_backup(
                        &game,
                        &key,
                        &roots,
                        &StrictPath::from_std_path_buf(&app_dir()),
                        &steam_id,
                        &filter,
                        &None,
                        &ranking,
                        &config.backup.toggled_paths,
                        &config.backup.toggled_registry,
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
                self.restore_screen.log.remove_games(games);
                self.config.restore.recent_games.retain(|x| !games.contains(x));
                self.config.save();
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
                        return (None, None, OperationStepDecision::Cancelled);
                    }

                    let scan_info = scan_game_for_restoration(&name, &backup_id, &mut layout);
                    if !config.is_game_enabled_for_restore(&name) {
                        return (Some(scan_info), None, OperationStepDecision::Ignored);
                    }

                    let backup_info = if scan_info.backup.is_some() && !preview {
                        Some(layout.restore(&scan_info, &config.get_redirects()))
                    } else {
                        None
                    };
                    (Some(scan_info), backup_info, OperationStepDecision::Processed)
                },
                move |(scan_info, backup_info, decision)| Message::RestoreStep {
                    scan_info,
                    backup_info,
                    decision,
                    full,
                },
            ));
        }

        self.operation_steps_active = 100.min(self.operation_steps.len());
        Command::batch(self.operation_steps.drain(..self.operation_steps_active))
    }

    fn complete_backup(&mut self, preview: bool, full: bool) {
        let mut failed = false;

        if full {
            self.config.backup.recent_games.clear();
        }

        for entry in &self.backup_screen.log.entries {
            self.config
                .backup
                .recent_games
                .insert(entry.scan_info.game_name.clone());
            if let Some(backup_info) = &entry.backup_info {
                if !backup_info.successful() {
                    failed = true;
                }
            }
        }

        if !preview && full {
            self.backup_screen.previewed_games.clear();
        }

        self.config.save();

        if failed {
            self.modal_theme = Some(ModalTheme::Error {
                variant: Error::SomeEntriesFailed,
            });
            return;
        }

        self.go_idle();
    }

    fn complete_restore(&mut self, full: bool) {
        let mut failed = false;

        if full {
            self.config.restore.recent_games.clear();
        }

        for entry in &self.restore_screen.log.entries {
            self.config
                .restore
                .recent_games
                .insert(entry.scan_info.game_name.clone());
            if let Some(backup_info) = &entry.backup_info {
                if !backup_info.successful() {
                    failed = true;
                }
            }
        }

        self.config.save();

        if failed {
            self.modal_theme = Some(ModalTheme::Error {
                variant: Error::SomeEntriesFailed,
            });
        }

        self.go_idle();
    }
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
                let _ = Config::archive_invalid();
                Config::default()
            }
        };
        translator.set_language(config.language);
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
                other_screen: OtherScreenComponent::new(&config),
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
                self.go_idle();
                Command::none()
            }
            Message::Ignore => Command::none(),
            Message::Error(error) => {
                self.modal_theme = Some(ModalTheme::Error { variant: error });
                Command::none()
            }
            Message::ConfirmBackupStart { games } => {
                self.modal_theme = Some(ModalTheme::ConfirmBackup { games });
                Command::none()
            }
            Message::ConfirmRestoreStart { games } => {
                self.modal_theme = Some(ModalTheme::ConfirmRestore { games });
                Command::none()
            }
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
                        Ok(_) => Message::BackupStart {
                            preview,
                            games: games.clone(),
                        },
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
                    if scan_info.found_anything() {
                        self.backup_screen.duplicate_detector.add_game(&scan_info);
                        self.backup_screen.previewed_games.insert(scan_info.game_name.clone());
                        self.backup_screen
                            .log
                            .update_game(scan_info, backup_info, &self.config.backup.sort);
                    } else {
                        self.config.backup.recent_games.remove(&scan_info.game_name);
                        self.backup_screen.log.remove_game(&scan_info.game_name);
                    }
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
            } => {
                self.progress.current += 1.0;
                if let Some(scan_info) = scan_info {
                    if scan_info.found_anything() {
                        self.restore_screen.duplicate_detector.add_game(&scan_info);
                        self.restore_screen
                            .log
                            .update_game(scan_info, backup_info, &self.config.backup.sort);
                    } else {
                        self.restore_screen.log.remove_game(&scan_info.game_name);
                        self.config.restore.recent_games.remove(&scan_info.game_name);
                    }
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
            Message::ProcessGameOnDemand { game, restore } => {
                if restore {
                    Command::perform(async move {}, move |_| Message::ConfirmRestoreStart {
                        games: Some(vec![game.clone()]),
                    })
                } else {
                    Command::perform(async move {}, move |_| Message::ConfirmBackupStart {
                        games: Some(vec![game.clone()]),
                    })
                }
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
            Message::FindRoots => {
                let missing = self.config.find_missing_roots();
                if missing.is_empty() {
                    self.modal_theme = Some(ModalTheme::NoMissingRoots);
                } else {
                    self.modal_theme = Some(ModalTheme::ConfirmAddMissingRoots(missing));
                }
                Command::none()
            }
            Message::ConfirmAddMissingRoots(missing) => {
                for root in missing {
                    let mut row = RootEditorRow::default();
                    row.text_history.push(&root.path.render());
                    self.backup_screen.root_editor.rows.push(row);
                    self.config.roots.push(root);
                }
                self.config.save();
                Command::perform(async move {}, move |_| Message::Idle)
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
                Command::none()
            }
            Message::SelectedRootStore(index, store) => {
                self.config.roots[index].store = store;
                self.config.save();
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
                        self.other_screen
                            .ignored_items_editor
                            .entry
                            .files
                            .push(crate::gui::ignored_items_editor::IgnoredItemsEditorEntryRow::default());
                        self.config
                            .backup
                            .filter
                            .ignored_paths
                            .push(StrictPath::new("".to_string()));
                    }
                    EditAction::Change(index, value) => {
                        self.other_screen.ignored_items_editor.entry.files[index]
                            .text_history
                            .push(&value);
                        self.config.backup.filter.ignored_paths[index] = StrictPath::new(value);
                    }
                    EditAction::Remove(index) => {
                        self.other_screen.ignored_items_editor.entry.files.remove(index);
                        self.config.backup.filter.ignored_paths.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedBackupFilterIgnoredRegistry(action) => {
                match action {
                    EditAction::Add => {
                        self.other_screen
                            .ignored_items_editor
                            .entry
                            .registry
                            .push(crate::gui::ignored_items_editor::IgnoredItemsEditorEntryRow::default());
                        self.config
                            .backup
                            .filter
                            .ignored_registry
                            .push(RegistryItem::new("".to_string()));
                    }
                    EditAction::Change(index, value) => {
                        self.other_screen.ignored_items_editor.entry.registry[index]
                            .text_history
                            .push(&value);
                        self.config.backup.filter.ignored_registry[index] = RegistryItem::new(value);
                    }
                    EditAction::Remove(index) => {
                        self.other_screen.ignored_items_editor.entry.registry.remove(index);
                        self.config.backup.filter.ignored_registry.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::SwitchScreen(screen) => {
                self.screen = screen;
                Command::none()
            }
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        self.backup_screen.log.toggle_game_expanded(&name);
                    }
                    Screen::Restore => {
                        self.restore_screen.log.toggle_game_expanded(&name);
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
            Message::ToggleSpecificBackupRegistryIgnored { name, path, .. } => {
                self.config.backup.toggled_registry.toggle(&name, &path);
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
            Message::CustomizeGame { name } => {
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

                let mut gui_entry = CustomGamesEditorEntry::new(&name);
                for item in game.files.iter() {
                    gui_entry.files.push(CustomGamesEditorEntryRow::new(item));
                }
                for item in game.registry.iter() {
                    gui_entry.registry.push(CustomGamesEditorEntryRow::new(item));
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
                let url = format!("https://www.pcgamingwiki.com/wiki/{}", game.replace(' ', "_"));
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
                            modifiers.logo() || modifiers.control()
                        } else {
                            modifiers.control()
                        };
                        let shortcut = match (key_code, activated, modifiers.shift()) {
                            (KeyCode::Z, true, false) => Some(Shortcut::Undo),
                            (KeyCode::Y, true, false) | (KeyCode::Z, true, true) => Some(Shortcut::Redo),
                            _ => None,
                        };

                        if let Some(shortcut) = shortcut {
                            let mut matched = false;

                            if self.backup_screen.backup_target_input.is_focused() {
                                apply_shortcut_to_strict_path_field(
                                    &shortcut,
                                    &mut self.config.backup.path,
                                    &mut self.backup_screen.backup_target_history,
                                );
                                matched = true;
                            } else if self.restore_screen.restore_source_input.is_focused() {
                                apply_shortcut_to_strict_path_field(
                                    &shortcut,
                                    &mut self.config.restore.path,
                                    &mut self.restore_screen.restore_source_history,
                                );
                                matched = true;
                            } else if self.backup_screen.log.search.game_name_input.is_focused() {
                                apply_shortcut_to_string_field(
                                    &shortcut,
                                    &mut self.backup_screen.log.search.game_name,
                                    &mut self.backup_screen.log.search.game_name_history,
                                );
                                matched = true;
                            } else if self.restore_screen.log.search.game_name_input.is_focused() {
                                apply_shortcut_to_string_field(
                                    &shortcut,
                                    &mut self.restore_screen.log.search.game_name,
                                    &mut self.restore_screen.log.search.game_name_history,
                                );
                                matched = true;
                            } else {
                                for (i, root) in self.backup_screen.root_editor.rows.iter_mut().enumerate() {
                                    if root.text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.roots[i].path,
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
                                            &mut redirect.source_text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                    if redirect.target_text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.restore.redirects[i].target,
                                            &mut redirect.target_text_history,
                                        );
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
                                                &mut registry_row.text_history,
                                            );
                                            matched = true;
                                            break;
                                        }
                                    }
                                }
                                for (i, row) in self
                                    .other_screen
                                    .ignored_items_editor
                                    .entry
                                    .files
                                    .iter_mut()
                                    .enumerate()
                                {
                                    if matched {
                                        break;
                                    }
                                    if row.text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.backup.filter.ignored_paths[i],
                                            &mut row.text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                }
                                for (i, row) in self
                                    .other_screen
                                    .ignored_items_editor
                                    .entry
                                    .registry
                                    .iter_mut()
                                    .enumerate()
                                {
                                    if matched {
                                        break;
                                    }
                                    if row.text_state.is_focused() {
                                        apply_shortcut_to_registry_path_field(
                                            &shortcut,
                                            &mut self.config.backup.filter.ignored_registry[i],
                                            &mut row.text_history,
                                        );
                                        matched = true;
                                        break;
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
                self.translator.set_language(language);
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
            Message::ToggleBackupSettings => {
                self.backup_screen.show_settings = !self.backup_screen.show_settings;
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events_with(|event, _| Some(event)).map(Message::SubscribedEvent)
    }

    fn view(&mut self) -> Element<Message> {
        if let Some(m) = &self.modal_theme {
            return self
                .modal
                .view(m, &self.config, &self.translator)
                .style(style::Container::Primary(self.config.theme))
                .into();
        }

        let content = Column::new()
            .align_items(Alignment::Center)
            .push(
                Row::new()
                    .padding([2, 20, 25, 20])
                    .spacing(20)
                    .push(make_nav_button(
                        &mut self.nav_to_backup_button,
                        self.translator.nav_backup_button(),
                        Screen::Backup,
                        self.screen,
                        self.config.theme,
                    ))
                    .push(make_nav_button(
                        &mut self.nav_to_restore_button,
                        self.translator.nav_restore_button(),
                        Screen::Restore,
                        self.screen,
                        self.config.theme,
                    ))
                    .push(make_nav_button(
                        &mut self.nav_to_custom_games_button,
                        self.translator.nav_custom_games_button(),
                        Screen::CustomGames,
                        self.screen,
                        self.config.theme,
                    ))
                    .push(make_nav_button(
                        &mut self.nav_to_other_button,
                        self.translator.nav_other_button(),
                        Screen::Other,
                        self.screen,
                        self.config.theme,
                    )),
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
                    Screen::Other => self.other_screen.view(&self.config, &self.translator, &self.operation),
                }
                .padding([0, 5, 5, 5])
                .height(Length::FillPortion(10_000)),
            )
            .push(self.progress.view());

        Container::new(content)
            .style(style::Container::Primary(self.config.theme))
            .into()
    }
}
