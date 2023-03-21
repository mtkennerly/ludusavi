use std::ops::RangeInclusive;

use iced::{Alignment, Length};

use crate::gui::{
    common::{IcedButtonExt, Message},
    icon::Icon,
    style,
    widget::{Button, Container, Row, Text},
};

#[derive(Clone, Debug, Default)]
pub struct NumberInput {}

impl NumberInput {
    pub fn view(&self, value: i32, label: String, range: RangeInclusive<i32>, change: fn(i32) -> Message) -> Container {
        Container::new(
            Row::new()
                .spacing(5)
                .align_items(Alignment::Center)
                .push(Text::new(label))
                .push(Text::new(value.to_string()))
                .push({
                    Button::new(Icon::Remove.as_text().width(Length::Shrink))
                        .on_press_if(|| &value > range.start(), || (change)(value - 1))
                        .style(style::Button::Negative)
                })
                .push({
                    Button::new(Icon::Add.as_text().width(Length::Shrink))
                        .on_press_if(|| &value < range.end(), || (change)(value + 1))
                        .style(style::Button::Primary)
                }),
        )
    }
}
