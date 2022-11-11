use crate::{
    config::Config,
    gui::{
        common::{EditAction, Message},
        custom_games_editor::{CustomGamesEditor, CustomGamesEditorEntry, CustomGamesEditorEntryRow},
        style,
    },
    lang::Translator,
};

use crate::gui::widget::{Button, Column, Container, Row, Text};
use iced::{alignment::Horizontal as HorizontalAlignment, Alignment, Length};

#[derive(Default)]
pub struct CustomGamesScreenComponent {
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

        Self { games_editor }
    }

    pub fn view(&self, config: &Config, translator: &Translator, operating: bool) -> Container {
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
                                Text::new(translator.add_game_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::EditedCustomGame(EditAction::Add))
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push({
                            Button::new(
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
                            .style(style::Button::Primary)
                        }),
                )
                .push(self.games_editor.view(config, translator, operating)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
