use iced::{Alignment, Length};

use crate::{
    config::Config,
    gui::{
        button,
        custom_games_editor::{CustomGamesEditor, CustomGamesEditorEntry, CustomGamesEditorEntryRow},
        widget::{Column, Container, Row},
    },
    lang::Translator,
};

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
                        .push(button::add_game())
                        .push(button::toggle_all_custom_games(config.are_all_custom_games_enabled())),
                )
                .push(self.games_editor.view(config, translator, operating)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
