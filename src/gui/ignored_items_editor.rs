use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, EditAction, Message, UndoSubject},
        icon::Icon,
        style,
    },
    lang::Translator,
    shortcuts::TextHistory,
};

use crate::gui::widget::{Button, Column, Container, Row, Text, TextInput, Undoable};
use iced::Length;

use super::common::IcedButtonExt;

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
pub struct IgnoredItemsEditorEntry {
    pub text_history: TextHistory,
    pub files: Vec<IgnoredItemsEditorEntryRow>,
    pub registry: Vec<IgnoredItemsEditorEntryRow>,
}

impl IgnoredItemsEditorEntry {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }
}

#[derive(Default)]
pub struct IgnoredItemsEditor {
    pub entry: IgnoredItemsEditorEntry,
}

impl IgnoredItemsEditor {
    pub fn new(config: &Config) -> Self {
        let mut editor = IgnoredItemsEditor::default();

        let mut row = IgnoredItemsEditorEntry::new();
        for file in &config.backup.filter.ignored_paths {
            row.files.push(IgnoredItemsEditorEntryRow::new(&file.raw()))
        }
        for key in &config.backup.filter.ignored_registry {
            row.registry.push(IgnoredItemsEditorEntryRow::new(&key.raw()))
        }
        editor.entry = row;

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
                                    self.entry
                                        .files
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(Icon::ArrowUpward.as_button_small().on_press_if(
                                                        || ii > 0,
                                                        || {
                                                            Message::EditedBackupFilterIgnoredPath(EditAction::move_up(
                                                                ii,
                                                            ))
                                                        },
                                                    ))
                                                    .push(Icon::ArrowDownward.as_button_small().on_press_if(
                                                        || ii < self.entry.files.len() - 1,
                                                        || {
                                                            Message::EditedBackupFilterIgnoredPath(
                                                                EditAction::move_down(ii),
                                                            )
                                                        },
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
                                                    .push(
                                                        Button::new(Icon::FolderOpen.as_text())
                                                            .on_press(Message::BrowseDir(
                                                                BrowseSubject::BackupFilterIgnoredPath(ii),
                                                            ))
                                                            .style(style::Button::Primary),
                                                    )
                                                    .push(
                                                        Button::new(Icon::RemoveCircle.as_text())
                                                            .on_press(Message::EditedBackupFilterIgnoredPath(
                                                                EditAction::Remove(ii),
                                                            ))
                                                            .style(style::Button::Negative),
                                                    ),
                                            )
                                        })
                                        .push(
                                            Button::new(Icon::AddCircle.as_text())
                                                .on_press(Message::EditedBackupFilterIgnoredPath(EditAction::Add))
                                                .style(style::Button::Primary),
                                        ),
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
                                    self.entry
                                        .registry
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(Icon::ArrowUpward.as_button_small().on_press_if(
                                                        || ii > 0,
                                                        || {
                                                            Message::EditedBackupFilterIgnoredRegistry(
                                                                EditAction::move_up(ii),
                                                            )
                                                        },
                                                    ))
                                                    .push(Icon::ArrowDownward.as_button_small().on_press_if(
                                                        || ii < self.entry.registry.len() - 1,
                                                        || {
                                                            Message::EditedBackupFilterIgnoredRegistry(
                                                                EditAction::move_down(ii),
                                                            )
                                                        },
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
                                                    .push(
                                                        Button::new(Icon::RemoveCircle.as_text())
                                                            .on_press(Message::EditedBackupFilterIgnoredRegistry(
                                                                EditAction::Remove(ii),
                                                            ))
                                                            .style(style::Button::Negative),
                                                    ),
                                            )
                                        })
                                        .push(
                                            Button::new(Icon::AddCircle.as_text())
                                                .on_press(Message::EditedBackupFilterIgnoredRegistry(EditAction::Add))
                                                .style(style::Button::Primary),
                                        ),
                                ),
                        ),
                )
                .style(style::Container::GameListEntry),
            )
        })
    }
}
