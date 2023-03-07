use std::collections::HashSet;

use crate::{
    cache::Cache,
    config::{Config, Sort, SortKey, ToggledPaths, ToggledRegistry},
    gui::{
        badge::Badge,
        common::{GameAction, IcedButtonExt, IcedExtension, Message, Screen},
        file_tree::FileTree,
        search::SearchComponent,
        style,
    },
    lang::Translator,
    manifest::Manifest,
    prelude::{BackupInfo, DuplicateDetector, OperationStatus, ScanInfo},
};

use crate::gui::widget::{Button, Checkbox, Column, Container, PickList, Row, Text, Tooltip};
use fuzzy_matcher::FuzzyMatcher;
use iced::{alignment::Horizontal as HorizontalAlignment, keyboard::Modifiers, widget::tooltip, Alignment, Length};

use super::common::{OngoingOperation, ScrollSubject};

#[derive(Default)]
pub struct GameListEntry {
    pub scan_info: ScanInfo,
    pub backup_info: Option<BackupInfo>,
    pub selected_backup: Option<String>,
    pub tree: FileTree,
    pub duplicates: usize,
    pub popup_menu: crate::gui::popup_menu::State<GameAction>,
}

impl GameListEntry {
    fn view(
        &self,
        restoring: bool,
        translator: &Translator,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Option<OngoingOperation>,
        expanded: bool,
        modifiers: &Modifiers,
    ) -> Container {
        let successful = match &self.backup_info {
            Some(x) => x.successful(),
            _ => true,
        };

        let scanned = self.scan_info.found_anything();
        let enabled = if restoring {
            config.is_game_enabled_for_restore(&self.scan_info.game_name)
        } else {
            config.is_game_enabled_for_backup(&self.scan_info.game_name)
        };
        let all_items_ignored = self.scan_info.all_ignored();
        let customized = config.is_game_customized(&self.scan_info.game_name);
        let customized_pure = customized && !manifest.0.contains_key(&self.scan_info.game_name);
        let name_for_checkbox = self.scan_info.game_name.clone();
        let operating = operation.is_some();
        let changes = self.scan_info.count_changes();

        Container::new(
            Column::new()
                .padding(5)
                .spacing(5)
                .align_items(Alignment::Center)
                .push(
                    Row::new()
                        .spacing(15)
                        .align_items(Alignment::Center)
                        .push(
                            Checkbox::new("", enabled, move |enabled| Message::ToggleGameListEntryEnabled {
                                name: name_for_checkbox.clone(),
                                enabled,
                                restoring,
                            })
                            .spacing(0)
                            .style(style::Checkbox),
                        )
                        .push(
                            Button::new(
                                Text::new(self.scan_info.game_name.clone())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press_some(if scanned {
                                Some(Message::ToggleGameListEntryExpanded {
                                    name: self.scan_info.game_name.clone(),
                                })
                            } else if !operating {
                                if restoring {
                                    Some(Message::RestoreStart {
                                        preview: true,
                                        games: Some(vec![self.scan_info.game_name.clone()]),
                                    })
                                } else {
                                    Some(Message::BackupStart {
                                        preview: true,
                                        games: Some(vec![self.scan_info.game_name.clone()]),
                                    })
                                }
                            } else {
                                None
                            })
                            .style(if !scanned {
                                style::Button::GameListEntryTitleUnscanned
                            } else if !enabled || all_items_ignored {
                                style::Button::GameListEntryTitleDisabled
                            } else if successful {
                                style::Button::GameListEntryTitle
                            } else {
                                style::Button::GameListEntryTitleFailed
                            })
                            .width(Length::Fill)
                            .padding(2),
                        )
                        .push_some(|| {
                            if changes.brand_new() {
                                Some(Badge::new_entry(translator).view())
                            } else if changes.updated() {
                                Some(Badge::changed_entry(translator).view())
                            } else {
                                None
                            }
                        })
                        .push_if(
                            || self.scan_info.any_ignored(),
                            || {
                                Badge::new(
                                    &translator
                                        .processed_subset(self.scan_info.total_items(), self.scan_info.enabled_items()),
                                )
                                .view()
                            },
                        )
                        .push_if(
                            || duplicate_detector.is_game_duplicated(&self.scan_info),
                            || Badge::new(&translator.badge_duplicates()).view(),
                        )
                        .push_if(|| !successful, || Badge::new(&translator.badge_failed()).view())
                        .push(
                            Row::new()
                                .push_some(|| {
                                    if self.scan_info.available_backups.len() == 1 {
                                        self.scan_info.backup.as_ref().map(|backup| {
                                            Container::new(Text::new(backup.label()).size(18))
                                                .padding([2, 0, 0, 0])
                                                .width(165)
                                                .align_x(HorizontalAlignment::Center)
                                        })
                                    } else if !self.scan_info.available_backups.is_empty() {
                                        if operating {
                                            return self.scan_info.backup.as_ref().map(|backup| {
                                                Container::new(
                                                    Container::new(Text::new(backup.label()).size(15))
                                                        .padding(2)
                                                        .width(165)
                                                        .align_x(HorizontalAlignment::Center)
                                                        .style(style::Container::DisabledBackup),
                                                )
                                                .padding([2, 0, 0, 0])
                                            });
                                        }

                                        let game = self.scan_info.game_name.clone();
                                        let content = Container::new(
                                            PickList::new(
                                                &self.scan_info.available_backups,
                                                self.scan_info.backup.as_ref().cloned(),
                                                move |backup| Message::SelectedBackupToRestore {
                                                    game: game.clone(),
                                                    backup,
                                                },
                                            )
                                            .text_size(15)
                                            .style(style::PickList::Backup),
                                        )
                                        .width(165)
                                        .padding([0, 0, 0, 0])
                                        .align_x(HorizontalAlignment::Center);
                                        Some(content)
                                    } else {
                                        None
                                    }
                                })
                                .push({
                                    let confirm = !modifiers.alt();
                                    let action = if modifiers.shift() {
                                        Some(if restoring {
                                            GameAction::PreviewRestore
                                        } else {
                                            GameAction::PreviewBackup
                                        })
                                    } else if modifiers.command() {
                                        Some(if restoring {
                                            GameAction::Restore { confirm }
                                        } else {
                                            GameAction::Backup { confirm }
                                        })
                                    } else {
                                        None
                                    };
                                    if let Some(action) = action {
                                        let button = Button::new(action.icon().into_text().width(45))
                                            .on_press_if(
                                                || !operating,
                                                || Message::GameAction {
                                                    action,
                                                    game: self.scan_info.game_name.clone(),
                                                },
                                            )
                                            .style(style::Button::GameActionPrimary)
                                            .padding(2);
                                        Container::new(
                                            Tooltip::new(button, action.to_string(), tooltip::Position::Top)
                                                .size(16)
                                                .gap(5)
                                                .style(style::Container::Tooltip),
                                        )
                                    } else {
                                        let options =
                                            GameAction::options(restoring, operating, customized, customized_pure);
                                        let game_name = self.scan_info.game_name.clone();

                                        let menu = crate::gui::popup_menu::PopupMenu::new(options, move |action| {
                                            Message::GameAction {
                                                action,
                                                game: game_name.clone(),
                                            }
                                        })
                                        .style(style::PickList::Popup);
                                        Container::new(menu)
                                    }
                                })
                                .push(
                                    Container::new(Text::new({
                                        let summed = self.scan_info.sum_bytes(&self.backup_info);
                                        if summed == 0 && !self.scan_info.found_anything() {
                                            "".to_string()
                                        } else {
                                            translator.adjusted_size(summed)
                                        }
                                    }))
                                    .width(115)
                                    .center_x(),
                                ),
                        ),
                )
                .push_if(
                    || expanded,
                    || {
                        self.tree
                            .view(translator, &self.scan_info.game_name, config, restoring)
                            .width(Length::Fill)
                    },
                ),
        )
        .style(style::Container::GameListEntry)
    }

    pub fn populate_tree(&mut self, config: &Config, duplicate_detector: &DuplicateDetector) {
        self.tree = FileTree::new(self.scan_info.clone(), config, &self.backup_info, duplicate_detector);
    }

    pub fn clear_tree(&mut self) {
        self.tree = Default::default();
    }
}

#[derive(Default)]
pub struct GameList {
    pub entries: Vec<GameListEntry>,
    pub search: SearchComponent,
    expanded_games: HashSet<String>,
    pub modifiers: Modifiers,
}

impl GameList {
    pub fn view(
        &self,
        restoring: bool,
        translator: &Translator,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Option<OngoingOperation>,
    ) -> Container {
        let use_search = self.search.show;
        let search_game_name = self.search.game_name.clone();

        Container::new(
            Column::new()
                .push_some(|| {
                    self.search.view(
                        if restoring { Screen::Restore } else { Screen::Backup },
                        translator,
                        if restoring {
                            &config.restore.sort
                        } else {
                            &config.backup.sort
                        },
                    )
                })
                .push({
                    let content = self.entries.iter().fold(
                        Column::new().width(Length::Fill).padding([0, 15, 5, 15]).spacing(10),
                        |parent, x| {
                            if !use_search
                                || fuzzy_matcher::skim::SkimMatcherV2::default()
                                    .fuzzy_match(&x.scan_info.game_name, &search_game_name)
                                    .is_some()
                            {
                                parent.push(x.view(
                                    restoring,
                                    translator,
                                    config,
                                    manifest,
                                    duplicate_detector,
                                    operation,
                                    self.expanded_games.contains(&x.scan_info.game_name),
                                    &self.modifiers,
                                ))
                            } else {
                                parent
                            }
                        },
                    );
                    ScrollSubject::game_list(restoring).into_widget(content)
                }),
        )
    }

