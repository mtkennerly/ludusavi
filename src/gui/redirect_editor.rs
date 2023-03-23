use iced::Length;

use crate::{
    config::{Config, RedirectKind},
    gui::{
        common::{BrowseSubject, CommonButton, EditAction, Message, RedirectEditActionField, UndoSubject},
        shortcuts::TextHistory,
        style,
        widget::{Column, Container, PickList, Row, TextInput, Undoable},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct RedirectEditorRow {
    pub source_text_history: TextHistory,
    pub target_text_history: TextHistory,
}

impl RedirectEditorRow {
    pub fn new(initial_source: &str, initial_target: &str) -> Self {
        Self {
            source_text_history: TextHistory::new(initial_source, 100),
            target_text_history: TextHistory::new(initial_target, 100),
        }
    }
}

#[derive(Default)]
pub struct RedirectEditor {
    pub rows: Vec<RedirectEditorRow>,
}

impl RedirectEditor {
    pub fn view(&self, config: &Config, translator: &Translator) -> Container {
        let redirects = config.get_redirects();

        let inner = Container::new({
            self.rows
                .iter()
                .enumerate()
                .fold(Column::new().padding(5).spacing(4), |parent, (i, _)| {
                    parent.push(
                        Row::new()
                            .spacing(20)
                            .push(CommonButton::MoveUp {
                                action: Message::EditedRedirect(EditAction::move_up(i), None),
                                index: i,
                            })
                            .push(CommonButton::MoveDown {
                                action: Message::EditedRedirect(EditAction::move_down(i), None),
                                index: i,
                                max: self.rows.len(),
                            })
                            .push(
                                PickList::new(RedirectKind::ALL, Some(redirects[i].kind), move |v| {
                                    Message::SelectedRedirectKind(i, v)
                                })
                                .style(style::PickList::Primary),
                            )
                            .push(Undoable::new(
                                TextInput::new(
                                    &translator.redirect_source_placeholder(),
                                    &redirects[i].source.raw(),
                                    move |v| {
                                        Message::EditedRedirect(
                                            EditAction::Change(i, v),
                                            Some(RedirectEditActionField::Source),
                                        )
                                    },
                                )
                                .style(style::TextInput)
                                .width(Length::FillPortion(3))
                                .padding(5),
                                move |action| Message::UndoRedo(action, UndoSubject::RedirectSource(i)),
                            ))
                            .push(CommonButton::OpenFolder {
                                subject: BrowseSubject::RedirectSource(i),
                            })
                            .push(Undoable::new(
                                TextInput::new(
                                    &translator.redirect_target_placeholder(),
                                    &redirects[i].target.raw(),
                                    move |v| {
                                        Message::EditedRedirect(
                                            EditAction::Change(i, v),
                                            Some(RedirectEditActionField::Target),
                                        )
                                    },
                                )
                                .style(style::TextInput)
                                .width(Length::FillPortion(3))
                                .padding(5),
                                move |action| Message::UndoRedo(action, UndoSubject::RedirectTarget(i)),
                            ))
                            .push(CommonButton::OpenFolder {
                                subject: BrowseSubject::RedirectTarget(i),
                            })
                            .push(CommonButton::Remove {
                                action: Message::EditedRedirect(EditAction::Remove(i), None),
                            }),
                    )
                })
                .push(CommonButton::Add {
                    action: Message::EditedRedirect(EditAction::Add, None),
                })
        })
        .style(style::Container::GameListEntry);

        Container::new(inner)
    }
}
