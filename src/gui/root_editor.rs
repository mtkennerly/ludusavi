use iced::Length;

use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, CommonButton, EditAction, Message, UndoSubject},
        shortcuts::TextHistory,
        style,
        widget::{Column, Container, PickList, Row, Text, TextInput, Undoable},
    },
    lang::Translator,
    manifest::Store,
};

#[derive(Default)]
pub struct RootEditorRow {
    pub text_history: TextHistory,
}

impl RootEditorRow {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
        }
    }
}

#[derive(Default)]
pub struct RootEditor {
    pub rows: Vec<RootEditorRow>,
}

impl RootEditor {
    pub fn view(&self, config: &Config, translator: &Translator) -> Container {
        let mut content = Column::new().width(Length::Fill).spacing(5);
        let roots = config.roots.clone();
        if roots.is_empty() {
            content = content.push(Text::new(translator.no_roots_are_configured()));
        } else {
            content = self.rows.iter().enumerate().fold(content, |parent, (i, _)| {
                parent.push(
                    Row::new()
                        .spacing(20)
                        .push(CommonButton::MoveUp {
                            action: Message::EditedRoot(EditAction::move_up(i)),
                            index: i,
                        })
                        .push(CommonButton::MoveDown {
                            action: Message::EditedRoot(EditAction::move_down(i)),
                            index: i,
                            max: self.rows.len(),
                        })
                        .push(Undoable::new(
                            TextInput::new("", &roots[i].path.raw(), move |v| {
                                Message::EditedRoot(EditAction::Change(i, v))
                            })
                            .style(style::TextInput)
                            .width(Length::FillPortion(3))
                            .padding(5),
                            move |action| Message::UndoRedo(action, UndoSubject::Root(i)),
                        ))
                        .push(
                            PickList::new(Store::ALL, Some(roots[i].store), move |v| {
                                Message::SelectedRootStore(i, v)
                            })
                            .style(style::PickList::Primary),
                        )
                        .push(CommonButton::OpenFolder {
                            subject: BrowseSubject::Root(i),
                        })
                        .push(CommonButton::Remove {
                            action: Message::EditedRoot(EditAction::Remove(i)),
                        }),
                )
            });
        };

        content = content.push(
            Row::new()
                .spacing(20)
                .push(CommonButton::Add {
                    action: Message::EditedRoot(EditAction::Add),
                })
                .push(CommonButton::Refresh {
                    action: Message::FindRoots,
                    ongoing: false,
                }),
        );

        Container::new(content)
    }
}
