use iced::{Alignment, Length};

use crate::{
    gui::{
        badge::Badge,
        common::{Message, TreeNodeKey},
        icon::Icon,
        style,
        widget::{Button, Checkbox, Column, Container, IcedParentExt, Row, Text},
    },
    lang::TRANSLATOR,
    path::StrictPath,
    resource::config::{Config, ToggledPaths, ToggledRegistry},
    scan::{
        registry_compat::RegistryItem, BackupInfo, DuplicateDetector, ScanChange, ScanInfo, ScannedFile,
        ScannedRegistryValues,
    },
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum FileTreeNodeType {
    #[default]
    File,
    RegistryKey,
    RegistryValue(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FileTreeNodePath {
    File(StrictPath),
    RegistryKey(RegistryItem),
    RegistryValue(RegistryItem, String),
}

#[derive(Clone, Debug, Default)]
struct FileTreeNode {
    keys: Vec<TreeNodeKey>,
    expanded: bool,
    path: Option<FileTreeNodePath>,
    nodes: std::collections::BTreeMap<TreeNodeKey, FileTreeNode>,
    successful: bool,
    ignored: bool,
    duplicated: bool,
    change: ScanChange,
    scanned_file: Option<ScannedFile>,
    node_type: FileTreeNodeType,
}

impl FileTreeNode {
    pub fn new(keys: Vec<TreeNodeKey>, path: Option<FileTreeNodePath>, node_type: FileTreeNodeType) -> Self {
        Self {
            keys,
            path,
            node_type,
            ..Default::default()
        }
    }

    pub fn anything_showable(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }
        for node in self.nodes.values() {
            if node.anything_showable() {
                return true;
            }
        }
        false
    }

    pub fn view(&self, level: u16, label: String, game_name: &str, _config: &Config, restoring: bool) -> Container {
        let expanded = self.expanded;

        let make_enabler = || {
            if restoring {
                return None;
            }
            if let Some(path) = &self.path {
                let game_name = game_name.to_string();
                let path = path.clone();
                return Some(
                    Container::new(
                        Checkbox::new("", !self.ignored, move |enabled| match &path {
                            FileTreeNodePath::File(path) => Message::ToggleSpecificBackupPathIgnored {
                                name: game_name.clone(),
                                path: path.clone(),
                                enabled,
                            },
                            FileTreeNodePath::RegistryKey(path) => Message::ToggleSpecificBackupRegistryIgnored {
                                name: game_name.clone(),
                                path: path.clone(),
                                value: None,
                                enabled,
                            },
                            FileTreeNodePath::RegistryValue(path, name) => {
                                Message::ToggleSpecificBackupRegistryIgnored {
                                    name: game_name.clone(),
                                    path: path.clone(),
                                    value: Some(name.clone()),
                                    enabled,
                                }
                            }
                        })
                        .spacing(5)
                        .style(style::Checkbox),
                    )
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                );
            }
            None
        };

        if self.nodes.is_empty() {
            return Container::new(
                Row::new()
                    .align_items(Alignment::Center)
                    .padding([0, 0, 0, 35 * level])
                    .spacing(10)
                    .push(match self.node_type {
                        FileTreeNodeType::File | FileTreeNodeType::RegistryValue(_) => {
                            Container::new(Icon::SubdirectoryArrowRight.as_text().height(25).width(25).size(25))
                        }
                        FileTreeNodeType::RegistryKey => Container::new(
                            Button::new(Icon::KeyboardArrowDown.into_text().width(15).size(15))
                                .style(style::Button::Primary)
                                .height(25)
                                .width(25),
                        ),
                    })
                    .push_some(make_enabler)
                    .push(Text::new(label))
                    .push_some(|| {
                        let badge = match self.change {
                            ScanChange::Same | ScanChange::Unknown => return None,
                            ScanChange::New => Badge::new_entry(),
                            ScanChange::Different => Badge::changed_entry(),
                        };
                        Some(badge.view())
                    })
                    .push_if(|| self.duplicated, || Badge::new(&TRANSLATOR.badge_duplicated()).view())
                    .push_if(|| !self.successful, || Badge::new(&TRANSLATOR.badge_failed()).view())
                    .push_some(|| {
                        self.scanned_file.as_ref().and_then(|scanned| {
                            let restoring = scanned.restoring();
                            scanned.alt(restoring).as_ref().map(|alt| {
                                let msg = if restoring {
                                    TRANSLATOR.badge_redirected_from(alt)
                                } else {
                                    TRANSLATOR.badge_redirecting_to(alt)
                                };
                                Badge::new(&msg).view()
                            })
                        })
                    }),
            );
        } else if self.nodes.len() == 1 {
            let keys: Vec<_> = self.nodes.keys().cloned().collect();
            let key = &keys[0];
            if !self.nodes.get(key).unwrap().nodes.is_empty() {
                return Container::new(self.nodes.get(key).unwrap().view(
                    level,
                    format!("{}/{}", label, key.raw()),
                    game_name,
                    _config,
                    restoring,
                ));
            }
        }

        Container::new(
            self.nodes.iter().filter(|(_, v)| v.anything_showable()).fold(
                Column::new().push(
                    Row::new()
                        .align_items(Alignment::Center)
                        .padding([0, 10, 0, 35 * level])
                        .spacing(10)
                        .push(
                            Button::new(
                                (if expanded {
                                    Icon::KeyboardArrowDown
                                } else {
                                    Icon::KeyboardArrowRight
                                })
                                .into_text()
                                .width(15)
                                .size(15),
                            )
                            .on_press(Message::ToggleGameListEntryTreeExpanded {
                                name: game_name.to_string(),
                                keys: self.keys.clone(),
                            })
                            .style(style::Button::Primary)
                            .height(25)
                            .width(25),
                        )
                        .push_some(make_enabler)
                        .push(Text::new(
                            if label.is_empty() && self.node_type == FileTreeNodeType::File {
                                "/".to_string()
                            } else {
                                label
                            },
                        ))
                        .push_some(|| {
                            if let Some(FileTreeNodePath::File(path)) = &self.path {
                                return Some(
                                    Button::new(Icon::OpenInNew.as_text().width(Length::Shrink).size(15))
                                        .on_press(Message::OpenDir { path: path.clone() })
                                        .style(style::Button::Primary)
                                        .height(25),
                                );
                            }
                            None
                        }),
                ),
                |parent, (k, v)| {
                    parent.push_if(
                        || expanded,
                        || v.view(level + 1, k.raw().to_string(), game_name, _config, restoring),
                    )
                },
            ),
        )
    }

    fn insert_keys(
        &mut self,
        keys: &[TreeNodeKey],
        prefix_keys: &[TreeNodeKey],
        successful: bool,
        duplicated: bool,
        change: ScanChange,
        scanned_file: Option<ScannedFile>,
        registry_values: Option<&ScannedRegistryValues>,
        duplicate_detector: &DuplicateDetector,
    ) -> &mut Self {
        let node_type = self.node_type.clone();
        let mut node = self;
        let mut inserted_keys = vec![];
        for key in prefix_keys.iter() {
            inserted_keys.push(key.clone());
        }
        let mut full_keys: Vec<_> = prefix_keys.to_vec();
        for key in keys.iter() {
            inserted_keys.push(key.clone());
            full_keys.push(key.clone());
            let raw_path = inserted_keys.iter().map(|x| x.raw()).collect::<Vec<_>>().join("/");
            node = node.nodes.entry(key.clone()).or_insert_with(|| {
                FileTreeNode::new(
                    full_keys.clone(),
                    match &node_type {
                        FileTreeNodeType::File => Some(FileTreeNodePath::File(StrictPath::new(raw_path))),
                        FileTreeNodeType::RegistryKey => {
                            Some(FileTreeNodePath::RegistryKey(RegistryItem::new(raw_path)))
                        }
                        FileTreeNodeType::RegistryValue(name) => Some(FileTreeNodePath::RegistryValue(
                            RegistryItem::new(raw_path),
                            name.clone(),
                        )),
                    },
                    node_type.clone(),
                )
            });
        }

        node.successful = successful;
        node.duplicated = duplicated;
        node.change = change;
        node.scanned_file = scanned_file;

        if let Some(registry_values) = registry_values {
            let raw_key_path = inserted_keys.iter().map(|x| x.raw()).collect::<Vec<_>>().join("/");
            for (value_name, value) in registry_values {
                let mut full_keys = full_keys.clone();
                full_keys.push(TreeNodeKey::RegistryKey(value_name.clone()));
                let mut node = node
                    .nodes
                    .entry(TreeNodeKey::RegistryValue(value_name.clone()))
                    .or_insert_with(|| {
                        FileTreeNode::new(
                            full_keys,
                            Some(FileTreeNodePath::RegistryValue(
                                RegistryItem::new(raw_key_path.clone()),
                                value_name.clone(),
                            )),
                            FileTreeNodeType::RegistryValue(value_name.clone()),
                        )
                    });
                node.successful = true;
                node.duplicated = duplicate_detector
                    .is_registry_value_duplicated(&RegistryItem::new(raw_key_path.clone()), value_name);
                node.change = value.change;
                node.ignored = value.ignored;
            }
        }

        node
    }

    fn expand_or_collapse_keys(&mut self, keys: &[TreeNodeKey]) -> &mut Self {
        let mut node = self;
        let mut visited_keys = vec![];
        for key in keys.iter() {
            visited_keys.push(key.clone());
            node = node.nodes.entry(key.clone()).or_insert_with(Default::default);
        }

        node.expanded = !node.expanded;

        node
    }

    fn expand_short(&mut self) {
        if self.nodes.len() < 30 {
            self.expanded = true;
        }
        for item in self.nodes.values_mut() {
            item.expand_short();
        }
    }

    pub fn update_ignored(&mut self, game: &str, ignored_paths: &ToggledPaths, ignored_registry: &ToggledRegistry) {
        match &self.path {
            Some(FileTreeNodePath::File(path)) => {
                self.ignored = ignored_paths.is_ignored(game, path);
            }
            Some(FileTreeNodePath::RegistryKey(path)) => {
                self.ignored = ignored_registry.is_ignored(game, path, None);
            }
            Some(FileTreeNodePath::RegistryValue(path, name)) => {
                self.ignored = ignored_registry.is_ignored(game, path, Some(name));
            }
            None => {}
        }
        for item in self.nodes.values_mut() {
            item.update_ignored(game, ignored_paths, ignored_registry);
        }
    }
}

#[derive(Debug, Default)]
pub struct FileTree {
    nodes: std::collections::BTreeMap<TreeNodeKey, FileTreeNode>,
}

impl FileTree {
    pub fn new(
        scan_info: ScanInfo,
        config: &Config,
        backup_info: &Option<BackupInfo>,
        duplicate_detector: &DuplicateDetector,
    ) -> Self {
        let mut nodes = std::collections::BTreeMap::<TreeNodeKey, FileTreeNode>::new();

        for item in scan_info.found_files.iter() {
            let mut successful = true;
            if let Some(backup_info) = &backup_info {
                if backup_info.failed_files.contains(item) {
                    successful = false;
                }
            }

            let rendered = item.readable(scan_info.restoring());
            let components: Vec<_> = rendered.split('/').map(|x| TreeNodeKey::File(x.to_string())).collect();

            nodes
                .entry(components[0].clone())
                .or_insert_with(|| FileTreeNode::new(vec![components[0].clone()], None, FileTreeNodeType::File))
                .insert_keys(
                    &components[1..],
                    &[components[0].clone()],
                    successful,
                    duplicate_detector.is_file_duplicated(item),
                    item.change,
                    Some(item.clone()),
                    None,
                    duplicate_detector,
                );
        }
        for item in scan_info.found_registry_keys.iter() {
            let mut successful = true;
            if let Some(backup_info) = &backup_info {
                if backup_info.failed_registry.contains(&item.path) {
                    successful = false;
                }
            }

            let components: Vec<_> = item
                .path
                .split()
                .iter()
                .map(|x| TreeNodeKey::RegistryKey(x.to_string()))
                .collect();

            nodes
                .entry(components[0].clone())
                .or_insert_with(|| FileTreeNode::new(vec![components[0].clone()], None, FileTreeNodeType::RegistryKey))
                .insert_keys(
                    &components[1..],
                    &components[0..1],
                    successful,
                    duplicate_detector.is_registry_duplicated(&item.path),
                    item.change,
                    None,
                    Some(&item.values),
                    duplicate_detector,
                );
        }

        for item in nodes.values_mut() {
            item.expand_short();
            item.update_ignored(
                &scan_info.game_name,
                &config.backup.toggled_paths,
                &config.backup.toggled_registry,
            );
        }

        Self { nodes }
    }

    pub fn view(&self, game_name: &str, config: &Config, restoring: bool) -> Container {
        Container::new(
            self.nodes
                .iter()
                .filter(|(_, v)| v.anything_showable())
                .fold(Column::new().spacing(4), |parent, (k, v)| {
                    parent.push(v.view(0, k.raw().to_string(), game_name, config, restoring))
                }),
        )
    }

    pub fn expand_or_collapse_keys(&mut self, keys: &[TreeNodeKey]) {
        if keys.is_empty() {
            return;
        }
        for (k, v) in self.nodes.iter_mut() {
            if k == &keys[0] {
                v.expand_or_collapse_keys(&keys[1..]);
                break;
            }
        }
    }

    pub fn update_ignored(&mut self, game: &str, ignored_paths: &ToggledPaths, ignored_registry: &ToggledRegistry) {
        for item in self.nodes.values_mut() {
            item.update_ignored(game, ignored_paths, ignored_registry);
        }
    }
}
