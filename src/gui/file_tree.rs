use std::collections::{BTreeMap, HashMap};

use iced::{padding, Alignment};
use itertools::Itertools;

use crate::{
    gui::{
        badge::Badge,
        button,
        common::{Message, TreeNodeKey},
        icon::Icon,
        style,
        widget::{checkbox, text, Button, Column, Container, IcedParentExt, Row},
    },
    lang::TRANSLATOR,
    path::StrictPath,
    resource::{
        config::{self, Config},
        manifest::Os,
    },
    scan::{
        registry::RegistryItem, BackupError, BackupInfo, DuplicateDetector, Duplication, ScanChange, ScanInfo,
        ScanKind, ScannedFile, ScannedRegistryValues,
    },
};

fn check_ignored(game: &str, path: &FileTreeNodePath, config: &Config, scan_kind: ScanKind) -> bool {
    let (paths, registries) = match scan_kind {
        ScanKind::Backup => (&config.backup.toggled_paths, &config.backup.toggled_registry),
        ScanKind::Restore => (&config.restore.toggled_paths, &config.restore.toggled_registry),
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
    scanned_file: Option<(StrictPath, ScannedFile)>,
    node_type: FileTreeNodeType,
}

impl FileTreeNode {
    pub fn new(
        game: &str,
        keys: Vec<TreeNodeKey>,
        path: FileTreeNodePath,
        node_type: FileTreeNodeType,
        config: &Config,
        scan_kind: ScanKind,
    ) -> Self {
        let ignored = check_ignored(game, &path, config, scan_kind);
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
        level: u32,
        label: String,
        game_name: &str,
        _config: &Config,
        scan_kind: ScanKind,
        expansion: &Expansion,
    ) -> Container {
        let expanded = expansion.expanded(&self.keys);

        let make_enabler = || {
            let game_name = game_name.to_string();
            let path = self.path.clone();
            Some(
                Container::new(
                    checkbox(
                        "",
                        !self.ignored,
                        Message::config(move |_| match &path {
                            FileTreeNodePath::File(path) => config::Event::ToggleSpecificGamePathIgnored {
                                name: game_name.clone(),
                                path: path.clone(),
                                scan_kind,
                            },
                            FileTreeNodePath::RegistryKey(path) => config::Event::ToggleSpecificGameRegistryIgnored {
                                name: game_name.clone(),
                                path: path.clone(),
                                value: None,
                                scan_kind,
                            },
                            FileTreeNodePath::RegistryValue(path, name) => {
                                config::Event::ToggleSpecificGameRegistryIgnored {
                                    name: game_name.clone(),
                                    path: path.clone(),
                                    value: Some(name.clone()),
                                    scan_kind,
                                }
                            }
                        }),
                    )
                    .spacing(5)
                    .class(style::Checkbox),
                )
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            )
        };

        if self.nodes.is_empty() {
            return Container::new(
                Row::new()
                    .align_y(Alignment::Center)
                    .padding(padding::left(35 * level).right(10))
                    .spacing(10)
                    .push(match self.node_type {
                        FileTreeNodeType::File | FileTreeNodeType::RegistryValue(_) => {
                            Container::new(Icon::SubdirectoryArrowRight.text().height(25).width(25).size(25))
                        }
                        FileTreeNodeType::RegistryKey => Container::new(
                            Button::new(Icon::KeyboardArrowDown.text_small())
                                .class(style::Button::Primary)
                                .padding(5)
                                .height(25)
                                .width(25),
                        ),
                    })
                    .push(make_enabler())
                    .push(text(label))
                    .push({
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
                    .push(
                        self.error
                            .as_ref()
                            .map(|x| Badge::new(&TRANSLATOR.badge_failed()).tooltip(x.clone()).view()),
                    )
                    .push({
                        self.scanned_file.as_ref().and_then(|(scan_key, scanned)| {
                            let scan_kind = scanned.scan_kind();
                            scanned.alt(scan_key, scan_kind).as_ref().map(|alt| {
                                let msg = match scan_kind {
                                    ScanKind::Backup => TRANSLATOR.badge_redirecting_to(alt),
                                    ScanKind::Restore => TRANSLATOR.badge_redirected_from(alt),
                                };
                                Badge::new(&msg).view()
                            })
                        })
                    })
                    .push({
                        self.scanned_file.as_ref().map(|(_, f)| {
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
                    scan_kind,
                    expansion,
                ));
            }
        }

        Container::new(
            self.nodes.iter().filter(|(_, v)| v.anything_showable()).fold(
                Column::new().push(
                    Row::new()
                        .align_y(Alignment::Center)
                        .padding(padding::left(35 * level).right(10))
                        .spacing(10)
                        .push(button::expand(
                            expanded,
                            Message::ToggleGameListEntryTreeExpanded {
                                name: game_name.to_string(),
                                keys: self.keys.clone(),
                            },
                        ))
                        .push(make_enabler())
                        .push(
                            Row::new()
                                .align_y(Alignment::Center)
                                .spacing(10)
                                .push(text(if label.is_empty() && self.node_type == FileTreeNodeType::File {
                                    "/".to_string()
                                } else {
                                    label
                                }))
                                .push(
                                    self.error
                                        .as_ref()
                                        .map(|x| Badge::new(&TRANSLATOR.badge_failed()).tooltip(x.clone()).view()),
                                )
                                .push(match &self.path {
                                    FileTreeNodePath::File(path) => Some(
                                        Button::new(Icon::OpenInNew.text_small())
                                            .on_press(Message::OpenDir { path: path.clone() })
                                            .class(style::Button::Primary)
                                            .padding(5)
                                            .height(25),
                                    ),
                                    _ => None,
                                })
                                .push(match &self.path {
                                    FileTreeNodePath::RegistryKey(item) if Os::HOST == Os::Windows => Some(
                                        Button::new(Icon::OpenInNew.text_small())
                                            .on_press(Message::OpenRegistry(item.clone()))
                                            .class(style::Button::Primary)
                                            .padding(5)
                                            .height(25),
                                    ),
                                    _ => None,
                                })
                                .push(match &self.path {
                                    FileTreeNodePath::RegistryKey(item) => Some(
                                        Button::new(Icon::Copy.text_small())
                                            .on_press(Message::CopyText(item.interpret()))
                                            .class(style::Button::Primary)
                                            .padding(5)
                                            .height(25),
                                    ),
                                    _ => None,
                                })
                                .push({
                                    let total_bytes = self.calculate_directory_size(true);
                                    let total_size = total_bytes.map(|bytes| TRANSLATOR.adjusted_size(bytes));

                                    let included_bytes = self.calculate_directory_size(false);
                                    let included_size = included_bytes.map(|bytes| TRANSLATOR.adjusted_size(bytes));

                                    let text = match (included_size, total_size) {
                                        (Some(included), Some(total)) => {
                                            if included_bytes == total_bytes {
                                                Some(included)
                                            } else {
                                                Some(format!("{included} / {total}"))
                                            }
                                        }
                                        (Some(included), None) => Some(format!("{included} / ?")),
                                        (None, Some(total)) => Some(total.to_string()),
                                        (None, None) => None,
                                    };

                                    text.map(|text| Badge::new(&text).faded(included_bytes.is_none()).view())
                                })
                                .wrap(),
                        ),
                ),
                |parent, (k, v)| {
                    parent.push_if(expanded, || {
                        v.view(level + 1, k.raw().to_string(), game_name, _config, scan_kind, expansion)
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
        scanned_file: Option<(StrictPath, ScannedFile)>,
        registry_values: Option<&ScannedRegistryValues>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        scan_kind: ScanKind,
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
            let raw_path = inserted_keys.iter().map(|x| x.raw()).join("/");
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
                    scan_kind,
                )
            });
        }

        node.error = error.map(|x| x.message());
        node.duplicated = duplicated;
        node.change = change;
        node.scanned_file = scanned_file;

        if let Some(registry_values) = registry_values {
            let raw_key_path = inserted_keys.iter().map(|x| x.raw()).join("/");
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
                            scan_kind,
                        )
                    });
                node.error = None;
                node.duplicated = duplicate_detector
                    .is_registry_value_duplicated(&RegistryItem::new(raw_key_path.clone()), value_name);
                node.change = value.change(scan_kind);
            }
        }

        node
    }

    fn calculate_directory_size(&self, include_ignored: bool) -> Option<u64> {
        let mut size = 0;
        for child_node in self.nodes.values() {
            if child_node.nodes.is_empty() {
                if let Some((_, scanned_file)) = &child_node.scanned_file {
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
        backup_info: Option<&BackupInfo>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        scan_kind: ScanKind,
    ) -> Self {
        let nodes = Self::initialize_nodes(scan_info, backup_info, duplicate_detector, config, scan_kind);
        let expansion = Expansion::new(&nodes);
        Self { nodes, expansion }
    }

    pub fn clear_nodes(&mut self) {
        self.nodes.clear();
    }

    pub fn reset_nodes(
        &mut self,
        scan_info: ScanInfo,
        backup_info: Option<&BackupInfo>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        scan_kind: ScanKind,
    ) {
        self.nodes = Self::initialize_nodes(scan_info, backup_info, duplicate_detector, config, scan_kind);
    }

    fn initialize_nodes(
        scan_info: ScanInfo,
        backup_info: Option<&BackupInfo>,
        duplicate_detector: &DuplicateDetector,
        config: &Config,
        scan_kind: ScanKind,
    ) -> BTreeMap<TreeNodeKey, FileTreeNode> {
        let mut nodes = BTreeMap::<TreeNodeKey, FileTreeNode>::new();

        for (scan_key, item) in &scan_info.found_files {
            let rendered = item.readable(scan_key, scan_info.scan_kind());
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
                        scan_kind,
                    )
                })
                .insert_keys(
                    &scan_info.game_name,
                    &components[1..],
                    &[components[0].clone()],
                    backup_info.as_ref().and_then(|x| x.failed_files.get(scan_key)),
                    duplicate_detector.is_file_duplicated(scan_key, item),
                    item.change(),
                    Some((scan_key.clone(), item.clone())),
                    None,
                    duplicate_detector,
                    config,
                    scan_kind,
                );
        }
        for (scan_key, item) in &scan_info.found_registry_keys {
            let components: Vec<_> = scan_key
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
                        scan_kind,
                    )
                })
                .insert_keys(
                    &scan_info.game_name,
                    &components[1..],
                    &components[0..1],
                    backup_info.as_ref().and_then(|x| x.failed_registry.get(scan_key)),
                    duplicate_detector.is_registry_duplicated(scan_key),
                    item.change(scan_kind),
                    None,
                    Some(&item.values),
                    duplicate_detector,
                    config,
                    scan_kind,
                );
        }

        nodes
    }

    pub fn view(&self, game_name: &str, config: &Config, scan_kind: ScanKind) -> Container {
        Container::new(
            self.nodes.iter().filter(|(_, v)| v.anything_showable()).fold(
                Column::new().spacing(4),
                |parent, (k, v)| {
                    parent.push(v.view(0, k.raw().to_string(), game_name, config, scan_kind, &self.expansion))
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
