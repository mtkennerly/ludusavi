use crate::{
    config::Theme,
    gui::{common::Message, icon::Icon, style},
};
use iced::{
    button::{self, Button},
    Alignment, Container, Length, Row, Text,
};
use std::ops::RangeInclusive;

#[derive(Clone, Debug, Default)]
pub struct NumberInput {
    up_state: button::State,
    down_state: button::State,
}

impl NumberInput {
    pub fn view(
        &mut self,
        value: u8,
        label: &str,
        range: RangeInclusive<u8>,
        change: fn(u8) -> Message,
        theme: Theme,
    ) -> Container<Message> {
        Container::new(
            Row::new()
                .spacing(5)
                .align_items(Alignment::Center)
                .push(Text::new(label))
                .push(Text::new(value.to_string()))
                .push({
                    let button = Button::new(&mut self.down_state, Icon::Remove.as_text().width(Length::Shrink));
                    if &value > range.start() {
                        button
                            .on_press((change)(value - 1))
                            .style(style::Button::Negative(theme))
                    } else {
                        button.style(style::Button::Disabled(theme))
                    }
                })
                .push({
                    let button = Button::new(&mut self.up_state, Icon::Add.as_text().width(Length::Shrink));
                    if &value < range.end() {
                        button
                            .on_press((change)(value + 1))
                            .style(style::Button::Primary(theme))
                    } else {
                        button.style(style::Button::Disabled(theme))
                    }
                }),
        )
    }
}
