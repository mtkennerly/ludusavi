use iced::{Alignment, Length};

use crate::{
    cache::Cache,
    config::{BackupFormat, Config, ZipCompression},
    gui::{
        button,
        common::{make_status_row, BrowseSubject, IcedExtension, Message, OngoingOperation, Screen, UndoSubject},
        game_list::GameList,
        shortcuts::TextHistory,
        style,
        widget::{Checkbox, Column, Container, PickList, Row, Text, TextInput, Undoable},
    },
    lang::Translator,
    manifest::Manifest,
    scan::DuplicateDetector,
};

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
                        .push(button::operation(OngoingOperation::PreviewBackup, operation.to_owned()))
                        .push(button::operation(OngoingOperation::Backup, operation.to_owned()))
                        .push(button::toggle_all_scanned_games(
                            self.log.all_entries_selected(config, false),
                        ))
                        .push(button::search(Screen::Backup, self.log.search.show)),
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
                        .push(button::settings(self.show_settings))
                        .push(button::open_folder(BrowseSubject::BackupTarget)),
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
