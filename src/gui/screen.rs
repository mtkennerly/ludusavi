use std::collections::HashSet;

use iced::{keyboard, padding, Alignment, Length};

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
        widget::{checkbox, number_input, pick_list, text, Button, Column, Container, Element, IcedParentExt, Row},
    },
    lang::{Language, TRANSLATOR},
    prelude::{AVAILABLE_PARALELLISM, STEAM_DECK},
    resource::{
        cache::Cache,
        config::{BackupFormat, CloudFilter, Config, SortKey, Theme, ZipCompression},
        manifest::{Manifest, Store},
    },
    scan::{DuplicateDetector, Duplication, OperationStatus},
};

const RCLONE_URL: &str = "https://rclone.org/downloads";
const RELEASE_URL: &str = "https://github.com/mtkennerly/ludusavi/releases";

fn template(content: Column) -> Element {
    Container::new(content.spacing(15).align_x(Alignment::Center))
        .height(Length::Fill)
        .center_x(Length::Fill)
        .padding(padding::all(5))
        .into()
}

fn make_status_row<'a>(status: &OperationStatus, duplication: Duplication) -> Row<'a> {
    let size = 25;

    Row::new()
        .padding([0, 20])
        .align_y(Alignment::Center)
        .spacing(15)
        .push(text(TRANSLATOR.processed_games(status)).size(size))
        .push_if(status.changed_games.new > 0, || {
            Badge::new_entry_with_count(status.changed_games.new).view()
        })
        .push_if(status.changed_games.different > 0, || {
            Badge::changed_entry_with_count(status.changed_games.different).view()
        })
        .push(text("|").size(size))
        .push(text(TRANSLATOR.processed_bytes(status)).size(size))
        .push_if(!duplication.resolved(), || {
            Badge::new(&TRANSLATOR.badge_duplicates()).view()
        })
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
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
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
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(text(TRANSLATOR.backup_target_label()))
                    .push(histories.input(UndoSubject::BackupTarget))
                    .push(button::choose_folder(BrowseSubject::BackupTarget, modifiers))
                    .push("|")
                    .push(text(TRANSLATOR.sort_label()))
                    .push(
                        pick_list(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                            screen,
                            value,
                        })
                        .class(style::PickList::Primary),
                    )
                    .push(button::sort_order(screen, sort.reversed)),
            )
            .push_if(self.show_settings, || {
                Row::new()
                    .padding([0, 20])
                    .spacing(20)
                    .height(30)
                    .align_y(Alignment::Center)
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
            })
            .push_if(self.show_settings, || {
                Row::new()
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(
                        Row::new()
                            .spacing(5)
                            .align_y(Alignment::Center)
                            .push(text(TRANSLATOR.backup_format_field()))
                            .push(
                                pick_list(
                                    BackupFormat::ALL,
                                    Some(config.backup.format.chosen),
                                    Message::SelectedBackupFormat,
                                )
                                .class(style::PickList::Primary),
                            ),
                    )
                    .push_if(config.backup.format.chosen == BackupFormat::Zip, || {
                        Row::new()
                            .spacing(5)
                            .align_y(Alignment::Center)
                            .push(text(TRANSLATOR.backup_compression_field()))
                            .push(
                                pick_list(
                                    ZipCompression::ALL,
                                    Some(config.backup.format.zip.compression),
                                    Message::SelectedBackupCompression,
                                )
                                .class(style::PickList::Primary),
                            )
                    })
                    .push_maybe(match (config.backup.format.level(), config.backup.format.range()) {
                        (Some(level), Some(range)) => Some(number_input(
                            level,
                            TRANSLATOR.backup_compression_level_field(),
                            range,
                            Message::EditedCompressionLevel,
                        )),
                        _ => None,
                    })
            })
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
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
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
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(text(TRANSLATOR.restore_source_label()))
                    .push(histories.input(UndoSubject::RestoreSource))
                    .push(button::choose_folder(BrowseSubject::RestoreSource, modifiers))
                    .push("|")
                    .push(text(TRANSLATOR.sort_label()))
                    .push(
                        pick_list(SortKey::ALL, Some(sort.key), move |value| Message::EditedSortKey {
                            screen,
                            value,
                        })
                        .class(style::PickList::Primary),
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

#[derive(Default)]
pub struct CustomGames {
    pub filter: editor::CustomGamesFilter,
}

impl CustomGames {
    pub fn view<'a>(
        &self,
        config: &Config,
        manifest: &Manifest,
        operating: bool,
        histories: &TextHistories,
        modifiers: &keyboard::Modifiers,
    ) -> Element<'a> {
        let content = Column::new()
            .push(
                Row::new()
                    .padding([0, 20])
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(button::add_game())
                    .push(button::toggle_all_custom_games(config.are_all_custom_games_enabled()))
                    .push(button::sort(Message::SortCustomGames))
                    .push(button::filter(Screen::CustomGames, self.filter.enabled)),
            )
            .push_maybe(self.filter.enabled.then(|| {
                Row::new()
                    .padding(padding::left(20).right(20))
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(text(TRANSLATOR.filter_label()))
                    .push(histories.input(UndoSubject::CustomGamesSearchGameName))
            }))
            .push(editor::custom_games(
                config,
                manifest,
                operating,
                histories,
                modifiers,
                &self.filter,
            ));

        template(content)
    }
}

