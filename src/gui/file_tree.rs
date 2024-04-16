use std::collections::{BTreeMap, HashMap};

use iced::Alignment;

use crate::{
    gui::{
        badge::Badge,
        common::{Message, TreeNodeKey},
        icon::Icon,
        style,
        widget::{checkbox, text, Button, Column, Container, IcedParentExt, Row},
    },
    lang::TRANSLATOR,
    path::StrictPath,
    resource::config::Config,
    scan::{
        registry_compat::RegistryItem, BackupError, BackupInfo, DuplicateDetector, Duplication, ScanChange, ScanInfo,
        ScannedFile, ScannedRegistryValues,
    },
};

fn check_ignored(game: &str, path: &FileTreeNodePath, config: &Config, restoring: bool) -> bool {
    let (paths, registries) = if restoring {
        (&config.restore.toggled_paths, &config.restore.toggled_registry)
    } else {
        (&config.backup.toggled_paths, &config.backup.toggled_registry)
    };

    match path {
        FileTreeNodePath::File(path) => paths.is_ignored(game, path),
        FileTreeNodePath::RegistryKey(path) => registries.is_ignored(game, path, None),
        FileTreeNodePath::RegistryValue(path, name) => registries.is_ignored(game, path, Some(name)),
    }
}

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

#[derive(Clone, Debug)]
struct FileTreeNode {
    keys: Vec<TreeNodeKey>,
    path: FileTreeNodePath,
    nodes: BTreeMap<TreeNodeKey, FileTreeNode>,
    error: Option<String>,
    ignored: bool,
    duplicated: Duplication,
    change: ScanChange,
    scanned_file: Option<ScannedFile>,
    node_type: FileTreeNodeType,
}

