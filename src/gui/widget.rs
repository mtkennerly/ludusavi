use std::ops::RangeInclusive;

use iced::{widget as w, Alignment, Length};

use crate::gui::{
    common::Message,
    icon::Icon,
    style::{self, Theme},
};

pub type Renderer = iced::Renderer<Theme>;

pub type Element<'a> = iced::Element<'a, Message, Renderer>;

pub type Button<'a> = w::Button<'a, Message, Renderer>;
pub type Checkbox<'a> = w::Checkbox<'a, Message, Renderer>;
pub type Column<'a> = w::Column<'a, Message, Renderer>;
pub type Container<'a> = w::Container<'a, Message, Renderer>;
pub type PickList<'a, T> = w::PickList<'a, T, Message, Renderer>;
pub type ProgressBar = w::ProgressBar<Renderer>;
pub type Row<'a> = w::Row<'a, Message, Renderer>;
pub type Scrollable<'a> = w::Scrollable<'a, Message, Renderer>;
pub type Text<'a> = w::Text<'a, Renderer>;
pub type TextInput<'a> = w::TextInput<'a, Message, Renderer>;
pub type Tooltip<'a> = w::Tooltip<'a, Message, Renderer>;
pub type Undoable<'a, F> = crate::gui::undoable::Undoable<'a, Message, Renderer, F>;

pub use w::Space;

pub mod id {
    use once_cell::sync::Lazy;

    pub static BACKUP_SCROLL: Lazy<iced::widget::scrollable::Id> = Lazy::new(iced::widget::scrollable::Id::unique);
    pub static RESTORE_SCROLL: Lazy<iced::widget::scrollable::Id> = Lazy::new(iced::widget::scrollable::Id::unique);
    pub static CUSTOM_GAMES_SCROLL: Lazy<iced::widget::scrollable::Id> =
        Lazy::new(iced::widget::scrollable::Id::unique);
    pub static OTHER_SCROLL: Lazy<iced::widget::scrollable::Id> = Lazy::new(iced::widget::scrollable::Id::unique);
    pub static MODAL_SCROLL: Lazy<iced::widget::scrollable::Id> = Lazy::new(iced::widget::scrollable::Id::unique);

    pub fn backup_scroll() -> iced::widget::scrollable::Id {
        (*BACKUP_SCROLL).clone()
    }

    pub fn restore_scroll() -> iced::widget::scrollable::Id {
        (*RESTORE_SCROLL).clone()
    }

    pub fn custom_games_scroll() -> iced::widget::scrollable::Id {
        (*CUSTOM_GAMES_SCROLL).clone()
    }

    pub fn other_scroll() -> iced::widget::scrollable::Id {
        (*OTHER_SCROLL).clone()
    }

    pub fn modal_scroll() -> iced::widget::scrollable::Id {
        (*MODAL_SCROLL).clone()
    }
}

pub fn number_input<'a>(
    value: i32,
    label: String,
    range: RangeInclusive<i32>,
    change: fn(i32) -> Message,
) -> Element<'a> {
    Container::new(
        Row::new()
            .spacing(5)
            .align_items(Alignment::Center)
            .push(Text::new(label))
            .push(Text::new(value.to_string()))
            .push({
                Button::new(Icon::Remove.as_text().width(Length::Shrink))
                    .on_press_if(|| &value > range.start(), || (change)(value - 1))
                    .style(style::Button::Negative)
            })
            .push({
                Button::new(Icon::Add.as_text().width(Length::Shrink))
                    .on_press_if(|| &value < range.end(), || (change)(value + 1))
                    .style(style::Button::Primary)
            }),
    )
    .into()
}

pub trait IcedParentExt<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>;

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<Element<'a>>;
}

impl<'a> IcedParentExt<'a> for Column<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>,
    {
        if condition() {
            self.push(element().into())
        } else {
            self
        }
    }

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<Element<'a>>,
    {
        if let Some(element) = element() {
            self.push(element.into())
        } else {
            self
        }
    }
}

impl<'a> IcedParentExt<'a> for Row<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>,
    {
        if condition() {
            self.push(element().into())
        } else {
            self
        }
    }

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<Element<'a>>,
    {
        if let Some(element) = element() {
            self.push(element.into())
        } else {
            self
        }
    }
}

pub trait IcedButtonExt<'a> {
    fn on_press_if(self, condition: impl FnOnce() -> bool, msg: impl FnOnce() -> Message) -> Self;
    fn on_press_some(self, msg: Option<Message>) -> Self;
}

impl<'a> IcedButtonExt<'a> for Button<'a> {
    fn on_press_if(self, condition: impl FnOnce() -> bool, msg: impl FnOnce() -> Message) -> Self {
        if condition() {
            self.on_press(msg())
        } else {
            self
        }
    }

    fn on_press_some(self, msg: Option<Message>) -> Self {
        match msg {
            Some(msg) => self.on_press(msg),
            None => self,
        }
    }
}
