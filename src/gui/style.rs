use crate::config::Theme;
use iced::{button, checkbox, container, pick_list, scrollable, text_input, Background, Color, Vector};
use iced_style::menu;

macro_rules! rgb8 {
    ($r:expr, $g:expr, $b:expr) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

trait ColorExt {
    fn alpha(self, alpha: f32) -> Color;
}

impl ColorExt for Color {
    fn alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}

mod light {
    use super::*;
    pub const BACKGROUND: Color = Color::WHITE;
    pub const FIELD: Color = rgb8!(230, 230, 230);
    pub const TEXT: Color = Color::BLACK;
    pub const TEXT_INVERTED: Color = Color::WHITE;
    pub const TEXT_BUTTON: Color = Color::WHITE;
    pub const TEXT_SKIPPED: Color = Color::BLACK;
    pub const TEXT_SELECTION: Color = Color::from_rgb(0.8, 0.8, 1.0);
    pub const POSITIVE: Color = rgb8!(28, 107, 223);
    pub const NEGATIVE: Color = rgb8!(255, 0, 0);
    pub const DISABLED: Color = rgb8!(169, 169, 169);
    pub const NAVIGATION: Color = rgb8!(136, 0, 219);
    pub const SUCCESS: Color = rgb8!(77, 127, 201);
    pub const FAILURE: Color = rgb8!(201, 77, 77);
    pub const SKIPPED: Color = rgb8!(230, 230, 230);
}

mod dark {
    use super::*;
    pub use light::*;
    pub const BACKGROUND: Color = rgb8!(41, 41, 41);
    pub const FIELD: Color = rgb8!(74, 74, 74);
    pub const TEXT: Color = Color::WHITE;
    pub const TEXT_INVERTED: Color = Color::BLACK;
}

impl Theme {
    pub fn background(&self) -> Color {
        match self {
            Self::Light => light::BACKGROUND,
            Self::Dark => dark::BACKGROUND,
        }
    }

    pub fn field(&self) -> Color {
        match self {
            Self::Light => light::FIELD,
            Self::Dark => dark::FIELD,
        }
    }

    pub fn text(&self) -> Color {
        match self {
            Self::Light => light::TEXT,
            Self::Dark => dark::TEXT,
        }
    }

    pub fn text_inverted(&self) -> Color {
        match self {
            Self::Light => light::TEXT_INVERTED,
            Self::Dark => dark::TEXT_INVERTED,
        }
    }

    pub fn text_button(&self) -> Color {
        match self {
            Self::Light => light::TEXT_BUTTON,
            Self::Dark => dark::TEXT_BUTTON,
        }
    }

    pub fn text_skipped(&self) -> Color {
        match self {
            Self::Light => light::TEXT_SKIPPED,
            Self::Dark => dark::TEXT_SKIPPED,
        }
    }

    pub fn text_selection(&self) -> Color {
        match self {
            Self::Light => light::TEXT_SELECTION,
            Self::Dark => dark::TEXT_SELECTION,
        }
    }

    pub fn positive(&self) -> Color {
        match self {
            Self::Light => light::POSITIVE,
            Self::Dark => dark::POSITIVE,
        }
    }

    pub fn negative(&self) -> Color {
        match self {
            Self::Light => light::NEGATIVE,
            Self::Dark => dark::NEGATIVE,
        }
    }

    pub fn disabled(&self) -> Color {
        match self {
            Self::Light => light::DISABLED,
            Self::Dark => dark::DISABLED,
        }
    }

    pub fn navigation(&self) -> Color {
        match self {
            Self::Light => light::NAVIGATION,
            Self::Dark => dark::NAVIGATION,
        }
    }

    pub fn success(&self) -> Color {
        match self {
            Self::Light => light::SUCCESS,
            Self::Dark => dark::SUCCESS,
        }
    }

    pub fn failure(&self) -> Color {
        match self {
            Self::Light => light::FAILURE,
            Self::Dark => dark::FAILURE,
        }
    }

    pub fn skipped(&self) -> Color {
        match self {
            Self::Light => light::SKIPPED,
            Self::Dark => dark::SKIPPED,
        }
    }
}

pub enum Button {
    Primary(Theme),
    Disabled(Theme),
    Negative(Theme),
    GameListEntryTitle(Theme),
    GameListEntryTitleFailed(Theme),
    GameListEntryTitleDisabled(Theme),
    GameListEntryTitleUnscanned(Theme),
}
impl Button {
    fn theme(&self) -> &Theme {
        match self {
            Self::Primary(theme) => theme,
            Self::Disabled(theme) => theme,
            Self::Negative(theme) => theme,
            Self::GameListEntryTitle(theme) => theme,
            Self::GameListEntryTitleFailed(theme) => theme,
            Self::GameListEntryTitleDisabled(theme) => theme,
            Self::GameListEntryTitleUnscanned(theme) => theme,
        }
    }
}
impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        let t = self.theme();
        button::Style {
            background: match self {
                Self::Primary(_) => Some(t.positive().into()),
                Self::GameListEntryTitle(_) => Some(t.success().into()),
                Self::GameListEntryTitleFailed(_) => Some(t.failure().into()),
                Self::GameListEntryTitleDisabled(_) => Some(t.skipped().into()),
                Self::GameListEntryTitleUnscanned(_) => None,
                Self::Disabled(_) => Some(t.disabled().into()),
                Self::Negative(_) => Some(t.negative().into()),
            },
            border_radius: match self {
                Self::GameListEntryTitle(_)
                | Self::GameListEntryTitleFailed(_)
                | Self::GameListEntryTitleDisabled(_)
                | Self::GameListEntryTitleUnscanned(_) => 10.0,
                _ => 4.0,
            },
            shadow_offset: Vector::new(1.0, 1.0),
            text_color: match self {
                Self::GameListEntryTitleDisabled(_) => t.text_skipped().alpha(0.8),
                Self::GameListEntryTitleUnscanned(_) => t.text().alpha(0.8),
                _ => t.text_button().alpha(0.8),
            },
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        let t = self.theme();
        button::Style {
            text_color: match self {
                Self::GameListEntryTitleDisabled(_) => t.text_skipped(),
                Self::GameListEntryTitleUnscanned(_) => t.text(),
                _ => t.text_button(),
            },
            shadow_offset: Vector::new(1.0, 2.0),
            ..self.active()
        }
    }
}

