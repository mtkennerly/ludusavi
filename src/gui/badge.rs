use iced::widget::tooltip;

use crate::{
    gui::{
        style,
        widget::{Container, Text, Tooltip},
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
}

impl Badge {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            left_margin: 0,
            change: None,
            tooltip: None,
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

    pub fn changed_entry_with_count(count: usize) -> Self {
        Self {
            text: format!("{}{}", crate::lang::CHANGE_SYMBOL, count),
            change: Some(ScanChange::Different),
            tooltip: Some(TRANSLATOR.updated_tooltip()),
            ..Default::default()
        }
    }

    pub fn view(self) -> Container<'static> {
        Container::new({
            let content = Container::new(Text::new(self.text).size(14))
                .padding([2, 12, 2, 12])
                .style(match self.change {
                    None => style::Container::Badge,
                    Some(change) => style::Container::ChangeBadge(change),
                });

            match self.tooltip {
                None => content,
                Some(tooltip) => Container::new(
                    Tooltip::new(content, tooltip, tooltip::Position::Top)
                        .size(16)
                        .gap(5)
                        .style(style::Container::Tooltip),
                ),
            }
        })
        .padding([3, 0, 0, self.left_margin])
        .center_x()
        .center_y()
    }
}