impl FileTreeNode {
    pub fn new(
        game: &str,
        keys: Vec<TreeNodeKey>,
        path: FileTreeNodePath,
        node_type: FileTreeNodeType,
        config: &Config,
        restoring: bool,
    ) -> Self {
        let ignored = check_ignored(game, &path, config, restoring);
        Self {
            keys,
            path,
            nodes: Default::default(),
            error: None,
            ignored,
            duplicated: Default::default(),
            change: Default::default(),
            scanned_file: None,
            node_type,
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

    pub fn view(
        &self,
        level: u16,
        label: String,
        game_name: &str,
        _config: &Config,
        restoring: bool,
        expansion: &Expansion,
    ) -> Container {
        let expanded = expansion.expanded(&self.keys);

        let make_enabler = || {
            let game_name = game_name.to_string();
            let path = self.path.clone();
            Some(
                Container::new(
                    checkbox("", !self.ignored, move |enabled| match &path {
                        FileTreeNodePath::File(path) => Message::ToggleSpecificGamePathIgnored {
                            name: game_name.clone(),
                            path: path.clone(),
                            enabled,
                            restoring,
                        },
                        FileTreeNodePath::RegistryKey(path) => Message::ToggleSpecificGameRegistryIgnored {
                            name: game_name.clone(),
                            path: path.clone(),
                            value: None,
                            enabled,
                            restoring,
                        },
                        FileTreeNodePath::RegistryValue(path, name) => Message::ToggleSpecificGameRegistryIgnored {
                            name: game_name.clone(),
                            path: path.clone(),
                            value: Some(name.clone()),
                            enabled,
                            restoring,
                        },
                    })
                    .spacing(5)
                    .style(style::Checkbox),
                )
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            )
        };

        if self.nodes.is_empty() {
            return Container::new(
                Row::new()
                    .align_items(Alignment::Center)
                    .padding([0, 10, 0, 35 * level])
                    .spacing(10)
                    .push(match self.node_type {
                        FileTreeNodeType::File | FileTreeNodeType::RegistryValue(_) => {
                            Container::new(Icon::SubdirectoryArrowRight.text().height(25).width(25).size(25))
                        }
                        FileTreeNodeType::RegistryKey => Container::new(
                            Button::new(Icon::KeyboardArrowDown.text_small())
                                .style(style::Button::Primary)
                                .height(25)
                                .width(25),
                        ),
                    })
                    .push_maybe(make_enabler())
                    .push(text(label))
                    .push_maybe({
                        match self.change {
                            ScanChange::Same | ScanChange::Unknown => None,
                            ScanChange::New => Some(Badge::new_entry().view()),
                            ScanChange::Different => Some(Badge::changed_entry().view()),
                            ScanChange::Removed => Some(Badge::removed_entry().view()),
                        }
                    })
                    .push_if(!self.duplicated.unique(), || {
                        Badge::new(&TRANSLATOR.badge_duplicated())
                            .faded(self.duplicated.resolved())
                            .view()
                    })
                    .push_maybe(
                        self.error
                            .as_ref()
                            .map(|x| Badge::new(&TRANSLATOR.badge_failed()).tooltip(x.clone()).view()),
                    )
                    .push_maybe({
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
                    })
                    .push_maybe({
                        self.scanned_file.as_ref().map(|f| {
                            let size = TRANSLATOR.adjusted_size(f.size);
                            Badge::new(&size).faded(f.ignored).view()
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
                    expansion,
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
                                .text_small(),
                            )
                            .on_press(Message::ToggleGameListEntryTreeExpanded {
                                name: game_name.to_string(),
                                keys: self.keys.clone(),
                            })
                            .style(style::Button::Primary)
                            .height(25)
                            .width(25),
                        )
                        .push_maybe(make_enabler())
                        .push(text(if label.is_empty() && self.node_type == FileTreeNodeType::File {
                            "/".to_string()
                        } else {
                            label
                        }))
                        .push_maybe(
                            self.error
                                .as_ref()
                                .map(|x| Badge::new(&TRANSLATOR.badge_failed()).tooltip(x.clone()).view()),
                        )
                        .push_maybe(match &self.path {
                            FileTreeNodePath::File(path) => Some(
                                Button::new(Icon::OpenInNew.text_small())
                                    .on_press(Message::OpenDir { path: path.clone() })
                                    .style(style::Button::Primary)
                                    .height(25),
                            ),
                            FileTreeNodePath::RegistryKey(..) | FileTreeNodePath::RegistryValue(..) => None,
                        })
                        .push_maybe({
                            let total_bytes = self.calculate_directory_size(true);
                            let total_size = total_bytes.map(|bytes| TRANSLATOR.adjusted_size(bytes));

                            let included_bytes = self.calculate_directory_size(false);
                            let included_size = included_bytes.map(|bytes| TRANSLATOR.adjusted_size(bytes));

                            let text = match (included_size, total_size) {
                                (Some(included), Some(total)) => {
                                    if included_bytes == total_bytes {
                                        Some(included)
                                    } else {
                                        Some(format!("{} / {}", included, total))
                                    }
                                }
                                (Some(included), None) => Some(format!("{} / ?", included)),
                                (None, Some(total)) => Some(total.to_string()),
                                (None, None) => None,
                            };

                            text.map(|text| Badge::new(&text).faded(included_bytes.is_none()).view())
                        }),
                ),
                |parent, (k, v)| {
                    parent.push_if(expanded, || {
                        v.view(level + 1, k.raw().to_string(), game_name, _config, restoring, expansion)
                    })
                },
            ),
        )
    }

    fn insert_keys(
        &mut self,
        game: &str,
        keys: &[TreeNodeKey],
        prefix_keys: &[TreeNodeKey],
        error: Option<&BackupError>,
        duplicated: Duplication,
        change: ScanChange,
        scanned_file: Option<ScannedFile>,
        registry_values: Option<&ScannedRegistryValues>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        restoring: bool,
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
                    game,
                    full_keys.clone(),
                    match &node_type {
                        FileTreeNodeType::File => FileTreeNodePath::File(StrictPath::new(raw_path)),
                        FileTreeNodeType::RegistryKey => FileTreeNodePath::RegistryKey(RegistryItem::new(raw_path)),
                        FileTreeNodeType::RegistryValue(name) => {
                            FileTreeNodePath::RegistryValue(RegistryItem::new(raw_path), name.clone())
                        }
                    },
                    node_type.clone(),
                    config,
                    restoring,
                )
            });
        }