pub enum NavButton {
    Active(Theme),
    Inactive(Theme),
}
impl NavButton {
    fn theme(&self) -> &Theme {
        match self {
            Self::Active(theme) => theme,
            Self::Inactive(theme) => theme,
        }
    }
}
impl button::StyleSheet for NavButton {
    fn active(&self) -> button::Style {
        let t = self.theme();
        button::Style {
            background: match self {
                Self::Active(_) => Some(t.navigation().alpha(0.9).into()),
                Self::Inactive(_) => Some(Background::Color(Color::TRANSPARENT)),
            },
            border_radius: 10.0,
            border_width: 1.0,
            border_color: t.navigation(),
            text_color: match self {
                Self::Active(_) => t.text_button(),
                Self::Inactive(_) => t.text(),
            },
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        let t = self.theme();
        button::Style {
            background: match self {
                Self::Active(_) => Some(t.navigation().alpha(0.95).into()),
                Self::Inactive(_) => Some(t.navigation().alpha(0.2).into()),
            },
            ..self.active()
        }
    }
}

pub enum Container {
    Primary(Theme),
    ModalBackground(Theme),
    GameListEntry(Theme),
    Badge(Theme),
    DisabledBackup(Theme),
}
impl Container {
    fn theme(&self) -> &Theme {
        match self {
            Self::Primary(theme) => theme,
            Self::ModalBackground(theme) => theme,
            Self::GameListEntry(theme) => theme,
            Self::Badge(theme) => theme,
            Self::DisabledBackup(theme) => theme,
        }
    }
}
impl container::StyleSheet for Container {
    fn style(&self) -> container::Style {
        let t = self.theme();
        container::Style {
            background: match self {
                Self::ModalBackground(_) => Some(t.field().into()),
                Self::DisabledBackup(_) => Some(t.disabled().into()),
                _ => Some(t.background().into()),
            },
            border_color: match self {
                Self::GameListEntry(_) => t.field(),
                _ => t.text(),
            },
            border_width: match self {
                Self::GameListEntry(_) | Self::Badge(_) => 1.0,
                _ => 0.0,
            },
            border_radius: match self {
                Self::GameListEntry(_) | Self::Badge(_) | Self::DisabledBackup(_) => 10.0,
                _ => 0.0,
            },
            text_color: match self {
                Self::DisabledBackup(_) => Some(t.text_inverted()),
                _ => Some(t.text()),
            },
        }
    }
}

pub struct Scrollable(pub Theme);
impl scrollable::StyleSheet for Scrollable {
    fn active(&self) -> scrollable::Scrollbar {
        let t = &self.0;
        scrollable::Scrollbar {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border_radius: 5.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: t.text().alpha(0.7),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(&self) -> scrollable::Scrollbar {
        let active = self.active();
        let t = &self.0;

        scrollable::Scrollbar {
            background: Some(t.text().alpha(0.4).into()),
            scroller: scrollable::Scroller {
                color: t.text().alpha(0.8),
                ..active.scroller
            },
            ..active
        }
    }
}

pub enum PickList {
    Primary(Theme),
    Backup(Theme),
}
impl PickList {
    fn theme(&self) -> &Theme {
        match self {
            Self::Primary(theme) => theme,
            Self::Backup(theme) => theme,
        }
    }
}
impl pick_list::StyleSheet for PickList {
    fn active(&self) -> pick_list::Style {
        let t = self.theme();
        pick_list::Style {
            border_radius: match self {
                Self::Primary(_) => 5.0,
                Self::Backup(_) => 10.0,
            },
            background: t.field().alpha(0.6).into(),
            border_color: t.text().alpha(0.7),
            text_color: t.text(),
            ..Default::default()
        }
    }

    fn hovered(&self) -> pick_list::Style {
        let t = self.theme();
        pick_list::Style {
            background: t.field().into(),
            ..self.active()
        }
    }

    fn menu(&self) -> menu::Style {
        let t = self.theme();
        pick_list::Menu {
            background: t.field().into(),
            border_color: t.text().alpha(0.5),
            text_color: t.text(),
            selected_background: t.positive().into(),
            ..Default::default()
        }
    }
}

pub struct Checkbox(pub Theme);
impl checkbox::StyleSheet for Checkbox {
    fn active(&self, _is_checked: bool) -> checkbox::Style {
        let t = &self.0;
        checkbox::Style {
            background: t.field().alpha(0.6).into(),
            checkmark_color: t.text(),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: t.text().alpha(0.6),
            text_color: Some(t.text()),
        }
    }

    fn hovered(&self, is_checked: bool) -> checkbox::Style {
        let t = &self.0;
        checkbox::Style {
            background: t.field().into(),
            ..self.active(is_checked)
        }
    }
}

pub struct TextInput(pub Theme);
impl text_input::StyleSheet for TextInput {
    fn active(&self) -> text_input::Style {
        let t = &self.0;
        text_input::Style {
            background: t.background().into(),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: t.text().alpha(0.8),
        }
    }

    fn focused(&self) -> text_input::Style {
        let t = &self.0;
        text_input::Style {
            border_color: t.text(),
            ..self.active()
        }
    }

    fn placeholder_color(&self) -> Color {
        self.0.text().alpha(0.5)
    }

    fn value_color(&self) -> Color {
        self.0.text()
    }

    fn selection_color(&self) -> Color {
        self.0.text_selection()
    }
}
