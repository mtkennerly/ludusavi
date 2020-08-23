use crate::{
    config::Config,
    gui::{badge::Badge, common::Message, icon::Icon, style},
    lang::Translator,
    path::StrictPath,
    prelude::{game_file_restoration_target, BackupInfo, DuplicateDetector, ScanInfo},
};
use iced::{button, Align, Button, Column, Container, Length, Row, Space, Text};

#[derive(Clone, Debug, Default)]
struct FileTreeNode {
    keys: Vec<String>,
    expand_button: button::State,
    expanded: bool,
    path: Option<StrictPath>,
    nodes: std::collections::BTreeMap<String, FileTreeNode>,
    successful: bool,
    duplicated: bool,
    redirected_from: Option<StrictPath>,
}

impl FileTreeNode {
    pub fn new(keys: Vec<String>, path: Option<StrictPath>) -> Self {
        Self {
            keys,
            path,
            ..Default::default()
        }
    }

    pub fn view(&mut self, level: u16, label: &str, translator: &Translator, game_name: &str) -> Container<Message> {
        let expanded = self.expanded;

        if self.nodes.is_empty() {
            return Container::new(
                Row::new()
                    .push(Space::new(Length::Units(35 * level), Length::Shrink))
                    .push(
                        Icon::SubdirectoryArrowRight
                            .as_text()
                            .height(Length::Units(25))
                            .width(Length::Units(25))
                            .size(25),
                    )
                    .push(Space::new(Length::Units(10), Length::Shrink))
                    .push(Text::new(label))
                    .push(
                        Badge::new(&translator.badge_duplicated())
                            .left_margin(15)
                            .view_if(self.duplicated),
                    )
                    .push(
                        Badge::new(&translator.badge_failed())
                            .left_margin(15)
                            .view_if(!self.successful),
                    )
                    .push(match &self.redirected_from {
                        Some(r) => Badge::new(&translator.badge_redirected_from(&r)).left_margin(15).view(),
                        None => Container::new(Space::new(Length::Shrink, Length::Shrink)),
                    }),
            );
        } else if self.nodes.len() == 1 {
            let keys: Vec<_> = self.nodes.keys().cloned().collect();
            let key = &keys[0];
            if !self.nodes.get::<str>(&key).unwrap().nodes.is_empty() {
                return Container::new(self.nodes.get_mut::<str>(&key).unwrap().view(
                    level,
                    &format!("{}/{}", label, key),
                    &translator,
                    &game_name,
                ));
            }
        }

        Container::new(
            self.nodes.iter_mut().fold(
                Column::new().push(
                    Row::new()
                        .align_items(Align::Center)
                        .push(Space::new(Length::Units(35 * level), Length::Shrink))
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
                            .style(style::Button::Primary)
                            .height(Length::Units(25))
                            .width(Length::Units(25)),
                        )
                        .push(Space::new(Length::Units(10), Length::Shrink))
                        .push(Text::new(label)),
                ),
                |parent, (k, v)| {
                    if expanded {
                        parent.push(v.view(level + 1, k, &translator, &game_name))
                    } else {
                        parent
                    }
                },
            ),
        )
    }

    fn insert_keys(
        &mut self,
        keys: &[&str],
        prefix_keys: &[&str],
        canonical_leaf_path: Option<StrictPath>,
        successful: bool,
        duplicated: bool,
        redirected_from: Option<StrictPath>,
    ) -> &mut Self {
        let mut node = self;
        let mut inserted_keys = vec![];
        let mut full_keys: Vec<_> = prefix_keys.iter().map(|x| x.to_string()).collect();
        for key in keys.iter() {
            inserted_keys.push(key.to_string());
            full_keys.push(key.to_string());
            node = node.nodes.entry(key.to_string()).or_insert_with(|| {
                FileTreeNode::new(full_keys.clone(), Some(StrictPath::new(inserted_keys.join("/"))))
            });
        }

        node.path = canonical_leaf_path;
        node.successful = successful;
        node.duplicated = duplicated;
        node.redirected_from = redirected_from;

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
        translator: &Translator,
    ) -> Self {
        let mut nodes = std::collections::BTreeMap::<String, FileTreeNode>::new();

        for item in scan_info.found_files.iter() {
            let mut redirected_from = None;
            let path_to_show = if let Some(original_path) = &item.original_path {
                let (target, original_target) = game_file_restoration_target(&original_path, &config.get_redirects());
                redirected_from = original_target;
                target.clone()
            } else {
                item.path.clone()
            };

            let mut successful = true;
            if let Some(backup_info) = &backup_info {
                if backup_info.failed_files.contains(&item) {
                    successful = false;
                }
            }

            let rendered = path_to_show.render();
            let components: Vec<_> = rendered.split('/').collect();

            nodes
                .entry(components[0].to_string())
                .or_insert_with(|| FileTreeNode::new(vec![components[0].to_string()], None))
                .insert_keys(
                    &components[1..],
                    &[components[0]],
                    Some(path_to_show.clone()),
                    successful,
                    duplicate_detector.is_file_duplicated(&item),
                    redirected_from,
                );
        }
        for item in scan_info.found_registry_keys.iter() {
            let mut successful = true;
            if let Some(backup_info) = &backup_info {
                if backup_info.failed_registry.contains(item) {
                    successful = false;
                }
            }

            nodes
                .entry(translator.registry_label())
                .or_insert_with(|| FileTreeNode::new(vec![translator.registry_label()], None))
                .insert_keys(
                    &[item],
                    &[&translator.registry_label()],
                    None,
                    successful,
                    duplicate_detector.is_registry_duplicated(&item),
                    None,
                );
        }

        for item in nodes.values_mut() {
            item.expand_short();
        }

        Self { nodes }
    }

    pub fn view(&mut self, translator: &Translator, game_name: &str) -> Container<Message> {
        Container::new(self.nodes.iter_mut().fold(Column::new().spacing(4), |parent, (k, v)| {
            parent.push(v.view(0, k, &translator, &game_name))
        }))
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
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
}