        node.error = error.map(|x| x.message());
        node.duplicated = duplicated;
        node.change = change;
        node.scanned_file = scanned_file;

        if let Some(registry_values) = registry_values {
            let raw_key_path = inserted_keys.iter().map(|x| x.raw()).collect::<Vec<_>>().join("/");
            for (value_name, value) in registry_values {
                let mut full_keys = full_keys.clone();
                full_keys.push(TreeNodeKey::RegistryKey(value_name.clone()));
                let node = node
                    .nodes
                    .entry(TreeNodeKey::RegistryValue(value_name.clone()))
                    .or_insert_with(|| {
                        FileTreeNode::new(
                            game,
                            full_keys,
                            FileTreeNodePath::RegistryValue(
                                RegistryItem::new(raw_key_path.clone()),
                                value_name.clone(),
                            ),
                            FileTreeNodeType::RegistryValue(value_name.clone()),
                            config,
                            restoring,
                        )
                    });
                node.error = None;
                node.duplicated = duplicate_detector
                    .is_registry_value_duplicated(&RegistryItem::new(raw_key_path.clone()), value_name);
                node.change = value.change(restoring);
            }
        }

        node
    }

    fn calculate_directory_size(&self, include_ignored: bool) -> Option<u64> {
        let mut size = 0;
        for child_node in self.nodes.values() {
            if child_node.nodes.is_empty() {
                if let Some(scanned_file) = &child_node.scanned_file {
                    if include_ignored || !scanned_file.ignored {
                        size += scanned_file.size;
                    }
                }
            } else {
                let child_size = child_node.calculate_directory_size(include_ignored).unwrap_or(0);
                size += child_size;
            }
        }

        if size == 0 {
            None
        } else {
            Some(size)
        }
    }
}

#[derive(Debug)]
struct FileTreeStateNode {
    expanded: bool,
    nodes: HashMap<TreeNodeKey, FileTreeStateNode>,
}

#[derive(Debug, Default)]
struct Expansion(HashMap<TreeNodeKey, FileTreeStateNode>);

impl Expansion {
    pub fn new(nodes: &BTreeMap<TreeNodeKey, FileTreeNode>) -> Self {
        Self(Self::recurse_nodes(nodes))
    }

    fn recurse_nodes(nodes: &BTreeMap<TreeNodeKey, FileTreeNode>) -> HashMap<TreeNodeKey, FileTreeStateNode> {
        let mut expansion = HashMap::<TreeNodeKey, FileTreeStateNode>::new();

        for (key, node) in nodes {
            expansion.insert(
                key.clone(),
                FileTreeStateNode {
                    expanded: !node.ignored && node.nodes.len() < 30,
                    nodes: Self::recurse_nodes(&node.nodes),
                },
            );
        }

        expansion
    }

    pub fn expanded(&self, keys: &[TreeNodeKey]) -> bool {
        if keys.is_empty() {
            return false;
        }

        let mut node = self.0.get(&keys[0]);
        for key in &keys[1..] {
            match node {
                Some(state) => {
                    node = state.nodes.get(key);
                }
                None => break,
            }
        }

        node.map(|x| x.expanded).unwrap_or(true)
    }
}

#[derive(Debug, Default)]
pub struct FileTree {
    nodes: BTreeMap<TreeNodeKey, FileTreeNode>,
    expansion: Expansion,
}

