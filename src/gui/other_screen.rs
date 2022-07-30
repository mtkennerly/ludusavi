use crate::{
    config::Config,
    gui::{
        common::{Message, OngoingOperation},
        ignored_items_editor::IgnoredItemsEditor,
        style,
    },
    lang::Translator,
};

use iced::{scrollable, Alignment, Checkbox, Column, Container, Length, Padding, Row, Scrollable, Text};

#[derive(Default)]
pub struct OtherScreenComponent {
    scroll: scrollable::State,
    pub ignored_items_editor: IgnoredItemsEditor,
}

impl OtherScreenComponent {
    pub fn new(config: &Config) -> Self {
        Self {
            ignored_items_editor: IgnoredItemsEditor::new(config),
            ..Default::default()
        }
    }

    pub fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Scrollable::new(&mut self.scroll)
                .width(Length::Fill)
                .padding(Padding {
                    top: 0,
                    bottom: 5,
                    left: 5,
                    right: 5,
                })
                .style(style::Scrollable)
                .push(
                    Column::new()
                        .push(
                            Row::new()
                                .padding(Padding {
                                    top: 0,
                                    bottom: 0,
                                    left: 20,
                                    right: 20,
                                })
                                .spacing(20)
                                .align_items(Alignment::Center)
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
                                .align_items(Alignment::Center)
                                .push(Checkbox::new(
                                    config.backup.filter.exclude_store_screenshots,
                                    translator.explanation_for_exclude_store_screenshots(),
                                    Message::EditedExcludeStoreScreenshots,
                                )),
                        )
                        .push(
                            Row::new()
                                .padding(Padding {
                                    top: 0,
                                    bottom: 0,
                                    left: 20,
                                    right: 20,
                                })
                                .push(Text::new(translator.ignored_items_label())),
                        )
                        .push(self.ignored_items_editor.view(config, translator, operation)),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
