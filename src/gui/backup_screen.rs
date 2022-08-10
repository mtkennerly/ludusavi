use crate::{
    config::Config,
    gui::{
        common::*,
        game_list::GameList,
        icon::Icon,
        root_editor::{RootEditor, RootEditorRow},
        style,
    },
    lang::Translator,
    manifest::Manifest,
    prelude::DuplicateDetector,
    shortcuts::TextHistory,
};

use iced::{
    alignment::Horizontal as HorizontalAlignment, button, text_input, Alignment, Button, Checkbox, Column, Container,
    Length, Row, Text, TextInput,
};

#[derive(Default)]
pub struct BackupScreenComponent {
    pub log: GameList,
    start_button: button::State,
    preview_button: button::State,
    add_root_button: button::State,
    find_roots_button: button::State,
    select_all_button: button::State,
    toggle_search_button: button::State,
    pub backup_target_input: text_input::State,
    pub backup_target_history: TextHistory,
    backup_target_browse_button: button::State,
    pub root_editor: RootEditor,
    pub recent_found_games: std::collections::HashSet<String>,
    pub duplicate_detector: DuplicateDetector,
    full_retention_input: crate::gui::number_input::NumberInput,
    diff_retention_input: crate::gui::number_input::NumberInput,
}

impl BackupScreenComponent {
    pub fn new(config: &Config) -> Self {
        let mut root_editor = RootEditor::default();
        for root in &config.roots {
            root_editor.rows.push(RootEditorRow::new(&root.path.raw()))
        }

        Self {
            root_editor,
            backup_target_history: TextHistory::new(&config.backup.path.raw(), 100),
            ..Default::default()
        }
    }

    pub fn view(
        &mut self,
        config: &Config,
        manifest: &Manifest,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .align_items(Alignment::Center)
                .spacing(20)
                .push(
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(
                            Button::new(
                                &mut self.preview_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewBackup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewBackup) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::BackupStart {
                                    preview: true,
                                    games: None,
                                },
                                Some(OngoingOperation::PreviewBackup) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::PreviewBackup) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.start_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::Backup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelBackup) => translator.cancelling_button(),
                                    _ => translator.backup_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::ConfirmBackupStart { games: None },
                                Some(OngoingOperation::Backup) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::Backup) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.add_root_button,
                                Text::new(translator.add_root_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::EditedRoot(EditAction::Add))
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push(
                            Button::new(
                                &mut self.find_roots_button,
                                Text::new(translator.find_roots_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::FindRoots)
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push({
                            let restoring = false;
                            Button::new(
                                &mut self.select_all_button,
                                Text::new(if self.log.all_entries_selected(config, restoring) {
                                    translator.deselect_all_button()
                                } else {
                                    translator.select_all_button()
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(if self.log.all_entries_selected(config, restoring) {
                                Message::DeselectAllGames
                            } else {
                                Message::SelectAllGames
                            })
                            .width(Length::Units(125))
                            .style(style::Button::Primary)
                        })
                        .push(
                            Button::new(&mut self.toggle_search_button, Icon::Search.as_text())
                                .on_press(Message::ToggleSearch { screen: Screen::Backup })
                                .style(if self.log.search.show {
                                    style::Button::Negative
                                } else {
                                    style::Button::Primary
                                }),
                        ),
                )
                .push(make_status_row(
                    translator,
                    &self.log.compute_operation_status(config, false),
                    self.duplicate_detector.any_duplicates(),
                ))
                .push(
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(Text::new(translator.backup_target_label()))
                        .push(
                            TextInput::new(
                                &mut self.backup_target_input,
                                "",
                                &config.backup.path.raw(),
                                Message::EditedBackupTarget,
                            )
                            .padding(5),
                        )
                        .push_if(
                            || config.backup.merge,
                            || {
                                self.full_retention_input.view(
                                    config.backup.retention.full,
                                    &translator.full_retention(),
                                    1..=9,
                                    Message::EditedFullRetention,
                                )
                            },
                        )
                        .push_if(
                            || config.backup.merge,
                            || {
                                self.diff_retention_input.view(
                                    config.backup.retention.differential,
                                    &translator.differential_retention(),
                                    0..=9,
                                    Message::EditedDiffRetention,
                                )
                            },
                        )
                        .push(Checkbox::new(
                            config.backup.merge,
                            translator.backup_merge_label(),
                            Message::EditedBackupMerge,
                        ))
                        .push(
                            Button::new(&mut self.backup_target_browse_button, Icon::FolderOpen.as_text())
                                .on_press(match operation {
                                    None => Message::BrowseDir(BrowseSubject::BackupTarget),
                                    Some(_) => Message::Ignore,
                                })
                                .style(match operation {
                                    None => style::Button::Primary,
                                    Some(_) => style::Button::Disabled,
                                }),
                        ),
                )
                .push(self.root_editor.view(config, translator, operation))
                .push(
                    self.log
                        .view(false, translator, config, manifest, &self.duplicate_detector, operation),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
