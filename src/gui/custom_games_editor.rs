use iced::{widget::tooltip, Alignment, Length};

use crate::{
    config::Config,
    gui::{
        button,
        common::{BrowseSubject, Message, ScrollSubject, TextHistories, UndoSubject},
        style,
        widget::{Checkbox, Column, Container, Row, Space, Text, Tooltip},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct CustomGamesEditor {}

impl CustomGamesEditor {
    pub fn view<'a>(
        config: &Config,
        translator: &Translator,
        operating: bool,
        histories: &TextHistories,
    ) -> Container<'a> {
        if config.custom_games.is_empty() {
            return Container::new(Space::new(Length::Shrink, Length::Shrink));
        }

        let content = config.custom_games.iter().enumerate().fold(
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
                                            .push(button::move_down(
                                                Message::EditedCustomGame,
                                                i,
                                                config.custom_games.len(),
                                            )),
                                    )
                                    .push(histories.input(UndoSubject::CustomGameName(i)))
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
                                                        .push(histories.input(UndoSubject::CustomGameFile(i, ii)))
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
                                                        .push(histories.input(UndoSubject::CustomGameRegistry(i, ii)))
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
