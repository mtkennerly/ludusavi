use crate::config::{Config, RootsConfig};
use crate::lang::Translator;
use crate::manifest::{Manifest, SteamMetadata, Store};
use crate::prelude::{
    app_dir, back_up_game, get_target_from_backup_file, prepare_backup_target, restore_game, scan_game_for_backup,
    scan_game_for_restoration, Error, ScanInfo,
};

use iced::{
    button, executor, scrollable, text_input, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Radio, Row, Scrollable, Space, Text, TextInput,
};

#[derive(Default)]
struct WidgetState {
    log_scroll: scrollable::State,
    roots_scroll: scrollable::State,
    backup_button: button::State,
    restore_button: button::State,
    preview_button: button::State,
    nav_backup_button: button::State,
    nav_restore_button: button::State,
    add_root_button: button::State,
    modal_positive_button: button::State,
    modal_negative_button: button::State,
    backup_target_input: text_input::State,
    restore_source_input: text_input::State,
    root_rows: Vec<(button::State, text_input::State)>,
}

#[derive(Default)]
struct App {
    total_games: usize,
    log: Vec<String>,
    error: Option<Error>,
    widgets: WidgetState,
    config: Config,
    manifest: Manifest,
    translator: Translator,
    operation: Option<OngoingOperation>,
    screen: Screen,
    modal: Option<Modal>,
    original_working_dir: std::path::PathBuf,
}

#[derive(Debug, Clone)]
enum Message {
    Idle,
    ConfirmBackupStart,
    BackupStart,
    ConfirmRestoreStart,
    RestoreStart,
    PreviewBackupStart,
    PreviewRestoreStart,
    BackupStep { game: String, info: ScanInfo },
    RestoreStep { game: String, info: ScanInfo },
    EditedBackupTarget(String),
    EditedRestoreSource(String),
    EditedRootPath(usize, String),
    EditedRootStore(usize, Store),
    AddRoot,
    RemoveRoot(usize),
    SwitchScreenToRestore,
    SwitchScreenToBackup,
}

#[derive(Debug, Clone, PartialEq)]
enum OngoingOperation {
    Backup,
    PreviewBackup,
    Restore,
    PreviewRestore,
}

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Backup,
    Restore,
}

#[derive(Debug, Clone, PartialEq)]
enum Modal {
    ConfirmBackup,
    ConfirmRestore,
}

