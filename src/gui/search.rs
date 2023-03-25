use iced::Alignment;

use crate::{
    gui::{
        common::{Message, Screen, TextHistories, UndoSubject},
        style,
        widget::{Checkbox, Container, PickList, Row, Text},
    },
    lang::TRANSLATOR,
    resource::config::{Sort, SortKey},
};

#[derive(Default)]
pub struct SearchComponent {
    pub show: bool,
    pub game_name: String,
}

impl SearchComponent {
    pub fn view(&self, screen: Screen, sort: &Sort, histories: &TextHistories) -> Option<Container> {
        if !self.show {
            return None;
        }
        Some(Container::new(
            Row::new()
                .padding([0, 20, 20, 20])
                .spacing(20)
                .align_items(Alignment::Center)
                .push(Text::new(TRANSLATOR.search_label()))
                .push(histories.input(match screen {
                    Screen::Restore => UndoSubject::RestoreSearchGameName,
                    _ => UndoSubject::BackupSearchGameName,
                }))
                .push(Text::new(TRANSLATOR.sort_label()))
                .push(
                    PickList::new(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                        screen,
                        value,
                    })
                    .style(style::PickList::Primary),
                )
                .push(
                    Checkbox::new(TRANSLATOR.sort_reversed(), sort.reversed, move |value| {
                        Message::EditedSortReversed { screen, value }
                    })
                    .style(style::Checkbox),
                ),
        ))
    }
}
