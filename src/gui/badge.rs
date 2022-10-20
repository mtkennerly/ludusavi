use crate::{
    config::Theme,
    gui::{common::Message, style},
    prelude::ScanChange,
};
use iced::{Container, Text};

#[derive(Default)]
pub struct Badge {
    text: String,
    left_margin: u16,
    change: Option<ScanChange>,
}

impl Badge {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            left_margin: 0,
            change: None,
        }
    }

    pub fn left_margin(mut self, margin: u16) -> Self {
        self.left_margin = margin;
        self
    }

    pub fn change(mut self, change: ScanChange) -> Self {
        self.change = Some(change);
        self
    }

    pub fn view(self, theme: Theme) -> Container<'static, Message> {
        Container::new(
            Container::new(Text::new(self.text).size(14))
                .padding([2, 12, 2, 12])
                .style(match self.change {
                    None => style::Container::Badge(theme),
                    Some(change) => style::Container::ChangeBadge(theme, change),
                }),
        )
        .padding([3, 0, 0, self.left_margin])
        .center_x()
        .center_y()
    }
}