impl FileTree {
    pub fn new(
        scan_info: ScanInfo,
        backup_info: &Option<BackupInfo>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        restoring: bool,
    ) -> Self {
        let nodes = Self::initialize_nodes(scan_info, backup_info, duplicate_detector, config, restoring);
        let expansion = Expansion::new(&nodes);
        Self { nodes, expansion }
    }

    pub fn clear_nodes(&mut self) {
        self.nodes.clear();
    }

    pub fn reset_nodes(
        &mut self,
        scan_info: ScanInfo,
        backup_info: &Option<BackupInfo>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        restoring: bool,
    ) {
        self.nodes = Self::initialize_nodes(scan_info, backup_info, duplicate_detector, config, restoring);
    }

    fn initialize_nodes(
        scan_info: ScanInfo,
        backup_info: &Option<BackupInfo>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        restoring: bool,
    ) -> BTreeMap<TreeNodeKey, FileTreeNode> {
        let mut nodes = BTreeMap::<TreeNodeKey, FileTreeNode>::new();

        for item in scan_info.found_files.iter() {
            let rendered = item.readable(scan_info.restoring());
            let components: Vec<_> = rendered.split('/').map(|x| TreeNodeKey::File(x.to_string())).collect();

            nodes
                .entry(components[0].clone())
                .or_insert_with(|| {
                    FileTreeNode::new(
                        &scan_info.game_name,
                        vec![components[0].clone()],
                        FileTreeNodePath::File(StrictPath::new({
                            let tip = components[0].raw().to_string();
                            if tip.is_empty() {
                                "/".to_string()
                            } else {
                                tip
                            }
                        })),
                        FileTreeNodeType::File,
                        config,
                        restoring,
                    )
                })
                .insert_keys(
                    &scan_info.game_name,
                    &components[1..],
                    &[components[0].clone()],
                    backup_info.as_ref().and_then(|x| x.failed_files.get(item)),
                    duplicate_detector.is_file_duplicated(item),
                    item.change(),
                    Some(item.clone()),
                    None,
                    duplicate_detector,
                    config,
                    restoring,
                );
        }
        for item in scan_info.found_registry_keys.iter() {
            let components: Vec<_> = item
                .path
                .split()
                .iter()
                .map(|x| TreeNodeKey::RegistryKey(x.to_string()))
                .collect();

            nodes
                .entry(components[0].clone())
                .or_insert_with(|| {
                    FileTreeNode::new(
                        &scan_info.game_name,
                        vec![components[0].clone()],
                        FileTreeNodePath::RegistryKey(RegistryItem::new(components[0].raw().to_string())),
                        FileTreeNodeType::RegistryKey,
                        config,
                        restoring,
                    )
                })
                .insert_keys(
                    &scan_info.game_name,
                    &components[1..],
                    &components[0..1],
                    backup_info.as_ref().and_then(|x| x.failed_registry.get(&item.path)),
                    duplicate_detector.is_registry_duplicated(&item.path),
                    item.change(restoring),
                    None,
                    Some(&item.values),
                    duplicate_detector,
                    config,
                    restoring,
                );
        }

        nodes
    }

    pub fn view(&self, game_name: &str, config: &Config, restoring: bool) -> Container {
        Container::new(
            self.nodes.iter().filter(|(_, v)| v.anything_showable()).fold(
                Column::new().spacing(4),
                |parent, (k, v)| {
                    parent.push(v.view(0, k.raw().to_string(), game_name, config, restoring, &self.expansion))
                },
            ),
        )
    }

    pub fn expand_or_collapse_keys(&mut self, keys: &[TreeNodeKey]) {
        if keys.is_empty() {
            return;
        }

        let mut node = self.expansion.0.get_mut(&keys[0]);
        for key in &keys[1..] {
            match node {
                Some(state) => {
                    node = state.nodes.get_mut(key);
                }
                None => break,
            }
        }

        if let Some(state) = node {
            state.expanded = !state.expanded
        }
    }
}
