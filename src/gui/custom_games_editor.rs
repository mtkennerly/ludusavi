use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, EditAction, IcedButtonExt, Message},
        icon::Icon,
        style,
    },
    lang::Translator,
    shortcuts::TextHistory,
};

use iced::{
    button, scrollable, text_input, tooltip, Button, Checkbox, Column, Container, Length, Row, Scrollable, Space, Text,
    TextInput, Tooltip,
};

#[derive(Default)]
pub struct CustomGamesEditorEntryRow {
    button_state: button::State,
    pub text_state: text_input::State,
    pub text_history: TextHistory,
    browse_button_state: button::State,
}

impl CustomGamesEditorEntryRow {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct CustomGamesEditorEntry {
    remove_button_state: button::State,
    preview_button_state: button::State,
    add_file_button_state: button::State,
    add_registry_button_state: button::State,
    pub text_state: text_input::State,
    pub text_history: TextHistory,
    pub files: Vec<CustomGamesEditorEntryRow>,
    pub registry: Vec<CustomGamesEditorEntryRow>,
}

impl CustomGamesEditorEntry {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct CustomGamesEditor {
    pub scroll: scrollable::State,
    pub entries: Vec<CustomGamesEditorEntry>,
}

impl CustomGamesEditor {
    pub fn view(&mut self, config: &Config, translator: &Translator, operating: bool) -> Container<Message> {
        if config.custom_games.is_empty() {
            return Container::new(Space::new(Length::Shrink, Length::Shrink));
        }

        Container::new({
            self.entries.iter_mut().enumerate().fold(
                Scrollable::new(&mut self.scroll)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding([0, 15, 5, 15])
                    .spacing(10)
                    .style(style::Scrollable(config.theme)),
                |parent: Scrollable<'_, Message>, (i, x)| {
                    parent.push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(5)
                                .push(
                                    Row::new()
                                        .spacing(20)
                                        .push(
                                            Column::new().width(Length::Units(80)).push(
                                                Checkbox::new(config.is_custom_game_enabled(i), "", move |enabled| {
                                                    Message::ToggleCustomGameEnabled { index: i, enabled }
                                                })
                                                .style(style::Checkbox(config.theme)),
                                            ),
                                        )
                                        .push(
                                            TextInput::new(
                                                &mut x.text_state,
                                                &translator.custom_game_name_placeholder(),
                                                &config.custom_games[i].name,
                                                move |v| Message::EditedCustomGame(EditAction::Change(i, v)),
                                            )
                                            .style(style::TextInput(config.theme))
                                            .width(Length::Fill)
                                            .padding(5),
                                        )
                                        .push(
                                            Tooltip::new(
                                                Button::new(&mut x.preview_button_state, Icon::Refresh.as_text())
                                                    .on_press_if(
                                                        || !operating,
                                                        || Message::BackupStart {
                                                            games: Some(vec![config.custom_games[i].name.clone()]),
                                                            preview: true,
                                                        },
                                                    )
                                                    .style(style::Button::Primary(config.theme)),
                                                translator.preview_button_in_custom_mode(),
                                                tooltip::Position::Top,
                                            )
                                            .size(16)
                                            .gap(5)
                                            .style(style::Container::Tooltip(config.theme)),
                                        )
                                        .push(
                                            Button::new(&mut x.remove_button_state, Icon::Delete.as_text())
                                                .on_press(Message::EditedCustomGame(EditAction::Remove(i)))
                                                .style(style::Button::Negative(config.theme)),
                                        ),
                                )
                                .push(
                                    Row::new()
                                        .push(
                                            Column::new()
                                                .width(Length::Units(100))
                                                .push(Text::new(translator.custom_files_label())),
                                        )
                                        .push(
                                            x.files
                                                .iter_mut()
                                                .enumerate()
                                                .fold(Column::new().spacing(4), |column, (ii, xx)| {
                                                    column.push(
                                                        Row::new()
                                                            .spacing(20)
                                                            .push(
                                                                TextInput::new(
                                                                    &mut xx.text_state,
                                                                    "",
                                                                    &config.custom_games[i].files[ii],
                                                                    move |v| {
                                                                        Message::EditedCustomGameFile(
                                                                            i,
                                                                            EditAction::Change(ii, v),
                                                                        )
                                                                    },
                                                                )
                                                                .style(style::TextInput(config.theme))
                                                                .padding(5),
                                                            )
                                                            .push(
                                                                Button::new(
                                                                    &mut xx.browse_button_state,
                                                                    Icon::FolderOpen.as_text(),
                                                                )
                                                                .on_press(Message::BrowseDir(
                                                                    BrowseSubject::CustomGameFile(i, ii),
                                                                ))
                                                                .style(style::Button::Primary(config.theme)),
                                                            )
                                                            .push(
                                                                Button::new(
                                                                    &mut xx.button_state,
                                                                    Icon::RemoveCircle.as_text(),
                                                                )
                                                                .on_press(Message::EditedCustomGameFile(
                                                                    i,
                                                                    EditAction::Remove(ii),
                                                                ))
                                                                .style(style::Button::Negative(config.theme)),
                                                            ),
                                                    )
                                                })
                                                .push(
                                                    Button::new(
                                                        &mut x.add_file_button_state,
                                                        Icon::AddCircle.as_text(),
                                                    )
                                                    .on_press(Message::EditedCustomGameFile(i, EditAction::Add))
                                                    .style(style::Button::Primary(config.theme)),
                                                ),
                                        ),
                                )
                                .push(
                                    Row::new()
                                        .push(
                                            Column::new()
                                                .width(Length::Units(100))
                                                .push(Text::new(translator.custom_registry_label())),
                                        )
                                        .push(
                                            x.registry
                                                .iter_mut()
                                                .enumerate()
                                                .fold(Column::new().spacing(4), |column, (ii, xx)| {
                                                    column.push(
                                                        Row::new()
                                                            .spacing(20)
                                                            .push(
                                                                TextInput::new(
                                                                    &mut xx.text_state,
                                                                    "",
                                                                    &config.custom_games[i].registry[ii],
                                                                    move |v| {
                                                                        Message::EditedCustomGameRegistry(
                                                                            i,
                                                                            EditAction::Change(ii, v),
                                                                        )
                                                                    },
                                                                )
                                                                .style(style::TextInput(config.theme))
                                                                .padding(5),
                                                            )
                                                            .push(
                                                                Button::new(
                                                                    &mut xx.button_state,
                                                                    Icon::RemoveCircle.as_text(),
                                                                )
                                                                .on_press(Message::EditedCustomGameRegistry(
                                                                    i,
                                                                    EditAction::Remove(ii),
                                                                ))
                                                                .style(style::Button::Negative(config.theme)),
                                                            ),
                                                    )
                                                })
                                                .push(
                                                    Button::new(
                                                        &mut x.add_registry_button_state,
                                                        Icon::AddCircle.as_text(),
                                                    )
                                                    .on_press(Message::EditedCustomGameRegistry(i, EditAction::Add))
                                                    .style(style::Button::Primary(config.theme)),
                                                ),
                                        ),
                                ),
                        )
                        .style(style::Container::GameListEntry(config.theme)),
                    )
                },
            )
        })
    }
}
