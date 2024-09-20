use std::time::Instant;

use iced::{alignment, padding};

use crate::gui::{
    style,
    widget::{text, Container},
};

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

    pub fn view(&self) -> Container {
        Container::new(
            Container::new(text(self.text.clone()))
                .padding([3, 40])
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .class(style::Container::Notification),
        )
        .padding(padding::bottom(5))
    }
}
