use crate::config::{Config, RootsConfig};
use crate::lang::Translator;
use crate::manifest::{Manifest, SteamMetadata, Store};
use crate::prelude::{
    app_dir, back_up_game, game_file_restoration_target, prepare_backup_target, restore_game, scan_dir_for_restoration,
    scan_game_for_backup, Error, ScanInfo,
};
use crate::shortcuts::{Shortcut, TextHistory};

use iced::{
    button, executor, scrollable, text_input, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, ProgressBar, Radio, Row, Scrollable, Space, Subscription, Text, TextInput,
};

#[derive(Default)]
struct App {
    config: Config,
    manifest: Manifest,
    translator: Translator,
    operation: Option<OngoingOperation>,
    screen: Screen,
    modal_theme: Option<ModalTheme>,
    original_working_dir: std::path::PathBuf,
    modal: ModalComponent,
    backup_screen: BackupScreenComponent,
    restore_screen: RestoreScreenComponent,
    operation_should_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Clone)]
enum Message {
    Idle,
    Ignore,
    ConfirmBackupStart,
    BackupStart { preview: bool },
    ConfirmRestoreStart,
    RestoreStart { preview: bool },
    BackupStep { info: Option<ScanInfo> },
    RestoreStep { info: Option<ScanInfo> },
    CancelOperation,
    EditedBackupTarget(String),
    EditedRestoreSource(String),
    EditedRootPath(usize, String),
    EditedRootStore(usize, Store),
    AddRoot,
    RemoveRoot(usize),
    SwitchScreenToRestore,
    SwitchScreenToBackup,
    ToggleGameListEntryExpanded { name: String },
    SubscribedEvent(iced_native::Event),
}

#[derive(Debug, Clone, PartialEq)]
enum OngoingOperation {
    Backup,
    CancelBackup,
    PreviewBackup,
    CancelPreviewBackup,
    Restore,
    CancelRestore,
    PreviewRestore,
    CancelPreviewRestore,
}

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Backup,
    Restore,
}

#[derive(Debug, Clone, PartialEq)]
enum ModalTheme {
    Error { variant: Error },
    ConfirmBackup,
    ConfirmRestore,
}

impl Default for Screen {
    fn default() -> Self {
        Self::Backup
    }
}

#[derive(Default)]
struct ModalComponent {
    positive_button: button::State,
    negative_button: button::State,
}

impl ModalComponent {
    fn view(&mut self, theme: &ModalTheme, config: &Config, translator: &Translator) -> Container<Message> {
        let positive_button = Button::new(
            &mut self.positive_button,
            Text::new(match theme {
                ModalTheme::Error { .. } => translator.okay_button(),
                _ => translator.continue_button(),
            })
            .horizontal_alignment(HorizontalAlignment::Center),
        )
        .on_press(match theme {
            ModalTheme::Error { .. } => Message::Idle,
            ModalTheme::ConfirmBackup => Message::BackupStart { preview: false },
            ModalTheme::ConfirmRestore => Message::RestoreStart { preview: false },
        })
        .width(Length::Units(125))
        .style(style::Button::Primary);

        let negative_button = Button::new(
            &mut self.negative_button,
            Text::new(translator.cancel_button()).horizontal_alignment(HorizontalAlignment::Center),
        )
        .on_press(Message::Idle)
        .width(Length::Units(125))
        .style(style::Button::Negative);

        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(match theme {
                    ModalTheme::Error { .. } => Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(positive_button),
                    _ => Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(positive_button)
                        .push(negative_button),
                })
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(Text::new(match theme {
                            ModalTheme::Error { variant } => translator.handle_error(variant),
                            ModalTheme::ConfirmBackup => translator.modal_confirm_backup(
                                &crate::path::absolute(&config.backup.path),
                                crate::path::exists(&config.backup.path),
                            ),
                            ModalTheme::ConfirmRestore => {
                                translator.modal_confirm_restore(&crate::path::absolute(&config.restore.path))
                            }
                        }))
                        .height(Length::Fill),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}

#[derive(Default)]
struct GameListEntry {
    name: String,
    files: std::collections::HashSet<String>,
    registry_keys: std::collections::HashSet<String>,
    button: button::State,
    expanded: bool,
}

impl GameListEntry {
    fn view(&mut self, restoring: bool) -> Container<Message> {
        let mut lines = Vec::<String>::new();

        if self.expanded {
            for item in itertools::sorted(&self.files) {
                if restoring {
                    if let Ok(target) = game_file_restoration_target(&item) {
                        lines.push(target);
                    }
                } else {
                    lines.push(item.clone());
                }
            }
            for item in itertools::sorted(&self.registry_keys) {
                lines.push(item.clone());
            }
        }

        Container::new(
            Column::new()
                .padding(5)
                .spacing(5)
                .align_items(Align::Center)
                .push(
                    Row::new().push(
                        Button::new(
                            &mut self.button,
                            Text::new(self.name.clone()).horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::ToggleGameListEntryExpanded {
                            name: self.name.clone(),
                        })
                        .style(style::Button::GameListEntryTitle)
                        .width(Length::Fill)
                        .padding(2),
                    ),
                )
                .push(
                    Row::new().push(
                        Container::new(Text::new(lines.join("\n")))
                            .width(Length::Fill)
                            .style(style::Container::GameListEntryBody),
                    ),
                ),
        )
        .style(style::Container::GameListEntry)
    }
}

#[derive(Default)]
struct GameList {
    entries: Vec<GameListEntry>,
    scroll: scrollable::State,
}

impl GameList {
    fn view(&mut self, restoring: bool) -> Container<Message> {
        self.entries.sort_by_key(|x| x.name.clone());
        Container::new({
            self.entries.iter_mut().enumerate().fold(
                Scrollable::new(&mut self.scroll)
                    .width(Length::Fill)
                    .padding(10)
                    .style(style::Scrollable),
                |parent: Scrollable<'_, Message>, (_i, x)| {
                    parent
                        .push(x.view(restoring))
                        .push(Space::new(Length::Units(0), Length::Units(10)))
                },
            )
        })
    }
}