impl Default for Screen {
    fn default() -> Self {
        Self::Backup
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let translator = Translator::default();
        let mut error: Option<Error> = None;
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                error = Some(x);
                Config::default()
            }
        };
        let manifest = match Manifest::load(&mut config) {
            Ok(x) => x,
            Err(x) => {
                error = Some(x);
                Manifest::default()
            }
        };

        let mut widgets = WidgetState::default();
        while widgets.root_rows.len() < config.roots.len() {
            widgets
                .root_rows
                .push((button::State::default(), text_input::State::default()));
        }

        (
            Self {
                translator,
                error,
                config,
                manifest,
                widgets,
                original_working_dir: std::env::current_dir().unwrap(),
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
                self.error = None;
                self.modal = None;
                std::env::set_current_dir(&self.original_working_dir).unwrap();
                Command::none()
            }
            Message::ConfirmBackupStart => {
                self.modal = Some(Modal::ConfirmBackup);
                Command::none()
            }
            Message::ConfirmRestoreStart => {
                self.modal = Some(Modal::ConfirmRestore);
                Command::none()
            }
            Message::BackupStart => {
                if self.operation.is_some() {
                    return Command::none();
                }

                self.total_games = 0;
                self.log.clear();
                self.error = None;
                self.modal = None;

                let backup_path = crate::path::absolute(&self.config.backup.path);
                if let Err(e) = prepare_backup_target(&backup_path) {
                    self.error = Some(e);
                    return Command::none();
                }

                self.config.save();
                self.operation = Some(OngoingOperation::Backup);

                self.log.push(self.translator.start_of_backup());
                std::env::set_current_dir(app_dir()).unwrap();

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let game = self.manifest.0[&key].clone();
                    let roots = self.config.roots.clone();
                    let key2 = key.clone();
                    let backup_path2 = backup_path.clone();
                    let steam_id = game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;
                    commands.push(Command::perform(
                        async move {
                            let info =
                                scan_game_for_backup(&game, &key, &roots, &app_dir().to_string_lossy(), &steam_id);
                            back_up_game(&info, &backup_path2, &key);
                            info
                        },
                        move |info| Message::BackupStep {
                            game: key2.clone(),
                            info,
                        },
                    ));
                }

                commands.push(Command::perform(async move {}, move |_| Message::Idle));
                Command::batch(commands)
            }
            Message::PreviewBackupStart => {
                if self.operation.is_some() {
                    return Command::none();
                }
                self.config.save();
                self.operation = Some(OngoingOperation::PreviewBackup);
                self.total_games = 0;
                self.log.clear();
                self.error = None;

                self.log.push(self.translator.start_of_backup_preview());
                std::env::set_current_dir(app_dir()).unwrap();

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let game = self.manifest.0[&key].clone();
                    let roots = self.config.roots.clone();
                    let key2 = key.clone();
                    let steam_id = game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;
                    commands.push(Command::perform(
                        async move {
                            scan_game_for_backup(&game, &key, &roots, &app_dir().to_string_lossy(), &steam_id)
                        },
                        move |info| Message::BackupStep {
                            game: key2.clone(),
                            info,
                        },
                    ));
                }

                commands.push(Command::perform(async move {}, move |_| Message::Idle));
                Command::batch(commands)
            }
            Message::RestoreStart => {
                if self.operation.is_some() {
                    return Command::none();
                }

                self.total_games = 0;
                self.log.clear();
                self.error = None;
                self.modal = None;

                let restore_path = crate::path::normalize(&self.config.restore.path);
                if !crate::path::is_dir(&restore_path) {
                    self.error = Some(Error::RestorationSourceInvalid { path: restore_path });
                    return Command::none();
                }

                self.config.save();
                self.operation = Some(OngoingOperation::Restore);

                self.log.push(self.translator.start_of_restore());

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let source = restore_path.clone();
                    let key2 = key.clone();
                    commands.push(Command::perform(
                        async move {
                            let info = scan_game_for_restoration(&key, &source);
                            restore_game(&info);
                            info
                        },
                        move |info| Message::RestoreStep {
                            game: key2.clone(),
                            info,
                        },
                    ));
                }

                commands.push(Command::perform(async move {}, move |_| Message::Idle));
                Command::batch(commands)
            }
            Message::PreviewRestoreStart => {
                if self.operation.is_some() {
                    return Command::none();
                }

                self.total_games = 0;
                self.log.clear();
                self.error = None;

                let restore_path = crate::path::normalize(&self.config.restore.path);
                if !crate::path::is_dir(&restore_path) {
                    self.error = Some(Error::RestorationSourceInvalid { path: restore_path });
                    return Command::none();
                }

                self.config.save();
                self.operation = Some(OngoingOperation::PreviewRestore);

                self.log.push(self.translator.start_of_restore_preview());

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let source = restore_path.clone();
                    let key2 = key.clone();
                    commands.push(Command::perform(
                        async move { scan_game_for_restoration(&key, &source) },
                        move |info| Message::RestoreStep {
                            game: key2.clone(),
                            info,
                        },
                    ));
                }

                commands.push(Command::perform(async move {}, move |_| Message::Idle));
                Command::batch(commands)
            }
            Message::BackupStep { game, info } => {
                if !info.found_files.is_empty() {
                    self.total_games += 1;
                    self.log.push(game);
                    for file in itertools::sorted(info.found_files) {
                        self.log.push(format!(". . . . . {}", file));
                    }
                }
                Command::none()
            }
            Message::RestoreStep { game, info } => {
                if !info.found_files.is_empty() {
                    self.total_games += 1;
                    self.log.push(game);
                    for file in itertools::sorted(info.found_files) {
                        if let Ok(target) = get_target_from_backup_file(&file) {
                            self.log.push(format!(". . . . . {}", target));
                        }
                    }
                }
                Command::none()
            }
            Message::EditedBackupTarget(text) => {
                self.config.backup.path = text;
                Command::none()
            }
            Message::EditedRestoreSource(text) => {
                self.config.restore.path = text;
                Command::none()
            }
            Message::EditedRootPath(index, path) => {
                self.config.roots[index].path = path;
                Command::none()
            }
            Message::EditedRootStore(index, store) => {
                self.config.roots[index].store = store;
                Command::none()
            }
            Message::AddRoot => {
                self.widgets
                    .root_rows
                    .push((button::State::default(), text_input::State::default()));
                self.config.roots.push(RootsConfig {
                    path: "".into(),
                    store: Store::Other,
                });
                Command::none()
            }
            Message::RemoveRoot(index) => {
                self.widgets.root_rows.remove(index);
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
        }
    }

    fn view(&mut self) -> Element<Message> {
        if let Some(m) = &self.modal {
            return Container::new(
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
                                    &mut self.widgets.modal_positive_button,
                                    Text::new(self.translator.continue_button())
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .on_press(match m {
                                    Modal::ConfirmBackup => Message::BackupStart,
                                    Modal::ConfirmRestore => Message::RestoreStart,
                                })
                                .width(Length::Units(125))
                                .style(style::Button::Primary),
                            )
                            .push(
                                Button::new(
                                    &mut self.widgets.modal_negative_button,
                                    Text::new(self.translator.cancel_button())
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .on_press(Message::Idle)
                                .width(Length::Units(125))
                                .style(style::Button::Negative),
                            ),
                    )
                    .push(
                        Row::new()
                            .padding(20)
                            .spacing(20)
                            .align_items(Align::Center)
                            .push(Text::new(match m {
                                Modal::ConfirmBackup => self.translator.modal_confirm_backup(
                                    &crate::path::absolute(&self.config.backup.path),
                                    crate::path::exists(&self.config.backup.path),
                                ),
                                Modal::ConfirmRestore => self
                                    .translator
                                    .modal_confirm_restore(&crate::path::absolute(&self.config.restore.path)),
                            }))
                            .height(Length::Fill),
                    ),
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .into();
        }

        if let Some(e) = &self.error {
            return Container::new(
                Column::new()
                    .padding(5)
                    .align_items(Align::Center)
                    .push(
                        Row::new().padding(20).spacing(20).align_items(Align::Center).push(
                            Button::new(
                                &mut self.widgets.modal_positive_button,
                                Text::new(self.translator.okay_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::Idle)
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        ),
                    )
                    .push(
                        Row::new()
                            .padding(20)
                            .spacing(20)
                            .align_items(Align::Center)
                            .push(Text::new(self.translator.handle_error(e)))
                            .width(Length::Units(640))
                            .height(Length::Units(480)),
                    ),
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .into();
        }

        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(match self.screen {
                    Screen::Backup => Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.widgets.preview_button,
                                Text::new(self.translator.preview_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::PreviewBackupStart)
                            .width(Length::Units(125))
                            .style(if self.operation.is_some() {
                                style::Button::Disabled
                            } else {
                                style::Button::Primary
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.widgets.backup_button,
                                Text::new(self.translator.backup_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::ConfirmBackupStart)
                            .width(Length::Units(125))
                            .style(if self.operation.is_some() {
                                style::Button::Disabled
                            } else {
                                style::Button::Primary
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.widgets.add_root_button,
                                Text::new(self.translator.add_root_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::AddRoot)
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push(
                            Button::new(
                                &mut self.widgets.nav_restore_button,
                                Text::new(self.translator.nav_restore_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::SwitchScreenToRestore)
                            .width(Length::Units(125))
                            .style(style::Button::Navigation),
                        ),
                    Screen::Restore => Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.widgets.preview_button,
                                Text::new(self.translator.preview_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::PreviewRestoreStart)
                            .width(Length::Units(125))
                            .style(if self.operation.is_some() {
                                style::Button::Disabled
                            } else {
                                style::Button::Primary
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.widgets.restore_button,
                                Text::new(self.translator.restore_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::ConfirmRestoreStart)
                            .width(Length::Units(125))
                            .style(if self.operation.is_some() {
                                style::Button::Disabled
                            } else {
                                style::Button::Primary
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.widgets.nav_backup_button,
                                Text::new(self.translator.nav_backup_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::SwitchScreenToBackup)
                            .width(Length::Units(125))
                            .style(style::Button::Navigation),
                        ),
                })
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(self.translator.processed_games(self.total_games)).size(50)),
                )
                .push({
                    let mut row = Row::new().padding(20).align_items(Align::Center);
                    row = match self.screen {
                        Screen::Backup => row
                            .push(Text::new(self.translator.backup_target_label()))
                            .push(Space::new(Length::Units(20), Length::Units(0)))
                            .push(
                                TextInput::new(
                                    &mut self.widgets.backup_target_input,
                                    "",
                                    &self.config.backup.path,
                                    Message::EditedBackupTarget,
                                )
                                .padding(5),
                            ),
                        Screen::Restore => row
                            .push(Text::new(self.translator.restore_source_label()))
                            .push(Space::new(Length::Units(20), Length::Units(0)))
                            .push(
                                TextInput::new(
                                    &mut self.widgets.restore_source_input,
                                    "",
                                    &self.config.restore.path,
                                    Message::EditedRestoreSource,
                                )
                                .padding(5),
                            ),
                    };
                    row
                })
                .push({
                    match self.screen {
                        Screen::Backup => {
                            let translator = self.translator;
                            let roots = self.config.roots.clone();
                            if roots.is_empty() {
                                Container::new(Text::new(translator.no_roots_are_configured()))
                            } else {
                                Container::new({
                                    self.widgets.root_rows.iter_mut().enumerate().fold(
                                        Scrollable::new(&mut self.widgets.roots_scroll)
                                            .width(Length::Fill)
                                            .max_height(100)
                                            .style(style::Scrollable),
                                        |parent: Scrollable<'_, Message>, (i, x)| {
                                            parent
                                                .push(
                                                    Row::new()
                                                        .push(
                                                            Button::new(
                                                                &mut x.0,
                                                                Text::new(translator.remove_root_button())
                                                                    .horizontal_alignment(HorizontalAlignment::Center)
                                                                    .size(14),
                                                            )
                                                            .on_press(Message::RemoveRoot(i))
                                                            .style(style::Button::Negative),
                                                        )
                                                        .push(Space::new(Length::Units(20), Length::Units(0)))
                                                        .push(
                                                            TextInput::new(&mut x.1, "", &roots[i].path, move |v| {
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
                        Screen::Restore => Container::new(Row::new()),
                    }
                })
                .push(Space::new(Length::Units(0), Length::Units(30)))
                .push(Container::new({
                    self.log.iter_mut().enumerate().fold(
                        Scrollable::new(&mut self.widgets.log_scroll)
                            .width(Length::Fill)
                            .style(style::Scrollable),
                        |parent: Scrollable<'_, Message>, (_i, x)| parent.push(Text::new(x.to_string())),
                    )
                })),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
        .into()
    }
}

mod style {
    use iced::{button, scrollable, Background, Color, Vector};

    pub enum Button {
        Primary,
        Disabled,
        Negative,
        Navigation,
    }
    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: match self {
                    Button::Primary => Some(Background::Color(Color::from_rgb8(28, 107, 223))),
                    Button::Disabled => Some(Background::Color(Color::from_rgb8(169, 169, 169))),
                    Button::Negative => Some(Background::Color(Color::from_rgb8(255, 0, 0))),
                    Button::Navigation => Some(Background::Color(Color::from_rgb8(136, 0, 219))),
                },
                border_radius: 4,
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

    pub struct Scrollable;
    impl scrollable::StyleSheet for Scrollable {
        fn active(&self) -> scrollable::Scrollbar {
            scrollable::Scrollbar {
                background: Some(Background::Color([0.0, 0.0, 0.0, 0.3].into())),
                border_radius: 5,
                border_width: 0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: [0.0, 0.0, 0.0, 0.7].into(),
                    border_radius: 5,
                    border_width: 0,
                    border_color: Color::TRANSPARENT,
                },
            }
        }

        fn hovered(&self) -> scrollable::Scrollbar {
            let active = self.active();

            scrollable::Scrollbar {
                background: Some(Background::Color([0.0, 0.0, 0.0, 0.4].into())),
                scroller: scrollable::Scroller {
                    color: [0.0, 0.0, 0.0, 0.8].into(),
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
