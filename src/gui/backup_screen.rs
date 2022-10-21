use crate::{
    cache::Cache,
    config::{BackupFormat, Config, ZipCompression},
    gui::{
        common::{
            make_status_row, BrowseSubject, EditAction, IcedButtonExt, IcedExtension, Message, OngoingOperation, Screen,
        },
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
    alignment::Horizontal as HorizontalAlignment, button, pick_list, text_input, Alignment, Button, Checkbox, Column,
    Container, Length, PickList, Row, Text, TextInput,
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
    pub previewed_games: std::collections::HashSet<String>,
    pub duplicate_detector: DuplicateDetector,
    full_retention_input: crate::gui::number_input::NumberInput,
    diff_retention_input: crate::gui::number_input::NumberInput,
    format_selector: pick_list::State<BackupFormat>,
    compression_selector: pick_list::State<ZipCompression>,
    compression_level_input: crate::gui::number_input::NumberInput,
    settings_button: button::State,
    pub show_settings: bool,
}

impl BackupScreenComponent {
    pub fn new(config: &Config, cache: &Cache) -> Self {
        let mut root_editor = RootEditor::default();
        for root in &config.roots {
            root_editor.rows.push(RootEditorRow::new(&root.path.raw()))
        }

        Self {
            log: GameList::with_recent_games(false, config, cache),
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
                            .on_press_some(match operation {
                                None => Some(Message::BackupPrep {
                                    preview: true,
                                    games: None,
                                }),
                                Some(OngoingOperation::PreviewBackup) => Some(Message::CancelOperation),
                                _ => None,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                Some(OngoingOperation::PreviewBackup | OngoingOperation::CancelPreviewBackup) => {
                                    style::Button::Negative(config.theme)
                                }
                                _ => style::Button::Primary(config.theme),
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
                            .on_press_some(match operation {
                                None => Some(Message::ConfirmBackupStart { games: None }),
                                Some(OngoingOperation::Backup) => Some(Message::CancelOperation),
                                _ => None,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                Some(OngoingOperation::Backup | OngoingOperation::CancelBackup) => {
                                    style::Button::Negative(config.theme)
                                }
                                _ => style::Button::Primary(config.theme),
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
                            .style(style::Button::Primary(config.theme)),
                        )
                        .push(
                            Button::new(
                                &mut self.find_roots_button,
                                Text::new(translator.find_roots_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::FindRoots)
                            .width(Length::Units(125))
                            .style(style::Button::Primary(config.theme)),
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
                            .style(style::Button::Primary(config.theme))
                        })
                        .push(
                            Button::new(&mut self.toggle_search_button, Icon::Search.as_text())
                                .on_press(Message::ToggleSearch { screen: Screen::Backup })
                                .style(if self.log.search.show {
                                    style::Button::Negative(config.theme)
                                } else {
                                    style::Button::Primary(config.theme)
                                }),
                        ),
                )
                .push(make_status_row(
                    translator,
                    &self.log.compute_operation_status(config, false),
                    self.duplicate_detector.any_duplicates(),
                    config.theme,
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
                            .style(style::TextInput(config.theme))
                            .padding(5),
                        )
                        .push(
                            Button::new(&mut self.settings_button, Icon::Settings.as_text())
                                .on_press(Message::ToggleBackupSettings)
                                .style(if self.show_settings {
                                    style::Button::Negative(config.theme)
                                } else {
                                    style::Button::Primary(config.theme)
                                }),
                        )
                        .push(
                            Button::new(&mut self.backup_target_browse_button, Icon::FolderOpen.as_text())
                                .on_press(Message::BrowseDir(BrowseSubject::BackupTarget))
                                .style(style::Button::Primary(config.theme)),
                        ),
                )
                .push_if(
                    || self.show_settings,
                    || {
                        Row::new()
                            .padding([0, 20, 0, 20])
                            .spacing(20)
                            .height(Length::Units(30))
                            .align_items(Alignment::Center)
                            .push(
                                Checkbox::new(
                                    config.backup.merge,
                                    translator.backup_merge_label(),
                                    Message::EditedBackupMerge,
                                )
                                .style(style::Checkbox(config.theme)),
                            )
                            .push_if(
                                || config.backup.merge,
                                || {
                                    self.full_retention_input.view(
                                        config.backup.retention.full as i32,
                                        &translator.full_retention(),
                                        1..=255,
                                        |x| Message::EditedFullRetention(x as u8),
                                        config.theme,
                                    )
                                },
                            )
                            .push_if(
                                || config.backup.merge,
                                || {
                                    self.diff_retention_input.view(
                                        config.backup.retention.differential as i32,
                                        &translator.differential_retention(),
                                        0..=255,
                                        |x| Message::EditedDiffRetention(x as u8),
                                        config.theme,
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
                                            &mut self.format_selector,
                                            BackupFormat::ALL,
                                            Some(config.backup.format.chosen),
                                            Message::SelectedBackupFormat,
                                        )
                                        .style(style::PickList::Primary(config.theme)),
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
                                                &mut self.compression_selector,
                                                ZipCompression::ALL,
                                                Some(config.backup.format.zip.compression),
                                                Message::SelectedBackupCompression,
                                            )
                                            .style(style::PickList::Primary(config.theme)),
                                        )
                                },
                            )
                            .push_some(|| match (config.backup.format.level(), config.backup.format.range()) {
                                (Some(level), Some(range)) => Some(self.compression_level_input.view(
                                    level,
                                    &translator.backup_compression_level_field(),
                                    range,
                                    Message::EditedCompressionLevel,
                                    config.theme,
                                )),
                                _ => None,
                            })
                    },
                )
                .push(self.root_editor.view(config, translator))
                .push(
                    self.log
                        .view(false, translator, config, manifest, &self.duplicate_detector, operation),
                ),
        )
        .style(style::Container::Primary(config.theme))
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
