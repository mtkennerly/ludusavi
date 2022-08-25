use crate::{
    config::Config,
    gui::{
        common::Message,
        common::OngoingOperation,
        common::{BrowseSubject, EditAction},
        icon::Icon,
        style,
    },
    lang::Translator,
    shortcuts::TextHistory,
};

use iced::{button, text_input, Button, Column, Container, Length, Row, Text, TextInput};

#[derive(Default)]
pub struct IgnoredItemsEditorEntryRow {
    button_state: button::State,
    pub text_state: text_input::State,
    pub text_history: TextHistory,
    browse_button_state: button::State,
}

impl IgnoredItemsEditorEntryRow {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct IgnoredItemsEditorEntry {
    add_file_button_state: button::State,
    add_registry_button_state: button::State,
    pub text_state: text_input::State,
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
            row.files.push(IgnoredItemsEditorEntryRow::new(&file.render()))
        }
        for key in &config.backup.filter.ignored_registry {
            row.registry.push(IgnoredItemsEditorEntryRow::new(&key.render()))
        }
        editor.entry = row;

        editor
    }

    pub fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
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
                                        .width(Length::Units(100))
                                        .push(Text::new(translator.custom_files_label())),
                                )
                                .push(
                                    self.entry
                                        .files
                                        .iter_mut()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, xx)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(
                                                        TextInput::new(
                                                            &mut xx.text_state,
                                                            "",
                                                            &config.backup.filter.ignored_paths[ii].raw(),
                                                            move |v| {
                                                                Message::EditedBackupFilterIgnoredPath(
                                                                    EditAction::Change(ii, v),
                                                                )
                                                            },
                                                        )
                                                        .style(style::TextInput(config.theme))
                                                        .padding(5),
                                                    )
                                                    .push(
                                                        Button::new(
                                                            &mut xx.browse_button_state,
                                                            Icon::FolderOpen.as_text(),
                                                        )
                                                        .on_press(match operation {
                                                            None => Message::BrowseDir(
                                                                BrowseSubject::BackupFilterIgnoredPath(ii),
                                                            ),
                                                            Some(_) => Message::Ignore,
                                                        })
                                                        .style(match operation {
                                                            None => style::Button::Primary(config.theme),
                                                            Some(_) => style::Button::Disabled(config.theme),
                                                        }),
                                                    )
                                                    .push(
                                                        Button::new(&mut xx.button_state, Icon::RemoveCircle.as_text())
                                                            .on_press(Message::EditedBackupFilterIgnoredPath(
                                                                EditAction::Remove(ii),
                                                            ))
                                                            .style(style::Button::Negative(config.theme)),
                                                    ),
                                            )
                                        })
                                        .push(
                                            Button::new(
                                                &mut self.entry.add_file_button_state,
                                                Icon::AddCircle.as_text(),
                                            )
                                            .on_press(Message::EditedBackupFilterIgnoredPath(EditAction::Add))
                                            .style(style::Button::Primary(config.theme)),
                                        ),
                                ),
                        )
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(Length::Units(100))
                                        .push(Text::new(translator.custom_registry_label())),
                                )
                                .push(
                                    self.entry
                                        .registry
                                        .iter_mut()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, xx)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(
                                                        TextInput::new(
                                                            &mut xx.text_state,
                                                            "",
                                                            &config.backup.filter.ignored_registry[ii].raw(),
                                                            move |v| {
                                                                Message::EditedBackupFilterIgnoredRegistry(
                                                                    EditAction::Change(ii, v),
                                                                )
                                                            },
                                                        )
                                                        .style(style::TextInput(config.theme))
                                                        .padding(5),
                                                    )
                                                    .push(
                                                        Button::new(&mut xx.button_state, Icon::RemoveCircle.as_text())
                                                            .on_press(Message::EditedBackupFilterIgnoredRegistry(
                                                                EditAction::Remove(ii),
                                                            ))
                                                            .style(style::Button::Negative(config.theme)),
                                                    ),
                                            )
                                        })
                                        .push(
                                            Button::new(
                                                &mut self.entry.add_registry_button_state,
                                                Icon::AddCircle.as_text(),
                                            )
                                            .on_press(Message::EditedBackupFilterIgnoredRegistry(EditAction::Add))
                                            .style(style::Button::Primary(config.theme)),
                                        ),
                                ),
                        ),
                )
                .style(style::Container::GameListEntry(config.theme)),
            )
        })
    }
}
