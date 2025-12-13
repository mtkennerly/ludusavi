use std::collections::{BTreeSet, HashSet};

use iced::{
    alignment::Horizontal as HorizontalAlignment, keyboard::Modifiers, padding, widget::tooltip, Alignment, Length,
};

use crate::{
    gui::{
        badge::Badge,
        button,
        common::{
            BackupPhase, GameAction, GameSelection, Message, Operation, RestorePhase, Screen, ScrollSubject,
            UndoSubject,
        },
        file_tree::FileTree,
        icon::Icon,
        search::FilterComponent,
        shortcuts::TextHistories,
        style,
        widget::{
            checkbox, pick_list, text, text_editor, Button, Column, Container, IcedButtonExt, IcedParentExt, Row,
            Tooltip,
        },
    },
    lang::TRANSLATOR,
    resource::{
        cache::Cache,
        config::{self, Config, Sort},
        manifest::{self, Manifest, Os},
    },
    scan::{
        game_filter, layout::GameLayout, BackupInfo, DuplicateDetector, OperationStatus, ScanChange, ScanInfo, ScanKind,
    },
};

#[derive(Default)]
pub struct GameListEntry {
    pub scan_info: ScanInfo,
    pub backup_info: Option<BackupInfo>,
    pub tree: Option<FileTree>,
    pub comment_editor: Option<iced::widget::text_editor::Content<iced::Renderer>>,
    pub game_layout: Option<GameLayout>,
    /// The `scan_info` gets mutated in response to things like toggling saves off,
    /// so we need a persistent flag to say if the game has been scanned yet.
    pub scanned: bool,
}

