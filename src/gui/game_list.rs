use std::collections::HashSet;

use iced::{alignment::Horizontal as HorizontalAlignment, keyboard::Modifiers, widget::tooltip, Alignment, Length};

use crate::{
    gui::{
        badge::Badge,
        button,
        common::{GameAction, Message, OngoingOperation, Screen, ScrollSubject},
        file_tree::FileTree,
        icon::Icon,
        search::FilterComponent,
        shortcuts::TextHistories,
        style,
        widget::{
            Button, Checkbox, Column, Container, IcedButtonExt, IcedParentExt, PickList, Row, Text, TextInput, Tooltip,
        },
    },
    lang::TRANSLATOR,
    resource::{
        cache::Cache,
        config::{Config, Sort, ToggledPaths, ToggledRegistry},
        manifest::Manifest,
    },
    scan::{layout::GameLayout, BackupInfo, DuplicateDetector, OperationStatus, ScanChange, ScanInfo},
};

#[derive(Default)]
pub struct GameListEntry {
    pub scan_info: ScanInfo,
    pub backup_info: Option<BackupInfo>,
    pub selected_backup: Option<String>,
    pub tree: Option<FileTree>,
    pub popup_menu: crate::gui::popup_menu::State<GameAction>,
    pub show_comment_editor: bool,
    pub game_layout: Option<GameLayout>,
}

impl GameListEntry {
    fn view(
        &self,
        restoring: bool,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Option<OngoingOperation>,
        expanded: bool,
        modifiers: &Modifiers,
        filtering_duplicates: bool,
    ) -> Container {
        let successful = match &self.backup_info {
            Some(x) => x.successful(),
            _ => true,
        };

        let scanned = self.scan_info.found_anything();
        let enabled = config.is_game_enabled_for_operation(&self.scan_info.game_name, restoring);
        let all_items_ignored = self.scan_info.all_ignored();
        let customized = config.is_game_customized(&self.scan_info.game_name);
        let customized_pure = customized && !manifest.0.contains_key(&self.scan_info.game_name);
        let name_for_checkbox = self.scan_info.game_name.clone();
        let name_for_comment = self.scan_info.game_name.clone();
        let name_for_comment2 = self.scan_info.game_name.clone();
        let name_for_duplicate_toggle = self.scan_info.game_name.clone();
        let operating = operation.is_some();
        let changes = self.scan_info.count_changes();
        let duplication = duplicate_detector.is_game_duplicated(&self.scan_info.game_name);

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
                        .push_some(|| match changes.overall() {
                            ScanChange::New => Some(Badge::new_entry().view()),
                            ScanChange::Different => Some(Badge::changed_entry().view()),
                            ScanChange::Removed => None,
                            ScanChange::Same => None,
                            ScanChange::Unknown => None,
                        })
                        .push_if(
                            || self.scan_info.any_ignored(),
                            || {
                                Badge::new(
                                    &TRANSLATOR
                                        .processed_subset(self.scan_info.total_items(), self.scan_info.enabled_items()),
                                )
                                .view()
                            },
                        )
                        .push_if(
                            || !duplication.unique(),
                            || {
                                Badge::new(&TRANSLATOR.badge_duplicates())
                                    .faded(duplication.resolved())
                                    .on_press(Message::FilterDuplicates {
                                        restoring,
                                        game: (!filtering_duplicates).then_some(name_for_duplicate_toggle),
                                    })
                                    .view()
                            },
                        )
                        .push_if(|| !successful, || Badge::new(&TRANSLATOR.badge_failed()).view())
                        .push_some(|| {
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.comment().as_ref())
                                .map(|comment| {
                                    Tooltip::new(
                                        Icon::Comment.as_text().width(Length::Shrink),
                                        comment,
                                        tooltip::Position::Top,
                                    )
                                    .size(16)
                                    .gap(5)
                                    .style(style::Container::Tooltip)
                                })
                        })
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
                                        let options = GameAction::options(
                                            restoring,
                                            operating,
                                            customized,
                                            customized_pure,
                                            self.scan_info.backup.is_some(),
                                        );
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
                                        let summed = self.scan_info.sum_bytes(self.backup_info.as_ref());
                                        if summed == 0 && !self.scan_info.found_anything() {
                                            "".to_string()
                                        } else {
                                            TRANSLATOR.adjusted_size(summed)
                                        }
                                    }))
                                    .width(115)
                                    .center_x(),
                                ),
                        ),
                )
                .push_some(move || {
                    if !self.show_comment_editor {
                        return None;
                    }
                    let comment = self
                        .scan_info
                        .backup
                        .as_ref()
                        .and_then(|x| x.comment().as_ref())
                        .map(|x| x.as_str())
                        .unwrap_or_else(|| "");
                    Some(
                        Row::new()
                            .align_items(Alignment::Center)
                            .padding([0, 20])
                            .spacing(20)
                            .push(Text::new(TRANSLATOR.comment_label()))
                            .push(TextInput::new(&TRANSLATOR.comment_label(), comment, move |value| {
                                Message::EditedBackupComment {
                                    game: name_for_comment.clone(),
                                    comment: value,
                                }
                            }))
                            .push(button::close(Message::GameAction {
                                action: GameAction::Comment,
                                game: name_for_comment2,
                            })),
                    )
                })
                .push_some(|| {
                    expanded
                        .then(|| {
                            self.tree.as_ref().map(|tree| {
                                tree.view(&self.scan_info.game_name, config, restoring)
                                    .width(Length::Fill)
                            })
                        })
                        .flatten()
                }),
        )
        .style(style::Container::GameListEntry)
    }

    pub fn refresh_tree(&mut self, config: &Config, duplicate_detector: &DuplicateDetector) {
        match self.tree.as_mut() {
            Some(tree) => tree.reset_nodes(self.scan_info.clone(), config, &self.backup_info, duplicate_detector),
            None => {
                self.tree = Some(FileTree::new(
                    self.scan_info.clone(),
                    config,
                    &self.backup_info,
                    duplicate_detector,
                ))
            }
        }
    }

    pub fn clear_tree(&mut self) {
        if let Some(tree) = self.tree.as_mut() {
            tree.clear_nodes();
        }
    }
}

