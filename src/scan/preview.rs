use std::collections::HashMap;

use crate::{
    path::StrictPath,
    resource::config::{ToggledPaths, ToggledRegistry},
    scan::{
        layout::Backup,
        registry::{self, RegistryItem},
        BackupInfo, ScanChange, ScanChangeCount, ScanKind, ScannedFile, ScannedRegistry,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanInfo {
    pub game_name: String,
    /// The key is the actual location on disk.
    /// When `ScannedFile::container` is set, this is the path inside of the container
    /// and should be used in its raw form.
    pub found_files: HashMap<StrictPath, ScannedFile>,
    pub found_registry_keys: HashMap<RegistryItem, ScannedRegistry>,
    /// Only populated by a restoration scan.
    pub available_backups: Vec<Backup>,
    /// Only populated by a restoration scan.
    pub backup: Option<Backup>,
    /// Cheaper version of `!available_backups.is_empty()`, always populated.
    pub has_backups: bool,
    /// Full registry data, if any.
    pub dumped_registry: Option<registry::Hives>,
    /// Last known configuration.
    pub only_constructive_backups: bool,
}

impl ScanInfo {
    pub fn sum_bytes(&self, backup_info: Option<&BackupInfo>) -> u64 {
        let successful_bytes = self
            .found_files
            .values()
            .filter(|x| x.will_take_space())
            .map(|x| x.size)
            .sum::<u64>();
        let failed_bytes = if let Some(backup_info) = &backup_info {
            self.found_files
                .iter()
                .map(|(scan_key, v)| {
                    if backup_info.failed_files.contains_key(scan_key) {
                        v.size
                    } else {
                        0
                    }
                })
                .sum::<u64>()
        } else {
            0
        };
        successful_bytes.checked_sub(failed_bytes).unwrap_or_default()
    }

    pub fn total_possible_bytes(&self) -> u64 {
        self.found_files.values().map(|x| x.size).sum::<u64>()
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
        for (scan_key, v) in self.found_files.iter_mut() {
            v.ignored = toggled_paths.is_ignored(&self.game_name, v.effective(scan_key));
        }
        for (scan_key, v) in self.found_registry_keys.iter_mut() {
            v.ignored = toggled_registry.is_ignored(&self.game_name, scan_key, None);
            for (value_name, value) in &mut v.values {
                value.ignored = toggled_registry.is_ignored(&self.game_name, scan_key, Some(value_name));
            }
        }
    }

    pub fn all_ignored(&self) -> bool {
        if !self.found_anything() {
            return false;
        }
        self.found_files.values().all(|x| x.ignored)
            && self
                .found_registry_keys
                .values()
                .all(|x| x.ignored && x.values.values().all(|y| y.ignored))
    }

    pub fn any_ignored(&self) -> bool {
        self.found_files.values().any(|x| x.ignored)
            || self
                .found_registry_keys
                .values()
                .any(|x| x.ignored || x.values.values().any(|y| y.ignored))
    }

    fn all_inert(&self) -> bool {
        self.found_anything()
            && self.found_files.values().all(|x| x.change().is_inert())
            && self.found_registry_keys.values().all(|x| {
                x.change(self.scan_kind()).is_inert()
                    && x.values.values().all(|y| y.change(self.scan_kind()).is_inert())
            })
    }

    /// Total removal means that this game no longer has any saves on the system.
    fn is_total_removal(&self) -> bool {
        // We check the saves' un-normalized `change` because
        // extant ignored saves shouldn't count toward total removal.
        self.found_anything()
            && self.found_files.values().all(|x| x.change == ScanChange::Removed)
            && self
                .found_registry_keys
                .values()
                .all(|x| x.change == ScanChange::Removed && x.values.values().all(|y| y.change == ScanChange::Removed))
    }

    pub fn found_constructive(&self) -> bool {
        let relevant = |change: ScanChange, ignored: bool| match change.normalize(ignored, self.scan_kind()) {
            ScanChange::New => true,
            ScanChange::Different => true,
            ScanChange::Removed => false,
            ScanChange::Same => false,
            ScanChange::Unknown => false,
        };

        self.found_files.values().any(|x| relevant(x.change, x.ignored))
            || self
                .found_registry_keys
                .values()
                .any(|x| relevant(x.change, x.ignored) || x.values.values().any(|y| relevant(y.change, y.ignored)))
    }

    pub fn total_items(&self) -> usize {
        self.found_files.len()
            + self
                .found_registry_keys
                .values()
                .map(|x| 1 + x.values.len())
                .sum::<usize>()
    }

    pub fn enabled_items(&self) -> usize {
        self.found_files.values().filter(|x| !x.ignored).count()
            + self
                .found_registry_keys
                .values()
                .map(|x| if x.ignored { 0 } else { 1 } + x.values.values().filter(|y| !y.ignored).count())
                .sum::<usize>()
    }

    pub fn scan_kind(&self) -> ScanKind {
        if self.backup.is_some() {
            ScanKind::Restore
        } else {
            ScanKind::Backup
        }
    }

    fn is_brand_new(&self) -> bool {
        // We check the saves' un-normalized `change` because
        // ignored saves should still count toward being brand new.
        self.found_anything()
            && self.found_files.values().all(|x| x.change == ScanChange::New)
            && self
                .found_registry_keys
                .values()
                .all(|x| x.change == ScanChange::New && x.values.values().all(|y| y.change == ScanChange::New))
    }

    pub fn count_changes(&self) -> ScanChangeCount {
        let mut count = ScanChangeCount::new();
        let all_ignored = self.all_ignored();

        for entry in self.found_files.values() {
            if all_ignored {
                count.add(ScanChange::Same);
            } else {
                count.add(entry.change());
            }
        }
        for (scan_key, entry) in &self.found_registry_keys {
            if all_ignored {
                count.add(ScanChange::Same);
            } else {
                let change = entry.change(self.scan_kind());
                if change == ScanChange::Removed && self.found_registry_keys.keys().any(|x| scan_key.is_prefix_of(x)) {
                    // There's a child key, so we won't be removing this parent key,
                    // even if we do remove some of its values.
                    count.add(ScanChange::Same);
                } else {
                    count.add(change);
                }
            }

            for entry in entry.values.values() {
                if all_ignored {
                    count.add(ScanChange::Same);
                } else {
                    count.add(entry.change(self.scan_kind()));
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
            } else if self.has_backups {
                // This can happen when all of the paths are affected by a new redirect.
                ScanChange::Different
            } else {
                ScanChange::New
            }
        } else if self.all_inert() {
            ScanChange::Same
        } else {
            self.count_changes()
                .overall(self.scan_kind().is_backup() && self.only_constructive_backups)
        }
    }

    pub fn needs_cloud_sync(&self) -> bool {
        match self.overall_change() {
            ScanChange::New => true,
            ScanChange::Different => true,
            ScanChange::Removed => false,
            ScanChange::Same => false,
            ScanChange::Unknown => false,
        }
    }

    /// This is meant to be used for the GUI after a backup/restore,
    /// so we don't show the previous change state anymore.
    pub fn clear_processed_changes(&mut self, backup_info: &BackupInfo, scan_kind: ScanKind) {
        let resolve = |old: ScanChange, ignored: bool| match (scan_kind, ignored) {
            (ScanKind::Backup, true) => ScanChange::New,
            (ScanKind::Backup, false) => ScanChange::Same,
            (ScanKind::Restore, true) => old,
            (ScanKind::Restore, false) => ScanChange::Same,
        };

        self.found_files = self
            .found_files
            .clone()
            .into_iter()
            .filter(|(_, v)| v.change != ScanChange::Removed)
            .map(|(scan_key, mut v)| {
                if !backup_info.failed_files.contains_key(&scan_key) {
                    v.change = resolve(v.change, v.ignored);
                }
                (scan_key, v)
            })
            .collect();

        self.found_registry_keys = self
            .found_registry_keys
            .clone()
            .into_iter()
            .filter(|(_, v)| v.change != ScanChange::Removed)
            .map(|(scan_key, mut v)| {
                if !backup_info.failed_registry.contains_key(&scan_key) {
                    v.change = resolve(v.change, v.ignored);

                    for item in v.values.values_mut() {
                        item.change = resolve(item.change, item.ignored);
                    }
                }
                (scan_key, v)
            })
            .collect();
    }

    /// Is the backup newer than the current live data?
    pub fn is_downgraded_backup(&self, backup: chrono::DateTime<chrono::Utc>) -> bool {
        if self.overall_change() == ScanChange::Same {
            return false;
        }

        if self.backup.is_some() {
            // It's a restore.
            return false;
        }

        self.found_files.iter().all(|(scan_key, file)| {
            let Ok(live) = file.effective(scan_key).get_mtime() else {
                return true;
            };
            let live = chrono::DateTime::<chrono::Utc>::from(live);
            live < backup
        })
    }

    /// Is the backup older than the current live data?
    pub fn is_downgraded_restore(&self) -> bool {
        if self.overall_change() == ScanChange::Same {
            return false;
        }

        let Some(backup) = self.backup.as_ref().map(|x| *x.when()) else {
            return false;
        };

        self.found_files.iter().any(|(scan_key, file)| {
            let Ok(live) = file.effective(scan_key).get_mtime() else {
                return false;
            };
            let live = chrono::DateTime::<chrono::Utc>::from(live);
            live > backup
        })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, hash_map};

    use crate::{path::StrictPath, scan::ScannedRegistryValue};

    use super::*;

    #[test]
    fn game_is_brand_new() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile::default().change_as(ScanChange::New),
                "b".into(): ScannedFile::default().change_as(ScanChange::New),
            },
            ..Default::default()
        };
        assert_eq!(ScanChange::New, scan.overall_change());

        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile::default().change_as(ScanChange::New),
                "b".into(): ScannedFile::default().change_as(ScanChange::New).ignored(),
            },
            ..Default::default()
        };
        assert_eq!(ScanChange::New, scan.overall_change());
    }

    #[test]
    fn game_is_total_removal() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile::default().change_as(ScanChange::Removed),
                "b".into(): ScannedFile::default().change_as(ScanChange::Removed),
            },
            ..Default::default()
        };
        assert_eq!(ScanChange::Removed, scan.overall_change());
        assert!(scan.all_inert());

        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile::default().change_as(ScanChange::Removed),
                "b".into(): ScannedFile::default().change_as(ScanChange::Removed).ignored(),
            },
            ..Default::default()
        };
        assert_eq!(ScanChange::Removed, scan.overall_change());
        assert!(scan.all_inert());

        // Ignored non-removed files don't count toward total removal.
        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile::default().change_as(ScanChange::Removed),
                "b".into(): ScannedFile::default().change_as(ScanChange::Same).ignored(),
            },
            ..Default::default()
        };
        assert_eq!(ScanChange::Same, scan.overall_change());
        assert!(scan.all_inert());
    }

    #[test]
    fn count_changes_when_all_files_ignored() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile {
                    ignored: true,
                    change: ScanChange::Different,
                    ..Default::default()
                },
                "b".into(): ScannedFile {
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
            found_files: hash_map! {
                "removed".into(): ScannedFile::default().change_as(ScanChange::Removed),
                "same".into(): ScannedFile::default().change_as(ScanChange::Same),
            },
            ..Default::default()
        };

        assert_eq!(ScanChange::Different, scan.overall_change());
    }

    #[test]
    fn overall_change_when_game_is_different_but_inert() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "removed".into(): ScannedFile::default().change_as(ScanChange::Removed),
                "same".into(): ScannedFile::default().change_as(ScanChange::Different).ignored(),
            },
            ..Default::default()
        };

        assert_eq!(ScanChange::Same, scan.overall_change());
    }

    #[test]
    fn overall_change_when_game_is_fully_redirected() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "/new".into(): ScannedFile {
                    redirected: Some(StrictPath::new("/old")),
                    change: ScanChange::New,
                    ..Default::default()
                },
            },
            has_backups: false,
            ..Default::default()
        };

        assert_eq!(ScanChange::New, scan.overall_change());

        let scan = ScanInfo {
            found_files: hash_map! {
                "/new".into(): ScannedFile {
                    redirected: Some(StrictPath::new("/old")),
                    change: ScanChange::New,
                    ..Default::default()
                },
            },
            has_backups: true,
            ..Default::default()
        };

        assert_eq!(ScanChange::Different, scan.overall_change());
    }

    #[test]
    fn count_changes_when_all_registry_keys_ignored() {
        let scan = ScanInfo {
            found_registry_keys: hash_map! {
                "a".into(): ScannedRegistry {
                    ignored: true,
                    change: ScanChange::Different,
                    values: Default::default(),
                },
                "b".into(): ScannedRegistry {
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
            found_registry_keys: hash_map! {
                "k".into(): ScannedRegistry {
                    change: ScanChange::Same,
                    ignored: true,
                    values: btree_map! {
                        "a".to_string(): ScannedRegistryValue { ignored: true, change: ScanChange::Different },
                        "b".to_string(): ScannedRegistryValue { ignored: true, change: ScanChange::Same },
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
            found_registry_keys: hash_map! {
                "k".into(): ScannedRegistry {
                    change: ScanChange::Same,
                    ignored: true,
                    values: btree_map! {
                        "a".to_string(): ScannedRegistryValue { ignored: false, change: ScanChange::Same },
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
    fn registry_key_ignored_but_child_key_is_not() {
        let scan = ScanInfo {
            found_registry_keys: hash_map! {
                "HKEY_CURRENT_USER/foo".into(): ScannedRegistry {
                    change: ScanChange::Same,
                    ignored: true,
                    values: Default::default(),
                },
                "HKEY_CURRENT_USER/foo/bar".into(): ScannedRegistry {
                    change: ScanChange::Same,
                    ignored: false,
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
                same: 2,
            },
            scan.count_changes(),
        );
        assert_eq!(ScanChange::Same, scan.overall_change());
    }

    #[test]
    fn registry_key_ignored_but_sibling_key_is_not() {
        let scan = ScanInfo {
            found_registry_keys: hash_map! {
                "HKEY_CURRENT_USER/foo".into(): ScannedRegistry {
                    change: ScanChange::Same,
                    ignored: true,
                    values: Default::default(),
                },
                "HKEY_CURRENT_USER/bar".into(): ScannedRegistry {
                    change: ScanChange::Same,
                    ignored: false,
                    values: Default::default(),
                },
            },
            ..Default::default()
        };

        assert_eq!(
            ScanChangeCount {
                new: 0,
                different: 0,
                removed: 1,
                same: 1,
            },
            scan.count_changes(),
        );
        assert_eq!(ScanChange::Different, scan.overall_change());
    }

    #[test]
    fn no_can_report_game_when_total_removal() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile {
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
        assert_eq!(ScanChange::Removed, scan.overall_change());
        assert!(!scan.can_report_game());
    }

    #[test]
    fn can_report_game_when_inert_but_not_total_removal() {
        let scan = ScanInfo {
            found_files: hash_map! {
                "a".into(): ScannedFile {
                    ignored: false,
                    change: ScanChange::Removed,
                    ..Default::default()
                },
                "b".into(): ScannedFile {
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
        assert_eq!(ScanChange::Same, scan.overall_change());
        assert!(scan.can_report_game());
    }
}
