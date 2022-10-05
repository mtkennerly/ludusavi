use crate::{
    config::{Config, Theme},
    gui::{
        common::Message,
        ignored_items_editor::IgnoredItemsEditor,
        redirect_editor::{RedirectEditor, RedirectEditorRow},
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
    theme_selector: pick_list::State<Theme>,
    pub redirect_editor: RedirectEditor,
}

impl OtherScreenComponent {
    pub fn new(config: &Config) -> Self {
        let mut redirect_editor = RedirectEditor::default();
        for redirect in &config.get_redirects() {
            redirect_editor
                .rows
                .push(RedirectEditorRow::new(&redirect.source.raw(), &redirect.target.raw()))
        }

        Self {
            ignored_items_editor: IgnoredItemsEditor::new(config),
            redirect_editor,
            ..Default::default()
        }
    }

    pub fn view(&mut self, config: &Config, translator: &Translator) -> Container<Message> {
        Container::new(
            Scrollable::new(&mut self.scroll)
                .width(Length::Fill)
                .style(style::Scrollable(config.theme))
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
                                    .style(style::PickList::Primary(config.theme)),
                                ),
                        )
                        .push(
                            Row::new()
                                .align_items(iced::Alignment::Center)
                                .spacing(20)
                                .push(Text::new(translator.field_theme()))
                                .push(
                                    PickList::new(
                                        &mut self.theme_selector,
                                        Theme::ALL,
                                        Some(config.theme),
                                        Message::SelectedTheme,
                                    )
                                    .style(style::PickList::Primary(config.theme)),
                                ),
                        )
                        .push(
                            Checkbox::new(
                                config.backup.filter.exclude_store_screenshots,
                                translator.explanation_for_exclude_store_screenshots(),
                                Message::EditedExcludeStoreScreenshots,
                            )
                            .style(style::Checkbox(config.theme)),
                        )
                        .push(
                            Column::new().push(Text::new(translator.ignored_items_label())).push(
                                self.ignored_items_editor
                                    .view(config, translator)
                                    .padding([10, 0, 0, 0]),
                            ),
                        )
                        .push(
                            Column::new()
                                .push(Text::new(translator.redirects_label()))
                                .push(self.redirect_editor.view(config, translator).padding([10, 0, 0, 0])),
                        ),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(5)
        .center_x()
    }
}
