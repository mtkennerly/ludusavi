use iced::{
    alignment,
    widget::{button, text, Column, Container, Row},
    Alignment, Element, Length, Size, Task,
};

use crate::{
    gui::{icon::Icon, style},
    lang::TRANSLATOR,
    prelude::{run_command, Privacy},
    resource::config,
};

const POSITIVE_CHOICE: &str = "::ludusavi-positive::";

#[allow(unused)]
pub fn info(message: &str) {
    show(Kind::Info, message);
}

pub fn error(message: &str) {
    show(Kind::Error, message);
}

pub fn confirm(message: &str) -> bool {
    show(Kind::Confirm, message)
}

pub fn show(kind: Kind, message: &str) -> bool {
    let exe = std::env::current_exe().unwrap().to_string_lossy().to_string();
    match run_command(
        &exe,
        &["dialog", "--kind", kind.slug(), "--message", message],
        &[0],
        Privacy::Public,
    ) {
        Ok(info) => info.stdout.contains(POSITIVE_CHOICE),
        Err(e) => {
            log::error!("Failed to show custom dialog: {e:?}");
            false
        }
    }
}

pub fn run(theme: config::Theme, kind: Kind, message: String) -> iced::Result {
    let app = iced::application(DialogApp::title, DialogApp::update, DialogApp::view)
        .theme(DialogApp::theme)
        .settings(iced::Settings {
            default_font: crate::gui::font::TEXT,
            ..Default::default()
        })
        .window(iced::window::Settings {
            min_size: Some(Size::new(320.0, 180.0)),
            exit_on_close_request: true,
            position: iced::window::Position::Centered,
            #[cfg(target_os = "linux")]
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: std::env::var(crate::prelude::ENV_LINUX_APP_ID)
                    .unwrap_or_else(|_| crate::prelude::LINUX_APP_ID.to_string()),
                ..Default::default()
            },
            icon: match image::load_from_memory(include_bytes!("../../assets/icon.png")) {
                Ok(buffer) => {
                    let buffer = buffer.to_rgba8();
                    let width = buffer.width();
                    let height = buffer.height();
                    let dynamic_image = image::DynamicImage::ImageRgba8(buffer);
                    match iced::window::icon::from_rgba(dynamic_image.into_bytes(), width, height) {
                        Ok(icon) => Some(icon),
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            },
            ..Default::default()
        });

    app.run_with(move || {
        (
            DialogApp::new(theme, kind, message),
            Task::batch([
                iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::TEXT_DATA)).map(|_| Message::Ignore),
                iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::ICONS_DATA)).map(|_| Message::Ignore),
                iced::window::get_oldest().and_then(iced::window::gain_focus),
                iced::window::get_oldest().and_then(|id| iced::window::resize(id, iced::Size::new(320.0, 180.0))),
            ]),
        )
    })
}

fn icon<'a>(icon: Icon) -> Element<'a, Message, crate::gui::style::Theme> {
    text(icon.as_char().to_string())
        .font(crate::gui::font::ICONS)
        .size(40)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Kind {
    Info,
    Error,
    Confirm,
}

impl Kind {
    pub const ALL_CLI: &'static [&'static str] = &[Self::INFO, Self::ERROR, Self::CONFIRM];
    const INFO: &'static str = "info";
    const ERROR: &'static str = "error";
    const CONFIRM: &'static str = "confirm";
}

impl Kind {
    pub fn slug(&self) -> &str {
        match self {
            Self::Info => Self::INFO,
            Self::Error => Self::ERROR,
            Self::Confirm => Self::CONFIRM,
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::INFO => Ok(Self::Info),
            Self::ERROR => Ok(Self::Error),
            Self::CONFIRM => Ok(Self::Confirm),
            _ => Err(format!("invalid dialog kind: {}", s)),
        }
    }
}

struct DialogApp {
    theme: config::Theme,
    kind: Kind,
    message: String,
    positive: String,
    negative: String,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Ignore,
    Positive,
    Negative,
}

impl DialogApp {
    fn new(theme: config::Theme, kind: Kind, message: String) -> Self {
        let positive = match kind {
            Kind::Info => TRANSLATOR.okay_button(),
            Kind::Error => TRANSLATOR.okay_button(),
            Kind::Confirm => TRANSLATOR.continue_button(),
        };

        let negative = TRANSLATOR.cancel_button();

        Self {
            theme,
            kind,
            message,
            positive,
            negative,
        }
    }

    fn theme(&self) -> crate::gui::style::Theme {
        crate::gui::style::Theme::from(self.theme)
    }

    fn title(&self) -> String {
        TRANSLATOR.app_name()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Ignore => {}
            Message::Positive => {
                println!("{POSITIVE_CHOICE}");
                std::process::exit(0);
            }
            Message::Negative => {
                std::process::exit(0);
            }
        }
    }

    fn view(&self) -> Element<Message, crate::gui::style::Theme> {
        Container::new(
            Column::new()
                .spacing(20)
                .padding(20)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .push(
                    Row::new()
                        .spacing(20)
                        .align_y(Alignment::Center)
                        .push(match self.kind {
                            Kind::Info => icon(Icon::Info),
                            Kind::Error => icon(Icon::Error),
                            Kind::Confirm => icon(Icon::Question),
                        })
                        .push(text(&self.message)),
                )
                .push(
                    Row::new()
                        .spacing(20)
                        .align_y(Alignment::Center)
                        .push(button(text(&self.positive)).on_press(Message::Positive))
                        .push_maybe(match self.kind {
                            Kind::Info => None,
                            Kind::Error => None,
                            Kind::Confirm => Some(
                                button(text(&self.negative))
                                    .class(style::Button::Negative)
                                    .on_press(Message::Negative),
                            ),
                        }),
                ),
        )
        .center(Length::Fill)
        .into()
    }
}
