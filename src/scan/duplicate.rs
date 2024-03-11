use std::collections::{HashMap, HashSet};

use crate::{
    prelude::StrictPath,
    scan::{registry_compat::RegistryItem, ScanChange, ScanInfo, ScannedFile},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Duplication {
    #[default]
    Unique,
    Resolved,
    Duplicate,
}

impl Duplication {
    pub fn unique(&self) -> bool {
        matches!(self, Self::Unique)
    }

    pub fn resolved(&self) -> bool {
        matches!(self, Self::Resolved | Self::Unique)
    }

    pub fn evaluate<'a>(items: impl Iterator<Item = &'a DuplicateDetectorEntry> + Clone) -> Duplication {
        let mut total = 0;
        let mut enabled = 0;
        let mut removed = 0;

        for item in items {
            total += 1;
            if item.enabled {
                enabled += 1;
            }
            if item.change == ScanChange::Removed {
                removed += 1;
            }
        }

        if total < 2 {
            Duplication::Unique
        } else if enabled <= 1 || removed >= total - 1 {
            Duplication::Resolved
        } else {
            Duplication::Duplicate
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DuplicateDetectorEntry {
    enabled: bool,
    change: ScanChange,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DuplicateDetectorCount {
    non_unique: u32,
    resolved: u32,
}

impl DuplicateDetectorCount {
    pub fn evaluate(&self) -> Duplication {
        if self.non_unique == 0 {
            Duplication::Unique
        } else if self.non_unique == self.resolved {
            Duplication::Resolved
        } else {
            Duplication::Duplicate
        }
    }

    pub fn add(&mut self, other: &Self) {
        self.non_unique += other.non_unique;
        self.resolved += other.resolved;
    }
}

#[derive(Clone, Debug, Default)]
pub struct DuplicateDetector {
    files: HashMap<StrictPath, HashMap<String, DuplicateDetectorEntry>>,
    registry: HashMap<RegistryItem, HashMap<String, DuplicateDetectorEntry>>,
    registry_values: HashMap<RegistryItem, HashMap<String, HashMap<String, DuplicateDetectorEntry>>>,
    game_files: HashMap<String, HashSet<StrictPath>>,
    game_registry: HashMap<String, HashSet<RegistryItem>>,
    game_registry_values: HashMap<String, HashMap<RegistryItem, HashSet<String>>>,
    game_duplicated_items: HashMap<String, DuplicateDetectorCount>,
}

impl DuplicateDetector {
    pub fn add_game(&mut self, scan_info: &ScanInfo, game_enabled: bool) -> HashSet<String> {
        let mut stale = self.remove_game_and_refresh(&scan_info.game_name, false);
        stale.insert(scan_info.game_name.clone());

        for item in scan_info.found_files.iter() {
            let path = self.pick_path(item);
            if let Some(existing) = self.files.get(&path).map(|x| x.keys()) {
                // Len 0: No games to update counts for.
                // Len 2+: These games already include the item in their duplicate counts.
                if existing.len() == 1 {
                    stale.extend(existing.cloned());
                }
            }
            self.files.entry(path.clone()).or_default().insert(
                scan_info.game_name.clone(),
                DuplicateDetectorEntry {
                    enabled: game_enabled && !item.ignored,
                    change: item.change(),
                },
            );
            self.game_files
                .entry(scan_info.game_name.clone())
                .or_default()
                .insert(path);
        }

        for item in scan_info.found_registry_keys.iter() {
            let path = item.path.clone();
            if let Some(existing) = self.registry.get(&path).map(|x| x.keys()) {
                if existing.len() == 1 {
                    stale.extend(existing.cloned());
                }
            }
            self.registry.entry(path.clone()).or_default().insert(
                scan_info.game_name.clone(),
                DuplicateDetectorEntry {
                    enabled: game_enabled && !item.ignored,
                    change: item.change(scan_info.restoring()),
                },
            );
            self.game_registry
                .entry(scan_info.game_name.clone())
                .or_default()
                .insert(path.clone());

            for (value_name, value) in item.values.iter() {
                self.registry_values
                    .entry(path.clone())
                    .or_default()
                    .entry(value_name.to_string())
                    .or_default()
                    .insert(
                        scan_info.game_name.clone(),
                        DuplicateDetectorEntry {
                            enabled: game_enabled && !value.ignored,
                            change: value.change(scan_info.restoring()),
                        },
                    );
                self.game_registry_values
                    .entry(scan_info.game_name.clone())
                    .or_default()
                    .entry(path.clone())
                    .or_default()
                    .insert(value_name.to_string());
            }
        }

        for game in &stale {
            self.game_duplicated_items
                .insert(game.clone(), self.count_duplicated_items_for(game));
        }

        stale.extend(self.duplicate_games(&scan_info.game_name));
        stale.remove(&scan_info.game_name);
        stale
    }

    pub fn remove_game(&mut self, game: &str) -> HashSet<String> {
        self.remove_game_and_refresh(game, true)
    }

    fn remove_game_and_refresh(&mut self, game: &str, refresh: bool) -> HashSet<String> {
        let mut stale = HashSet::new();

        self.game_duplicated_items.remove(game);

        if let Some(files) = self.game_files.remove(game) {
            for file in files {
                if let Some(games) = self.files.get_mut(&file) {
                    games.remove(game);
                    for duplicate in games.keys() {
                        stale.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry.remove(game) {
            for registry in registry_keys {
                if let Some(games) = self.registry.get_mut(&registry) {
                    games.remove(game);
                    for duplicate in games.keys() {
                        stale.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry_values.remove(game) {
            for (registry_key, registry_values) in registry_keys {
                for registry_value in registry_values {
                    if let Some(games) = self
                        .registry_values
                        .get_mut(&registry_key)
                        .and_then(|x| x.get_mut(&registry_value))
                    {
                        games.remove(game);
                        for duplicate in games.keys() {
                            stale.insert(duplicate.clone());
                        }
                    }
                }
            }
        }

        if refresh {
            for game in &stale {
                self.game_duplicated_items
                    .insert(game.clone(), self.count_duplicated_items_for(game));
            }
        }

        stale
    }

    pub fn is_game_duplicated(&self, game: &str) -> Duplication {
        self.count_duplicates_for(game).evaluate()
    }

    fn pick_path(&self, file: &ScannedFile) -> StrictPath {
        match &file.original_path {
            Some(op) => op.clone(),
            None => file.path.clone(),
        }
    }

    pub fn file(&self, file: &ScannedFile) -> HashMap<String, DuplicateDetectorEntry> {
        match self.files.get(&self.pick_path(file)) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_file_duplicated(&self, file: &ScannedFile) -> Duplication {
        Duplication::evaluate(self.file(file).values())
    }

    pub fn registry(&self, path: &RegistryItem) -> HashMap<String, DuplicateDetectorEntry> {
        match self.registry.get(path) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_registry_duplicated(&self, path: &RegistryItem) -> Duplication {
        Duplication::evaluate(self.registry(path).values())
    }

    pub fn registry_value(&self, path: &RegistryItem, value: &str) -> HashMap<String, DuplicateDetectorEntry> {
        match self.registry_values.get(path).and_then(|key| key.get(value)) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_registry_value_duplicated(&self, path: &RegistryItem, value: &str) -> Duplication {
        Duplication::evaluate(self.registry_value(path, value).values())
    }

    pub fn clear(&mut self) {
        self.files.clear();
        self.registry.clear();
        self.registry_values.clear();
        self.game_duplicated_items.clear();
    }

    pub fn overall(&self) -> Duplication {
        let mut count = DuplicateDetectorCount::default();

        for item in self.game_duplicated_items.values() {
            count.add(item);
        }

        count.evaluate()
    }

    fn count_duplicated_items_for(&self, game: &str) -> DuplicateDetectorCount {
        let mut tally = DuplicateDetectorCount::default();
        for item in self.files.values() {
            if item.contains_key(game) && item.len() > 1 {
                tally.non_unique += 1;
                if item.values().filter(|x| !x.change.is_inert()).count() <= 1 {
                    tally.resolved += 1;
                }
            }
        }
        for item in self.registry.values() {
            if item.contains_key(game) && item.len() > 1 {
                tally.non_unique += 1;
                if item.values().filter(|x| !x.change.is_inert()).count() <= 1 {
                    tally.resolved += 1;
                }
            }
        }
        for item in self.registry_values.values() {
            for item in item.values() {
                if item.contains_key(game) && item.len() > 1 {
                    tally.non_unique += 1;
                    if item.values().filter(|x| !x.change.is_inert()).count() <= 1 {
                        tally.resolved += 1;
                    }
                }
            }
        }
        tally
    }

    fn count_duplicates_for(&self, game: &str) -> DuplicateDetectorCount {
        self.game_duplicated_items.get(game).copied().unwrap_or_default()
    }

    pub fn duplicate_games(&self, game: &str) -> HashSet<String> {
        let mut duplicates = HashSet::new();

        if let Some(files) = self.game_files.get(game) {
            for file in files {
                if let Some(games) = self.files.get(file) {
                    if games.len() < 2 {
                        continue;
                    }
                    for duplicate in games.keys() {
                        duplicates.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry.get(game) {
            for registry in registry_keys {
                if let Some(games) = self.registry.get(registry) {
                    if games.len() < 2 {
                        continue;
                    }
                    for duplicate in games.keys() {
                        duplicates.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry_values.get(game) {
            for (registry_key, registry_values) in registry_keys {
                for registry_value in registry_values {
                    if let Some(games) = self
                        .registry_values
                        .get(registry_key)
                        .and_then(|x| x.get(registry_value))
                    {
                        if games.len() < 2 {
                            continue;
                        }
                        for duplicate in games.keys() {
                            duplicates.insert(duplicate.clone());
                        }
                    }
                }
            }
        }

        duplicates.remove(game);
        duplicates
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{hash_map, hash_set};

    use super::*;
    use crate::{scan::ScannedRegistry, testing::s};

    #[test]
    fn can_add_games_in_backup_mode() {
        let mut detector = DuplicateDetector::default();

        let game1 = s("game1");
        let game2 = s("game2");
        let file1 = ScannedFile::new("file1.txt", 1, "1");
        let file2 = ScannedFile::new("file2.txt", 2, "2");
        let reg1 = s("reg1");
        let reg2 = s("reg2");

        detector.add_game(
            &ScanInfo {
                game_name: game1.clone(),
                found_files: hash_set! { file1.clone(), file2.clone() },
                found_registry_keys: hash_set! { ScannedRegistry::new(&reg1) },
                ..Default::default()
            },
            true,
        );
        detector.add_game(
            &ScanInfo {
                game_name: game2.clone(),
                found_files: hash_set! { file1.clone() },
                found_registry_keys: hash_set! { ScannedRegistry::new(&reg1), ScannedRegistry::new(&reg2) },
                ..Default::default()
            },
            true,
        );

        assert_eq!(Duplication::Duplicate, detector.is_file_duplicated(&file1));
        assert_eq!(
            hash_map! {
                game1.clone(): DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown },
                game2.clone(): DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown }
            },
            detector.file(&file1)
        );

        assert_eq!(Duplication::Unique, detector.is_file_duplicated(&file2));
        assert_eq!(
            hash_map! {
                game1.clone(): DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown }
            },
            detector.file(&file2)
        );

        assert_eq!(
            Duplication::Duplicate,
            detector.is_registry_duplicated(&RegistryItem::new(reg1.clone()))
        );
        assert_eq!(
            hash_map! {
                game1: DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown },
                game2.clone(): DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown }
            },
            detector.registry(&RegistryItem::new(reg1))
        );

        assert_eq!(
            Duplication::Unique,
            detector.is_registry_duplicated(&RegistryItem::new(reg2.clone()))
        );
        assert_eq!(
            hash_map! {
                game2: DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown }
            },
            detector.registry(&RegistryItem::new(reg2))
        );
    }

    #[test]
    fn can_add_games_in_restore_mode() {
        let mut detector = DuplicateDetector::default();

        let game1 = s("game1");
        let game2 = s("game2");
        let file1a = ScannedFile {
            path: StrictPath::new(s("file1a.txt")),
            size: 1,
            hash: "1".to_string(),
            original_path: Some(StrictPath::new(s("file1.txt"))),
            ignored: false,
            change: Default::default(),
            container: None,
            redirected: None,
        };
        let file1b = ScannedFile {
            path: StrictPath::new(s("file1b.txt")),
            size: 1,
            hash: "1b".to_string(),
            original_path: Some(StrictPath::new(s("file1.txt"))),
            ignored: false,
            change: Default::default(),
            container: None,
            redirected: None,
        };

        detector.add_game(
            &ScanInfo {
                game_name: game1.clone(),
                found_files: hash_set! { file1a.clone() },
                ..Default::default()
            },
            true,
        );
        detector.add_game(
            &ScanInfo {
                game_name: game2.clone(),
                found_files: hash_set! { file1b.clone() },
                ..Default::default()
            },
            true,
        );

        assert_eq!(Duplication::Duplicate, detector.is_file_duplicated(&file1a));
        assert_eq!(
            hash_map! {
                game1.clone(): DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown },
                game2.clone(): DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown }
            },
            detector.file(&file1a)
        );
        assert_eq!(
            Duplication::Unique,
            detector.is_file_duplicated(&ScannedFile {
                path: StrictPath::new(s("file1a.txt")),
                size: 1,
                hash: "1a".to_string(),
                original_path: None,
                ignored: false,
                change: Default::default(),
                container: None,
                redirected: None,
            })
        );

        assert_eq!(Duplication::Duplicate, detector.is_file_duplicated(&file1b));
        assert_eq!(
            hash_map! {
                game1: DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown },
                game2: DuplicateDetectorEntry { enabled: true, change: ScanChange::Unknown }
            },
            detector.file(&file1b)
        );
        assert_eq!(
            Duplication::Unique,
            detector.is_file_duplicated(&ScannedFile {
                path: StrictPath::new(s("file1b.txt")),
                size: 1,
                hash: "1b".to_string(),
                original_path: None,
                ignored: false,
                change: Default::default(),
                container: None,
                redirected: None,
            })
        );
    }

    #[test]
    fn removed_file_is_resolved() {
        let mut detector = DuplicateDetector::default();

        detector.add_game(
            &ScanInfo {
                game_name: "base".into(),
                found_files: hash_set! {
                    ScannedFile::with_name("unique-base"),
                    ScannedFile::with_name("file1").change_as(ScanChange::Removed),
                },
                ..Default::default()
            },
            true,
        );
        detector.add_game(
            &ScanInfo {
                game_name: "conflict".into(),
                found_files: hash_set! {
                    ScannedFile::with_name("unique-conflict"),
                    ScannedFile::with_name("file1").change_as(ScanChange::Removed),
                },
                ..Default::default()
            },
            true,
        );

        assert_eq!(Duplication::Resolved, detector.is_game_duplicated("conflict"));
        assert_eq!(
            Duplication::Resolved,
            detector.is_file_duplicated(&ScannedFile::with_name("file1"))
        );
    }

    #[test]
    fn ignored_file_is_resolved() {
        let mut detector = DuplicateDetector::default();

        detector.add_game(
            &ScanInfo {
                game_name: "base".into(),
                found_files: hash_set! {
                    ScannedFile::with_name("unique-base"),
                    ScannedFile::with_name("file1").change_as(ScanChange::Different),
                },
                ..Default::default()
            },
            true,
        );
        detector.add_game(
            &ScanInfo {
                game_name: "conflict".into(),
                found_files: hash_set! {
                    ScannedFile::with_name("unique-conflict"),
                    ScannedFile::with_name("file1").change_as(ScanChange::Different).ignored(),
                },
                ..Default::default()
            },
            true,
        );

        assert_eq!(Duplication::Resolved, detector.is_game_duplicated("conflict"));
        assert_eq!(
            Duplication::Resolved,
            detector.is_file_duplicated(&ScannedFile::with_name("file1"))
        );
    }
}