    pub fn all_entries_selected(&self, config: &Config, restoring: bool) -> bool {
        self.entries.iter().all(|x| {
            if restoring {
                config.is_game_enabled_for_restore(&x.scan_info.game_name)
            } else {
                config.is_game_enabled_for_backup(&x.scan_info.game_name)
            }
        })
    }

    pub fn compute_operation_status(&self, config: &Config, restoring: bool) -> OperationStatus {
        let mut status = OperationStatus::default();
        for entry in self.entries.iter() {
            status.total_games += 1;
            status.total_bytes += entry.scan_info.total_possible_bytes();
            if !entry.scan_info.all_ignored()
                && ((restoring && config.is_game_enabled_for_restore(&entry.scan_info.game_name))
                    || (!restoring && config.is_game_enabled_for_backup(&entry.scan_info.game_name)))
            {
                status.processed_games += 1;
                status.processed_bytes += entry.scan_info.sum_bytes(&None);
            }

            let changes = entry.scan_info.count_changes();
            if changes.brand_new() {
                status.changed_games.new += 1;
            } else if changes.updated() {
                status.changed_games.different += 1;
            } else {
                status.changed_games.same += 1;
            }
        }
        status
    }

    pub fn update_ignored(&mut self, game: &str, ignored_paths: &ToggledPaths, ignored_registry: &ToggledRegistry) {
        for item in self.entries.iter_mut() {
            if item.scan_info.game_name == game {
                item.scan_info.update_ignored(ignored_paths, ignored_registry);
                item.tree.update_ignored(game, ignored_paths, ignored_registry);
            }
        }
    }