pub fn other<'a>(
    updating_manifest: bool,
    config: &'a Config,
    cache: &'a Cache,
    operation: &Operation,
    histories: &'a TextHistories,
    modifiers: &keyboard::Modifiers,
) -> Element<'a> {
    let is_rclone_valid = config.apps.rclone.is_valid();
    let is_cloud_configured = config.cloud.remote.is_some();
    let is_cloud_path_valid = crate::cloud::validate_cloud_path(&config.cloud.path).is_ok();

    let content = Column::new()
        .push_if(*STEAM_DECK, || {
            Row::new()
                .padding([0, 20])
                .spacing(20)
                .align_y(iced::Alignment::Center)
                .push(
                    Button::new(text(TRANSLATOR.exit_button()).align_x(iced::alignment::Horizontal::Center))
                        .on_press(Message::Exit { user: true })
                        .width(125)
                        .class(style::Button::Negative)
                        .padding(5),
                )
        })
        .push({
            let content = Column::new()
                .spacing(20)
                .padding(padding::top(0).bottom(5).left(15).right(15))
                .width(Length::Fill)
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(text(TRANSLATOR.field_language()))
                        .push(
                            pick_list(Language::ALL, Some(config.language), Message::SelectedLanguage)
                                .class(style::PickList::Primary),
                        ),
                )
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(text(TRANSLATOR.field_theme()))
                        .push(
                            pick_list(Theme::ALL, Some(config.theme), Message::SelectedTheme)
                                .class(style::PickList::Primary),
                        ),
                )
                .push(
                    Row::new()
                        .align_y(iced::Alignment::Center)
                        .spacing(20)
                        .push(checkbox(
                            TRANSLATOR.new_version_check(),
                            config.release.check,
                            Message::AppReleaseToggle,
                        ))
                        .push(button::open_url_icon(RELEASE_URL.to_string())),
                )
                .push(
                    Column::new().spacing(5).push(text(TRANSLATOR.scan_field())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(10)
                                .push_maybe({
                                    AVAILABLE_PARALELLISM.map(|max_threads| {
                                        Column::new()
                                            .spacing(5)
                                            .push(checkbox(
                                                TRANSLATOR.override_max_threads(),
                                                config.runtime.threads.is_some(),
                                                Message::OverrideMaxThreads,
                                            ))
                                            .push_maybe({
                                                config.runtime.threads.map(|threads| {
                                                    Container::new(number_input(
                                                        threads.get() as i32,
                                                        TRANSLATOR.threads_label(),
                                                        1..=(max_threads.get() as i32),
                                                        |x| Message::EditedMaxThreads(x as usize),
                                                    ))
                                                    .padding(padding::left(35))
                                                })
                                            })
                                    })
                                })
                                .push(
                                    checkbox(
                                        TRANSLATOR.explanation_for_exclude_store_screenshots(),
                                        config.backup.filter.exclude_store_screenshots,
                                        Message::EditedExcludeStoreScreenshots,
                                    )
                                    .class(style::Checkbox),
                                )
                                .push(checkbox(
                                    TRANSLATOR.show_deselected_games(),
                                    config.scan.show_deselected_games,
                                    Message::SetShowDeselectedGames,
                                ))
                                .push(checkbox(
                                    TRANSLATOR.show_unchanged_games(),
                                    config.scan.show_unchanged_games,
                                    Message::SetShowUnchangedGames,
                                ))
                                .push(checkbox(
                                    TRANSLATOR.show_unscanned_games(),
                                    config.scan.show_unscanned_games,
                                    Message::SetShowUnscannedGames,
                                ))
                                .push(checkbox(
                                    TRANSLATOR.field(&TRANSLATOR.explanation_for_exclude_cloud_games()),
                                    config.backup.filter.cloud.exclude,
                                    |exclude| {
                                        Message::EditedCloudFilter(CloudFilter {
                                            exclude,
                                            ..config.backup.filter.cloud
                                        })
                                    },
                                ))
                                .push(
                                    Row::new()
                                        .padding(padding::left(35))
                                        .spacing(10)
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Epic),
                                                config.backup.filter.cloud.epic,
                                                |epic| {
                                                    Message::EditedCloudFilter(CloudFilter {
                                                        epic,
                                                        ..config.backup.filter.cloud
                                                    })
                                                },
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Gog),
                                                config.backup.filter.cloud.gog,
                                                |gog| {
                                                    Message::EditedCloudFilter(CloudFilter {
                                                        gog,
                                                        ..config.backup.filter.cloud
                                                    })
                                                },
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                format!(
                                                    "{} / {}",
                                                    TRANSLATOR.store(&Store::Origin),
                                                    TRANSLATOR.store(&Store::Ea)
                                                ),
                                                config.backup.filter.cloud.origin,
                                                |origin| {
                                                    Message::EditedCloudFilter(CloudFilter {
                                                        origin,
                                                        ..config.backup.filter.cloud
                                                    })
                                                },
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Steam),
                                                config.backup.filter.cloud.steam,
                                                |steam| {
                                                    Message::EditedCloudFilter(CloudFilter {
                                                        steam,
                                                        ..config.backup.filter.cloud
                                                    })
                                                },
                                            )
                                            .class(style::Checkbox),
                                        )
                                        .push(
                                            checkbox(
                                                TRANSLATOR.store(&Store::Uplay),
                                                config.backup.filter.cloud.uplay,
                                                |uplay| {
                                                    Message::EditedCloudFilter(CloudFilter {
                                                        uplay,
                                                        ..config.backup.filter.cloud
                                                    })
                                                },
                                            )
                                            .class(style::Checkbox),
                                        ),
                                ),
                        )
                        .class(style::Container::GameListEntry),
                    ),
                )
                .push(
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .align_y(iced::Alignment::Center)
                                .push(text(TRANSLATOR.manifest_label()).width(100))
                                .push(button::refresh(
                                    Message::UpdateManifest { force: true },
                                    updating_manifest,
                                )),
                        )
                        .push(editor::manifest(config, cache, histories, modifiers).padding(padding::top(10))),
                )
                .push(
                    Column::new()
                        .spacing(5)
                        .push(
                            Row::new()
                                .align_y(iced::Alignment::Center)
                                .push(text(TRANSLATOR.cloud_field()).width(100)),
                        )
                        .push(
                            Container::new({
                                let mut column = Column::new().spacing(5).push(
                                    Row::new()
                                        .spacing(20)
                                        .align_y(Alignment::Center)
                                        .push(text(TRANSLATOR.rclone_label()).width(70))
                                        .push(histories.input(UndoSubject::RcloneExecutable))
                                        .push_if(!is_rclone_valid, || {
                                            Icon::Error.text().width(Length::Shrink).class(style::Text::Failure)
                                        })
                                        .push(button::choose_file(BrowseFileSubject::RcloneExecutable, modifiers))
                                        .push(histories.input(UndoSubject::RcloneArguments)),
                                );

                                if is_rclone_valid {
                                    let choice: RemoteChoice = config.cloud.remote.as_ref().into();
                                    column = column
                                        .push({
                                            let mut row = Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Center)
                                                .push(text(TRANSLATOR.remote_label()).width(70))
                                                .push_if(!operation.idle(), || {
                                                    text(choice.to_string())
                                                        .height(30)
                                                        .align_y(iced::alignment::Vertical::Center)
                                                })
                                                .push_if(operation.idle(), || {
                                                    pick_list(
                                                        RemoteChoice::ALL,
                                                        Some(choice),
                                                        Message::EditedCloudRemote,
                                                    )
                                                });

                                            if let Some(Remote::Custom { .. }) = &config.cloud.remote {
                                                row = row
                                                    .push(text(TRANSLATOR.remote_name_label()))
                                                    .push(histories.input(UndoSubject::CloudRemoteId));
                                            }

                                            if let Some(description) =
                                                config.cloud.remote.as_ref().and_then(|x| x.description())
                                            {
                                                row = row.push(text(description));
                                            }

                                            row
                                        })
                                        .push_if(choice != RemoteChoice::None, || {
                                            Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Center)
                                                .push(text(TRANSLATOR.folder_label()).width(70))
                                                .push(histories.input(UndoSubject::CloudPath))
                                                .push_if(!is_cloud_path_valid, || {
                                                    Icon::Error.text().width(Length::Shrink).class(style::Text::Failure)
                                                })
                                        })
                                        .push_if(is_cloud_configured && is_cloud_path_valid, || {
                                            Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Center)
                                                .push(button::upload(operation))
                                                .push(button::download(operation))
                                                .push(checkbox(
                                                    TRANSLATOR.synchronize_automatically(),
                                                    config.cloud.synchronize,
                                                    |_| Message::ToggleCloudSynchronize,
                                                ))
                                        })
                                        .push_if(!is_cloud_configured, || text(TRANSLATOR.cloud_not_configured()))
                                        .push_if(!is_cloud_path_valid, || {
                                            text(TRANSLATOR.prefix_warning(&TRANSLATOR.cloud_path_invalid()))
                                                .class(style::Text::Failure)
                                        });
                                } else {
                                    column = column
                                        .push(
                                            text(TRANSLATOR.prefix_warning(&TRANSLATOR.rclone_unavailable()))
                                                .class(style::Text::Failure),
                                        )
                                        .push(button::open_url(TRANSLATOR.get_rclone_button(), RCLONE_URL.to_string()));
                                }

                                column
                            })
                            .padding(5)
                            .class(style::Container::GameListEntry),
                        ),
                )
                .push(
                    Column::new().spacing(5).push(text(TRANSLATOR.roots_label())).push(
                        Container::new(
                            Column::new()
                                .padding(5)
                                .spacing(4)
                                .push(editor::root(config, histories, modifiers)),
                        )
                        .class(style::Container::GameListEntry),
                    ),
                )
                .push(
                    Column::new()
                        .push(text(TRANSLATOR.ignored_items_label()))
                        .push(editor::ignored_items(config, histories, modifiers).padding(padding::top(10))),
                )
                .push(
                    Column::new()
                        .push(text(TRANSLATOR.redirects_label()))
                        .push(editor::redirect(config, histories, modifiers).padding(padding::top(10))),
                );
            ScrollSubject::Other.into_widget(content)
        });

    template(content)
}
