use crate::{
    config::Config,
    gui::{
        common::{EditAction, Message},
        custom_games_editor::CustomGamesEditor,
        custom_games_editor::{CustomGamesEditorEntry, CustomGamesEditorEntryRow},
        style,
    },
    lang::Translator,
};

use iced::{
    alignment::Horizontal as HorizontalAlignment, button, Alignment, Button, Column, Container, Length, Row, Text,
};

#[derive(Default)]
pub struct CustomGamesScreenComponent {
    add_game_button: button::State,
    select_all_button: button::State,
    pub games_editor: CustomGamesEditor,
}

impl CustomGamesScreenComponent {
    pub fn new(config: &Config) -> Self {
        let mut games_editor = CustomGamesEditor::default();
        for custom_game in &config.custom_games {
            let mut row = CustomGamesEditorEntry::new(&custom_game.name.to_string());
            for file in &custom_game.files {
                row.files.push(CustomGamesEditorEntryRow::new(file))
            }
            for key in &custom_game.registry {
                row.registry.push(CustomGamesEditorEntryRow::new(key))
            }
            games_editor.entries.push(row);
        }

        Self {
            games_editor,
            ..Default::default()
        }
    }

    pub fn view(&mut self, config: &Config, translator: &Translator) -> Container<Message> {
        Container::new(
            Column::new()
                .spacing(20)
                .align_items(Alignment::Center)
                .push(
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(
                            Button::new(
                                &mut self.add_game_button,
                                Text::new(translator.add_game_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::EditedCustomGame(EditAction::Add))
                            .width(Length::Units(125))
                            .style(style::Button::Primary(config.theme)),
                        )
                        .push({
                            Button::new(
                                &mut self.select_all_button,
                                Text::new(if config.are_all_custom_games_enabled() {
                                    translator.disable_all_button()
                                } else {
                                    translator.enable_all_button()
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(if config.are_all_custom_games_enabled() {
                                Message::DeselectAllGames
                            } else {
                                Message::SelectAllGames
                            })
                            .width(Length::Units(125))
                            .style(style::Button::Primary(config.theme))
                        }),
                )
                .push(self.games_editor.view(config, translator)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
