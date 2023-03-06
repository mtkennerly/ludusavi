use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, EditAction, Message, UndoSubject},
        icon::Icon,
        style,
    },
    lang::Translator,
    manifest::Store,
    shortcuts::TextHistory,
};

use crate::gui::widget::{Button, Column, Container, PickList, Row, Text, TextInput, Undoable};
use iced::{alignment::Horizontal as HorizontalAlignment, Length};

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
                        .push(
                            Button::new(Icon::RemoveCircle.as_text())
                                .on_press(Message::EditedRoot(EditAction::Remove(i)))
                                .style(style::Button::Negative),
                        )
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
                        .push(
                            Button::new(Icon::FolderOpen.as_text())
                                .on_press(Message::BrowseDir(BrowseSubject::Root(i)))
                                .style(style::Button::Primary),
                        ),
                )
            });
        };

        content = content.push(
            Row::new()
                .spacing(20)
                .push(
                    Button::new(
                        Text::new(translator.add_root_button()).horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .on_press(Message::EditedRoot(EditAction::Add))
                    .width(125)
                    .style(style::Button::Primary),
                )
                .push(
                    Button::new(
                        Text::new(translator.find_roots_button()).horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .on_press(Message::FindRoots)
                    .width(125)
                    .style(style::Button::Primary),
                ),
        );

        Container::new(content)
    }
}