impl GameListEntry {
    fn view(
        &self,
        scan_kind: ScanKind,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        operation: &Operation,
        expanded: bool,
        modifiers: &Modifiers,
        filtering_duplicates: bool,
    ) -> Container {
        let successful = match &self.backup_info {
            Some(x) => x.successful(),
            _ => true,
        };

        let enabled = config.is_game_enabled_for_operation(&self.scan_info.game_name, scan_kind);
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
                .align_x(Alignment::Center)
                .push(
                    Row::new()
                        .spacing(15)
                        .align_y(Alignment::Center)
                        .push({
                            let name = name.clone();
                            checkbox(
                                "",
                                enabled,
                                Message::config(move |enabled| config::Event::GameListEntryEnabled {
                                    name: name.clone(),
                                    enabled,
                                    scan_kind,
                                }),
                            )
                            .spacing(0)
                            .class(style::Checkbox)
                        })
                        .push(
                            Button::new(text(display_name.to_string()).align_x(HorizontalAlignment::Center))
                                .on_press_maybe(if self.scanned {
                                    Some(Message::ToggleGameListEntryExpanded {
                                        name: self.scan_info.game_name.clone(),
                                    })
                                } else if !operating {
                                    match scan_kind {
                                        ScanKind::Backup => Some(Message::Backup(BackupPhase::Start {
                                            preview: true,
                                            repair: false,
                                            jump: false,
                                            games: Some(GameSelection::single(self.scan_info.game_name.clone())),
                                        })),
                                        ScanKind::Restore => Some(Message::Restore(RestorePhase::Start {
                                            preview: true,
                                            games: Some(GameSelection::single(self.scan_info.game_name.clone())),
                                        })),
                                    }
                                } else {
                                    None
                                })
                                .class(if !self.scanned {
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
                        .push(match changes {
                            ScanChange::New => Some(Badge::new_entry().faded(!enabled).view()),
                            ScanChange::Different => Some(Badge::changed_entry().faded(!enabled).view()),
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
                                    scan_kind,
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
                        .push({
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.comment().cloned())
                                .map(|comment| {
                                    Tooltip::new(
                                        Icon::Comment.text().width(Length::Shrink),
                                        text(comment).size(16),
                                        tooltip::Position::Top,
                                    )
                                    .gap(5)
                                    .class(style::Container::Tooltip)
                                })
                        })
                        .push({
                            manifest.0.get(&name).and_then(|data| {
                                (scan_kind.is_backup() && !data.notes.is_empty())
                                    .then(|| button::show_game_notes(name.clone(), data.notes.clone()))
                            })
                        })
                        .push({
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.os())
                                .and_then(|os| {
                                    (os != Os::HOST && os != Os::Other).then(|| Badge::new(&format!("{os:?}")).view())
                                })
                        })
                        .push({
                            self.scan_info
                                .backup
                                .as_ref()
                                .and_then(|backup| backup.locked().then_some(Icon::Lock.text().width(Length::Shrink)))
                        })
                        .push(
                            Row::new()
                                .push({
                                    if self.scan_info.available_backups.len() == 1 {
                                        self.scan_info.backup.as_ref().map(|backup| {
                                            Container::new(
                                                text(backup.label())
                                                    .size(14)
                                                    .line_height(1.1)
                                                    .align_x(HorizontalAlignment::Center),
                                            )
                                            .padding(padding::top(2).right(2))
                                            .center_x(150)
                                            .center_y(20)
                                        })
                                    } else if !self.scan_info.available_backups.is_empty() {
                                        if operating {
                                            self.scan_info.backup.as_ref().map(|backup| {
                                                Container::new(
                                                    Container::new(
                                                        text(backup.label())
                                                            .size(14)
                                                            .align_x(HorizontalAlignment::Center),
                                                    )
                                                    .padding(padding::top(2))
                                                    .center_x(148)
                                                    .center_y(25)
                                                    .class(style::Container::DisabledBackup),
                                                )
                                                .padding(padding::right(2))
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
                                                .class(style::PickList::Backup),
                                            )
                                            .padding(padding::right(2))
                                            .center_x(150)
                                            .center_y(25);
                                            Some(content)
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .push({
                                    let confirm = !modifiers.alt();
                                    let action = if modifiers.shift() {
                                        Some(match scan_kind {
                                            ScanKind::Backup => GameAction::PreviewBackup,
                                            ScanKind::Restore => GameAction::PreviewRestore,
                                        })
                                    } else if modifiers.command() {
                                        Some(match scan_kind {
                                            ScanKind::Backup => GameAction::Backup { confirm },
                                            ScanKind::Restore => GameAction::Restore { confirm },
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
                                            .class(style::Button::GameActionPrimary)
                                            .padding(2);
                                        Container::new(
                                            Tooltip::new(
                                                button,
                                                text(action.to_string()).size(16),
                                                tooltip::Position::Top,
                                            )
                                            .gap(5)
                                            .class(style::Container::Tooltip),
                                        )
                                    } else {
                                        let options = GameAction::options(
                                            scan_kind,
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
                                        .class(style::PickList::Popup);
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
                                    .center_x(115),
                                ),
                        ),
                )
                .push(self.comment_editor.as_ref().map(|x| {
                    Row::new()
                        .align_y(Alignment::Center)
                        .padding([0, 20])
                        .spacing(20)
                        .push(text(TRANSLATOR.comment_label()))
                        .push(text_editor(
                            x,
                            |action| Message::EditedBackupComment {
                                game: self.scan_info.game_name.clone(),
                                action,
                            },
                            UndoSubject::BackupComment(self.scan_info.game_name.clone()),
                        ))
                        .push(button::hide(Message::GameAction {
                            action: GameAction::Comment,
                            game: name.clone(),
                        }))
                }))
                .push({
                    expanded
                        .then(|| {
                            self.tree.as_ref().map(|tree| {
                                tree.view(&self.scan_info.game_name, config, scan_kind)
                                    .width(Length::Fill)
                            })
                        })
                        .flatten()
                }),
        )
        .id(name)
        .class(style::Container::GameListEntry)
    }

    pub fn refresh_tree(&mut self, duplicate_detector: &DuplicateDetector, config: &Config, scan_kind: ScanKind) {
        match self.tree.as_mut() {
            Some(tree) => tree.reset_nodes(
                self.scan_info.clone(),
                self.backup_info.as_ref(),
                duplicate_detector,
                config,
                scan_kind,
            ),
            None => {
                self.tree = Some(FileTree::new(
                    self.scan_info.clone(),
                    self.backup_info.as_ref(),
                    duplicate_detector,
                    config,
                    scan_kind,
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
    pub fn duplicatees(&self, duplicate_detector: &DuplicateDetector) -> Option<HashSet<String>> {
        self.filter_duplicates_of.as_ref().and_then(|game| {
            let mut duplicatees = duplicate_detector.duplicate_games(game);
            if duplicatees.is_empty() {
                None
            } else {
                duplicatees.insert(game.clone());
                Some(duplicatees)
            }
        })
    }

    pub fn view(
        &self,
        scan_kind: ScanKind,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        duplicatees: Option<&HashSet<String>>,
        operation: &Operation,
        histories: &TextHistories,
        modifiers: &Modifiers,
    ) -> Container {
        Container::new(
            Column::new()
                .spacing(15)
                .push({
                    self.search.view(
                        match scan_kind {
                            ScanKind::Backup => Screen::Backup,
                            ScanKind::Restore => Screen::Restore,
                        },
                        histories,
                        config.scan.show_deselected_games,
                        self.manifests(manifest),
                    )
                })
                .push({
                    let content = self
                        .entries
                        .iter()
                        .filter(|entry| {
                            self.filter_game(entry, scan_kind, config, manifest, duplicate_detector, duplicatees)
                        })
                        .fold(
                            Column::new()
                                .width(Length::Fill)
                                .padding(padding::bottom(5).left(15).right(15))
                                .spacing(5),
                            |parent, x| {
                                parent.push(x.view(
                                    scan_kind,
                                    config,
                                    manifest,
                                    duplicate_detector,
                                    operation,
                                    self.expanded_games.contains(&x.scan_info.game_name),
                                    modifiers,
                                    duplicatees.is_some(),
                                ))
                            },
                        );
                    ScrollSubject::game_list(scan_kind).into_widget(content)
                }),
        )
    }

    pub fn all_visible_entries_selected(
        &self,
        config: &Config,
        scan_kind: ScanKind,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        duplicatees: Option<&HashSet<String>>,
    ) -> bool {
        self.entries
            .iter()
            .filter(|entry| self.filter_game(entry, scan_kind, config, manifest, duplicate_detector, duplicatees))
            .all(|x| config.is_game_enabled_for_operation(&x.scan_info.game_name, scan_kind))
    }

    fn filter_game(
        &self,
        entry: &GameListEntry,
        scan_kind: ScanKind,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        duplicatees: Option<&HashSet<String>>,
    ) -> bool {
        let show = config.should_show_game(
            &entry.scan_info.game_name,
            scan_kind,
            entry.scan_info.overall_change().is_changed(),
            entry.scanned,
        );

        let qualifies = self.search.qualifies(
            &entry.scan_info,
            manifest,
            config.is_game_enabled_for_operation(&entry.scan_info.game_name, scan_kind),
            config.is_game_customized(&entry.scan_info.game_name),
            duplicate_detector.is_game_duplicated(&entry.scan_info.game_name),
            config.scan.show_deselected_games,
        );

        let duplicate = duplicatees
            .as_ref()
            .map(|xs| xs.contains(&entry.scan_info.game_name))
            .unwrap_or(true);

        show && qualifies && duplicate
    }

    pub fn visible_games(
        &self,
        scan_kind: ScanKind,
        config: &Config,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
    ) -> HashSet<String> {
        let duplicatees = self.filter_duplicates_of.as_ref().and_then(|game| {
            let mut duplicatees = duplicate_detector.duplicate_games(game);
            if duplicatees.is_empty() {
                None
            } else {
                duplicatees.insert(game.clone());
                Some(duplicatees)
            }
        });

        self.entries
            .iter()
            .filter(|entry| {
                self.filter_game(
                    entry,
                    scan_kind,
                    config,
                    manifest,
                    duplicate_detector,
                    duplicatees.as_ref(),
                )
            })
            .map(|x| x.scan_info.game_name.clone())
            .collect()
    }

    pub fn is_filtered(&self) -> bool {
        self.search.show || self.filter_duplicates_of.is_some()
    }

    pub fn compute_operation_status(
        &self,
        config: &Config,
        scan_kind: ScanKind,
        manifest: &Manifest,
        duplicate_detector: &DuplicateDetector,
        duplicatees: Option<&HashSet<String>>,
    ) -> OperationStatus {
        let mut status = OperationStatus::default();
        for entry in self.entries.iter() {
            if !self.filter_game(entry, scan_kind, config, manifest, duplicate_detector, duplicatees) {
                continue;
            }

            status.total_games += 1;
            status.total_bytes += entry.scan_info.total_possible_bytes();
            if !entry.scan_info.all_ignored()
                && config.is_game_enabled_for_operation(&entry.scan_info.game_name, scan_kind)
            {
                status.processed_games += 1;
                status.processed_bytes += entry.scan_info.sum_bytes(None);
                status.changed_games.add(entry.scan_info.overall_change());
            }
        }
        status
    }

    pub fn sort(&mut self, sort: &Sort, config: &Config) {
        self.entries.sort_by(|x, y| {
            crate::scan::compare_games(
                sort.key,
                config,
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
        scan_kind: ScanKind,
    ) {
        if self.expanded_games.contains(game) {
            self.collapse_game(game);
        } else {
            self.expand_game(game, duplicate_detector, config, scan_kind);
        }
    }

    pub fn expand_game(
        &mut self,
        game: &str,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        scan_kind: ScanKind,
    ) {
        if self.expanded_games.contains(game) {
            return;
        }

        self.expanded_games.insert(game.to_string());
        for entry in self.entries.iter_mut() {
            if entry.scan_info.game_name == game {
                entry.refresh_tree(duplicate_detector, config, scan_kind);
                break;
            }
        }
    }

    pub fn collapse_game(&mut self, game: &str) {
        if !self.expanded_games.contains(game) {
            return;
        }

        self.expanded_games.remove(game);
        for entry in self.entries.iter_mut() {
            if entry.scan_info.game_name == game {
                entry.clear_tree();
                break;
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.expanded_games.clear();
    }

    pub fn with_recent_games(scan_kind: ScanKind, config: &Config, cache: &Cache) -> Self {
        let games = match scan_kind {
            ScanKind::Backup => &cache.backup.recent_games,
            ScanKind::Restore => &cache.restore.recent_games,
        };
        let sort = match scan_kind {
            ScanKind::Backup => &config.backup.sort,
            ScanKind::Restore => &config.restore.sort,
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
                scan_kind,
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
        scan_kind: ScanKind,
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
                        self.entries[i].refresh_tree(duplicate_detector, config, scan_kind);
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
                    entry.refresh_tree(duplicate_detector, config, scan_kind);
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
                    entry.refresh_tree(duplicate_detector, config, scan_kind);
                }
            }
        }
    }

    pub fn refresh_game_tree(
        &mut self,
        game: &str,
        config: &Config,
        duplicate_detector: &mut DuplicateDetector,
        scan_kind: ScanKind,
    ) {
        if let Some(index) = self.find_game(game) {
            match scan_kind {
                ScanKind::Backup => {
                    self.entries[index]
                        .scan_info
                        .update_ignored(&config.backup.toggled_paths, &config.backup.toggled_registry);
                }
                ScanKind::Restore => {
                    self.entries[index]
                        .scan_info
                        .update_ignored(&config.restore.toggled_paths, &config.restore.toggled_registry);
                }
            }

            let stale = duplicate_detector.add_game(
                &self.entries[index].scan_info,
                config.is_game_enabled_for_operation(game, scan_kind),
            );

            self.entries[index].refresh_tree(duplicate_detector, config, scan_kind);

            for entry in &mut self.entries {
                if stale.contains(&entry.scan_info.game_name) {
                    entry.refresh_tree(duplicate_detector, config, scan_kind);
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
        scan_kind: ScanKind,
    ) {
        self.entries.retain(|entry| entry.scan_info.game_name != game);
        for entry in self.entries.iter_mut() {
            if duplicates.contains(&entry.scan_info.game_name) {
                entry.refresh_tree(duplicate_detector, config, scan_kind);
            }
        }
    }

    pub fn unscan_games(&mut self, games: &GameSelection) {
        for entry in self.entries.iter_mut() {
            if games.contains(&entry.scan_info.game_name) {
                entry.scanned = false;
                entry.scan_info.found_files.clear();
                entry.scan_info.found_registry_keys.clear();
                if !games.is_single() {
                    entry.clear_tree();
                    self.expanded_games.remove(&entry.scan_info.game_name);
                }
            }
        }
    }

    pub fn contains_unscanned_games(&self) -> bool {
        self.entries.iter().any(|x| !x.scanned)
    }

    pub fn toggle_backup_comment_editor(&mut self, game: &str) {
        let index = self.find_game(game);

        if let Some(i) = index {
            self.entries[i].comment_editor = match self.entries[i].comment_editor {
                Some(_) => None,
                None => Some(
                    self.entries[i]
                        .scan_info
                        .backup
                        .as_ref()
                        .and_then(|x| x.comment())
                        .map(|x| iced::widget::text_editor::Content::with_text(x))
                        .unwrap_or_default(),
                ),
            };
        }
    }

    pub fn set_comment(&mut self, game: &str, comment: String) -> bool {
        let Some(index) = self.find_game(game) else {
            return false;
        };
        let entry = &mut self.entries[index];

        let Some(editor) = entry.comment_editor.as_mut() else {
            return false;
        };
        *editor = iced::widget::text_editor::Content::with_text(&comment);

        let Some(backup) = &mut entry.scan_info.backup else {
            return false;
        };
        let Some(layout) = &mut entry.game_layout else {
            return false;
        };

        layout.set_backup_comment(&backup.id(), &comment);
        backup.set_comment(comment);

        true
    }

    pub fn apply_comment_action(&mut self, game: &str, action: iced::widget::text_editor::Action) -> Option<String> {
        let index = self.find_game(game)?;
        let entry = &mut self.entries[index];

        let editor = entry.comment_editor.as_mut()?;
        let backup = entry.scan_info.backup.as_mut()?;
        let layout = entry.game_layout.as_mut()?;

        editor.perform(action);
        let comment = editor.text().trim().to_string();

        layout.set_backup_comment(&backup.id(), &comment);
        backup.set_comment(comment.clone());

        Some(comment)
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

        layout.set_backup_locked(&backup.id(), new);
        backup.set_locked(new);

        true
    }

    pub fn save_layout(&mut self, game: &str) {
        let Some(index) = self.find_game(game) else { return };
        let entry = &mut self.entries[index];
        let Some(layout) = &mut entry.game_layout else { return };

        layout.save();
    }

    fn manifests(&self, manifest: &Manifest) -> Vec<game_filter::Manifest> {
        let mut manifests = BTreeSet::new();
        manifests.insert(&manifest::Source::Primary);
        manifests.insert(&manifest::Source::Custom);

        for entry in &self.entries {
            if let Some(data) = manifest.0.get(&entry.scan_info.game_name) {
                manifests.extend(data.sources.iter());
            }
        }

        manifests
            .into_iter()
            .map(|x| game_filter::Manifest::new(x.clone()))
            .collect()
    }
}
