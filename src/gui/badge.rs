use iced::widget::tooltip;

use crate::{
    gui::{
        common::Message,
        style,
        widget::{Button, Container, Text, Tooltip},
    },
    lang::TRANSLATOR,
    scan::ScanChange,
};

#[derive(Default)]
pub struct Badge {
    text: String,
    left_margin: u16,
    change: Option<ScanChange>,
    tooltip: Option<String>,
    on_press: Option<Message>,
    faded: bool,
}

impl Badge {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            left_margin: 0,
            change: None,
            tooltip: None,
            on_press: None,
            faded: false,
        }
    }

    pub fn scan_change(change: ScanChange) -> Self {
        Self {
            text: change.symbol().to_string(),
            change: Some(change),
            tooltip: match change {
                ScanChange::New => Some(TRANSLATOR.new_tooltip()),
                ScanChange::Different => Some(TRANSLATOR.updated_tooltip()),
                ScanChange::Removed => Some(TRANSLATOR.removed_tooltip()),
                ScanChange::Same => None,
                ScanChange::Unknown => None,
            },
            ..Default::default()
        }
    }

    pub fn new_entry() -> Self {
        Self {
            text: crate::lang::ADD_SYMBOL.to_string(),
            change: Some(ScanChange::New),
            tooltip: Some(TRANSLATOR.new_tooltip()),
            ..Default::default()
        }
    }

    pub fn new_entry_with_count(count: usize) -> Self {
        Self {
            text: format!("{}{}", crate::lang::ADD_SYMBOL, count),
            change: Some(ScanChange::New),
            tooltip: Some(TRANSLATOR.new_tooltip()),
            ..Default::default()
        }
    }

    pub fn changed_entry() -> Self {
        Self {
            text: crate::lang::CHANGE_SYMBOL.to_string(),
            change: Some(ScanChange::Different),
            tooltip: Some(TRANSLATOR.updated_tooltip()),
            ..Default::default()
        }
    }

    pub fn removed_entry() -> Self {
        Self {
            text: crate::lang::REMOVAL_SYMBOL.to_string(),
            change: Some(ScanChange::Removed),
            tooltip: Some(TRANSLATOR.removed_tooltip()),
            ..Default::default()
        }
    }

    pub fn changed_entry_with_count(count: usize) -> Self {
        Self {
            text: format!("{}{}", crate::lang::CHANGE_SYMBOL, count),
            change: Some(ScanChange::Different),
            tooltip: Some(TRANSLATOR.updated_tooltip()),
            ..Default::default()
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn faded(mut self, faded: bool) -> Self {
        self.faded = faded;
        self
    }

    pub fn view(self) -> Container<'static> {
        Container::new({
            let content = Container::new(Text::new(self.text).size(14))
                .padding([2, 12, 2, 12])
                .style(match self.change {
                    None => match self.on_press.as_ref() {
                        Some(Message::FilterDuplicates { game: None, .. }) => style::Container::BadgeActivated,
                        _ if self.faded => style::Container::BadgeFaded,
                        _ => style::Container::Badge,
                    },
                    Some(change) => style::Container::ChangeBadge(change),
                });

            let content = match self.tooltip {
                None => content,
                Some(tooltip) => Container::new(
                    Tooltip::new(content, tooltip, tooltip::Position::Top)
                        .size(16)
                        .gap(5)
                        .style(style::Container::Tooltip),
                ),
            };

            match self.on_press {
                Some(message) => Container::new(
                    Button::new(content)
                        .padding(0)
                        .on_press(message)
                        .style(style::Button::Badge),
                ),
                None => Container::new(content),
            }
        })
        .padding([3, 0, 0, self.left_margin])
        .center_x()
        .center_y()
    }
}
