use crate::{
    config::{Config, RedirectKind},
    gui::{
        common::{BrowseSubject, EditAction, Message, RedirectEditActionField, UndoSubject},
        icon::Icon,
        shortcuts::TextHistory,
        style,
    },
    lang::Translator,
};

use crate::gui::widget::{Button, Column, Container, PickList, Row, TextInput, Undoable};
use iced::Length;

use super::common::IcedButtonExt;

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
                            .push(
                                Icon::ArrowUpward
                                    .as_button_small()
                                    .on_press_if(|| i > 0, || Message::EditedRedirect(EditAction::move_up(i), None)),
                            )
                            .push(Icon::ArrowDownward.as_button_small().on_press_if(
                                || i < self.rows.len() - 1,
                                || Message::EditedRedirect(EditAction::move_down(i), None),
                            ))
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
                            .push(
                                Button::new(Icon::FolderOpen.as_text())
                                    .on_press(Message::BrowseDir(BrowseSubject::RedirectSource(i)))
                                    .style(style::Button::Primary),
                            )
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
                            .push(
                                Button::new(Icon::FolderOpen.as_text())
                                    .on_press(Message::BrowseDir(BrowseSubject::RedirectTarget(i)))
                                    .style(style::Button::Primary),
                            )
                            .push(
                                Button::new(Icon::RemoveCircle.as_text())
                                    .on_press(Message::EditedRedirect(EditAction::Remove(i), None))
                                    .style(style::Button::Negative),
                            ),
                    )
                })
                .push(
                    Button::new(Icon::AddCircle.as_text())
                        .on_press(Message::EditedRedirect(EditAction::Add, None))
                        .style(style::Button::Primary),
                )
        })
        .style(style::Container::GameListEntry);

        Container::new(inner)
    }
}
