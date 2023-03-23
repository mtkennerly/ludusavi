use iced::Length;

use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, CommonButton, EditAction, Message, UndoSubject},
        shortcuts::TextHistory,
        style,
        widget::{Column, Container, Row, Text, TextInput, Undoable},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct IgnoredItemsEditorEntryRow {
    pub text_history: TextHistory,
}

impl IgnoredItemsEditorEntryRow {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
        }
    }
}

#[derive(Default)]
pub struct IgnoredItemsEditor {
    pub files: Vec<IgnoredItemsEditorEntryRow>,
    pub registry: Vec<IgnoredItemsEditorEntryRow>,
}

impl IgnoredItemsEditor {
    pub fn new(config: &Config) -> Self {
        let mut editor = IgnoredItemsEditor::default();

        for file in &config.backup.filter.ignored_paths {
            editor.files.push(IgnoredItemsEditorEntryRow::new(&file.raw()))
        }
        for key in &config.backup.filter.ignored_registry {
            editor.registry.push(IgnoredItemsEditorEntryRow::new(&key.raw()))
        }

        editor
    }

    pub fn view(&self, config: &Config, translator: &Translator) -> Container {
        Container::new({
            Column::new().width(Length::Fill).height(Length::Fill).spacing(10).push(
                Container::new(
                    Column::new()
                        .padding(5)
                        .spacing(5)
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(100)
                                        .push(Text::new(translator.custom_files_label())),
                                )
                                .push(
                                    self.files
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(CommonButton::MoveUp {
                                                        action: Message::EditedBackupFilterIgnoredPath(
                                                            EditAction::move_up(ii),
                                                        ),
                                                        index: ii,
                                                    })
                                                    .push(CommonButton::MoveDown {
                                                        action: Message::EditedBackupFilterIgnoredPath(
                                                            EditAction::move_down(ii),
                                                        ),
                                                        index: ii,
                                                        max: self.files.len(),
                                                    })
                                                    .push(Undoable::new(
                                                        TextInput::new(
                                                            "",
                                                            &config.backup.filter.ignored_paths[ii].raw(),
                                                            move |v| {
                                                                Message::EditedBackupFilterIgnoredPath(
                                                                    EditAction::Change(ii, v),
                                                                )
                                                            },
                                                        )
                                                        .style(style::TextInput)
                                                        .padding(5),
                                                        move |action| {
                                                            Message::UndoRedo(
                                                                action,
                                                                UndoSubject::BackupFilterIgnoredPath(ii),
                                                            )
                                                        },
                                                    ))
                                                    .push(CommonButton::OpenFolder {
                                                        subject: BrowseSubject::BackupFilterIgnoredPath(ii),
                                                    })
                                                    .push(CommonButton::Remove {
                                                        action: Message::EditedBackupFilterIgnoredPath(
                                                            EditAction::Remove(ii),
                                                        ),
                                                    }),
                                            )
                                        })
                                        .push(CommonButton::Add {
                                            action: Message::EditedBackupFilterIgnoredPath(EditAction::Add),
                                        }),
                                ),
                        )
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(100)
                                        .push(Text::new(translator.custom_registry_label())),
                                )
                                .push(
                                    self.registry
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(CommonButton::MoveUp {
                                                        action: Message::EditedBackupFilterIgnoredRegistry(
                                                            EditAction::move_up(ii),
                                                        ),
                                                        index: ii,
                                                    })
                                                    .push(CommonButton::MoveDown {
                                                        action: Message::EditedBackupFilterIgnoredRegistry(
                                                            EditAction::move_down(ii),
                                                        ),
                                                        index: ii,
                                                        max: self.registry.len(),
                                                    })
                                                    .push(Undoable::new(
                                                        TextInput::new(
                                                            "",
                                                            &config.backup.filter.ignored_registry[ii].raw(),
                                                            move |v| {
                                                                Message::EditedBackupFilterIgnoredRegistry(
                                                                    EditAction::Change(ii, v),
                                                                )
                                                            },
                                                        )
                                                        .style(style::TextInput)
                                                        .padding(5),
                                                        move |action| {
                                                            Message::UndoRedo(
                                                                action,
                                                                UndoSubject::BackupFilterIgnoredRegistry(ii),
                                                            )
                                                        },
                                                    ))
                                                    .push(CommonButton::Remove {
                                                        action: Message::EditedBackupFilterIgnoredRegistry(
                                                            EditAction::Remove(ii),
                                                        ),
                                                    }),
                                            )
                                        })
                                        .push(CommonButton::Add {
                                            action: Message::EditedBackupFilterIgnoredRegistry(EditAction::Add),
                                        }),
                                ),
                        ),
                )
                .style(style::Container::GameListEntry),
            )
        })
    }
}
