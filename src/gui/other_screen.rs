use crate::{
    cache::Cache,
    config::{Config, Theme},
    gui::{
        common::{IcedExtension, Message},
        icon::Icon,
        ignored_items_editor::IgnoredItemsEditor,
        redirect_editor::{RedirectEditor, RedirectEditorRow},
        style,
    },
    lang::{Language, Translator},
};

use iced::{
    button, pick_list, scrollable, Button, Checkbox, Column, Container, Length, PickList, Row, Scrollable, Text,
};

use super::common::IcedButtonExt;

#[derive(Default)]
pub struct OtherScreenComponent {
    scroll: scrollable::State,
    pub ignored_items_editor: IgnoredItemsEditor,
    language_selector: pick_list::State<Language>,
    theme_selector: pick_list::State<Theme>,
    pub redirect_editor: RedirectEditor,
    refresh_manifest_button: button::State,
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

    pub fn view(
        &mut self,
        updating_manifest: bool,
        config: &Config,
        cache: &Cache,
        translator: &Translator,
    ) -> Container<Message> {
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
                            Column::new()
                                .spacing(5)
                                .push(
                                    Row::new()
                                        .align_items(iced::Alignment::Center)
                                        .push(Text::new(translator.manifest_label()).width(Length::Units(100)))
                                        .push(
                                            Button::new(&mut self.refresh_manifest_button, Icon::Refresh.as_text())
                                                .on_press_if(|| !updating_manifest, || Message::UpdateManifest)
                                                .style(style::Button::Primary(config.theme)),
                                        ),
                                )
                                .push_some(|| {
                                    let cached = cache.manifests.get(&config.manifest.url)?;
                                    let checked = match cached.checked {
                                        Some(x) => chrono::DateTime::<chrono::Local>::from(x)
                                            .format("%Y-%m-%dT%H:%M:%S")
                                            .to_string(),
                                        None => "?".to_string(),
                                    };
                                    let updated = match cached.updated {
                                        Some(x) => chrono::DateTime::<chrono::Local>::from(x)
                                            .format("%Y-%m-%dT%H:%M:%S")
                                            .to_string(),
                                        None => "?".to_string(),
                                    };
                                    Some(
                                        Container::new(
                                            Column::new()
                                                .padding(5)
                                                .spacing(4)
                                                .push(
                                                    Row::new()
                                                        .align_items(iced::Alignment::Center)
                                                        .push(
                                                            Container::new(Text::new(translator.checked_label()))
                                                                .width(Length::Units(100)),
                                                        )
                                                        .push(Container::new(Text::new(checked))),
                                                )
                                                .push(
                                                    Row::new()
                                                        .align_items(iced::Alignment::Center)
                                                        .push(
                                                            Container::new(Text::new(translator.updated_label()))
                                                                .width(Length::Units(100)),
                                                        )
                                                        .push(Container::new(Text::new(updated))),
                                                ),
                                        )
                                        .style(style::Container::GameListEntry(config.theme)),
                                    )
                                }),
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
