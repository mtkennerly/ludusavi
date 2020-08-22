use crate::{
    config::Config,
    gui::{common::Message, style},
    lang::Translator,
};

use iced::{scrollable, Align, Checkbox, Column, Container, Length, Row, Scrollable};

#[derive(Default)]
pub struct OtherScreenComponent {
    scroll: scrollable::State,
}

impl OtherScreenComponent {
    pub fn view(&mut self, config: &Config, translator: &Translator) -> Container<Message> {
        Container::new(
            Scrollable::new(&mut self.scroll)
                .width(Length::Fill)
                .padding(10)
                .style(style::Scrollable)
                .push(
                    Column::new()
                        .padding(5)
                        .push(
                            Row::new()
                                .padding(20)
                                .spacing(20)
                                .align_items(Align::Center)
                                .push(Checkbox::new(
                                    config.backup.filter.exclude_other_os_data,
                                    translator.explanation_for_exclude_other_os_data(),
                                    Message::EditedExcludeOtherOsData,
                                )),
                        )
                        .push(
                            Row::new()
                                .padding(20)
                                .spacing(20)
                                .align_items(Align::Center)
                                .push(Checkbox::new(
                                    config.backup.filter.exclude_store_screenshots,
                                    translator.explanation_for_exclude_store_screenshots(),
                                    Message::EditedExcludeStoreScreenshots,
                                )),
                        ),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
