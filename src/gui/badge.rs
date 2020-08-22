use crate::gui::{common::Message, style};
use iced::{Column, Container, Length, Row, Space, Text};

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

    pub fn view(self) -> Container<'static, Message> {
        Container::new(
            Row::new()
                .push(Space::new(Length::Units(self.left_margin), Length::Shrink))
                .push(
                    Column::new().push(Space::new(Length::Shrink, Length::Units(3))).push(
                        Container::new(
                            Row::new()
                                .push(Space::new(Length::Units(10), Length::Shrink))
                                .push(Text::new(self.text).size(14))
                                .push(Space::new(Length::Units(10), Length::Shrink)),
                        )
                        .padding(2)
                        .style(style::Container::Badge),
                    ),
                ),
        )
        .center_x()
        .center_y()
    }

    pub fn view_if(self, condition: bool) -> Container<'static, Message> {
        if condition {
            self.view()
        } else {
            Container::new(Space::new(Length::Shrink, Length::Shrink))
        }
    }
}