#[derive(Default)]
struct RootEditorRow {
    button_state: button::State,
    text_state: text_input::State,
    text_history: TextHistory,
}

impl RootEditorRow {
    fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct RootEditor {
    scroll: scrollable::State,
    rows: Vec<RootEditorRow>,
}

impl RootEditor {
    fn view(&mut self, config: &Config, translator: &Translator) -> Container<Message> {
        let roots = config.roots.clone();
        if roots.is_empty() {
            Container::new(Text::new(translator.no_roots_are_configured()))
        } else {
            Container::new({
                self.rows.iter_mut().enumerate().fold(
                    Scrollable::new(&mut self.scroll)
                        .width(Length::Fill)
                        .max_height(100)
                        .style(style::Scrollable),
                    |parent: Scrollable<'_, Message>, (i, x)| {
                        parent
                            .push(
                                Row::new()
                                    .push(
                                        Button::new(
                                            &mut x.button_state,
                                            Text::new(translator.remove_root_button())
                                                .horizontal_alignment(HorizontalAlignment::Center)
                                                .size(14),
                                        )
                                        .on_press(Message::RemoveRoot(i))
                                        .style(style::Button::Negative),
                                    )
                                    .push(Space::new(Length::Units(20), Length::Units(0)))
                                    .push(
                                        TextInput::new(&mut x.text_state, "", &roots[i].path, move |v| {
                                            Message::EditedRootPath(i, v)
                                        })
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(Space::new(Length::Units(20), Length::Units(0)))
                                    .push({
                                        Radio::new(
                                            Store::Steam,
                                            translator.store(&Store::Steam),
                                            Some(roots[i].store),
                                            move |v| Message::EditedRootStore(i, v),
                                        )
                                    })
                                    .push({
                                        Radio::new(
                                            Store::Other,
                                            translator.store(&Store::Other),
                                            Some(roots[i].store),
                                            move |v| Message::EditedRootStore(i, v),
                                        )
                                    }),
                            )
                            .push(Row::new().push(Space::new(Length::Units(0), Length::Units(5))))
                    },
                )
            })
        }
    }
}

#[derive(Default)]
struct DisappearingProgress {
    max: f32,
    current: f32,
}

impl DisappearingProgress {
    fn view(&mut self) -> ProgressBar {
        let visible = self.current > 0.0 && self.current < self.max;
        ProgressBar::new(0.0..=self.max, self.current).height(Length::FillPortion(if visible { 200 } else { 1 }))
    }

