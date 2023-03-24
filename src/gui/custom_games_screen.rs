use iced::{Alignment, Length};

use crate::{
    config::Config,
    gui::{
        button,
        common::TextHistories,
        custom_games_editor::CustomGamesEditor,
        widget::{Column, Container, Row},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct CustomGamesScreenComponent {}

impl CustomGamesScreenComponent {
    pub fn view<'a>(
        config: &Config,
        translator: &Translator,
        operating: bool,
        histories: &TextHistories,
    ) -> Container<'a> {
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
                .push(CustomGamesEditor::view(config, translator, operating, histories)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
