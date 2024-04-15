use std::collections::HashSet;

use iced::{alignment::Horizontal as HorizontalAlignment, keyboard::Modifiers, widget::tooltip, Alignment, Length};

use crate::{
    gui::{
        badge::Badge,
        button,
        common::{BackupPhase, GameAction, Message, Operation, RestorePhase, Screen, ScrollSubject, UndoSubject},
        file_tree::FileTree,
        icon::Icon,
        search::FilterComponent,
        shortcuts::TextHistories,
        style,
        widget::{checkbox, pick_list, text, Button, Column, Container, IcedButtonExt, IcedParentExt, Row, Tooltip},
    },
    lang::TRANSLATOR,
    resource::{
        cache::Cache,
        config::{Config, Sort},
        manifest::{Manifest, Os},
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
    /// The `scan_info` gets mutated in response to things like toggling saves off,
    /// so we need a persistent flag to say if the game has been scanned yet.
    pub scanned: bool,
}

impl GameListEntry {
    fn view(
        &self,
        restoring: bool,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Operation,
        expanded: bool,
        modifiers: &Modifiers,
        filtering_duplicates: bool,
        histories: &TextHistories,
    ) -> Container {
        let successful = match &self.backup_info {
            Some(x) => x.successful(),
            _ => true,
        };

        let enabled = config.is_game_enabled_for_operation(&self.scan_info.game_name, restoring);
        let all_items_ignored = self.scan_info.all_ignored();
        let customized = config.is_game_customized(&self.scan_info.game_name);
        let customized_pure = customized && !manifest.0.contains_key(&self.scan_info.game_name);
        let name = self.scan_info.game_name.clone();
        let operating = !operation.idle();
        let changes = self.scan_info.overall_change();
        let duplication = duplicate_detector.is_game_duplicated(&self.scan_info.game_name);
        let display_name = config.display_name(&self.scan_info.game_name);

        Container::new(
            Column::new()
                .padding(5)
                .spacing(5)
                .align_items(Alignment::Center)
                .push(
                    Row::new()
                        .spacing(15)
                        .align_items(Alignment::Center)
                        .push({
                            let name = name.clone();
                            checkbox("", enabled, move |enabled| Message::ToggleGameListEntryEnabled {
                                name: name.clone(),
                                enabled,
                                restoring,
                            })
                            .spacing(0)
                            .style(style::Checkbox)
                        })
                        .push(
                            Button::new(
                                text(display_name.to_string()).horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press_maybe(if self.scanned {
                                Some(Message::ToggleGameListEntryExpanded {
                                    name: self.scan_info.game_name.clone(),
                                })
                            } else if !operating {
                                if restoring {
                                    Some(Message::Restore(RestorePhase::Start {
                                        preview: true,
                                        games: Some(vec![self.scan_info.game_name.clone()]),
                                    }))
                                } else {
                                    Some(Message::Backup(BackupPhase::Start {
                                        preview: true,
                                        repair: false,
                                        games: Some(vec![self.scan_info.game_name.clone()]),
                                    }))
                                }
                            } else {
                                None
                            })
                            .style(if !self.scanned {
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
                        .push_maybe(match changes {
                            ScanChange::New => Some(Badge::new_entry().view()),
                            ScanChange::Different => Some(Badge::changed_entry().view()),
                            ScanChange::Removed => None,
                            ScanChange::Same => None,
                            ScanChange::Unknown => None,
                        })
                        .push_if(self.scan_info.any_ignored(), || {
                            Badge::new(
                                &TRANSLATOR
                                    .processed_subset(self.scan_info.total_items(), self.scan_info.enabled_items()),
                            )
                            .view()
                        })
                        .push_if(!duplication.unique(), || {
                            Badge::new(&TRANSLATOR.badge_duplicates())
                                .faded(duplication.resolved())
                                .on_press(Message::FilterDuplicates {
                                    restoring,
                                    game: (!filtering_duplicates).then_some(name.clone()),
                                })
                                .view()
                        })
                        .push_if(customized, || {
                            Badge::new(&TRANSLATOR.custom_label().to_uppercase())
                                .on_press(Message::ShowCustomGame { name: name.clone() })
                                .view()
                        })
                        .push_if(display_name != name, || {
                            Badge::new(&TRANSLATOR.alias_label().to_uppercase())
                                .on_press(Message::ShowCustomGame {
                                    name: display_name.to_string(),
                                })
                                .view()
                        })
                        .push_if(!successful, || Badge::new(&TRANSLATOR.badge_failed()).view())
                        .push_maybe({
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.comment())
                                .map(|comment| {
                                    Tooltip::new(
                                        Icon::Comment.text().width(Length::Shrink),
                                        text(comment).size(16),
                                        tooltip::Position::Top,
                                    )
                                    .gap(5)
                                    .style(style::Container::Tooltip)
                                })
                        })
                        .push_maybe({
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.os())
                                .and_then(|os| {
                                    (os != Os::HOST && os != Os::Other).then(|| Badge::new(&format!("{os:?}")).view())
                                })
                        })
                        .push_maybe({
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.locked().then_some(Icon::Lock.text().width(Length::Shrink)))
                        })
                        .push(
                            Row::new()
                                .push_maybe({
                                    if self.scan_info.available_backups.len() == 1 {
                                        self.scan_info.backup.as_ref().map(|backup| {
                                            Container::new(
                                                text(backup.label())
                                                    .size(14)
                                                    .line_height(1.1)
                                                    .horizontal_alignment(HorizontalAlignment::Center),
                                            )
                                            .padding([2, 2, 0, 0])
                                            .width(150)
                                            .height(20)
                                            .center_x()
                                            .center_y()
                                        })
                                    } else if !self.scan_info.available_backups.is_empty() {
                                        if operating {
                                            self.scan_info.backup.as_ref().map(|backup| {
                                                Container::new(
                                                    Container::new(
                                                        text(backup.label())
                                                            .size(14)
                                                            .horizontal_alignment(HorizontalAlignment::Center),
                                                    )
                                                    .padding([2, 0, 0, 0])
                                                    .width(148)
                                                    .height(25)
                                                    .center_x()
                                                    .center_y()
                                                    .style(style::Container::DisabledBackup),
                                                )
                                                .padding([0, 2, 0, 0])
                                            })
                                        } else {
                                            let game = self.scan_info.game_name.clone();
                                            let content = Container::new(
                                                pick_list(
                                                    self.scan_info.available_backups.clone(),
                                                    self.scan_info.backup.as_ref().cloned(),
                                                    move |backup| Message::SelectedBackupToRestore {
                                                        game: game.clone(),
                                                        backup,
                                                    },
                                                )
                                                .text_size(12)
                                                .width(Length::Fill)
                                                .style(style::PickList::Backup),
                                            )
                                            .width(150)
                                            .height(25)
                                            .padding([0, 2, 0, 0])
                                            .center_x()
                                            .center_y();
                                            Some(content)
                                        }
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
                                        let button = Button::new(action.icon().text().width(45))
                                            .on_press_if(!operating, || Message::GameAction {
                                                action,
                                                game: self.scan_info.game_name.clone(),
                                            })
                                            .style(style::Button::GameActionPrimary)
                                            .padding(2);
                                        Container::new(
                                            Tooltip::new(
                                                button,
                                                text(action.to_string()).size(16),
                                                tooltip::Position::Top,
                                            )
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
                                            self.scan_info
                                                .backup
                                                .as_ref()
                                                .map(|backup| backup.locked())
                                                .unwrap_or_default(),
                                        );
                                        let game_name = self.scan_info.game_name.clone();

                                        let menu = crate::gui::popup_menu::PopupMenu::new(options, move |action| {
                                            Message::GameAction {
                                                action,
                                                game: game_name.clone(),
                                            }
                                        })
                                        .width(49)
                                        .style(style::PickList::Popup);
                                        Container::new(menu)
                                    }
                                })
                                .push(
                                    Container::new(text({
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
                .push_if(self.show_comment_editor, || {
                    Row::new()
                        .align_items(Alignment::Center)
                        .padding([0, 20])
                        .spacing(20)
                        .push(text(TRANSLATOR.comment_label()))
                        .push(histories.input(UndoSubject::BackupComment(self.scan_info.game_name.clone())))
                        .push(button::hide(Message::GameAction {
                            action: GameAction::Comment,
                            game: name.clone(),
                        }))
                })
                .push_maybe({
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

    pub fn refresh_tree(&mut self, duplicate_detector: &DuplicateDetector, config: &Config, restoring: bool) {
        match self.tree.as_mut() {
            Some(tree) => tree.reset_nodes(
                self.scan_info.clone(),
                &self.backup_info,
                duplicate_detector,
                config,
                restoring,
            ),
            None => {
                self.tree = Some(FileTree::new(
                    self.scan_info.clone(),
                    &self.backup_info,
                    duplicate_detector,
                    config,
                    restoring,
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
    pub filter_duplicates_of: Option<String>,
}

impl GameList {
    pub fn view(
        &self,
        restoring: bool,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Operation,
        histories: &TextHistories,
        modifiers: &Modifiers,
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
                .push_maybe({
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
                                x.scan_info.overall_change().is_changed(),
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
                            Column::new().width(Length::Fill).padding([0, 15, 5, 15]).spacing(5),
                            |parent, x| {
                                parent.push(x.view(
                                    restoring,
                                    config,
                                    manifest,
                                    duplicate_detector,
                                    operation,
                                    self.expanded_games.contains(&x.scan_info.game_name),
                                    modifiers,
                                    duplicatees.is_some(),
                                    histories,
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

            status.changed_games.add(entry.scan_info.overall_change());
        }
        status
    }

    pub fn sort(&mut self, sort: &Sort, config: &Config) {
        self.entries.sort_by(|x, y| {
            crate::scan::compare_games(
                sort.key,
                config.display_name(&x.scan_info.game_name),
                &x.scan_info,
                x.backup_info.as_ref(),
                config.display_name(&y.scan_info.game_name),
                &y.scan_info,
                y.backup_info.as_ref(),
            )
        });
        if sort.reversed {
            self.entries.reverse();
        }
    }

    pub fn toggle_game_expanded(
        &mut self,
        game: &str,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        restoring: bool,
    ) {
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
                    entry.refresh_tree(duplicate_detector, config, restoring);
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
                &DuplicateDetector::default(),
                &Default::default(),
                None,
                config,
                restoring,
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
        duplicate_detector: &DuplicateDetector,
        duplicates: &HashSet<String>,
        game_layout: Option<GameLayout>,
        config: &Config,
        restoring: bool,
    ) {
        let game_name = scan_info.game_name.clone();
        let index = self.find_game(&game_name);
        let scanned = scan_info.found_anything();

        match index {
            Some(i) => {
                if scan_info.can_report_game() {
                    self.entries[i].scan_info = scan_info;
                    self.entries[i].backup_info = backup_info;
                    self.entries[i].game_layout = game_layout;
                    self.entries[i].scanned = scanned || self.entries[i].scanned;
                    if self.expanded_games.contains(&game_name) {
                        self.entries[i].refresh_tree(duplicate_detector, config, restoring);
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
                    scanned,
                    ..Default::default()
                };
                if self.expanded_games.contains(&game_name) {
                    entry.refresh_tree(duplicate_detector, config, restoring);
                }
                self.entries.push(entry);
                self.sort(sort, config);
            }
        }

        if !duplicates.is_empty() {
            for entry in self.entries.iter_mut() {
                if duplicates.contains(&entry.scan_info.game_name)
                    && self.expanded_games.contains(&entry.scan_info.game_name)
                {
                    entry.refresh_tree(duplicate_detector, config, restoring);
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
        if let Some(index) = self.find_game(game) {
            if restoring {
                self.entries[index]
                    .scan_info
                    .update_ignored(&config.restore.toggled_paths, &config.restore.toggled_registry);
            } else {
                self.entries[index]
                    .scan_info
                    .update_ignored(&config.backup.toggled_paths, &config.backup.toggled_registry);
            }

            let stale = duplicate_detector.add_game(
                &self.entries[index].scan_info,
                config.is_game_enabled_for_operation(game, restoring),
            );

            self.entries[index].refresh_tree(duplicate_detector, config, restoring);

            for entry in &mut self.entries {
                if stale.contains(&entry.scan_info.game_name) {
                    entry.refresh_tree(duplicate_detector, config, restoring);
                }
            }
        }
    }

    pub fn remove_game(
        &mut self,
        game: &str,
        duplicate_detector: &DuplicateDetector,
        duplicates: &HashSet<String>,
        config: &Config,
        restoring: bool,
    ) {
        self.entries.retain(|entry| entry.scan_info.game_name != game);
        for entry in self.entries.iter_mut() {
            if duplicates.contains(&entry.scan_info.game_name) {
                entry.refresh_tree(duplicate_detector, config, restoring);
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
        self.entries.iter().any(|x| !x.scanned)
    }

    pub fn toggle_backup_comment_editor(&mut self, game: &str) {
        let index = self.find_game(game);

        if let Some(i) = index {
            self.entries[i].show_comment_editor = !self.entries[i].show_comment_editor;
        }
    }

    pub fn set_comment(&mut self, game: &str, comment: String) -> bool {
        let Some(index) = self.find_game(game) else {
            return false;
        };
        let entry = &mut self.entries[index];
        let Some(backup) = &mut entry.scan_info.backup else {
            return false;
        };
        let Some(layout) = &mut entry.game_layout else {
            return false;
        };

        layout.set_backup_comment(backup.name(), &comment);
        backup.set_comment(comment);

        true
    }

    pub fn toggle_locked(&mut self, game: &str) -> bool {
        let Some(index) = self.find_game(game) else {
            return false;
        };
        let entry = &mut self.entries[index];
        let Some(backup) = &mut entry.scan_info.backup else {
            return false;
        };
        let Some(layout) = &mut entry.game_layout else {
            return false;
        };

        let new = !backup.locked();

        layout.set_backup_locked(backup.name(), new);
        backup.set_locked(new);

        true
    }

    pub fn save_layout(&mut self, game: &str) {
        let Some(index) = self.find_game(game) else { return };
        let entry = &mut self.entries[index];
        let Some(layout) = &mut entry.game_layout else { return };

        layout.save();
    }
}
