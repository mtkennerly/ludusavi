use std::collections::BTreeMap;

use crate::{
    prelude::StrictPath,
    scan::{registry_compat::RegistryItem, ScanChange},
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedFile {
    /// The actual location on disk.
    /// When `container` is set, this is the path inside of the container
    /// and should be used in its raw form.
    pub path: StrictPath,
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
    pub fn new<T: AsRef<str> + ToString, H: ToString>(path: T, size: u64, hash: H) -> Self {
        Self {
            path: StrictPath::new(path.to_string()),
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
    pub fn with_name<T: AsRef<str> + ToString>(path: T) -> Self {
        Self {
            path: StrictPath::new(path.to_string()),
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub fn with_change<T: AsRef<str> + ToString, H: ToString>(path: T, size: u64, hash: H, change: ScanChange) -> Self {
        Self {
            path: StrictPath::new(path.to_string()),
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

    pub fn original_path(&self) -> &StrictPath {
        match &self.original_path {
            Some(x) => x,
            None => &self.path,
        }
    }

    pub fn restoring(&self) -> bool {
        self.original_path.is_some()
    }

    /// This is stored in the mapping file.
    pub fn mapping_key(&self) -> String {
        self.effective().render()
    }

    /// This is used for operations.
    pub fn effective(&self) -> &StrictPath {
        self.redirected.as_ref().unwrap_or_else(|| self.original_path())
    }

    /// This is the main path to show to the user.
    pub fn readable(&self, restoring: bool) -> String {
        if restoring {
            self.redirected
                .as_ref()
                .unwrap_or_else(|| self.original_path())
                .render()
        } else {
            self.original_path().render()
        }
    }

    /// This is shown in the GUI/CLI to annotate the `readable` path.
    pub fn alt(&self, restoring: bool) -> Option<&StrictPath> {
        if restoring {
            if self.redirected.is_some() {
                Some(self.original_path())
            } else {
                None
            }
        } else {
            self.redirected.as_ref()
        }
    }

    /// This is shown in the GUI/CLI to annotate the `readable` path.
    pub fn alt_readable(&self, restoring: bool) -> Option<String> {
        self.alt(restoring).map(|x| x.render())
    }

    pub fn will_take_space(&self) -> bool {
        !self.ignored && self.change.will_take_space()
    }

    pub fn change(&self) -> ScanChange {
        self.change.normalize(self.ignored, self.restoring())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedRegistry {
    pub path: RegistryItem,
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
    pub fn new<T: AsRef<str> + ToString>(path: T) -> Self {
        Self {
            path: RegistryItem::new(path.to_string()),
            ignored: false,
            change: ScanChange::Unknown,
            values: Default::default(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn ignored(mut self) -> Self {
        self.ignored = true;
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn change_as(mut self, change: ScanChange) -> Self {
        self.change = change;
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_value(mut self, value_name: &str, change: ScanChange, ignored: bool) -> Self {
        self.values
            .insert(value_name.to_string(), ScannedRegistryValue { change, ignored });
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    pub fn change(&self, restoring: bool) -> ScanChange {
        self.change
            .normalize(self.ignored && self.values.values().all(|x| x.ignored), restoring)
    }
}

impl ScannedRegistryValue {
    pub fn change(&self, restoring: bool) -> ScanChange {
        self.change.normalize(self.ignored, restoring)
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
                path: RegistryItem::new("key".to_string()),
                ignored: true,
                change: ScanChange::Same,
                values: Default::default(),
            }
            .change(false)
        );
        assert_eq!(
            ScanChange::Removed,
            ScannedRegistry {
                path: RegistryItem::new("key".to_string()),
                ignored: true,
                change: ScanChange::Same,
                values: btree_map! {
                    "val1".to_string(): ScannedRegistryValue { ignored: true, change: ScanChange::New },
                },
            }
            .change(false)
        );
        assert_eq!(
            ScanChange::Same,
            ScannedRegistry {
                path: RegistryItem::new("key".to_string()),
                ignored: true,
                change: ScanChange::Same,
                values: btree_map! {
                    "val1".to_string(): ScannedRegistryValue { ignored: true, change: ScanChange::New },
                    "val2".to_string(): ScannedRegistryValue { ignored: false, change: ScanChange::Same },
                },
            }
            .change(false)
        );
    }
}
