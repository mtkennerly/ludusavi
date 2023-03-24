use iced::Length;

use crate::{
    config::Config,
    gui::{
        button,
        common::{BrowseSubject, Message, TextHistories, UndoSubject},
        style,
        widget::{Column, Container, PickList, Row, Text},
    },
    lang::Translator,
    manifest::Store,
};

#[derive(Default)]
pub struct RootEditor {}

impl RootEditor {
    pub fn view<'a>(config: &Config, translator: &Translator, histories: &TextHistories) -> Container<'a> {
        let mut content = Column::new().width(Length::Fill).spacing(5);
        if config.roots.is_empty() {
            content = content.push(Text::new(translator.no_roots_are_configured()));
        } else {
            content = config.roots.iter().enumerate().fold(content, |parent, (i, root)| {
                parent.push(
                    Row::new()
                        .spacing(20)
                        .push(button::move_up(Message::EditedRoot, i))
                        .push(button::move_down(Message::EditedRoot, i, config.roots.len()))
                        .push(histories.input(UndoSubject::Root(i)))
                        .push(
                            PickList::new(Store::ALL, Some(root.store), move |v| Message::SelectedRootStore(i, v))
                                .style(style::PickList::Primary),
                        )
                        .push(button::open_folder(BrowseSubject::Root(i)))
                        .push(button::remove(Message::EditedRoot, i)),
                )
            });
        };

        content = content.push(
            Row::new()
                .spacing(20)
                .push(button::add(Message::EditedRoot))
                .push(button::refresh(Message::FindRoots, false)),
        );

        Container::new(content)
    }
}