    fn complete(&self) -> bool {
        self.current >= self.max
    }
}

#[derive(Default)]
struct BackupScreenComponent {
    total_games: usize,
    log: GameList,
    start_button: button::State,
    preview_button: button::State,
    nav_button: button::State,
    add_root_button: button::State,
    backup_target_input: text_input::State,
    backup_target_history: TextHistory,
    root_editor: RootEditor,
    progress: DisappearingProgress,
}

impl BackupScreenComponent {
    fn new(config: &Config) -> Self {
        let mut root_editor = RootEditor::default();
        for root in &config.roots {
            root_editor.rows.push(RootEditorRow::new(&root.path))
        }

        Self {
            root_editor,
            backup_target_history: TextHistory::new(&config.backup.path, 100),
            ..Default::default()
        }
    }

    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.preview_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewBackup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewBackup) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::BackupStart { preview: true },
                                Some(OngoingOperation::PreviewBackup) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::PreviewBackup) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.start_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::Backup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelBackup) => translator.cancelling_button(),
                                    _ => translator.backup_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::ConfirmBackupStart,
                                Some(OngoingOperation::Backup) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::Backup) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.add_root_button,
                                Text::new(translator.add_root_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::AddRoot)
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push(
                            Button::new(
                                &mut self.nav_button,
                                Text::new(translator.nav_restore_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::SwitchScreenToRestore)
                            .width(Length::Units(125))
                            .style(style::Button::Navigation),
                        ),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.processed_games(self.total_games)).size(50)),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.backup_target_label()))
                        .push(Space::new(Length::Units(20), Length::Units(0)))
                        .push(
                            TextInput::new(
                                &mut self.backup_target_input,
                                "",
                                &config.backup.path,
                                Message::EditedBackupTarget,
                            )
                            .padding(5),
                        ),
                )
                .push(self.root_editor.view(&config, &translator))
                .push(Space::new(Length::Units(0), Length::Units(30)))
                .push(self.log.view(false).height(Length::FillPortion(10_000)))
                .push(self.progress.view()),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}

#[derive(Default)]
struct RestoreScreenComponent {
    total_games: usize,
    log: GameList,
    start_button: button::State,
    preview_button: button::State,
    nav_button: button::State,
    restore_source_input: text_input::State,
    restore_source_history: TextHistory,
    progress: DisappearingProgress,
}