#[derive(Default)]
pub struct GameList {
    pub entries: Vec<GameListEntry>,
    pub search: FilterComponent,
    expanded_games: HashSet<String>,
    pub modifiers: Modifiers,
    pub filter_duplicates_of: Option<String>,
}

impl GameList {
    pub fn view(
        &self,
        restoring: bool,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Option<OngoingOperation>,
        histories: &TextHistories,
    ) -> Container {
        let duplicatees = self.filter_duplicates_of.as_ref().and_then(|game| {
            let mut duplicatees = duplicate_detector.duplicate_games(game);
            if duplicatees.is_empty() {
                None
            } else {
                duplicatees.insert(game.clone());
                Some(duplicatees)
            }
        });

        Container::new(
            Column::new()
                .push_some(|| {
                    self.search.view(
                        if restoring { Screen::Restore } else { Screen::Backup },
                        histories,
                        config.scan.show_deselected_games,
                    )
                })
                .push({
                    let content = self
                        .entries
                        .iter()
                        .filter(|x| {
                            config.should_show_game(
                                &x.scan_info.game_name,
                                restoring,
                                x.scan_info.is_changed(),
                                x.scan_info.found_anything(),
                            )
                        })
                        .filter(|x| {
                            !self.search.show
                                || self.search.qualifies(
                                    &x.scan_info,
                                    config.is_game_enabled_for_operation(&x.scan_info.game_name, restoring),
                                    duplicate_detector.is_game_duplicated(&x.scan_info.game_name),
                                    config.scan.show_deselected_games,
                                )
                        })
                        .filter(|x| {
                            duplicatees
                                .as_ref()
                                .map(|xs| xs.contains(&x.scan_info.game_name))
                                .unwrap_or(true)
                        })
                        .fold(
                            Column::new().width(Length::Fill).padding([0, 15, 5, 15]).spacing(10),
                            |parent, x| {
                                parent.push(x.view(
                                    restoring,
                                    config,
                                    manifest,
                                    duplicate_detector,
                                    operation,
                                    self.expanded_games.contains(&x.scan_info.game_name),
                                    &self.modifiers,
                                    duplicatees.is_some(),
                                ))
                            },
                        );
                    ScrollSubject::game_list(restoring).into_widget(content)
                }),
        )
    }

    pub fn all_entries_selected(&self, config: &Config, restoring: bool) -> bool {
        self.entries
            .iter()
            .all(|x| config.is_game_enabled_for_operation(&x.scan_info.game_name, restoring))
    }

    pub fn compute_operation_status(&self, config: &Config, restoring: bool) -> OperationStatus {
        let mut status = OperationStatus::default();
        for entry in self.entries.iter() {
            status.total_games += 1;
            status.total_bytes += entry.scan_info.total_possible_bytes();
            if !entry.scan_info.all_ignored()
                && config.is_game_enabled_for_operation(&entry.scan_info.game_name, restoring)
            {
                status.processed_games += 1;
                status.processed_bytes += entry.scan_info.sum_bytes(None);
            }

            status.changed_games.add(entry.scan_info.count_changes().overall());
        }
        status
    }

