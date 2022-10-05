use crate::{
    config::{Config, ToggledPaths, ToggledRegistry},
    gui::{
        badge::Badge,
        common::{IcedExtension, Message},
        icon::Icon,
        style,
    },
    lang::Translator,
    path::StrictPath,
    prelude::{game_file_target, BackupInfo, DuplicateDetector, RegistryItem, ResolvedRedirect, ScanInfo},
};
use iced::{button, Alignment, Button, Checkbox, Column, Container, Length, Row, Space, Text};

#[derive(Clone, Debug, Eq, PartialEq)]
enum FileTreeNodeType {
    File,
    Registry,
}

impl Default for FileTreeNodeType {
    fn default() -> Self {
        Self::File
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FileTreeNodePath {
    File(StrictPath),
    Registry(RegistryItem),
}

#[derive(Clone, Debug, Default)]
struct FileTreeNode {
    keys: Vec<String>,
    expand_button: button::State,
    expanded: bool,
    open_button: button::State,
    path: Option<FileTreeNodePath>,
    nodes: std::collections::BTreeMap<String, FileTreeNode>,
    successful: bool,
    ignored: bool,
    duplicated: bool,
    redirect: Option<ResolvedRedirect>,
    node_type: FileTreeNodeType,
}

impl FileTreeNode {
    pub fn new(keys: Vec<String>, path: Option<FileTreeNodePath>, node_type: FileTreeNodeType) -> Self {
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

    pub fn view(
        &mut self,
        level: u16,
        label: &str,
        translator: &Translator,
        game_name: &str,
        config: &Config,
        restoring: bool,
    ) -> Container<Message> {
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
                        Checkbox::new(!self.ignored, "", move |enabled| match &path {
                            FileTreeNodePath::File(path) => Message::ToggleSpecificBackupPathIgnored {
                                name: game_name.clone(),
                                path: path.clone(),
                                enabled,
                            },
                            FileTreeNodePath::Registry(path) => Message::ToggleSpecificBackupRegistryIgnored {
                                name: game_name.clone(),
                                path: path.clone(),
                                enabled,
                            },
                        })
                        .style(style::Checkbox(config.theme)),
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
                    .padding([0, 0, 0, 35 * level])
                    .push(
                        Icon::SubdirectoryArrowRight
                            .as_text()
                            .height(Length::Units(25))
                            .width(Length::Units(25))
                            .size(25),
                    )
                    .push(Space::new(Length::Units(10), Length::Shrink))
                    .push_some(make_enabler)
                    .push(Text::new(label))
                    .push_if(
                        || self.duplicated,
                        || {
                            Badge::new(&translator.badge_duplicated())
                                .left_margin(15)
                                .view(config.theme)
                        },
                    )
                    .push_if(
                        || !self.successful,
                        || {
                            Badge::new(&translator.badge_failed())
                                .left_margin(15)
                                .view(config.theme)
                        },
                    )
                    .push_some(|| {
                        self.redirect.as_ref().and_then(|redirect| {
                            redirect.alt().as_ref().map(|alt| {
                                let msg = if redirect.restoring {
                                    translator.badge_redirected_from(alt)
                                } else {
                                    translator.badge_redirecting_to(alt)
                                };
                                Badge::new(&msg).left_margin(15).view(config.theme)
                            })
                        })
                    }),
            );
        } else if self.nodes.len() == 1 {
            let keys: Vec<_> = self.nodes.keys().cloned().collect();
            let key = &keys[0];
            if !self.nodes.get::<str>(key).unwrap().nodes.is_empty() {
                return Container::new(self.nodes.get_mut::<str>(key).unwrap().view(
                    level,
                    &format!("{}/{}", label, key),
                    translator,
                    game_name,
                    config,
                    restoring,
                ));
            }
        }

        Container::new(
            self.nodes.iter_mut().filter(|(_, v)| v.anything_showable()).fold(
                Column::new().push(
                    Row::new()
                        .align_items(Alignment::Center)
                        .padding([0, 10, 0, 35 * level])
                        .push(
                            Button::new(
                                &mut self.expand_button,
                                (if expanded {
                                    Icon::KeyboardArrowDown
                                } else {
                                    Icon::KeyboardArrowRight
                                })
                                .as_text()
                                .width(Length::Units(15))
                                .size(15),
                            )
                            .on_press(Message::ToggleGameListEntryTreeExpanded {
                                name: game_name.to_string(),
                                keys: self.keys.clone(),
                            })
                            .style(style::Button::Primary(config.theme))
                            .height(Length::Units(25))
                            .width(Length::Units(25)),
                        )
                        .push(Space::new(Length::Units(10), Length::Shrink))
                        .push_some(make_enabler)
                        .push(Text::new(label))
                        .push(Space::new(Length::Units(10), Length::Shrink))
                        .push_some(|| {
                            if let Some(FileTreeNodePath::File(path)) = &self.path {
                                return Some(
                                    Button::new(
                                        &mut self.open_button,
                                        Icon::OpenInNew.as_text().width(Length::Shrink).size(15),
                                    )
                                    .on_press(Message::OpenDir { path: path.clone() })
                                    .style(style::Button::Primary(config.theme))
                                    .height(Length::Units(25)),
                                );
                            }
                            None
                        }),
                ),
                |parent, (k, v)| {
                    parent.push_if(
                        || expanded,
                        || v.view(level + 1, k, translator, game_name, config, restoring),
                    )
                },
            ),
        )
    }

    fn insert_keys<T: AsRef<str> + ToString>(
        &mut self,
        keys: &[T],
        prefix_keys: &[T],
        successful: bool,
        duplicated: bool,
        redirect: Option<ResolvedRedirect>,
    ) -> &mut Self {
        let node_type = self.node_type.clone();
        let mut node = self;
        let mut inserted_keys = vec![];
        for key in prefix_keys.iter() {
            inserted_keys.push(key.to_string());
        }
        let mut full_keys: Vec<_> = prefix_keys.iter().map(|x| x.to_string()).collect();
        for key in keys.iter() {
            inserted_keys.push(key.to_string());
            full_keys.push(key.to_string());
            node = node.nodes.entry(key.to_string()).or_insert_with(|| {
                FileTreeNode::new(
                    full_keys.clone(),
                    match &node_type {
                        FileTreeNodeType::File => {
                            Some(FileTreeNodePath::File(StrictPath::new(inserted_keys.join("/"))))
                        }
                        FileTreeNodeType::Registry => {
                            Some(FileTreeNodePath::Registry(RegistryItem::new(inserted_keys.join("/"))))
                        }
                    },
                    node_type.clone(),
                )
            });
        }

        node.successful = successful;
        node.duplicated = duplicated;
        node.redirect = redirect;

        node
    }

    fn expand_or_collapse_keys(&mut self, keys: &[String]) -> &mut Self {
        let mut node = self;
        let mut visited_keys = vec![];
        for key in keys.iter() {
            visited_keys.push(key.to_string());
            node = node.nodes.entry(key.to_string()).or_insert_with(Default::default);
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
            Some(FileTreeNodePath::Registry(path)) => {
                self.ignored = ignored_registry.is_ignored(game, path);
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
    nodes: std::collections::BTreeMap<String, FileTreeNode>,
}

impl FileTree {
    pub fn new(
        scan_info: ScanInfo,
        config: &Config,
        backup_info: &Option<BackupInfo>,
        duplicate_detector: &DuplicateDetector,
    ) -> Self {
        let mut nodes = std::collections::BTreeMap::<String, FileTreeNode>::new();

        for item in scan_info.found_files.iter() {
            let resolved = game_file_target(item.original_path(), &config.get_redirects(), scan_info.restoring());

            let mut successful = true;
            if let Some(backup_info) = &backup_info {
                if backup_info.failed_files.contains(item) {
                    successful = false;
                }
            }

            let rendered = resolved.readable();
            let components: Vec<_> = rendered.split('/').collect();

            nodes
                .entry(components[0].to_string())
                .or_insert_with(|| FileTreeNode::new(vec![components[0].to_string()], None, FileTreeNodeType::File))
                .insert_keys(
                    &components[1..],
                    &[components[0]],
                    successful,
                    duplicate_detector.is_file_duplicated(item),
                    Some(resolved),
                );
        }
        for item in scan_info.found_registry_keys.iter() {
            let mut successful = true;
            if let Some(backup_info) = &backup_info {
                if backup_info.failed_registry.contains(&item.path) {
                    successful = false;
                }
            }

            let components: Vec<_> = item.path.split();

            nodes
                .entry(components[0].to_string())
                .or_insert_with(|| FileTreeNode::new(vec![components[0].to_string()], None, FileTreeNodeType::Registry))
                .insert_keys(
                    &components[1..],
                    &components[0..1],
                    successful,
                    duplicate_detector.is_registry_duplicated(&item.path),
                    None,
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

    pub fn view(
        &mut self,
        translator: &Translator,
        game_name: &str,
        config: &Config,
        restoring: bool,
    ) -> Container<Message> {
        Container::new(
            self.nodes
                .iter_mut()
                .filter(|(_, v)| v.anything_showable())
                .fold(Column::new().spacing(4), |parent, (k, v)| {
                    parent.push(v.view(0, k, translator, game_name, config, restoring))
                }),
        )
    }

    pub fn expand_or_collapse_keys(&mut self, keys: &[String]) {
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
