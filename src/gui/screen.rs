use iced::{Alignment, Length};

use crate::{
    gui::{
        badge::Badge,
        button,
        common::{BrowseSubject, Message, OngoingOperation, Screen, ScrollSubject, TextHistories, UndoSubject},
        editor,
        game_list::GameList,
        style,
        widget::{number_input, Button, Checkbox, Column, Container, Element, IcedParentExt, PickList, Row, Text},
    },
    lang::{Language, TRANSLATOR},
    prelude::STEAM_DECK,
    resource::{
        cache::Cache,
        config::{BackupFormat, Config, Theme, ZipCompression},
        manifest::Manifest,
    },
    scan::{DuplicateDetector, OperationStatus},
};

fn template(content: Column) -> Element {
    Container::new(content.spacing(20).align_items(Alignment::Center))
        .height(Length::Fill)
        .width(Length::Fill)
        .padding([0, 5, 5, 5])
        .center_x()
        .into()
}

fn make_status_row<'a>(status: &OperationStatus, found_any_duplicates: bool) -> Row<'a> {
    Row::new()
        .padding([0, 20, 0, 20])
        .align_items(Alignment::Center)
        .spacing(15)
        .push(Text::new(TRANSLATOR.processed_games(status)).size(35))
        .push_if(
            || status.changed_games.new > 0,
            || Badge::new_entry_with_count(status.changed_games.new).view(),
        )
        .push_if(
            || status.changed_games.different > 0,
            || Badge::changed_entry_with_count(status.changed_games.different).view(),
        )
        .push(Text::new("|").size(35))
        .push(Text::new(TRANSLATOR.processed_bytes(status)).size(35))
        .push_if(
            || found_any_duplicates,
            || Badge::new(&TRANSLATOR.badge_duplicates()).view(),
        )
}

#[derive(Default)]
pub struct Backup {
    pub log: GameList,
    pub previewed_games: std::collections::HashSet<String>,
    pub duplicate_detector: DuplicateDetector,
    pub show_settings: bool,
}

impl Backup {
    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(false, config, cache),
            ..Default::default()
        }
    }

    pub fn view(
        &self,
        config: &Config,
        manifest: &Manifest,
        operation: &Option<OngoingOperation>,
        histories: &TextHistories,
    ) -> Element {
        let content = Column::new()
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
                &self.log.compute_operation_status(config, false),
                self.duplicate_detector.any_duplicates(),
            ))
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(Text::new(TRANSLATOR.backup_target_label()))
                    .push(histories.input(UndoSubject::BackupTarget))
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
                                TRANSLATOR.backup_merge_label(),
                                config.backup.merge,
                                Message::EditedBackupMerge,
                            )
                            .style(style::Checkbox),
                        )
                        .push_if(
                            || config.backup.merge,
                            || {
                                number_input(
                                    config.backup.retention.full as i32,
                                    TRANSLATOR.full_retention(),
                                    1..=255,
                                    |x| Message::EditedFullRetention(x as u8),
                                )
                            },
                        )
                        .push_if(
                            || config.backup.merge,
                            || {
                                number_input(
                                    config.backup.retention.differential as i32,
                                    TRANSLATOR.differential_retention(),
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
                                .push(Text::new(TRANSLATOR.backup_format_field()))
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
                                    .push(Text::new(TRANSLATOR.backup_compression_field()))
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
                            (Some(level), Some(range)) => Some(number_input(
                                level,
                                TRANSLATOR.backup_compression_level_field(),
                                range,
                                Message::EditedCompressionLevel,
                            )),
                            _ => None,
                        })
                },
            )
            .push(
                self.log
                    .view(false, config, manifest, &self.duplicate_detector, operation, histories),
            );

        template(content)
    }
}

#[derive(Default)]
pub struct Restore {
    pub log: GameList,
    pub duplicate_detector: DuplicateDetector,
}

impl Restore {
    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(true, config, cache),
            ..Default::default()
        }
    }

    pub fn view(
        &self,
        config: &Config,
        manifest: &Manifest,
        operation: &Option<OngoingOperation>,
        histories: &TextHistories,
    ) -> Element {
        let content = Column::new()
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(button::operation(
                        OngoingOperation::PreviewRestore,
                        operation.to_owned(),
                    ))
                    .push(button::operation(OngoingOperation::Restore, operation.to_owned()))
                    .push(button::toggle_all_scanned_games(
                        self.log.all_entries_selected(config, true),
                    ))
                    .push(button::search(Screen::Restore, self.log.search.show)),
            )
            .push(make_status_row(
                &self.log.compute_operation_status(config, true),
                self.duplicate_detector.any_duplicates(),
            ))
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(Text::new(TRANSLATOR.restore_source_label()))
                    .push(histories.input(UndoSubject::RestoreSource))
                    .push(button::open_folder(BrowseSubject::RestoreSource)),
            )
            .push(
                self.log
                    .view(true, config, manifest, &self.duplicate_detector, operation, histories),
            );

        template(content)
    }
}

