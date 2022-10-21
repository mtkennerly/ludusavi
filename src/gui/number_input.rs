use crate::{
    config::Theme,
    gui::{common::Message, icon::Icon, style},
};
use iced::{
    button::{self, Button},
    Alignment, Container, Length, Row, Text,
};
use std::ops::RangeInclusive;

use super::common::IcedButtonExt;

#[derive(Clone, Debug, Default)]
pub struct NumberInput {
    up_state: button::State,
    down_state: button::State,
}

impl NumberInput {
    pub fn view(
        &mut self,
        value: i32,
        label: &str,
        range: RangeInclusive<i32>,
        change: fn(i32) -> Message,
        theme: Theme,
    ) -> Container<Message> {
        Container::new(
            Row::new()
                .spacing(5)
                .align_items(Alignment::Center)
                .push(Text::new(label))
                .push(Text::new(value.to_string()))
                .push({
                    Button::new(&mut self.down_state, Icon::Remove.as_text().width(Length::Shrink))
                        .on_press_if(|| &value > range.start(), || (change)(value - 1))
                        .style(style::Button::Negative(theme))
                })
                .push({
                    Button::new(&mut self.up_state, Icon::Add.as_text().width(Length::Shrink))
                        .on_press_if(|| &value < range.end(), || (change)(value + 1))
                        .style(style::Button::Primary(theme))
                }),
        )
    }
}
