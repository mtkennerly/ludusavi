use crate::{
    config::Config,
    gui::{
        common::{Message, OngoingOperation},
        ignored_items_editor::IgnoredItemsEditor,
        style,
    },
    lang::{Language, Translator},
};

use iced::{pick_list, scrollable, Checkbox, Column, Container, Length, PickList, Row, Scrollable, Text};

#[derive(Default)]
pub struct OtherScreenComponent {
    scroll: scrollable::State,
    pub ignored_items_editor: IgnoredItemsEditor,
    language_selector: pick_list::State<Language>,
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
                .padding([0, 15, 5, 15])
                .push(
                    Column::new()
                        .spacing(20)
                        .push(
                            Row::new()
                                .align_items(iced::Alignment::Center)
                                .spacing(20)
                                .push(Text::new(translator.field_language()))
                                .push(
                                    PickList::new(
                                        &mut self.language_selector,
                                        Language::ALL,
                                        Some(config.language),
                                        Message::SelectedLanguage,
                                    )
                                    .style(style::PickList::Primary),
                                ),
                        )
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
                                    .padding([10, 0, 0, 0]),
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