pub fn custom_games<'a>(config: &Config, operating: bool, histories: &TextHistories) -> Element<'a> {
    let content = Column::new()
        .push(
            Row::new()
                .padding([0, 20, 0, 20])
                .spacing(20)
                .align_items(Alignment::Center)
                .push(button::add_game())
                .push(button::toggle_all_custom_games(config.are_all_custom_games_enabled())),
        )
        .push(editor::custom_games(config, operating, histories));

    template(content)
}

pub fn other<'a>(updating_manifest: bool, config: &Config, cache: &Cache, histories: &TextHistories) -> Element<'a> {
    let content = Column::new()
        .push_if(
            || *STEAM_DECK,
            || {
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(iced::Alignment::Center)
                    .push(
                        Button::new(
                            Text::new(TRANSLATOR.exit_button())
                                .horizontal_alignment(iced::alignment::Horizontal::Center),
                        )
                        .on_press(Message::Exit)
                        .width(125)
                        .style(style::Button::Negative),
                    )
            },
        )
        .push({
            let content = Column::new()
                .spacing(20)
                .padding([0, 15, 5, 15])
                .width(Length::Fill)
                .push(
                    Row::new()
                        .align_items(iced::Alignment::Center)
                        .spacing(20)
                        .push(Text::new(TRANSLATOR.field_language()))
                        .push(
                            PickList::new(Language::ALL, Some(config.language), Message::SelectedLanguage)
                                .style(style::PickList::Primary),
                        ),
                )
                .push(
                    Row::new()
                        .align_items(iced::Alignment::Center)
                        .spacing(20)
                        .push(Text::new(TRANSLATOR.field_theme()))
                        .push(
                            PickList::new(Theme::ALL, Some(config.theme), Message::SelectedTheme)
                                .style(style::PickList::Primary),
                        ),
                )
                .push(
                    Checkbox::new(
                        TRANSLATOR.explanation_for_exclude_store_screenshots(),
                        config.backup.filter.exclude_store_screenshots,
                        Message::EditedExcludeStoreScreenshots,
                    )
                    .style(style::Checkbox),
                )
                .push(
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .align_items(iced::Alignment::Center)
                                .push(Text::new(TRANSLATOR.manifest_label()).width(100))
                                .push(button::refresh(Message::UpdateManifest, updating_manifest)),
                        )
                        .push_some(|| {
                            let cached = cache.manifests.get(&config.manifest.url)?;
                            let checked = match cached.checked {
                                Some(x) => chrono::DateTime::<chrono::Local>::from(x)
                                    .format("%Y-%m-%dT%H:%M:%S")
                                    .to_string(),
                                None => "?".to_string(),
                            };
                            let updated = match cached.updated {
                                Some(x) => chrono::DateTime::<chrono::Local>::from(x)
                                    .format("%Y-%m-%dT%H:%M:%S")
                                    .to_string(),
                                None => "?".to_string(),
                            };
                            Some(
                                Container::new(
                                    Column::new()
                                        .padding(5)
                                        .spacing(4)
                                        .push(
                                            Row::new()
                                                .align_items(iced::Alignment::Center)
                                                .push(Container::new(Text::new(TRANSLATOR.checked_label())).width(100))
                                                .push(Container::new(Text::new(checked))),
                                        )
                                        .push(
                                            Row::new()
                                                .align_items(iced::Alignment::Center)
                                                .push(Container::new(Text::new(TRANSLATOR.updated_label())).width(100))
                                                .push(Container::new(Text::new(updated))),
                                        ),
                                )
                                .style(style::Container::GameListEntry),
                            )
                        }),
                )
                .push(
                    Column::new().spacing(5).push(Text::new(TRANSLATOR.roots_label())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(4)
                                .push(editor::root(config, histories)),
                        )
                        .style(style::Container::GameListEntry),
                    ),
                )
                .push(
                    Column::new()
                        .push(Text::new(TRANSLATOR.ignored_items_label()))
                        .push(editor::ignored_items(config, histories).padding([10, 0, 0, 0])),
                )
                .push(
                    Column::new()
                        .push(Text::new(TRANSLATOR.redirects_label()))
                        .push(editor::redirect(config, histories).padding([10, 0, 0, 0])),
                );
            ScrollSubject::Other.into_widget(content)
        });

    template(content)
}
