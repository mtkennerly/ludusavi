use std::collections::BTreeMap;

use crate::{
    prelude::StrictPath,
    scan::{ScanChange, ScanKind},
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedFile {
    pub size: u64,
    pub hash: String,
    /// This is the restoration target path, without redirects applied.
    pub original_path: Option<StrictPath>,
    pub ignored: bool,
    pub change: ScanChange,
    /// An enclosing archive file, if any, depending on the `BackupFormat`.
    pub container: Option<StrictPath>,
    pub redirected: Option<StrictPath>,
}

impl ScannedFile {
    #[cfg(test)]
    pub fn new<H: ToString>(size: u64, hash: H) -> Self {
        Self {
            size,
            hash: hash.to_string(),
            original_path: None,
            ignored: false,
            change: Default::default(),
            container: None,
            redirected: None,
        }
    }

    #[cfg(test)]
    pub fn with_change<H: ToString>(size: u64, hash: H, change: ScanChange) -> Self {
        Self {
            size,
            hash: hash.to_string(),
            original_path: None,
            ignored: false,
            change,
            container: None,
            redirected: None,
        }
    }

    #[cfg(test)]
    pub fn ignored(mut self) -> Self {
        self.ignored = true;
        self
    }

    #[cfg(test)]
    pub fn change_as(mut self, change: ScanChange) -> Self {
        self.change = change;
        self
    }

    #[cfg(test)]
    pub fn change_new(mut self) -> Self {
        self.change = ScanChange::New;
        self
    }

    pub fn original_path<'a>(&'a self, scan_key: &'a StrictPath) -> &'a StrictPath {
        match &self.original_path {
            Some(x) => x,
            None => scan_key,
        }
    }

    pub fn scan_kind(&self) -> ScanKind {
        if self.original_path.is_some() {
            ScanKind::Restore
        } else {
            ScanKind::Backup
        }
    }

    /// This is stored in the mapping file.
    pub fn mapping_key(&self, scan_key: &StrictPath) -> String {
        self.effective(scan_key).render()
    }

    /// This is used for operations.
    pub fn effective<'a>(&'a self, scan_key: &'a StrictPath) -> &'a StrictPath {
        self.redirected.as_ref().unwrap_or_else(|| self.original_path(scan_key))
    }

    /// This is the main path to show to the user.
    pub fn readable(&self, scan_key: &StrictPath, scan_kind: ScanKind) -> String {
        match scan_kind {
            ScanKind::Backup => self.original_path(scan_key).render(),
            ScanKind::Restore => self
                .redirected
                .as_ref()
                .unwrap_or_else(|| self.original_path(scan_key))
                .render(),
        }
    }

    /// This is shown in the GUI/CLI to annotate the `readable` path.
    pub fn alt<'a>(&'a self, scan_key: &'a StrictPath, scan_kind: ScanKind) -> Option<&'a StrictPath> {
        match scan_kind {
            ScanKind::Backup => self.redirected.as_ref(),
            ScanKind::Restore => {
                if self.redirected.is_some() {
                    Some(self.original_path(scan_key))
                } else {
                    None
                }
            }
        }
    }

    /// This is shown in the GUI/CLI to annotate the `readable` path.
    pub fn alt_readable(&self, scan_key: &StrictPath, scan_kind: ScanKind) -> Option<String> {
        self.alt(scan_key, scan_kind).map(|x| x.render())
    }

    pub fn will_take_space(&self) -> bool {
        !self.ignored && self.change.will_take_space()
    }

    pub fn change(&self) -> ScanChange {
        self.change.normalize(self.ignored, self.scan_kind())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedRegistry {
    pub ignored: bool,
    pub change: ScanChange,
    pub values: ScannedRegistryValues,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedRegistryValue {
    pub ignored: bool,
    pub change: ScanChange,
}

pub type ScannedRegistryValues = BTreeMap<String, ScannedRegistryValue>;

impl ScannedRegistry {
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            ignored: false,
            change: ScanChange::Unknown,
            values: Default::default(),
        }
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn ignored(mut self) -> Self {
        self.ignored = true;
        self
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn change_as(mut self, change: ScanChange) -> Self {
        self.change = change;
        self
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn with_value(mut self, value_name: &str, change: ScanChange, ignored: bool) -> Self {
        self.values
            .insert(value_name.to_string(), ScannedRegistryValue { change, ignored });
        self
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn with_value_new(mut self, value_name: &str) -> Self {
        self.values.insert(
            value_name.to_string(),
            ScannedRegistryValue {
                change: ScanChange::New,
                ignored: false,
            },
        );
        self
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn with_value_same(mut self, value_name: &str) -> Self {
        self.values.insert(
            value_name.to_string(),
            ScannedRegistryValue {
                change: ScanChange::Same,
                ignored: false,
            },
        );
        self
    }

    pub fn change(&self, scan_kind: ScanKind) -> ScanChange {
        self.change.normalize(self.all_ignored(), scan_kind)
    }

    pub fn all_ignored(&self) -> bool {
        self.ignored && self.values.values().all(|x| x.ignored)
    }
}

impl ScannedRegistryValue {
    pub fn change(&self, scan_kind: ScanKind) -> ScanChange {
        self.change.normalize(self.ignored, scan_kind)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::btree_map;

    use super::*;

    #[test]
    fn ignored_key_normalizes_to_same_if_a_value_is_not_ignored() {
        assert_eq!(
            ScanChange::Removed,
            ScannedRegistry {
                ignored: true,
                change: ScanChange::Same,
                values: Default::default(),
            }
            .change(ScanKind::Backup)
        );
        assert_eq!(
            ScanChange::Removed,
            ScannedRegistry {
                ignored: true,
                change: ScanChange::Same,
                values: btree_map! {
                    "val1".to_string(): ScannedRegistryValue { ignored: true, change: ScanChange::New },
                },
            }
            .change(ScanKind::Backup)
        );
        assert_eq!(
            ScanChange::Same,
            ScannedRegistry {
                ignored: true,
                change: ScanChange::Same,
                values: btree_map! {
                    "val1".to_string(): ScannedRegistryValue { ignored: true, change: ScanChange::New },
                    "val2".to_string(): ScannedRegistryValue { ignored: false, change: ScanChange::Same },
                },
            }
            .change(ScanKind::Backup)
        );
    }
}
