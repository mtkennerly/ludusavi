use iced::{
    widget::{button, checkbox, container, pick_list, scrollable, text_input},
    Color, Vector,
};
use iced_style::menu;

use crate::{resource::config, scan::ScanChange};

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

pub struct Theme {
    background: Color,
    field: Color,
    text: Color,
    text_inverted: Color,
    text_button: Color,
    text_skipped: Color,
    text_selection: Color,
    positive: Color,
    negative: Color,
    disabled: Color,
    navigation: Color,
    success: Color,
    failure: Color,
    skipped: Color,
    added: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(config::Theme::Light)
    }
}

impl From<config::Theme> for Theme {
    fn from(source: config::Theme) -> Self {
        match source {
            config::Theme::Light => Self {
                background: Color::WHITE,
                field: rgb8!(230, 230, 230),
                text: Color::BLACK,
                text_inverted: Color::WHITE,
                text_button: Color::WHITE,
                text_skipped: Color::BLACK,
                text_selection: Color::from_rgb(0.8, 0.8, 1.0),
                positive: rgb8!(28, 107, 223),
                negative: rgb8!(255, 0, 0),
                disabled: rgb8!(169, 169, 169),
                navigation: rgb8!(136, 0, 219),
                success: rgb8!(77, 127, 201),
                failure: rgb8!(201, 77, 77),
                skipped: rgb8!(230, 230, 230),
                added: rgb8!(28, 223, 86),
            },
            config::Theme::Dark => Self {
                background: rgb8!(41, 41, 41),
                field: rgb8!(74, 74, 74),
                text: Color::WHITE,
                text_inverted: Color::BLACK,
                ..Self::from(config::Theme::Light)
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Application;
impl iced::application::StyleSheet for Theme {
    type Style = Application;

    fn appearance(&self, _style: &Self::Style) -> iced::application::Appearance {
        iced::application::Appearance {
            background_color: self.background,
            text_color: self.text,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Text {
    #[default]
    Default,
    Failure,
}
impl iced::widget::text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> iced_style::text::Appearance {
        match style {
            Text::Default => iced_style::text::Appearance { color: None },
            Text::Failure => iced_style::text::Appearance {
                color: Some(self.negative),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Menu;
impl iced_style::menu::StyleSheet for Theme {
    type Style = Menu;

    fn appearance(&self, _style: &Self::Style) -> menu::Appearance {
        menu::Appearance {
            background: self.field.into(),
            border_color: self.text.alpha(0.5),
            text_color: self.text,
            selected_background: self.positive.into(),
            border_width: 1.0,
            border_radius: 5.0,
            selected_text_color: Color::WHITE,
        }
    }
}

impl From<PickList> for Menu {
    fn from(_: PickList) -> Self {
        Self
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Button {
    #[default]
    Primary,
    Negative,
    GameActionPrimary,
    GameListEntryTitle,
    GameListEntryTitleFailed,
    GameListEntryTitleDisabled,
    GameListEntryTitleUnscanned,
    NavButtonActive,
    NavButtonInactive,
    Badge,
}
impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: match style {
                Self::Style::Primary | Self::Style::GameActionPrimary => Some(self.positive.into()),
                Self::Style::GameListEntryTitle => Some(self.success.into()),
                Self::Style::GameListEntryTitleFailed => Some(self.failure.into()),
                Self::Style::GameListEntryTitleDisabled => Some(self.skipped.into()),
                Self::Style::GameListEntryTitleUnscanned => None,
                Self::Style::Negative => Some(self.negative.into()),
                Self::Style::NavButtonActive => Some(self.navigation.alpha(0.9).into()),
                Self::Style::NavButtonInactive => None,
                Self::Style::Badge => None,
            },
            border_radius: match style {
                Self::Style::GameActionPrimary
                | Self::Style::GameListEntryTitle
                | Self::Style::GameListEntryTitleFailed
                | Self::Style::GameListEntryTitleDisabled
                | Self::Style::GameListEntryTitleUnscanned
                | Self::Style::NavButtonActive
                | Self::Style::NavButtonInactive => 10.0,
                _ => 4.0,
            },
            border_width: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => 1.0,
                _ => 0.0,
            },
            shadow_offset: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => Vector::new(0.0, 0.0),
                _ => Vector::new(1.0, 1.0),
            },
            text_color: match style {
                Self::Style::GameListEntryTitleDisabled => self.text_skipped.alpha(0.8),
                Self::Style::GameListEntryTitleUnscanned => self.text.alpha(0.8),
                Self::Style::NavButtonInactive => self.text,
                _ => self.text_button.alpha(0.8),
            },
            border_color: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => self.navigation,
                _ => Color::TRANSPARENT,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: match style {
                Self::Style::NavButtonActive => Some(self.navigation.alpha(0.95).into()),
                Self::Style::NavButtonInactive => Some(self.navigation.alpha(0.2).into()),
                _ => self.active(style).background,
            },
            border_radius: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => 10.0,
                _ => active.border_radius,
            },
            border_width: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => 1.0,
                _ => active.border_width,
            },
            border_color: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => self.navigation,
                _ => active.border_color,
            },
            text_color: match style {
                Self::Style::GameListEntryTitleDisabled => self.text_skipped,
                Self::Style::GameListEntryTitleUnscanned | Self::Style::NavButtonInactive => self.text,
                _ => self.text_button,
            },
            shadow_offset: match style {
                Self::Style::NavButtonActive | Self::Style::NavButtonInactive => Vector::new(0.0, 0.0),
                _ => Vector::new(1.0, 2.0),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Container {
    #[default]
    Wrapper,
    Primary,
    ModalBackground,
    GameListEntry,
    Badge,
    BadgeActivated,
    BadgeFaded,
    ChangeBadge(ScanChange),
    DisabledBackup,
    Notification,
    Tooltip,
}
impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: match style {
                Self::Style::Wrapper => Color::TRANSPARENT.into(),
                Self::Style::GameListEntry => self.field.alpha(0.15).into(),
                Self::Style::ModalBackground | Self::Style::Notification | Self::Style::Tooltip => self.field.into(),
                Self::Style::DisabledBackup => self.disabled.into(),
                Self::Style::BadgeActivated => self.negative.into(),
                _ => self.background.into(),
            },
            border_color: match style {
                Self::Style::GameListEntry | Self::Style::Notification => self.field,
                Self::Style::ChangeBadge(change) => match change {
                    ScanChange::New => self.added,
                    ScanChange::Different => self.positive,
                    ScanChange::Removed => self.negative,
                    ScanChange::Same | ScanChange::Unknown => self.disabled,
                },
                Self::Style::BadgeActivated => self.negative,
                Self::Style::BadgeFaded => self.disabled,
                _ => self.text,
            },
            border_width: match style {
                Self::Style::GameListEntry
                | Self::Style::Badge
                | Self::Style::BadgeActivated
                | Self::Style::BadgeFaded
                | Self::Style::ChangeBadge(..)
                | Self::Style::Notification => 1.0,
                _ => 0.0,
            },
            border_radius: match style {
                Self::Style::GameListEntry
                | Self::Style::Badge
                | Self::Style::BadgeActivated
                | Self::Style::BadgeFaded
                | Self::Style::ChangeBadge(..)
                | Self::Style::DisabledBackup => 10.0,
                Self::Style::Notification | Self::Style::Tooltip => 20.0,
                _ => 0.0,
            },
            text_color: match style {
                Self::Style::DisabledBackup => Some(self.text_inverted),
                Self::Style::ChangeBadge(change) => match change {
                    ScanChange::New => Some(self.added),
                    ScanChange::Different => Some(self.positive),
                    ScanChange::Removed => Some(self.negative),
                    ScanChange::Same | ScanChange::Unknown => Some(self.disabled),
                },
                Self::Style::BadgeActivated => Some(self.text_button),
                Self::Style::BadgeFaded => Some(self.disabled),
                _ => Some(self.text),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Scrollable;
impl scrollable::StyleSheet for Theme {
    type Style = Scrollable;

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: Color::TRANSPARENT.into(),
            border_radius: 5.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: self.text.alpha(0.7),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(&self, style: &Self::Style, is_mouse_over_scrollbar: bool) -> scrollable::Scrollbar {
        let active = self.active(style);

        if !is_mouse_over_scrollbar {
            return active;
        }

        scrollable::Scrollbar {
            background: self.text.alpha(0.4).into(),
            scroller: scrollable::Scroller {
                color: self.text.alpha(0.8),
                ..active.scroller
            },
            ..active
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum PickList {
    #[default]
    Primary,
    Backup,
    Popup,
}
impl pick_list::StyleSheet for Theme {
    type Style = PickList;

    fn active(&self, style: &Self::Style) -> pick_list::Appearance {
        pick_list::Appearance {
            border_radius: match style {
                Self::Style::Primary => 5.0,
                Self::Style::Backup | Self::Style::Popup => 10.0,
            },
            background: self.field.alpha(0.6).into(),
            border_color: self.text.alpha(0.7),
            text_color: self.text,
            placeholder_color: iced::Color::BLACK,
            border_width: 1.0,
            handle_color: self.text,
        }
    }

    fn hovered(&self, style: &Self::Style) -> pick_list::Appearance {
        pick_list::Appearance {
            background: self.field.into(),
            ..self.active(style)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Checkbox;
impl checkbox::StyleSheet for Theme {
    type Style = Checkbox;

    fn active(&self, _style: &Self::Style, _is_checked: bool) -> checkbox::Appearance {
        checkbox::Appearance {
            background: self.field.alpha(0.6).into(),
            icon_color: self.text,
            border_radius: 5.0,
            border_width: 1.0,
            border_color: self.text.alpha(0.6),
            text_color: Some(self.text),
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        checkbox::Appearance {
            background: self.field.into(),
            ..self.active(style, is_checked)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextInput;
impl text_input::StyleSheet for Theme {
    type Style = TextInput;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Color::TRANSPARENT.into(),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: self.text.alpha(0.8),
            icon_color: self.text,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            border_color: self.text,
            ..self.active(style)
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        self.text.alpha(0.5)
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        self.text
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        self.text.alpha(0.5)
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        self.text_selection
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: self.disabled.into(),
            ..self.active(style)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressBar;
impl iced::widget::progress_bar::StyleSheet for Theme {
    type Style = ProgressBar;

    fn appearance(&self, _style: &Self::Style) -> iced_style::progress_bar::Appearance {
        iced_style::progress_bar::Appearance {
            background: self.disabled.into(),
            bar: self.added.into(),
            border_radius: 4.0,
        }
    }
}