    pub fn update_ignored(&mut self, game: &str, ignored_paths: &ToggledPaths, ignored_registry: &ToggledRegistry) {
        for item in self.entries.iter_mut() {
            if item.scan_info.game_name == game {
                item.scan_info.update_ignored(ignored_paths, ignored_registry);
                if let Some(tree) = item.tree.as_mut() {
                    tree.update_ignored(game, ignored_paths, ignored_registry);
                }
            }
        }
    }

    pub fn sort(&mut self, sort: &Sort) {
        self.entries.sort_by(|x, y| {
            crate::scan::compare_games(
                sort.key,
                &x.scan_info,
                x.backup_info.as_ref(),
                &y.scan_info,
                y.backup_info.as_ref(),
            )
        });
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
                    entry.refresh_tree(config, duplicate_detector);
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
                None,
            );
        }
        log
    }

    pub fn find_game(&self, game: &str) -> Option<usize> {
        let mut index = None;

        for (i, entry) in self.entries.iter().enumerate() {
            if entry.scan_info.game_name == game {
                index = Some(i);
                break;
            }
        }

        index
    }

    pub fn update_game(
        &mut self,
        scan_info: ScanInfo,
        backup_info: Option<BackupInfo>,
        sort: &Sort,
        config: &Config,
        duplicate_detector: &DuplicateDetector,
        duplicates: &HashSet<String>,
        game_layout: Option<GameLayout>,
    ) {
        let game_name = scan_info.game_name.clone();
        let index = self.find_game(&game_name);

        match index {
            Some(i) => {
                if scan_info.found_anything() {
                    self.entries[i].scan_info = scan_info;
                    self.entries[i].backup_info = backup_info;
                    self.entries[i].game_layout = game_layout;
                    if self.expanded_games.contains(&game_name) {
                        self.entries[i].refresh_tree(config, duplicate_detector);
                    }
                } else {
                    self.entries.remove(i);
                }
            }
            None => {
                let mut entry = GameListEntry {
                    scan_info,
                    backup_info,
                    game_layout,
                    ..Default::default()
                };
                if self.expanded_games.contains(&game_name) {
                    entry.refresh_tree(config, duplicate_detector);
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
                    entry.refresh_tree(config, duplicate_detector);
                }
            }
        }
    }

    pub fn refresh_game_tree(
        &mut self,
        game: &str,
        config: &Config,
        duplicate_detector: &mut DuplicateDetector,
        restoring: bool,
    ) {
        let mut affected_games = duplicate_detector.duplicate_games(game);
        affected_games.insert(game.to_string());

        // Can't toggle restore items.
        if !restoring {
            self.update_ignored(game, &config.backup.toggled_paths, &config.backup.toggled_registry);
        }

        if let Some(index) = self.find_game(game) {
            duplicate_detector.add_game(
                &self.entries[index].scan_info,
                config.is_game_enabled_for_operation(game, restoring),
            );
            affected_games.extend(duplicate_detector.duplicate_games(game));
        }

        for entry in &mut self.entries {
            if affected_games.contains(&entry.scan_info.game_name) {
                entry.refresh_tree(config, duplicate_detector);
            }
        }
    }

    pub fn remove_game(
        &mut self,
        game: &str,
        config: &Config,
        duplicate_detector: &DuplicateDetector,
        duplicates: &HashSet<String>,
    ) {
        self.entries.retain(|entry| entry.scan_info.game_name != game);
        for entry in self.entries.iter_mut() {
            if duplicates.contains(&entry.scan_info.game_name) {
                entry.refresh_tree(config, duplicate_detector);
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

    pub fn toggle_backup_comment_editor(&mut self, game: &str) {
        let index = self.find_game(game);

        if let Some(i) = index {
            self.entries[i].show_comment_editor = !self.entries[i].show_comment_editor;
        }
    }

    pub fn set_comment(&mut self, game: &str, comment: String) {
        let Some(index) = self.find_game(game) else { return };
        let entry = &mut self.entries[index];
        let Some(backup) = &mut entry.scan_info.backup else { return };
        let Some(layout) = &mut entry.game_layout else { return };

        layout.set_backup_comment(backup.name(), &comment);
        backup.set_comment(comment);
        layout.save();
    }
}
