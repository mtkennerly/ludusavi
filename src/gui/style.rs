use iced::{button, container, scrollable, Background, Color, Vector};

pub enum Button {
    Primary,
    Disabled,
    Negative,
    GameListEntryTitle,
    GameListEntryTitleFailed,
    GameListEntryTitleDisabled,
}
impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        button::Style {
            background: match self {
                Self::Primary => Some(Background::Color(Color::from_rgb8(28, 107, 223))),
                Self::GameListEntryTitle => Some(Background::Color(Color::from_rgb8(77, 127, 201))),
                Self::GameListEntryTitleFailed => Some(Background::Color(Color::from_rgb8(201, 77, 77))),
                Self::GameListEntryTitleDisabled => Some(Background::Color(Color::from_rgb8(230, 230, 230))),
                Self::Disabled => Some(Background::Color(Color::from_rgb8(169, 169, 169))),
                Self::Negative => Some(Background::Color(Color::from_rgb8(255, 0, 0))),
            },
            border_radius: match self {
                Self::GameListEntryTitle | Self::GameListEntryTitleFailed | Self::GameListEntryTitleDisabled => 10,
                _ => 4,
            },
            shadow_offset: Vector::new(1.0, 1.0),
            text_color: match self {
                Self::GameListEntryTitleDisabled => Color::from_rgb8(0x44, 0x44, 0x44),
                _ => Color::from_rgb8(0xEE, 0xEE, 0xEE),
            },
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            text_color: match self {
                Self::GameListEntryTitleDisabled => Color::BLACK,
                _ => Color::WHITE,
            },
            shadow_offset: Vector::new(1.0, 2.0),
            ..self.active()
        }
    }
}

pub enum NavButton {
    Active,
    Inactive,
}
impl button::StyleSheet for NavButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: match self {
                Self::Active => Some(Background::Color(Color::from_rgba8(136, 0, 219, 0.9))),
                Self::Inactive => Some(Background::Color(Color::TRANSPARENT)),
            },
            border_radius: 10,
            border_width: 1,
            border_color: Color::from_rgb8(136, 0, 219),
            text_color: match self {
                Self::Active => Color::WHITE,
                Self::Inactive => Color::BLACK,
            },
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: match self {
                Self::Active => Some(Background::Color(Color::from_rgba8(136, 0, 219, 0.95))),
                Self::Inactive => Some(Background::Color(Color::from_rgba8(136, 0, 219, 0.2))),
            },
            ..self.active()
        }
    }
}

pub enum Container {
    ModalBackground,
    GameListEntry,
    Badge,
}

impl container::StyleSheet for Container {
    fn style(&self) -> container::Style {
        container::Style {
            background: match self {
                Self::ModalBackground => Some(Background::Color(Color::from_rgb8(230, 230, 230))),
                _ => None,
            },
            border_color: match self {
                Self::GameListEntry => Color::from_rgb8(230, 230, 230),
                _ => Color::BLACK,
            },
            border_width: match self {
                Self::GameListEntry | Self::Badge => 1,
                _ => 0,
            },
            border_radius: match self {
                Self::GameListEntry | Self::Badge => 10,
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