impl RestoreScreenComponent {
    fn new(config: &Config) -> Self {
        Self {
            restore_source_history: TextHistory::new(&config.backup.path, 100),
            ..Default::default()
        }
    }

    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.preview_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewRestore) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewRestore) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::RestoreStart { preview: true },
                                Some(OngoingOperation::PreviewRestore) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::PreviewRestore) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.start_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::Restore) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelRestore) => translator.cancelling_button(),
                                    _ => translator.restore_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::ConfirmRestoreStart,
                                Some(OngoingOperation::Restore) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::Restore) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.nav_button,
                                Text::new(translator.nav_backup_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::SwitchScreenToBackup)
                            .width(Length::Units(125))
                            .style(style::Button::Navigation),
                        ),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.processed_games(self.total_games)).size(50)),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.restore_source_label()))
                        .push(Space::new(Length::Units(20), Length::Units(0)))
                        .push(
                            TextInput::new(
                                &mut self.restore_source_input,
                                "",
                                &config.restore.path,
                                Message::EditedRestoreSource,
                            )
                            .padding(5),
                        ),
                )
                .push(Space::new(Length::Units(0), Length::Units(30)))
                .push(self.log.view(true).height(Length::FillPortion(10_000)))
                .push(self.progress.view()),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
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
                Config::default()
            }
        };
        let manifest = match Manifest::load(&mut config) {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(ModalTheme::Error { variant: x });
                Manifest::default()
            }
        };

        (
            Self {
                backup_screen: BackupScreenComponent::new(&config),
                restore_screen: RestoreScreenComponent::new(&config),
                translator,
                config,
                manifest,
                original_working_dir: std::env::current_dir().unwrap(),
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
                self.backup_screen.progress.current = 0.0;
                self.backup_screen.progress.max = 0.0;
                self.restore_screen.progress.current = 0.0;
                self.restore_screen.progress.max = 0.0;
                self.operation_should_cancel
                    .swap(false, std::sync::atomic::Ordering::Relaxed);
                std::env::set_current_dir(&self.original_working_dir).unwrap();
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

                self.backup_screen.total_games = 0;
                self.backup_screen.log.entries.clear();
                self.modal_theme = None;
                self.backup_screen.progress.current = 0.0;
                self.backup_screen.progress.max = self.manifest.0.len() as f32;

                let backup_path = crate::path::absolute(&self.config.backup.path);
                if !preview {
                    if let Err(e) = prepare_backup_target(&backup_path) {
                        self.modal_theme = Some(ModalTheme::Error { variant: e });
                        return Command::none();
                    }
                }

                self.config.save();
                self.operation = Some(if preview {
                    OngoingOperation::PreviewBackup
                } else {
                    OngoingOperation::Backup
                });

                std::env::set_current_dir(app_dir()).unwrap();

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let game = self.manifest.0[&key].clone();
                    let roots = self.config.roots.clone();
                    let backup_path2 = backup_path.clone();
                    let steam_id = game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;
                    let cancel_flag = self.operation_should_cancel.clone();
                    commands.push(Command::perform(
                        async move {
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(std::time::Duration::from_millis(1));
                                return None;
                            }
                            let info =
                                scan_game_for_backup(&game, &key, &roots, &app_dir().to_string_lossy(), &steam_id);
                            if !preview {
                                back_up_game(&info, &backup_path2, &key);
                            }
                            Some(info)
                        },
                        move |info| Message::BackupStep { info },
                    ));
                }

                Command::batch(commands)
            }
            Message::RestoreStart { preview } => {
                if self.operation.is_some() {
                    return Command::none();
                }

                self.restore_screen.total_games = 0;
                self.restore_screen.log.entries.clear();
                self.modal_theme = None;

                let restore_path = crate::path::normalize(&self.config.restore.path);
                if !crate::path::is_dir(&restore_path) {
                    self.modal_theme = Some(ModalTheme::Error {
                        variant: Error::RestorationSourceInvalid { path: restore_path },
                    });
                    return Command::none();
                }

                self.config.save();
                self.operation = Some(if preview {
                    OngoingOperation::PreviewRestore
                } else {
                    OngoingOperation::Restore
                });
                self.restore_screen.progress.current = 0.0;
                self.restore_screen.progress.max = crate::path::count_subdirectories(&self.config.restore.path) as f32;

                let mut commands: Vec<Command<Message>> = vec![];
                for subdir in walkdir::WalkDir::new(crate::path::normalize(&restore_path))
                    .max_depth(1)
                    .follow_links(false)
                    .into_iter()
                    .skip(1) // the restore path itself
                    .filter_map(|e| e.ok())
                {
                    let cancel_flag = self.operation_should_cancel.clone();
                    commands.push(Command::perform(
                        async move {
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(std::time::Duration::from_millis(1));
                                return None;
                            }
                            let info = scan_dir_for_restoration(&subdir.path().to_string_lossy());
                            if !preview {
                                restore_game(&info);
                            }
                            Some(info)
                        },
                        move |info| Message::RestoreStep { info },
                    ));
                }

                Command::batch(commands)
            }
            Message::BackupStep { info } => {
                self.backup_screen.progress.current += 1.0;
                if let Some(info) = info {
                    if !info.found_files.is_empty() || !info.found_registry_keys.is_empty() {
                        self.backup_screen.total_games += 1;
                        self.backup_screen.log.entries.push(GameListEntry {
                            name: info.game_name,
                            files: info.found_files,
                            registry_keys: info.found_registry_keys,
                            ..Default::default()
                        });
                    }
                }
                if self.backup_screen.progress.complete() {
                    return Command::perform(async move {}, move |_| Message::Idle);
                }
                Command::none()
            }
            Message::RestoreStep { info } => {
                self.restore_screen.progress.current += 1.0;
                if let Some(info) = info {
                    if !info.found_files.is_empty() || !info.found_registry_keys.is_empty() {
                        self.restore_screen.total_games += 1;
                        self.restore_screen.log.entries.push(GameListEntry {
                            name: info.game_name,
                            files: info.found_files,
                            registry_keys: info.found_registry_keys,
                            ..Default::default()
                        });
                    }
                }
                if self.restore_screen.progress.complete() {
                    return Command::perform(async move {}, move |_| Message::Idle);
                }
                Command::none()
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
            Message::EditedBackupTarget(text) => {
                self.backup_screen.backup_target_history.push(&text);
                self.config.backup.path = text;
                Command::none()
            }
            Message::EditedRestoreSource(text) => {
                self.restore_screen.restore_source_history.push(&text);
                self.config.restore.path = text;
                Command::none()
            }
            Message::EditedRootPath(index, path) => {
                self.backup_screen.root_editor.rows[index].text_history.push(&path);
                self.config.roots[index].path = path;
                Command::none()
            }
            Message::EditedRootStore(index, store) => {
                self.config.roots[index].store = store;
                Command::none()
            }
            Message::AddRoot => {
                self.backup_screen.root_editor.rows.push(RootEditorRow::default());
                self.config.roots.push(RootsConfig {
                    path: "".into(),
                    store: Store::Other,
                });
                Command::none()
            }
            Message::RemoveRoot(index) => {
                self.backup_screen.root_editor.rows.remove(index);
                self.config.roots.remove(index);
                Command::none()
            }
            Message::SwitchScreenToBackup => {
                self.screen = Screen::Backup;
                Command::none()
            }
            Message::SwitchScreenToRestore => {
                self.screen = Screen::Restore;
                Command::none()
            }
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &mut self.backup_screen.log.entries {
                            if entry.name == name {
                                entry.expanded = !entry.expanded;
                            }
                        }
                    }
                    Screen::Restore => {
                        for entry in &mut self.restore_screen.log.entries {
                            if entry.name == name {
                                entry.expanded = !entry.expanded;
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::SubscribedEvent(event) => {
                if let iced_native::Event::Keyboard(key) = event {
                    if let iced_native::input::keyboard::Event::Input {
                        state,
                        key_code,
                        modifiers,
                    } = key
                    {
                        let activated = if cfg!(target_os = "mac") {
                            modifiers.logo || modifiers.control
                        } else {
                            modifiers.control
                        };
                        let shortcut = match (key_code, state, activated, modifiers.shift) {
                            (
                                iced_native::input::keyboard::KeyCode::Z,
                                iced_native::input::ButtonState::Pressed,
                                true,
                                false,
                            ) => Some(Shortcut::Undo),
                            (
                                iced_native::input::keyboard::KeyCode::Y,
                                iced_native::input::ButtonState::Pressed,
                                true,
                                false,
                            )
                            | (
                                iced_native::input::keyboard::KeyCode::Z,
                                iced_native::input::ButtonState::Pressed,
                                true,
                                true,
                            ) => Some(Shortcut::Redo),
                            (
                                iced_native::input::keyboard::KeyCode::C,
                                iced_native::input::ButtonState::Pressed,
                                true,
                                false,
                            ) => Some(Shortcut::ClipboardCopy),
                            (
                                iced_native::input::keyboard::KeyCode::X,
                                iced_native::input::ButtonState::Pressed,
                                true,
                                false,
                            ) => Some(Shortcut::ClipboardCut),
                            _ => None,
                        };

                        if let Some(shortcut) = shortcut {
                            if self.backup_screen.backup_target_input.is_focused() {
                                match shortcut {
                                    Shortcut::Undo => {
                                        self.config.backup.path = self.backup_screen.backup_target_history.undo();
                                    }
                                    Shortcut::Redo => {
                                        self.config.backup.path = self.backup_screen.backup_target_history.redo();
                                    }
                                    Shortcut::ClipboardCopy => {
                                        crate::shortcuts::copy_to_clipboard_from_iced(
                                            &self.config.backup.path,
                                            &self.backup_screen.backup_target_input.cursor(),
                                        );
                                    }
                                    Shortcut::ClipboardCut => {
                                        self.config.backup.path = crate::shortcuts::cut_to_clipboard_from_iced(
                                            &self.config.backup.path,
                                            &self.backup_screen.backup_target_input.cursor(),
                                        );
                                        self.backup_screen.backup_target_history.push(&self.config.backup.path);
                                    }
                                }
                            } else if self.restore_screen.restore_source_input.is_focused() {
                                match shortcut {
                                    Shortcut::Undo => {
                                        self.config.restore.path = self.restore_screen.restore_source_history.undo();
                                    }
                                    Shortcut::Redo => {
                                        self.config.restore.path = self.restore_screen.restore_source_history.redo();
                                    }
                                    Shortcut::ClipboardCopy => {
                                        crate::shortcuts::copy_to_clipboard_from_iced(
                                            &self.config.restore.path,
                                            &self.restore_screen.restore_source_input.cursor(),
                                        );
                                    }
                                    Shortcut::ClipboardCut => {
                                        self.config.restore.path = crate::shortcuts::cut_to_clipboard_from_iced(
                                            &self.config.restore.path,
                                            &self.restore_screen.restore_source_input.cursor(),
                                        );
                                        self.restore_screen
                                            .restore_source_history
                                            .push(&self.config.restore.path);
                                    }
                                }
                            } else {
                                for (i, root) in self.backup_screen.root_editor.rows.iter_mut().enumerate() {
                                    if root.text_state.is_focused() {
                                        match shortcut {
                                            Shortcut::Undo => {
                                                self.config.roots[i].path = root.text_history.undo();
                                            }
                                            Shortcut::Redo => {
                                                self.config.roots[i].path = root.text_history.redo();
                                            }
                                            Shortcut::ClipboardCopy => {
                                                crate::shortcuts::copy_to_clipboard_from_iced(
                                                    &self.config.roots[i].path,
                                                    &root.text_state.cursor(),
                                                );
                                            }
                                            Shortcut::ClipboardCut => {
                                                self.config.roots[i].path =
                                                    crate::shortcuts::cut_to_clipboard_from_iced(
                                                        &self.config.roots[i].path,
                                                        &root.text_state.cursor(),
                                                    );
                                                self.backup_screen.root_editor.rows[i]
                                                    .text_history
                                                    .push(&self.config.roots[i].path);
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                };
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events().map(Message::SubscribedEvent)
    }

    fn view(&mut self) -> Element<Message> {
        if let Some(m) = &self.modal_theme {
            return self.modal.view(m, &self.config, &self.translator).into();
        }

        match self.screen {
            Screen::Backup => self.backup_screen.view(&self.config, &self.translator, &self.operation),
            Screen::Restore => self
                .restore_screen
                .view(&self.config, &self.translator, &self.operation),
        }
        .into()
    }
}

mod style {
    use iced::{button, container, scrollable, Background, Color, Vector};

    pub enum Button {
        Primary,
        Disabled,
        Negative,
        Navigation,
        GameListEntryTitle,
    }
    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: match self {
                    Button::Primary => Some(Background::Color(Color::from_rgb8(28, 107, 223))),
                    Button::GameListEntryTitle => Some(Background::Color(Color::from_rgb8(77, 127, 201))),
                    Button::Disabled => Some(Background::Color(Color::from_rgb8(169, 169, 169))),
                    Button::Negative => Some(Background::Color(Color::from_rgb8(255, 0, 0))),
                    Button::Navigation => Some(Background::Color(Color::from_rgb8(136, 0, 219))),
                },
                border_radius: match self {
                    Button::GameListEntryTitle => 10,
                    _ => 4,
                },
                shadow_offset: Vector::new(1.0, 1.0),
                text_color: Color::from_rgb8(0xEE, 0xEE, 0xEE),
                ..button::Style::default()
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                text_color: Color::WHITE,
                shadow_offset: Vector::new(1.0, 2.0),
                ..self.active()
            }
        }
    }

    pub enum Container {
        GameListEntry,
        GameListEntryTitle,
        GameListEntryBody,
    }

    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: match self {
                    Container::GameListEntryTitle => Some(Background::Color(Color::from_rgb8(230, 230, 230))),
                    _ => None,
                },
                border_color: match self {
                    Container::GameListEntry => Color::from_rgb8(230, 230, 230),
                    _ => Color::BLACK,
                },
                border_width: match self {
                    Container::GameListEntry => 1,
                    _ => 0,
                },
                border_radius: match self {
                    Container::GameListEntry | Container::GameListEntryTitle => 10,
                    _ => 0,
                },
                ..container::Style::default()
            }
        }
    }

    pub struct Scrollable;
    impl scrollable::StyleSheet for Scrollable {
        fn active(&self) -> scrollable::Scrollbar {
            scrollable::Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border_radius: 5,
                border_width: 0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: Color::from_rgba8(0, 0, 0, 0.7),
                    border_radius: 5,
                    border_width: 0,
                    border_color: Color::TRANSPARENT,
                },
            }
        }

        fn hovered(&self) -> scrollable::Scrollbar {
            let active = self.active();

            scrollable::Scrollbar {
                background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.4))),
                scroller: scrollable::Scroller {
                    color: Color::from_rgba8(0, 0, 0, 0.8),
                    ..active.scroller
                },
                ..active
            }
        }
    }
}

pub fn run_gui() {
    App::run(iced::Settings::default())
}
