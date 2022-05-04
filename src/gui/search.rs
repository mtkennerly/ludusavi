use crate::{
    gui::common::{Message, Screen},
    lang::Translator,
    shortcuts::TextHistory,
};

use iced::{text_input, Alignment, Container, Length, Row, Space, Text, TextInput};

#[derive(Default)]
pub struct SearchComponent {
    pub show: bool,
    pub game_name: String,
    pub game_name_input: text_input::State,
    pub game_name_history: TextHistory,
}

impl SearchComponent {
    pub fn view(&mut self, screen: Screen, translator: &Translator) -> Container<Message> {
        if !self.show {
            return Container::new(Space::new(Length::Shrink, Length::Shrink));
        }
        Container::new(
            Row::new()
                .spacing(20)
                .align_items(Alignment::Center)
                .push(Space::new(Length::Shrink, Length::Shrink))
                .push(Text::new(translator.search_label()))
                .push(
                    TextInput::new(
                        &mut self.game_name_input,
                        &translator.search_game_name_placeholder(),
                        &self.game_name,
                        move |value| Message::EditedSearchGameName {
                            screen: screen.clone(),
                            value,
                        },
                    )
                    .padding(5),
                )
                .push(Space::new(Length::Shrink, Length::Shrink)),
        )
    }
}