    pub fn sort(&mut self, sort: &Sort) {
        match sort.key {
            SortKey::Name => self
                .entries
                .sort_by(|x, y| crate::prelude::compare_games_by_name(&x.scan_info.game_name, &y.scan_info.game_name)),
            SortKey::Size => self.entries.sort_by(|x, y| {
                crate::prelude::compare_games_by_size(&x.scan_info, &x.backup_info, &y.scan_info, &y.backup_info)
            }),
        }
        if sort.reversed {
            self.entries.reverse();
        }
    }

    pub fn toggle_game_expanded(&mut self, game: &str, config: &Config, duplicate_detector: &DuplicateDetector) {
        if self.expanded_games.contains(game) {
            self.expanded_games.remove(game);
            for entry in self.entries.iter_mut() {
                if entry.scan_info.game_name == game {
                    entry.clear_tree();
                    break;
                }
            }
        } else {
            self.expanded_games.insert(game.to_string());
            for entry in self.entries.iter_mut() {
                if entry.scan_info.game_name == game {
                    entry.populate_tree(config, duplicate_detector);
                    break;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.expanded_games.clear();
    }

    pub fn with_recent_games(restoring: bool, config: &Config, cache: &Cache) -> Self {
        let games = if restoring {
            &cache.restore.recent_games
        } else {
            &cache.backup.recent_games
        };
        let sort = if restoring {
            &config.restore.sort
        } else {
            &config.backup.sort
        };

        let mut log = Self::default();
        for game in games {
            log.update_game(
                ScanInfo {
                    game_name: game.clone(),
                    ..Default::default()
                },
                Default::default(),
                sort,
                config,
                &DuplicateDetector::default(),
                &Default::default(),
            );
        }
        log
    }

    pub fn update_game(
        &mut self,
        scan_info: ScanInfo,
        backup_info: Option<BackupInfo>,
        sort: &Sort,
        config: &Config,
        duplicate_detector: &DuplicateDetector,
        duplicates: &std::collections::HashSet<String>,
    ) {
        let mut index = None;
        let game_name = scan_info.game_name.clone();

        for (i, entry) in self.entries.iter().enumerate() {
            if entry.scan_info.game_name == game_name {
                index = Some(i);
                break;
            }
        }

        match index {
            Some(i) => {
                if scan_info.found_anything() {
                    self.entries[i].scan_info = scan_info;
                    self.entries[i].backup_info = backup_info;
                    if self.expanded_games.contains(&game_name) {
                        self.entries[i].populate_tree(config, duplicate_detector);
                    }
                } else {
                    self.entries.remove(i);
                }
            }
            None => {
                let mut entry = GameListEntry {
                    scan_info,
                    backup_info,
                    ..Default::default()
                };
                if self.expanded_games.contains(&game_name) {
                    entry.populate_tree(config, duplicate_detector);
                }
                self.entries.push(entry);
                self.sort(sort);
            }
        }

        if !duplicates.is_empty() {
            for entry in self.entries.iter_mut() {
                if duplicates.contains(&entry.scan_info.game_name)
                    && self.expanded_games.contains(&entry.scan_info.game_name)
                {
                    entry.populate_tree(config, duplicate_detector);
                }
            }
        }
    }

    pub fn remove_game(
        &mut self,
        game: &str,
        config: &Config,
        duplicate_detector: &DuplicateDetector,
        duplicates: &std::collections::HashSet<String>,
    ) {
        self.entries.retain(|entry| entry.scan_info.game_name != game);
        for entry in self.entries.iter_mut() {
            if duplicates.contains(&entry.scan_info.game_name) {
                entry.populate_tree(config, duplicate_detector);
            }
        }
    }

    pub fn unscan_games(&mut self, games: &[String]) {
        for entry in self.entries.iter_mut() {
            if games.contains(&entry.scan_info.game_name) {
                entry.scan_info.found_files.clear();
                entry.scan_info.found_registry_keys.clear();
            }
        }
    }

    pub fn contains_unscanned_games(&self) -> bool {
        self.entries.iter().any(|x| !x.scan_info.found_anything())
    }
}
