use crate::{
    config::Theme,
    gui::{common::Message, style},
};
use iced::{alignment, Container, Text};
use std::time::Instant;

pub struct Notification {
    text: String,
    created: Instant,
    expires: Option<u64>,
}

impl Notification {
    pub fn new(text: String) -> Self {
        Self {
            text,
            created: Instant::now(),
            expires: None,
        }
    }

    pub fn expires(mut self, expires: u64) -> Self {
        self.expires = Some(expires);
        self
    }

    pub fn expired(&self) -> bool {
        match self.expires {
            None => false,
            Some(expires) => (Instant::now() - self.created).as_secs() > expires,
        }
    }

    pub fn view(&self, theme: Theme) -> Container<'static, Message> {
        Container::new(
            Container::new(Text::new(self.text.clone()))
                .padding([3, 40])
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(style::Container::Notification(theme)),
        )
        .padding([0, 0, 5, 0])
    }
}
