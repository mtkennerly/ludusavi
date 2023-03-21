use iced::{widget::tooltip, Alignment, Length};

use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, EditAction, IcedButtonExt, Message, ScrollSubject, UndoSubject},
        icon::Icon,
        shortcuts::TextHistory,
        style,
        widget::{Button, Checkbox, Column, Container, Row, Space, Text, TextInput, Tooltip, Undoable},
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
                                            .push(Icon::ArrowUpward.as_button_small().on_press_if(
                                                || i > 0,
                                                || Message::EditedCustomGame(EditAction::move_up(i)),
                                            ))
                                            .push(Icon::ArrowDownward.as_button_small().on_press_if(
                                                || i < self.entries.len() - 1,
                                                || Message::EditedCustomGame(EditAction::move_down(i)),
                                            )),
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
                                            Button::new(Icon::Refresh.as_text())
                                                .on_press_if(
                                                    || !operating,
                                                    || Message::BackupStart {
                                                        games: Some(vec![config.custom_games[i].name.clone()]),
                                                        preview: true,
                                                    },
                                                )
                                                .style(style::Button::Primary),
                                            translator.preview_button_in_custom_mode(),
                                            tooltip::Position::Top,
                                        )
                                        .size(16)
                                        .gap(5)
                                        .style(style::Container::Tooltip),
                                    )
                                    .push(
                                        Button::new(Icon::Delete.as_text())
                                            .on_press(Message::EditedCustomGame(EditAction::Remove(i)))
                                            .style(style::Button::Negative),
                                    ),
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
                                                        .push(Icon::ArrowUpward.as_button_small().on_press_if(
                                                            || ii > 0,
                                                            || {
                                                                Message::EditedCustomGameFile(
                                                                    i,
                                                                    EditAction::move_up(ii),
                                                                )
                                                            },
                                                        ))
                                                        .push(Icon::ArrowDownward.as_button_small().on_press_if(
                                                            || ii < x.files.len() - 1,
                                                            || {
                                                                Message::EditedCustomGameFile(
                                                                    i,
                                                                    EditAction::move_down(ii),
                                                                )
                                                            },
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
                                                        .push(
                                                            Button::new(Icon::FolderOpen.as_text())
                                                                .on_press(Message::BrowseDir(
                                                                    BrowseSubject::CustomGameFile(i, ii),
                                                                ))
                                                                .style(style::Button::Primary),
                                                        )
                                                        .push(
                                                            Button::new(Icon::RemoveCircle.as_text())
                                                                .on_press(Message::EditedCustomGameFile(
                                                                    i,
                                                                    EditAction::Remove(ii),
                                                                ))
                                                                .style(style::Button::Negative),
                                                        ),
                                                )
                                            })
                                            .push(
                                                Button::new(Icon::AddCircle.as_text())
                                                    .on_press(Message::EditedCustomGameFile(i, EditAction::Add))
                                                    .style(style::Button::Primary),
                                            ),
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
                                                        .push(Icon::ArrowUpward.as_button_small().on_press_if(
                                                            || ii > 0,
                                                            || {
                                                                Message::EditedCustomGameRegistry(
                                                                    i,
                                                                    EditAction::move_up(ii),
                                                                )
                                                            },
                                                        ))
                                                        .push(Icon::ArrowDownward.as_button_small().on_press_if(
                                                            || ii < x.registry.len() - 1,
                                                            || {
                                                                Message::EditedCustomGameRegistry(
                                                                    i,
                                                                    EditAction::move_down(ii),
                                                                )
                                                            },
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
                                                        .push(
                                                            Button::new(Icon::RemoveCircle.as_text())
                                                                .on_press(Message::EditedCustomGameRegistry(
                                                                    i,
                                                                    EditAction::Remove(ii),
                                                                ))
                                                                .style(style::Button::Negative),
                                                        ),
                                                )
                                            })
                                            .push(
                                                Button::new(Icon::AddCircle.as_text())
                                                    .on_press(Message::EditedCustomGameRegistry(i, EditAction::Add))
                                                    .style(style::Button::Primary),
                                            ),
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
