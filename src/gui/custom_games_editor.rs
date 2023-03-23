use iced::{widget::tooltip, Alignment, Length};

use crate::{
    config::Config,
    gui::{
        button,
        common::{BrowseSubject, EditAction, Message, ScrollSubject, UndoSubject},
        shortcuts::TextHistory,
        style,
        widget::{Checkbox, Column, Container, Row, Space, Text, TextInput, Tooltip, Undoable},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct CustomGamesEditorEntryRow {
    pub text_history: TextHistory,
}

impl CustomGamesEditorEntryRow {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
        }
    }
}

#[derive(Default)]
pub struct CustomGamesEditorEntry {
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
    pub entries: Vec<CustomGamesEditorEntry>,
}

impl CustomGamesEditor {
    pub fn view(&self, config: &Config, translator: &Translator, operating: bool) -> Container {
        if config.custom_games.is_empty() {
            return Container::new(Space::new(Length::Shrink, Length::Shrink));
        }

        let content = self.entries.iter().enumerate().fold(
            Column::new().width(Length::Fill).padding([0, 15, 5, 15]).spacing(10),
            |parent, (i, x)| {
                parent.push(
                    Container::new(
                        Column::new()
                            .padding(5)
                            .spacing(5)
                            .push(
                                Row::new()
                                    .spacing(20)
                                    .align_items(iced::Alignment::Center)
                                    .push(
                                        Row::new()
                                            .width(110)
                                            .spacing(20)
                                            .align_items(Alignment::Center)
                                            .push(
                                                Checkbox::new("", config.is_custom_game_enabled(i), move |enabled| {
                                                    Message::ToggleCustomGameEnabled { index: i, enabled }
                                                })
                                                .spacing(0)
                                                .style(style::Checkbox),
                                            )
                                            .push(button::move_up(Message::EditedCustomGame, i))
                                            .push(button::move_down(Message::EditedCustomGame, i, self.entries.len())),
                                    )
                                    .push(Undoable::new(
                                        TextInput::new(
                                            &translator.custom_game_name_placeholder(),
                                            &config.custom_games[i].name,
                                            move |v| Message::EditedCustomGame(EditAction::Change(i, v)),
                                        )
                                        .style(style::TextInput)
                                        .width(Length::Fill)
                                        .padding(5),
                                        move |action| Message::UndoRedo(action, UndoSubject::CustomGameName(i)),
                                    ))
                                    .push(
                                        Tooltip::new(
                                            button::refresh(
                                                Message::BackupStart {
                                                    games: Some(vec![config.custom_games[i].name.clone()]),
                                                    preview: true,
                                                },
                                                operating,
                                            ),
                                            translator.preview_button_in_custom_mode(),
                                            tooltip::Position::Top,
                                        )
                                        .size(16)
                                        .gap(5)
                                        .style(style::Container::Tooltip),
                                    )
                                    .push(button::delete(Message::EditedCustomGame, i)),
                            )
                            .push(
                                Row::new()
                                    .push(
                                        Column::new()
                                            .width(130)
                                            .push(Text::new(translator.custom_files_label())),
                                    )
                                    .push(
                                        x.files
                                            .iter()
                                            .enumerate()
                                            .fold(Column::new().spacing(4), |column, (ii, _)| {
                                                column.push(
                                                    Row::new()
                                                        .align_items(Alignment::Center)
                                                        .spacing(20)
                                                        .push(button::move_up_nested(
                                                            Message::EditedCustomGameFile,
                                                            i,
                                                            ii,
                                                        ))
                                                        .push(button::move_down_nested(
                                                            Message::EditedCustomGameFile,
                                                            i,
                                                            ii,
                                                            x.files.len(),
                                                        ))
                                                        .push(Undoable::new(
                                                            TextInput::new(
                                                                "",
                                                                &config.custom_games[i].files[ii],
                                                                move |v| {
                                                                    Message::EditedCustomGameFile(
                                                                        i,
                                                                        EditAction::Change(ii, v),
                                                                    )
                                                                },
                                                            )
                                                            .style(style::TextInput)
                                                            .padding(5),
                                                            move |action| {
                                                                Message::UndoRedo(
                                                                    action,
                                                                    UndoSubject::CustomGameFile(i, ii),
                                                                )
                                                            },
                                                        ))
                                                        .push(button::open_folder(BrowseSubject::CustomGameFile(i, ii)))
                                                        .push(button::remove_nested(
                                                            Message::EditedCustomGameFile,
                                                            i,
                                                            ii,
                                                        )),
                                                )
                                            })
                                            .push(button::add_nested(Message::EditedCustomGameFile, i)),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .push(
                                        Column::new()
                                            .width(130)
                                            .push(Text::new(translator.custom_registry_label())),
                                    )
                                    .push(
                                        x.registry
                                            .iter()
                                            .enumerate()
                                            .fold(Column::new().spacing(4), |column, (ii, _)| {
                                                column.push(
                                                    Row::new()
                                                        .spacing(20)
                                                        .align_items(Alignment::Center)
                                                        .push(button::move_up_nested(
                                                            Message::EditedCustomGameRegistry,
                                                            i,
                                                            ii,
                                                        ))
                                                        .push(button::move_down_nested(
                                                            Message::EditedCustomGameRegistry,
                                                            i,
                                                            ii,
                                                            x.registry.len(),
                                                        ))
                                                        .push(Undoable::new(
                                                            TextInput::new(
                                                                "",
                                                                &config.custom_games[i].registry[ii],
                                                                move |v| {
                                                                    Message::EditedCustomGameRegistry(
                                                                        i,
                                                                        EditAction::Change(ii, v),
                                                                    )
                                                                },
                                                            )
                                                            .style(style::TextInput)
                                                            .padding(5),
                                                            move |action| {
                                                                Message::UndoRedo(
                                                                    action,
                                                                    UndoSubject::CustomGameRegistry(i, ii),
                                                                )
                                                            },
                                                        ))
                                                        .push(button::remove_nested(
                                                            Message::EditedCustomGameRegistry,
                                                            i,
                                                            ii,
                                                        )),
                                                )
                                            })
                                            .push(button::add_nested(Message::EditedCustomGameRegistry, i)),
                                    ),
                            ),
                    )
                    .style(style::Container::GameListEntry),
                )
            },
        );

        Container::new(ScrollSubject::CustomGames.into_widget(content))
    }
}
