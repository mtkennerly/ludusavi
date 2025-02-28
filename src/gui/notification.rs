use std::time::Instant;

use iced::{alignment, padding, widget::ProgressBar};

use crate::gui::{
    style,
    widget::{text, Container},
};

pub struct Notification {
    text: String,
    created: Instant,
    expires: Option<u64>,
    progress: Option<(f32, f32)>,
}

impl Notification {
    pub fn new(text: String) -> Self {
        Self {
            text,
            created: Instant::now(),
            expires: None,
            progress: None,
        }
    }

    pub fn expires(mut self, expires: u64) -> Self {
        self.expires = Some(expires);
        self
    }

    pub fn progress(mut self, current: f32, max: f32) -> Self {
        self.progress = Some((current, max));
        self
    }

    pub fn expired(&self) -> bool {
        match self.expires {
            None => false,
            Some(expires) => (Instant::now() - self.created).as_secs() > expires,
        }
    }

    pub fn view(&self) -> Container {
        let mut content = Container::new(text(self.text.clone()))
            .padding([3, 40])
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .class(style::Container::Notification);

        if let Some((current, max)) = self.progress {
            content = content.push(ProgressBar::new(0.0..=max, current).height(8));
        }

        Container::new(content).padding(padding::bottom(5))
    }
}
