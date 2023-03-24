use iced::Alignment;

use crate::{
    config::{Sort, SortKey},
    gui::{
        common::{Message, Screen, TextHistories, UndoSubject},
        style,
        widget::{Checkbox, Container, PickList, Row, Text},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct SearchComponent {
    pub show: bool,
    pub game_name: String,
}

impl SearchComponent {
    pub fn view(
        &self,
        screen: Screen,
        translator: &Translator,
        sort: &Sort,
        histories: &TextHistories,
    ) -> Option<Container> {
        if !self.show {
            return None;
        }
        Some(Container::new(
            Row::new()
                .padding([0, 20, 20, 20])
                .spacing(20)
                .align_items(Alignment::Center)
                .push(Text::new(translator.search_label()))
                .push(histories.input(match screen {
                    Screen::Restore => UndoSubject::RestoreSearchGameName,
                    _ => UndoSubject::BackupSearchGameName,
                }))
                .push(Text::new(translator.sort_label()))
                .push(
                    PickList::new(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                        screen,
                        value,
                    })
                    .style(style::PickList::Primary),
                )
                .push(
                    Checkbox::new(translator.sort_reversed(), sort.reversed, move |value| {
                        Message::EditedSortReversed { screen, value }
                    })
                    .style(style::Checkbox),
                ),
        ))
    }
}
