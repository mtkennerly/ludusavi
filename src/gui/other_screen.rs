use iced::Length;

use crate::{
    cache::Cache,
    config::{Config, Theme},
    gui::{
        common::{IcedButtonExt, IcedExtension, Message, ScrollSubject},
        icon::Icon,
        ignored_items_editor::IgnoredItemsEditor,
        redirect_editor::{RedirectEditor, RedirectEditorRow},
        root_editor::{RootEditor, RootEditorRow},
        style,
        widget::{Button, Checkbox, Column, Container, PickList, Row, Text},
    },
    lang::{Language, Translator},
    prelude::STEAM_DECK,
};

#[derive(Default)]
pub struct OtherScreenComponent {
    pub ignored_items_editor: IgnoredItemsEditor,
    pub root_editor: RootEditor,
    pub redirect_editor: RedirectEditor,
}

impl OtherScreenComponent {
    pub fn new(config: &Config) -> Self {
        let mut root_editor = RootEditor::default();
        for root in &config.roots {
            root_editor.rows.push(RootEditorRow::new(&root.path.raw()))
        }

        let mut redirect_editor = RedirectEditor::default();
        for redirect in &config.get_redirects() {
            redirect_editor
                .rows
                .push(RedirectEditorRow::new(&redirect.source.raw(), &redirect.target.raw()))
        }

        Self {
            ignored_items_editor: IgnoredItemsEditor::new(config),
            root_editor,
            redirect_editor,
        }
    }

    pub fn view(&self, updating_manifest: bool, config: &Config, cache: &Cache, translator: &Translator) -> Container {
        Container::new(
            Column::new()
                .spacing(20)
                .align_items(iced::Alignment::Center)
                .push_if(
                    || *STEAM_DECK,
                    || {
                        Row::new()
                            .padding([0, 20, 0, 20])
                            .spacing(20)
                            .align_items(iced::Alignment::Center)
                            .push(
                                Button::new(
                                    Text::new(translator.exit_button())
                                        .horizontal_alignment(iced::alignment::Horizontal::Center),
                                )
                                .on_press(Message::Exit)
                                .width(125)
                                .style(style::Button::Negative),
                            )
                    },
                )
                .push({
                    let content = Column::new()
                        .spacing(20)
                        .padding([0, 15, 5, 15])
                        .width(Length::Fill)
                        .push(
                            Row::new()
                                .align_items(iced::Alignment::Center)
                                .spacing(20)
                                .push(Text::new(translator.field_language()))
                                .push(
                                    PickList::new(Language::ALL, Some(config.language), Message::SelectedLanguage)
                                        .style(style::PickList::Primary),
                                ),
                        )
                        .push(
                            Row::new()
                                .align_items(iced::Alignment::Center)
                                .spacing(20)
                                .push(Text::new(translator.field_theme()))
                                .push(
                                    PickList::new(Theme::ALL, Some(config.theme), Message::SelectedTheme)
                                        .style(style::PickList::Primary),
                                ),
                        )
                        .push(
                            Checkbox::new(
                                translator.explanation_for_exclude_store_screenshots(),
                                config.backup.filter.exclude_store_screenshots,
                                Message::EditedExcludeStoreScreenshots,
                            )
                            .style(style::Checkbox),
                        )
                        .push(
                            Column::new()
                                .spacing(5)
                                .push(
                                    Row::new()
                                        .align_items(iced::Alignment::Center)
                                        .push(Text::new(translator.manifest_label()).width(100))
                                        .push(
                                            Button::new(Icon::Refresh.as_text())
                                                .on_press_if(|| !updating_manifest, || Message::UpdateManifest)
                                                .style(style::Button::Primary),
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
                                                                .width(100),
                                                        )
                                                        .push(Container::new(Text::new(checked))),
                                                )
                                                .push(
                                                    Row::new()
                                                        .align_items(iced::Alignment::Center)
                                                        .push(
                                                            Container::new(Text::new(translator.updated_label()))
                                                                .width(100),
                                                        )
                                                        .push(Container::new(Text::new(updated))),
                                                ),
                                        )
                                        .style(style::Container::GameListEntry),
                                    )
                                }),
                        )
                        .push(
                            Column::new().spacing(5).push(Text::new(translator.roots_label())).push(
                                Container::new(
                                    Column::new()
                                        .padding(5)
                                        .spacing(4)
                                        .push(self.root_editor.view(config, translator)),
                                )
                                .style(style::Container::GameListEntry),
                            ),
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
                        );
                    ScrollSubject::Other.into_widget(content)
                }),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(5)
        .center_x()
    }
}
