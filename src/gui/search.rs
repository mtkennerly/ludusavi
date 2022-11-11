use crate::{
    config::{Sort, SortKey},
    gui::{
        common::{Message, Screen, UndoSubject},
        style,
    },
    lang::Translator,
    shortcuts::TextHistory,
};

use crate::gui::widget::{Checkbox, Container, PickList, Row, Text, TextInput, Undoable};
use iced::Alignment;

#[derive(Default)]
pub struct SearchComponent {
    pub show: bool,
    pub game_name: String,
    pub game_name_history: TextHistory,
}

impl SearchComponent {
    pub fn view(&self, screen: Screen, translator: &Translator, sort: &Sort) -> Option<Container> {
        if !self.show {
            return None;
        }
        Some(Container::new(
            Row::new()
                .padding([0, 20, 20, 20])
                .spacing(20)
                .align_items(Alignment::Center)
                .push(Text::new(translator.search_label()))
                .push(Undoable::new(
                    TextInput::new(
                        &translator.search_game_name_placeholder(),
                        &self.game_name,
                        move |value| Message::EditedSearchGameName { screen, value },
                    )
                    .style(style::TextInput)
                    .padding(5),
                    move |action| {
                        Message::UndoRedo(
                            action,
                            match screen {
                                Screen::Restore => UndoSubject::RestoreSearchGameName,
                                _ => UndoSubject::BackupSearchGameName,
                            },
                        )
                    },
                ))
                .push(Text::new(translator.sort_label()))
                .push(
                    PickList::new(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                        screen,
                        value,
                    })
                    .style(style::PickList::Primary),
                )
                .push(
                    Checkbox::new(sort.reversed, translator.sort_reversed(), move |value| {
                        Message::EditedSortReversed { screen, value }
                    })
                    .style(style::Checkbox),
                ),
        ))
    }
}
