use std::collections::HashSet;

use iced::{keyboard, Alignment, Length};

use crate::{
    cloud::{Remote, RemoteChoice},
    gui::{
        badge::Badge,
        button,
        common::{BrowseFileSubject, BrowseSubject, Message, Operation, Screen, ScrollSubject, UndoSubject},
        editor,
        game_list::GameList,
        icon::Icon,
        shortcuts::TextHistories,
        style,
        widget::{number_input, Button, Checkbox, Column, Container, Element, IcedParentExt, PickList, Row, Text},
    },
    lang::{Language, TRANSLATOR},
    prelude::{AVAILABLE_PARALELLISM, STEAM_DECK},
    resource::{
        cache::Cache,
        config::{BackupFormat, Config, SortKey, Theme, ZipCompression},
        manifest::Manifest,
    },
    scan::{DuplicateDetector, Duplication, OperationStatus},
};

const RCLONE_URL: &str = "https://rclone.org/downloads";

fn template(content: Column) -> Element {
    Container::new(content.spacing(15).align_items(Alignment::Center))
        .height(Length::Fill)
        .width(Length::Fill)
        .padding([0, 5, 5, 5])
        .center_x()
        .into()
}

fn make_status_row<'a>(status: &OperationStatus, duplication: Duplication) -> Row<'a> {
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
            || !duplication.resolved(),
            || Badge::new(&TRANSLATOR.badge_duplicates()).view(),
        )
}

