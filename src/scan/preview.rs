use std::collections::HashSet;

use crate::{
    resource::config::{ToggledPaths, ToggledRegistry},
    scan::{layout::Backup, BackupInfo, ScanChange, ScanChangeCount, ScannedFile, ScannedRegistry},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanInfo {
    pub game_name: String,
    pub found_files: HashSet<ScannedFile>,
    pub found_registry_keys: HashSet<ScannedRegistry>,
    /// Only populated by a restoration scan.
    pub available_backups: Vec<Backup>,
    /// Only populated by a restoration scan.
    pub backup: Option<Backup>,
}

impl ScanInfo {
    pub fn sum_bytes(&self, backup_info: Option<&BackupInfo>) -> u64 {
        let successful_bytes = self
            .found_files
            .iter()
            .filter(|x| x.will_take_space())
            .map(|x| x.size)
            .sum::<u64>();
        let failed_bytes = if let Some(backup_info) = &backup_info {
            backup_info.failed_files.iter().map(|x| x.size).sum::<u64>()
        } else {
            0
        };
        successful_bytes - failed_bytes
    }

    pub fn total_possible_bytes(&self) -> u64 {
        self.found_files.iter().map(|x| x.size).sum::<u64>()
    }

    pub fn can_report_game(&self) -> bool {
        self.found_anything()
            && match self.overall_change() {
                ScanChange::New => true,
                ScanChange::Different => true,
                ScanChange::Removed => false,
                ScanChange::Same => true,
                ScanChange::Unknown => true,
            }
    }

    pub fn found_anything(&self) -> bool {
        !self.found_files.is_empty() || !self.found_registry_keys.is_empty()
    }

    pub fn found_anything_processable(&self) -> bool {
        match self.overall_change() {
            ScanChange::New => true,
            ScanChange::Different => true,
            ScanChange::Removed => false,
            ScanChange::Same => false,
            ScanChange::Unknown => false,
        }
    }

    pub fn update_ignored(&mut self, toggled_paths: &ToggledPaths, toggled_registry: &ToggledRegistry) {
        self.found_files = self
            .found_files
            .iter()
            .map(|x| {
                let mut y = x.clone();
                y.ignored = toggled_paths.is_ignored(&self.game_name, &x.path);
                y
            })
            .collect();
        self.found_registry_keys = self
            .found_registry_keys
            .iter()
            .map(|x| {
                let mut y = x.clone();
                y.ignored = toggled_registry.is_ignored(&self.game_name, &x.path, None);
                for (value_name, value) in &mut y.values {
                    value.ignored = toggled_registry.is_ignored(&self.game_name, &x.path, Some(value_name));
                }
                y
            })
            .collect();
    }

    pub fn all_ignored(&self) -> bool {
        if !self.found_anything() {
            return false;
        }
        self.found_files.iter().all(|x| x.ignored)
            && self
                .found_registry_keys
                .iter()
                .all(|x| x.ignored && x.values.values().all(|y| y.ignored))
    }

    pub fn any_ignored(&self) -> bool {
        self.found_files.iter().any(|x| x.ignored)
            || self
                .found_registry_keys
                .iter()
                .any(|x| x.ignored || x.values.values().any(|y| y.ignored))
    }

    fn all_inert(&self) -> bool {
        if !self.found_anything() {
            return false;
        }

        self.found_files.iter().all(|x| x.change.is_inert(!x.ignored))
            && self
                .found_registry_keys
                .iter()
                .all(|x| x.change.is_inert(!x.ignored) && x.values.values().all(|y| y.change.is_inert(!y.ignored)))
    }

    fn is_total_removal(&self) -> bool {
        self.found_anything()
            && self.found_files.iter().all(|x| x.change == ScanChange::Removed)
            && self
                .found_registry_keys
                .iter()
                .all(|x| x.change == ScanChange::Removed && x.values.values().all(|y| y.change == ScanChange::Removed))
    }

    pub fn total_items(&self) -> usize {
        self.found_files.len()
            + self
                .found_registry_keys
                .iter()
                .map(|x| 1 + x.values.len())
                .sum::<usize>()
    }

    pub fn enabled_items(&self) -> usize {
        self.found_files.iter().filter(|x| !x.ignored).count()
            + self
                .found_registry_keys
                .iter()
                .map(|x| if x.ignored { 0 } else { 1 } + x.values.values().filter(|y| !y.ignored).count())
                .sum::<usize>()
    }

    pub fn restoring(&self) -> bool {
        self.backup.is_some()
    }

    fn is_brand_new(&self) -> bool {
        self.found_anything()
            && self.found_files.iter().all(|x| x.change == ScanChange::New)
            && self
                .found_registry_keys
                .iter()
                .all(|x| x.change == ScanChange::New && x.values.values().all(|y| y.change == ScanChange::New))
    }

    pub fn count_changes(&self) -> ScanChangeCount {
        let mut count = ScanChangeCount::new();
        let all_ignored = self.all_ignored();

        for entry in &self.found_files {
            if all_ignored {
                count.add(ScanChange::Same);
            } else {
                count.add(entry.change());
            }
        }
        for entry in &self.found_registry_keys {
            if all_ignored {
                count.add(ScanChange::Same);
            } else {
                count.add(entry.change(self.restoring()));
            }

            for entry in entry.values.values() {
                if all_ignored {
                    count.add(ScanChange::Same);
                } else {
                    count.add(entry.change(self.restoring()));
                }
            }
        }

        count
    }

    pub fn overall_change(&self) -> ScanChange {
        if self.is_total_removal() {
            ScanChange::Removed
        } else if self.is_brand_new() {
            if self.all_ignored() {
                ScanChange::Same
            } else {
                ScanChange::New
            }
        } else if self.all_inert() {
            ScanChange::Same
        } else {
            self.count_changes().overall()
        }
    }
}

#[cfg(test)]
mod tests {
    use maplit::*;
    use pretty_assertions::assert_eq;

    use crate::{
        path::StrictPath,
        scan::{registry_compat::RegistryItem, ScannedRegistryValue},
    };

    use super::*;

    #[test]
    fn count_changes_when_all_files_ignored() {
        let scan = ScanInfo {
            found_files: hashset! {
                ScannedFile {
                    path: StrictPath::new("a".into()),
                    ignored: true,
                    change: ScanChange::Different,
                    ..Default::default()
                },
                ScannedFile {
                    path: StrictPath::new("b".into()),
                    ignored: true,
                    change: ScanChange::Same,
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 0,
                same: 2
            },
            scan.count_changes(),
        );
    }

    #[test]
    fn overall_change_when_game_is_different_with_removed_file() {
        let scan = ScanInfo {
            found_files: hashset! {
                ScannedFile::with_name("removed").change_as(ScanChange::Removed),
                ScannedFile::with_name("same").change_as(ScanChange::Same),
            },
            ..Default::default()
        };

        assert_eq!(ScanChange::Different, scan.overall_change());
    }

    #[test]
    fn overall_change_when_game_is_different_but_inert() {
        let scan = ScanInfo {
            found_files: hashset! {
                ScannedFile::with_name("removed").change_as(ScanChange::Removed),
                ScannedFile::with_name("same").change_as(ScanChange::Different).ignored(),
            },
            ..Default::default()
        };

        assert_eq!(ScanChange::Same, scan.overall_change());
    }

    #[test]
    fn count_changes_when_all_registry_keys_ignored() {
        let scan = ScanInfo {
            found_registry_keys: hashset! {
                ScannedRegistry {
                    path: RegistryItem::new("a".into()),
                    ignored: true,
                    change: ScanChange::Different,
                    values: Default::default(),
                },
                ScannedRegistry {
                    path: RegistryItem::new("b".into()),
                    ignored: true,
                    change: ScanChange::Same,
                    values: Default::default(),
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 0,
                same: 2
            },
            scan.count_changes(),
        );
    }

    #[test]
    fn count_changes_when_all_registry_values_ignored() {
        let scan = ScanInfo {
            found_registry_keys: hashset! {
                ScannedRegistry {
                    path: RegistryItem::new("k".into()),
                    change: ScanChange::Same,
                    ignored: true,
                    values: btreemap! {
                        "a".to_string() => ScannedRegistryValue { ignored: true, change: ScanChange::Different },
                        "b".to_string() => ScannedRegistryValue { ignored: true, change: ScanChange::Same },
                    },
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 0,
                same: 3
            },
            scan.count_changes(),
        );
    }

    #[test]
    fn count_changes_when_registry_key_ignored_but_value_is_not() {
        let scan = ScanInfo {
            found_registry_keys: hashset! {
                ScannedRegistry {
                    path: RegistryItem::new("k".into()),
                    change: ScanChange::Same,
                    ignored: true,
                    values: btreemap! {
                        "a".to_string() => ScannedRegistryValue { ignored: false, change: ScanChange::Same },
                    },
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 0,
                same: 2,
            },
            scan.count_changes(),
        );
    }

    #[test]
    fn no_can_report_game_when_total_removal() {
        let scan = ScanInfo {
            found_files: hashset! {
                ScannedFile {
                    path: StrictPath::new("a".into()),
                    ignored: false,
                    change: ScanChange::Removed,
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 1,
                same: 0,
            },
            scan.count_changes(),
        );
        assert!(!scan.can_report_game());
    }

    #[test]
    fn can_report_game_when_inert_but_not_total_removal() {
        let scan = ScanInfo {
            found_files: hashset! {
                ScannedFile {
                    path: StrictPath::new("a".into()),
                    ignored: false,
                    change: ScanChange::Removed,
                    ..Default::default()
                },
                ScannedFile {
                    path: StrictPath::new("b".into()),
                    ignored: true,
                    change: ScanChange::Same,
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 2,
                same: 0,
            },
            scan.count_changes(),
        );
        assert!(scan.can_report_game());
    }
}
