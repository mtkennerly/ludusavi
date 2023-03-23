use iced::Length;

use crate::{
    config::Config,
    gui::{
        button,
        common::{BrowseSubject, EditAction, Message, UndoSubject},
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
                                                    .push(button::move_up(Message::EditedBackupFilterIgnoredPath, ii))
                                                    .push(button::move_down(
                                                        Message::EditedBackupFilterIgnoredPath,
                                                        ii,
                                                        self.files.len(),
                                                    ))
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
                                                    .push(button::open_folder(BrowseSubject::BackupFilterIgnoredPath(
                                                        ii,
                                                    )))
                                                    .push(button::remove(Message::EditedBackupFilterIgnoredPath, ii)),
                                            )
                                        })
                                        .push(button::add(Message::EditedBackupFilterIgnoredPath)),
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
                                                    .push(button::move_up(
                                                        Message::EditedBackupFilterIgnoredRegistry,
                                                        ii,
                                                    ))
                                                    .push(button::move_down(
                                                        Message::EditedBackupFilterIgnoredRegistry,
                                                        ii,
                                                        self.registry.len(),
                                                    ))
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
                                                    .push(button::remove(
                                                        Message::EditedBackupFilterIgnoredRegistry,
                                                        ii,
                                                    )),
                                            )
                                        })
                                        .push(button::add(Message::EditedBackupFilterIgnoredRegistry)),
                                ),
                        ),
                )
                .style(style::Container::GameListEntry),
            )
        })
    }
}