#[derive(Default)]
pub struct Backup {
    pub log: GameList,
    pub previewed_games: HashSet<String>,
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
        operation: &Operation,
        histories: &TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element {
        let screen = Screen::Backup;
        let sort = &config.backup.sort;

        let content = Column::new()
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(button::backup_preview(operation))
                    .push(button::backup(operation))
                    .push(button::toggle_all_scanned_games(
                        self.log.all_entries_selected(config, false),
                    ))
                    .push(button::filter(Screen::Backup, self.log.search.show))
                    .push(button::settings(self.show_settings)),
            )
            .push(make_status_row(
                &self.log.compute_operation_status(config, false),
                self.duplicate_detector.overall(),
            ))
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(Text::new(TRANSLATOR.backup_target_label()))
                    .push(histories.input(UndoSubject::BackupTarget))
                    .push(button::choose_folder(BrowseSubject::BackupTarget))
                    .push("|")
                    .push(Text::new(TRANSLATOR.sort_label()))
                    .push(
                        PickList::new(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                            screen,
                            value,
                        })
                        .style(style::PickList::Primary),
                    )
                    .push(button::sort_order(screen, sort.reversed)),
            )
            .push_if(
                || self.show_settings,
                || {
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .height(30)
                        .align_items(Alignment::Center)
                        .push({
                            number_input(
                                config.backup.retention.full as i32,
                                TRANSLATOR.full_retention(),
                                1..=255,
                                |x| Message::EditedFullRetention(x as u8),
                            )
                        })
                        .push({
                            number_input(
                                config.backup.retention.differential as i32,
                                TRANSLATOR.differential_retention(),
                                0..=255,
                                |x| Message::EditedDiffRetention(x as u8),
                            )
                        })
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
            .push(self.log.view(
                false,
                config,
                manifest,
                &self.duplicate_detector,
                operation,
                histories,
                modifiers,
            ));

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
        operation: &Operation,
        histories: &TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element {
        let screen = Screen::Restore;
        let sort = &config.restore.sort;

        let content = Column::new()
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(button::restore_preview(operation))
                    .push(button::restore(operation))
                    .push(button::toggle_all_scanned_games(
                        self.log.all_entries_selected(config, true),
                    ))
                    .push(button::validate_backups(operation))
                    .push(button::filter(Screen::Restore, self.log.search.show)),
            )
            .push(make_status_row(
                &self.log.compute_operation_status(config, true),
                self.duplicate_detector.overall(),
            ))
            .push(
                Row::new()
                    .padding([0, 20, 0, 20])
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(Text::new(TRANSLATOR.restore_source_label()))
                    .push(histories.input(UndoSubject::RestoreSource))
                    .push(button::choose_folder(BrowseSubject::RestoreSource))
                    .push("|")
                    .push(Text::new(TRANSLATOR.sort_label()))
                    .push(
                        PickList::new(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                            screen,
                            value,
                        })
                        .style(style::PickList::Primary),
                    )
                    .push(button::sort_order(screen, sort.reversed)),
            )
            .push(self.log.view(
                true,
                config,
                manifest,
                &self.duplicate_detector,
                operation,
                histories,
                modifiers,
            ));

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

pub fn other<'a>(
    updating_manifest: bool,
    config: &'a Config,
    cache: &'a Cache,
    operation: &Operation,
    histories: &'a TextHistories,
) -> Element<'a> {
    let is_rclone_valid = config.apps.rclone.is_valid();
    let is_cloud_configured = config.cloud.remote.is_some();
    let is_cloud_path_valid = crate::cloud::validate_cloud_path(&config.cloud.path).is_ok();

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
                        .on_press(Message::Exit { user: true })
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
                    Column::new().spacing(5).push(Text::new(TRANSLATOR.scan_field())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(10)
                                .push_some(|| {
                                    AVAILABLE_PARALELLISM.map(|max_threads| {
                                        Column::new()
                                            .spacing(5)
                                            .push(Checkbox::new(
                                                TRANSLATOR.override_max_threads(),
                                                config.runtime.threads.is_some(),
                                                Message::OverrideMaxThreads,
                                            ))
                                            .push_some(|| {
                                                config.runtime.threads.map(|threads| {
                                                    Container::new(number_input(
                                                        threads.get() as i32,
                                                        TRANSLATOR.threads_label(),
                                                        1..=(max_threads.get() as i32),
                                                        |x| Message::EditedMaxThreads(x as usize),
                                                    ))
                                                    .padding([0, 0, 0, 35])
                                                })
                                            })
                                    })
                                })
                                .push(
                                    Checkbox::new(
                                        TRANSLATOR.explanation_for_exclude_store_screenshots(),
                                        config.backup.filter.exclude_store_screenshots,
                                        Message::EditedExcludeStoreScreenshots,
                                    )
                                    .style(style::Checkbox),
                                )
                                .push(Checkbox::new(
                                    TRANSLATOR.show_deselected_games(),
                                    config.scan.show_deselected_games,
                                    Message::SetShowDeselectedGames,
                                ))
                                .push(Checkbox::new(
                                    TRANSLATOR.show_unchanged_games(),
                                    config.scan.show_unchanged_games,
                                    Message::SetShowUnchangedGames,
                                ))
                                .push(Checkbox::new(
                                    TRANSLATOR.show_unscanned_games(),
                                    config.scan.show_unscanned_games,
                                    Message::SetShowUnscannedGames,
                                )),
                        )
                        .style(style::Container::GameListEntry),
                    ),
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
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .align_items(iced::Alignment::Center)
                                .push(Text::new(TRANSLATOR.cloud_field()).width(100)),
                        )
                        .push(
                            Container::new({
                                let mut column = Column::new().spacing(5).push(
                                    Row::new()
                                        .spacing(20)
                                        .align_items(Alignment::Center)
                                        .push(Text::new(TRANSLATOR.rclone_label()).width(70))
                                        .push(histories.input(UndoSubject::RcloneExecutable))
                                        .push_if(
                                            || !is_rclone_valid,
                                            || Icon::Error.as_text().width(Length::Shrink).style(style::Text::Failure),
                                        )
                                        .push(button::choose_file(BrowseFileSubject::RcloneExecutable))
                                        .push(histories.input(UndoSubject::RcloneArguments)),
                                );

                                if is_rclone_valid {
                                    let choice: RemoteChoice = config.cloud.remote.as_ref().into();
                                    column = column
                                        .push({
                                            let mut row = Row::new()
                                                .spacing(20)
                                                .align_items(Alignment::Center)
                                                .push(Text::new(TRANSLATOR.remote_label()).width(70))
                                                .push_if(
                                                    || !operation.idle(),
                                                    || {
                                                        Text::new(choice.to_string())
                                                            .height(30)
                                                            .vertical_alignment(iced::alignment::Vertical::Center)
                                                    },
                                                )
                                                .push_if(
                                                    || operation.idle(),
                                                    || {
                                                        PickList::new(
                                                            RemoteChoice::ALL,
                                                            Some(choice),
                                                            Message::EditedCloudRemote,
                                                        )
                                                    },
                                                );

                                            if let Some(Remote::Custom { .. }) = &config.cloud.remote {
                                                row = row
                                                    .push(Text::new(TRANSLATOR.remote_name_label()))
                                                    .push(histories.input(UndoSubject::CloudRemoteId));
                                            }

                                            if let Some(description) =
                                                config.cloud.remote.as_ref().and_then(|x| x.description())
                                            {
                                                row = row.push(Text::new(description));
                                            }

                                            row
                                        })
                                        .push_if(
                                            || choice != RemoteChoice::None,
                                            || {
                                                Row::new()
                                                    .spacing(20)
                                                    .align_items(Alignment::Center)
                                                    .push(Text::new(TRANSLATOR.folder_label()).width(70))
                                                    .push(histories.input(UndoSubject::CloudPath))
                                                    .push_if(
                                                        || !is_cloud_path_valid,
                                                        || {
                                                            Icon::Error
                                                                .as_text()
                                                                .width(Length::Shrink)
                                                                .style(style::Text::Failure)
                                                        },
                                                    )
                                            },
                                        )
                                        .push_if(
                                            || is_cloud_configured && is_cloud_path_valid,
                                            || {
                                                Row::new()
                                                    .spacing(20)
                                                    .align_items(Alignment::Center)
                                                    .push(button::upload(operation))
                                                    .push(button::download(operation))
                                                    .push(Checkbox::new(
                                                        TRANSLATOR.synchronize_automatically(),
                                                        config.cloud.synchronize,
                                                        |_| Message::ToggleCloudSynchronize,
                                                    ))
                                            },
                                        )
                                        .push_if(
                                            || !is_cloud_configured,
                                            || Text::new(TRANSLATOR.cloud_not_configured()),
                                        )
                                        .push_if(
                                            || !is_cloud_path_valid,
                                            || {
                                                Text::new(TRANSLATOR.prefix_warning(&TRANSLATOR.cloud_path_invalid()))
                                                    .style(style::Text::Failure)
                                            },
                                        );
                                } else {
                                    column = column
                                        .push(
                                            Text::new(TRANSLATOR.prefix_warning(&TRANSLATOR.rclone_unavailable()))
                                                .style(style::Text::Failure),
                                        )
                                        .push(button::open_url(TRANSLATOR.get_rclone_button(), RCLONE_URL.to_string()));
                                }

                                column
                            })
                            .padding(5)
                            .style(style::Container::GameListEntry),
                        ),
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
