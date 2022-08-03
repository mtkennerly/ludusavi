use crate::{
    config::Config,
    gui::{
        common::{Message, OngoingOperation},
        ignored_items_editor::IgnoredItemsEditor,
        style,
    },
    lang::Translator,
};

use iced::{scrollable, Checkbox, Column, Container, Length, Padding, Scrollable, Text};

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
                .style(style::Scrollable)
                .padding(Padding {
                    top: 0,
                    bottom: 5,
                    left: 15,
                    right: 15,
                })
                .push(
                    Column::new()
                        .spacing(20)
                        .push(Checkbox::new(
                            config.backup.filter.exclude_other_os_data,
                            translator.explanation_for_exclude_other_os_data(),
                            Message::EditedExcludeOtherOsData,
                        ))
                        .push(Checkbox::new(
                            config.backup.filter.exclude_store_screenshots,
                            translator.explanation_for_exclude_store_screenshots(),
                            Message::EditedExcludeStoreScreenshots,
                        ))
                        .push(
                            Column::new().push(Text::new(translator.ignored_items_label())).push(
                                self.ignored_items_editor
                                    .view(config, translator, operation)
                                    .padding(Padding {
                                        top: 10,
                                        bottom: 0,
                                        left: 0,
                                        right: 0,
                                    }),
                            ),
                        ),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(5)
        .center_x()
    }
}
