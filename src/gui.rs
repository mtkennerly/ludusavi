use crate::config::{Config, RootsConfig};
use crate::lang::Translator;
use crate::manifest::{Manifest, Store};
use crate::prelude::{back_up_game, prepare_backup_target, scan_game, Error, ScanInfo};

use iced::{
    button, executor, scrollable, text_input, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Radio, Row, Scrollable, Settings, Space, Text, TextInput,
};

#[derive(Default)]
struct WidgetState {
    log_scroll: scrollable::State,
    roots_scroll: scrollable::State,
    backup_button: button::State,
    scan_button: button::State,
    add_root_button: button::State,
    backup_target_input: text_input::State,
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
}

#[derive(Debug, Clone)]
enum Message {
    BackupStart,
    BackupStep { game: String, info: ScanInfo },
    BackupEnd,
    ScanStart,
    ScanStep { game: String, info: ScanInfo },
    ScanEnd,
    EditedBackupTarget(String),
    EditedRootPath(usize, String),
    EditedRootStore(usize, Store),
    AddRoot,
    RemoveRoot(usize),
}

#[derive(Debug, Clone, PartialEq)]
enum OngoingOperation {
    Backup,
    Scan,
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
                ..Self::default()
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("Ludusavi v{}", env!("CARGO_PKG_VERSION"))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::BackupStart => {
                if self.operation.is_some() {
                    return Command::none();
                }
                self.config.save();
                self.operation = Some(OngoingOperation::Backup);
                self.total_games = 0;
                self.log.clear();
                self.error = None;

                if let Err(e) = prepare_backup_target(&self.config.backup.path) {
                    self.error = Some(e);
                }

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let game = self.manifest.0[&key].clone();
                    let roots = self.config.roots.clone();
                    let key2 = key.clone();
                    let backup_path = self.config.backup.path.clone();
                    commands.push(Command::perform(
                        async move {
                            let info = scan_game(&game, &key, &roots, &".".to_string());
                            back_up_game(&info, &backup_path, &key);
                            info
                        },
                        move |info| Message::BackupStep {
                            game: key2.clone(),
                            info,
                        },
                    ));
                }

                commands.push(Command::perform(
                    async move {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    },
                    move |_| Message::BackupEnd,
                ));
                Command::batch(commands)
            }
            Message::BackupStep { game, info } => {
                if !info.found_files.is_empty() {
                    self.total_games += 1;
                    self.log.push(game);
                    for file in info.found_files {
                        self.log.push(format!(". . . . . {}", file));
                    }
                }
                Command::none()
            }
            Message::BackupEnd => {
                self.operation = None;
                Command::none()
            }
            Message::ScanStart => {
                if self.operation.is_some() {
                    return Command::none();
                }
                self.config.save();
                self.operation = Some(OngoingOperation::Scan);
                self.total_games = 0;
                self.log.clear();
                self.error = None;

                let mut commands: Vec<Command<Message>> = vec![];
                for key in self.manifest.0.iter().map(|(k, _)| k.clone()) {
                    let game = self.manifest.0[&key].clone();
                    let roots = self.config.roots.clone();
                    let key2 = key.clone();
                    commands.push(Command::perform(
                        async move { scan_game(&game, &key, &roots, &".".to_string()) },
                        move |info| Message::ScanStep {
                            game: key2.clone(),
                            info,
                        },
                    ));
                }

                commands.push(Command::perform(
                    async move {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    },
                    move |_| Message::ScanEnd,
                ));
                Command::batch(commands)
            }
            Message::ScanStep { game, info } => {
                if !info.found_files.is_empty() {
                    self.total_games += 1;
                    self.log.push(game);
                    for file in info.found_files {
                        self.log.push(format!(". . . . . {}", file));
                    }
                }
                Command::none()
            }
            Message::ScanEnd => {
                self.operation = None;
                Command::none()
            }
            Message::EditedBackupTarget(text) => {
                self.config.backup.path = text;
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
        }
    }

    fn view(&mut self) -> Element<Message> {
        Container::new(
            Column::new()
                .padding(20)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.widgets.scan_button,
                                Text::new(self.translator.scan_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::ScanStart)
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
                            .on_press(Message::BackupStart)
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
                            .style(if self.operation.is_some() {
                                style::Button::Disabled
                            } else {
                                style::Button::Primary
                            }),
                        ),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(self.translator.processed_games(self.total_games)).size(50)),
                )
                .push({
                    if self.error.is_some() {
                        Row::new().padding(20).align_items(Align::Center).push(
                            Text::new(match &self.error {
                                Some(e) => self.translator.handle_error(e),
                                _ => "".to_string(),
                            })
                            .size(20),
                        )
                    } else {
                        Row::new()
                    }
                })
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
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
                )
                .push({
                    let translator = self.translator;
                    let roots = self.config.roots.clone();
                    if roots.is_empty() {
                        Container::new(Text::new(translator.no_roots_are_configured()))
                    } else {
                        Container::new({
                            self.widgets.root_rows.iter_mut().enumerate().fold(
                                Scrollable::new(&mut self.widgets.roots_scroll)
                                    .width(Length::Fill)
                                    .height(Length::Units(100))
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
    }
    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: Some(Background::Color(match self {
                    Button::Primary => Color::from_rgb(0.11, 0.42, 0.87),
                    Button::Disabled => Color::from_rgb(0.66, 0.66, 0.66),
                    Button::Negative => Color::from_rgb(1.0, 0.0, 0.0),
                })),
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
    App::run(Settings::default())
}
