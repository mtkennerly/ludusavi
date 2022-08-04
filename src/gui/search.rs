use crate::{
    config::{Sort, SortKey},
    gui::common::{Message, Screen},
    lang::Translator,
    shortcuts::TextHistory,
};

use iced::{
    pick_list::{self, PickList},
    text_input, Alignment, Checkbox, Container, Length, Row, Space, Text, TextInput,
};

#[derive(Default)]
pub struct SearchComponent {
    pub show: bool,
    pub game_name: String,
    pub game_name_input: text_input::State,
    pub game_name_history: TextHistory,
    pub sort_key_state: pick_list::State<SortKey>,
}

impl SearchComponent {
    pub fn view(&mut self, screen: Screen, translator: &Translator, sort: &Sort) -> Container<Message> {
        if !self.show {
            return Container::new(Space::new(Length::Shrink, Length::Shrink));
        }
        Container::new(
            Row::new()
                .padding([0, 20, 20, 20])
                .spacing(20)
                .align_items(Alignment::Center)
                .push(Text::new(translator.search_label()))
                .push(
                    TextInput::new(
                        &mut self.game_name_input,
                        &translator.search_game_name_placeholder(),
                        &self.game_name,
                        move |value| Message::EditedSearchGameName { screen, value },
                    )
                    .padding(5),
                )
                .push(Text::new(translator.sort_label()))
                .push(PickList::new(
                    &mut self.sort_key_state,
                    SortKey::ALL,
                    Some(sort.key),
                    move |value| Message::EditedSortKey { screen, value },
                ))
                .push(Checkbox::new(sort.reversed, translator.sort_reversed(), move |value| {
                    Message::EditedSortReversed { screen, value }
                })),
        )
    }
}
