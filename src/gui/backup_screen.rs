use crate::{
    cache::Cache,
    config::{BackupFormat, Config, ZipCompression},
    gui::{
        common::{
            make_status_row, BrowseSubject, IcedButtonExt, IcedExtension, Message, OngoingOperation, Screen,
            UndoSubject,
        },
        game_list::GameList,
        icon::Icon,
        shortcuts::TextHistory,
        style,
    },
    lang::Translator,
    manifest::Manifest,
    scan::DuplicateDetector,
};

use crate::gui::widget::{Button, Checkbox, Column, Container, PickList, Row, Text, TextInput, Undoable};
use iced::{alignment::Horizontal as HorizontalAlignment, Alignment, Length};

#[derive(Default)]
pub struct BackupScreenComponent {
    pub log: GameList,
    pub backup_target_history: TextHistory,
    pub previewed_games: std::collections::HashSet<String>,
    pub duplicate_detector: DuplicateDetector,
    full_retention_input: crate::gui::number_input::NumberInput,
    diff_retention_input: crate::gui::number_input::NumberInput,
    compression_level_input: crate::gui::number_input::NumberInput,
    pub show_settings: bool,
}

impl BackupScreenComponent {
    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(false, config, cache),
            backup_target_history: TextHistory::new(&config.backup.path.raw(), 100),
            ..Default::default()
        }
    }

    pub fn view(
        &self,
        config: &Config,
        manifest: &Manifest,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container {
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
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewBackup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewBackup) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press_some(match operation {
                                None => Some(Message::BackupPrep {
                                    preview: true,
                                    games: None,
                                }),
                                Some(OngoingOperation::PreviewBackup) => Some(Message::CancelOperation),
                                _ => None,
                            })
                            .width(125)
                            .style(match operation {
                                Some(OngoingOperation::PreviewBackup | OngoingOperation::CancelPreviewBackup) => {
                                    style::Button::Negative
                                }
                                _ => style::Button::Primary,
                            }),
                        )
                        .push(
                            Button::new(
                                Text::new(match operation {
                                    Some(OngoingOperation::Backup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelBackup) => translator.cancelling_button(),
                                    _ => translator.backup_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press_some(match operation {
                                None => Some(Message::ConfirmBackupStart { games: None }),
                                Some(OngoingOperation::Backup) => Some(Message::CancelOperation),
                                _ => None,
                            })
                            .width(125)
                            .style(match operation {
                                Some(OngoingOperation::Backup | OngoingOperation::CancelBackup) => {
                                    style::Button::Negative
                                }
                                _ => style::Button::Primary,
                            }),
                        )
                        .push({
                            let restoring = false;
                            Button::new(
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
                            .width(125)
                            .style(style::Button::Primary)
                        })
                        .push(
                            Button::new(Icon::Search.as_text())
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
                        .push(Undoable::new(
                            TextInput::new("", &config.backup.path.raw(), Message::EditedBackupTarget)
                                .style(style::TextInput)
                                .padding(5),
                            move |action| Message::UndoRedo(action, UndoSubject::BackupTarget),
                        ))
                        .push(
                            Button::new(Icon::Settings.as_text())
                                .on_press(Message::ToggleBackupSettings)
                                .style(if self.show_settings {
                                    style::Button::Negative
                                } else {
                                    style::Button::Primary
                                }),
                        )
                        .push(
                            Button::new(Icon::FolderOpen.as_text())
                                .on_press(Message::BrowseDir(BrowseSubject::BackupTarget))
                                .style(style::Button::Primary),
                        ),
                )
                .push_if(
                    || self.show_settings,
                    || {
                        Row::new()
                            .padding([0, 20, 0, 20])
                            .spacing(20)
                            .height(30)
                            .align_items(Alignment::Center)
                            .push(
                                Checkbox::new(
                                    translator.backup_merge_label(),
                                    config.backup.merge,
                                    Message::EditedBackupMerge,
                                )
                                .style(style::Checkbox),
                            )
                            .push_if(
                                || config.backup.merge,
                                || {
                                    self.full_retention_input.view(
                                        config.backup.retention.full as i32,
                                        translator.full_retention(),
                                        1..=255,
                                        |x| Message::EditedFullRetention(x as u8),
                                    )
                                },
                            )
                            .push_if(
                                || config.backup.merge,
                                || {
                                    self.diff_retention_input.view(
                                        config.backup.retention.differential as i32,
                                        translator.differential_retention(),
                                        0..=255,
                                        |x| Message::EditedDiffRetention(x as u8),
                                    )
                                },
                            )
                    },
                )
                .push_if(
                    || self.show_settings,
                    || {
                        Row::new()
                            .padding([0, 20, 0, 20])
                            .spacing(20)
                            .align_items(Alignment::Center)
                            .push(
                                Row::new()
                                    .spacing(5)
                                    .align_items(Alignment::Center)
                                    .push(Text::new(translator.backup_format_field()))
                                    .push(
                                        PickList::new(
                                            BackupFormat::ALL,
                                            Some(config.backup.format.chosen),
                                            Message::SelectedBackupFormat,
                                        )
                                        .style(style::PickList::Primary),
                                    ),
                            )
                            .push_if(
                                || config.backup.format.chosen == BackupFormat::Zip,
                                || {
                                    Row::new()
                                        .spacing(5)
                                        .align_items(Alignment::Center)
                                        .push(Text::new(translator.backup_compression_field()))
                                        .push(
                                            PickList::new(
                                                ZipCompression::ALL,
                                                Some(config.backup.format.zip.compression),
                                                Message::SelectedBackupCompression,
                                            )
                                            .style(style::PickList::Primary),
                                        )
                                },
                            )
                            .push_some(|| match (config.backup.format.level(), config.backup.format.range()) {
                                (Some(level), Some(range)) => Some(self.compression_level_input.view(
                                    level,
                                    translator.backup_compression_level_field(),
                                    range,
                                    Message::EditedCompressionLevel,
                                )),
                                _ => None,
                            })
                    },
                )
                .push(
                    self.log
                        .view(false, translator, config, manifest, &self.duplicate_detector, operation),
                ),
        )
        .style(style::Container::Primary)
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
