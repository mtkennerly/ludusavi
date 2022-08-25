use crate::{
    config::Theme,
    gui::{common::Message, style},
};
use iced::{Container, Text};

#[derive(Default)]
pub struct Badge {
    text: String,
    left_margin: u16,
}

impl Badge {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            left_margin: 0,
        }
    }

    pub fn left_margin(mut self, margin: u16) -> Self {
        self.left_margin = margin;
        self
    }

    pub fn view(self, theme: Theme) -> Container<'static, Message> {
        Container::new(
            Container::new(Text::new(self.text).size(14))
                .padding([2, 12, 2, 12])
                .style(style::Container::Badge(theme)),
        )
        .padding([3, 0, 0, self.left_margin])
        .center_x()
        .center_y()
    }
}
