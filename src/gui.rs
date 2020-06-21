use crate::config::Config;
use crate::lang::Translator;
use crate::manifest::Manifest;
use crate::prelude::Error;
use itertools::Itertools;

use iced::{
    button, executor, scrollable, Align, Application, Button, Column, Command, Container, Element, Length, Row,
    Scrollable, Settings, Text,
};

#[derive(Default)]
struct WidgetState {
    log_scroll: scrollable::State,
    backup_button: button::State,
}

#[derive(Default)]
struct App {
    total_games: i32,
    log: Vec<String>,
    error: Option<Error>,
    widgets: WidgetState,
    config: Config,
    manifest: Manifest,
    translator: Translator,
}

#[derive(Debug, Clone)]
enum Message {
    BackupStart,
    BackupStep { game: String },
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

        (
            Self {
                translator,
                error,
                config,
                manifest,
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
                self.total_games = 0;
                self.log.clear();
                self.log.push(self.translator.backing_up_roots(self.config.roots.len()));
                Command::batch(self.manifest.0.iter().map(|(k, _)| k.clone()).sorted().map(|key| {
                    Command::perform(
                        async move {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        },
                        move |_| Message::BackupStep { game: key.to_string() },
                    )
                }))
            }
            Message::BackupStep { game } => {
                self.total_games += 1;
                self.log.push(game);
                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        if let Some(e) = &self.error {
            Container::new(
                Column::new()
                    .padding(20)
                    .align_items(Align::Center)
                    .push(Text::new(self.translator.handle_error(&e))),
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .center_y()
        } else {
            Container::new(
                Column::new()
                    .padding(20)
                    .align_items(Align::Center)
                    .push(
                        Row::new().padding(20).spacing(20).align_items(Align::Center).push(
                            Button::new(&mut self.widgets.backup_button, Text::new("Back up"))
                                .on_press(Message::BackupStart)
                                .style(style::Button::Primary),
                        ),
                    )
                    .push(
                        Row::new()
                            .padding(20)
                            .align_items(Align::Center)
                            .push(Text::new(self.total_games.to_string()).size(50)),
                    )
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
        }
        .into()
    }
}

mod style {
    use iced::{button, scrollable, Background, Color, Vector};

    pub enum Button {
        Primary,
    }
    pub struct Scrollable;

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: Some(Background::Color(Color::from_rgb(0.11, 0.42, 0.87))),
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
