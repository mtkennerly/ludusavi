use std::collections::HashMap;
use winreg::types::{FromRegValue, ToRegValue};

use crate::{
    prelude::Error,
    resource::config::{BackupFilter, ToggledRegistry},
    scan::{
        registry::{Entries, Entry, Hives, RegistryItem, RegistryKind},
        BackupError, ScanChange, ScannedRegistry, ScannedRegistryValue, ScannedRegistryValues,
    },
};

impl From<winreg::enums::RegType> for RegistryKind {
    fn from(value: winreg::enums::RegType) -> Self {
        match value {
            winreg::enums::REG_NONE => Self::None,
            winreg::enums::REG_SZ => Self::Sz,
            winreg::enums::REG_EXPAND_SZ => Self::ExpandSz,
            winreg::enums::REG_BINARY => Self::Binary,
            winreg::enums::REG_DWORD => Self::Dword,
            winreg::enums::REG_DWORD_BIG_ENDIAN => Self::DwordBigEndian,
            winreg::enums::REG_LINK => Self::Link,
            winreg::enums::REG_MULTI_SZ => Self::MultiSz,
            winreg::enums::REG_RESOURCE_LIST => Self::ResourceList,
            winreg::enums::REG_FULL_RESOURCE_DESCRIPTOR => Self::FullResourceDescriptor,
            winreg::enums::REG_RESOURCE_REQUIREMENTS_LIST => Self::ResourceRequirementsList,
            winreg::enums::REG_QWORD => Self::Qword,
        }
    }
}

impl From<RegistryKind> for winreg::enums::RegType {
    fn from(value: RegistryKind) -> Self {
        match value {
            RegistryKind::None => Self::REG_NONE,
            RegistryKind::Sz => Self::REG_SZ,
            RegistryKind::ExpandSz => Self::REG_EXPAND_SZ,
            RegistryKind::Binary => Self::REG_BINARY,
            RegistryKind::Dword => Self::REG_DWORD,
            RegistryKind::DwordBigEndian => Self::REG_DWORD_BIG_ENDIAN,
            RegistryKind::Link => Self::REG_LINK,
            RegistryKind::MultiSz => Self::REG_MULTI_SZ,
            RegistryKind::ResourceList => Self::REG_RESOURCE_LIST,
            RegistryKind::FullResourceDescriptor => Self::REG_FULL_RESOURCE_DESCRIPTOR,
            RegistryKind::ResourceRequirementsList => Self::REG_RESOURCE_REQUIREMENTS_LIST,
            RegistryKind::Qword => Self::REG_QWORD,
        }
    }
}

impl From<winreg::RegValue> for Entry {
    fn from(item: winreg::RegValue) -> Self {
        macro_rules! map {
            ($variant:expr, $typ:ty) => {
                <$typ>::from_reg_value(&item)
                    .map($variant)
                    .unwrap_or_else(|_| Self::Raw {
                        kind: item.vtype.into(),
                        data: item.bytes,
                    })
            };
        }

        match item.vtype {
            winreg::enums::RegType::REG_SZ => map!(Self::Sz, String),
            winreg::enums::RegType::REG_EXPAND_SZ => map!(Self::ExpandSz, String),
            winreg::enums::RegType::REG_MULTI_SZ => map!(Self::MultiSz, String),
            winreg::enums::RegType::REG_DWORD => map!(Self::Dword, u32),
            winreg::enums::RegType::REG_QWORD => map!(Self::Qword, u64),
            winreg::enums::RegType::REG_BINARY => Self::Binary(item.bytes),
            _ => Self::Raw {
                kind: item.vtype.into(),
                data: item.bytes,
            },
        }
    }
}

impl From<&Entry> for Option<winreg::RegValue> {
    fn from(item: &Entry) -> Option<winreg::RegValue> {
        match item {
            Entry::Sz(x) => Some(x.to_reg_value()),
            Entry::ExpandSz(x) => Some(winreg::RegValue {
                bytes: x.to_reg_value().bytes,
                vtype: winreg::enums::RegType::REG_EXPAND_SZ,
            }),
            Entry::MultiSz(x) => Some(winreg::RegValue {
                bytes: x.to_reg_value().bytes,
                vtype: winreg::enums::RegType::REG_MULTI_SZ,
            }),
            Entry::Dword(x) => Some(x.to_reg_value()),
            Entry::Qword(x) => Some(x.to_reg_value()),
            Entry::Binary(x) => Some(winreg::RegValue {
                bytes: x.clone(),
                vtype: winreg::enums::RegType::REG_BINARY,
            }),
            Entry::Raw { kind, data } => Some(winreg::RegValue {
                bytes: data.clone(),
                vtype: (*kind).into(),
            }),
            Entry::Unknown => None,
        }
    }
}

fn get_hkey_from_name(name: &str) -> Option<winreg::HKEY> {
    match name {
        "HKEY_CURRENT_USER" => Some(winreg::enums::HKEY_CURRENT_USER),
        "HKEY_LOCAL_MACHINE" => Some(winreg::enums::HKEY_LOCAL_MACHINE),
        _ => None,
    }
}

pub fn scan_registry(
    game: &str,
    path: &str,
    filter: &BackupFilter,
    toggled: &ToggledRegistry,
    previous: Option<&Hives>,
) -> Result<HashMap<RegistryItem, ScannedRegistry>, Error> {
    let path = RegistryItem::new(path.to_string());

    let (hive_name, key) = path.split_hive().ok_or(Error::RegistryIssue)?;
    let hive = get_hkey_from_name(&hive_name).ok_or(Error::RegistryIssue)?;

    scan_registry_key(game, hive, &hive_name, &key, filter, toggled, previous)
}

fn scan_registry_key(
    game: &str,
    hive: winreg::HKEY,
    hive_name: &str,
    key: &str,
    filter: &BackupFilter,
    toggled: &ToggledRegistry,
    previous: Option<&Hives>,
) -> Result<HashMap<RegistryItem, ScannedRegistry>, Error> {
    let mut found = HashMap::new();
    let path = RegistryItem::new(format!("{hive_name}\\{key}"));

    let subkey = winreg::RegKey::predef(hive)
        .open_subkey(key)
        .map_err(|_| Error::RegistryIssue)?;

    if !filter.is_registry_ignored(&path) {
        let live_entries = read_registry_key(&subkey);
        let mut live_values = ScannedRegistryValues::new();

        for (live_entry_name, live_entry) in &live_entries.0 {
            live_values.insert(
                live_entry_name.clone(),
                ScannedRegistryValue {
                    ignored: toggled.is_ignored(game, &path, Some(live_entry_name)),
                    change: previous
                        .as_ref()
                        .and_then(|x| x.get(hive_name, key))
                        .and_then(|x| x.0.get(live_entry_name))
                        .map(|x| {
                            if x == live_entry {
                                ScanChange::Same
                            } else {
                                ScanChange::Different
                            }
                        })
                        .unwrap_or(ScanChange::New),
                },
            );
        }

        found.insert(
            path.rendered(),
            ScannedRegistry {
                ignored: toggled.is_ignored(game, &path, None),
                change: match previous {
                    None => ScanChange::New,
                    Some(previous) => match previous.get(hive_name, key) {
                        None => ScanChange::New,
                        Some(_) => ScanChange::Same,
                    },
                },
                values: live_values,
            },
        );

        for name in subkey.enum_keys().filter_map(|x| x.ok()) {
            if name.contains('/') {
                // TODO: Handle key names containing a slash.
                continue;
            }
            found.extend(
                scan_registry_key(
                    game,
                    hive,
                    hive_name,
                    &format!("{key}\\{name}"),
                    filter,
                    toggled,
                    previous,
                )
                .unwrap_or_default(),
            );
        }
    }

    Ok(found)
}

pub fn try_read_registry_key(hive_name: &str, key: &str) -> Option<Entries> {
    let hive = get_hkey_from_name(hive_name)?;
    let opened_key = winreg::RegKey::predef(hive).open_subkey(key).ok()?;
    Some(read_registry_key(&opened_key))
}

fn read_registry_key(key: &winreg::RegKey) -> Entries {
    let mut entries = Entries::default();
    for (name, value) in key.enum_values().filter_map(|x| x.ok()) {
        // TODO: The default value has an empty name. How should we represent it?
        let entry = Entry::from(value);
        if entry.is_set() {
            entries.0.insert(name, entry);
        }
    }
    entries
}

impl Hives {
    /// Since this backs up items that we already found during the scan,
    /// there shouldn't be any errors normally.
    pub fn back_up(
        &mut self,
        game: &str,
        scan: &HashMap<RegistryItem, ScannedRegistry>,
    ) -> Result<(), HashMap<RegistryItem, BackupError>> {
        let mut failed = HashMap::new();

        for (scan_key, scanned) in scan {
            if let Err(e) = self.back_up_key(game, scan_key, scanned) {
                failed.insert(scan_key.clone(), e);
            }
        }

        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed)
        }
    }

    pub fn back_up_key(
        &mut self,
        game: &str,
        scan_key: &RegistryItem,
        scan: &ScannedRegistry,
    ) -> Result<(), BackupError> {
        if scan.all_ignored() {
            return Ok(());
        }
        match scan.change {
            ScanChange::New | ScanChange::Different | ScanChange::Same => (),
            ScanChange::Removed | ScanChange::Unknown => return Ok(()),
        }

        let Some((hive_name, key)) = scan_key.split_hive() else {
            log::error!("[{game}] Unable to split hive: {:?}", scan_key);
            return Err(BackupError::Raw(format!("Unable to split hive: {}", scan_key.raw())));
        };

        let Some(hive) = get_hkey_from_name(&hive_name) else {
            log::error!("[{game}] Unable to parse hive name: {:?}", &hive_name);
            return Err(BackupError::Raw(format!("Unable to parse hive: {}", &hive_name)));
        };

        let subkey = match winreg::RegKey::predef(hive).open_subkey(&key) {
            Ok(x) => x,
            Err(e) => {
                log::error!("[{game}] Unable to open subkey: {}", &key);
                return Err(BackupError::Raw(format!("Unable to open subkey: {e:?}")));
            }
        };

        let parent = self
            .0
            .entry(hive_name.to_string())
            .or_default()
            .0
            .entry(key.to_string())
            .or_default();

        for (name, value) in subkey.enum_values().filter_map(|x| match x {
            Ok(x) => Some(x),
            Err(e) => {
                log::warn!("[{game}] Skipping invalid registry value: {e:?}");
                None
            }
        }) {
            let data = Entry::from(value);
            let ignored = scan.values.get(&name).map(|x| x.ignored).unwrap_or_default();
            if !ignored && data.is_set() {
                parent.0.insert(name.to_string(), data);
            }
        }

        Ok(())
    }

    pub fn restore(
        &self,
        game_name: &str,
        toggled: &ToggledRegistry,
    ) -> Result<(), HashMap<RegistryItem, BackupError>> {
        let mut failed = HashMap::new();

        for (hive_name, keys) in self.0.iter() {
            let hive = get_hkey_from_name(hive_name).map(winreg::RegKey::predef);
            if hive.is_none() {
                log::error!("[{}] Registry - unknown hive: {}", game_name, hive_name);
            }

            for (key_name, entries) in keys.0.iter() {
                let path = RegistryItem::from_hive_and_key(hive_name, key_name);

                let Some(hive) = hive.as_ref() else {
                    failed.insert(path.clone(), BackupError::Raw(format!("Unknown hive: {hive_name}")));
                    continue;
                };

                if toggled.is_ignored(game_name, &path, None)
                    && entries.0.keys().all(|x| toggled.is_ignored(game_name, &path, Some(x)))
                {
                    continue;
                }

                let key = match hive.create_subkey(key_name) {
                    Ok((key, _)) => key,
                    Err(e) => {
                        log::error!(
                            "[{}] Registry - failed to create subkey: {:?} | {e:?}",
                            game_name,
                            &path
                        );
                        failed.insert(path.clone(), BackupError::Raw(e.to_string()));
                        continue;
                    }
                };

                for (entry_name, entry) in entries.0.iter() {
                    if toggled.is_ignored(game_name, &path, Some(entry_name)) {
                        continue;
                    }

                    // TODO: Track errors by specific entry, rather than the parent key.
                    if let Some(value) = Option::<winreg::RegValue>::from(entry) {
                        if let Err(e) = key.set_raw_value(entry_name, &value) {
                            log::error!(
                                "[{}] Registry - failed to set value: {:?} ; {} | {e:?}",
                                game_name,
                                &path,
                                entry_name
                            );
                            failed.insert(path.clone(), BackupError::Raw(e.to_string()));
                        }
                    } else {
                        log::warn!(
                            "[{}] Registry - unparsed entry: {:?} ; {} | {:?}",
                            game_name,
                            &path,
                            entry_name,
                            entry
                        );
                        failed.insert(path.clone(), BackupError::Raw(format!("Unparsed entry: {entry:?}")));
                    }
                }
            }
        }

        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed)
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::btree_map;

    use super::*;
    use crate::{scan::registry::Keys, testing::s};

    #[test]
    fn can_store_key_from_full_path_of_leaf_key_with_values() {
        let scan_key = RegistryItem::from("HKEY_CURRENT_USER/Software/Ludusavi/game3");
        let scanned = ScannedRegistry::new().change_as(ScanChange::New);
        let mut hives = Hives::default();
        hives.back_up_key("foo", &scan_key, &scanned).unwrap();
        assert_eq!(
            Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\game3"): Entries(btree_map! {
                        s("sz"): Entry::Sz(s("foo")),
                        s("multiSz"): Entry::MultiSz(s("bar")),
                        s("expandSz"): Entry::ExpandSz(s("baz")),
                        s("dword"): Entry::Dword(1),
                        s("qword"): Entry::Qword(2),
                        s("binary"): Entry::Binary(vec![65]),
                    })
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_store_key_from_full_path_of_leaf_key_with_ignored_values() {
        let scan_key = RegistryItem::from("HKEY_CURRENT_USER/Software/Ludusavi/game3");
        let scanned = ScannedRegistry::new()
            .change_as(ScanChange::New)
            .with_value("binary", ScanChange::New, true);
        let mut hives = Hives::default();
        hives.back_up_key("foo", &scan_key, &scanned).unwrap();
        assert_eq!(
            Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\game3"): Entries(btree_map! {
                        s("sz"): Entry::Sz(s("foo")),
                        s("multiSz"): Entry::MultiSz(s("bar")),
                        s("expandSz"): Entry::ExpandSz(s("baz")),
                        s("dword"): Entry::Dword(1),
                        s("qword"): Entry::Qword(2),
                    })
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_store_key_from_full_path_of_leaf_key_with_invalid_values() {
        let scan_key = RegistryItem::from("HKEY_CURRENT_USER/Software/Ludusavi/invalid");
        let scanned = ScannedRegistry::new().change_as(ScanChange::New);
        let mut hives = Hives::default();
        hives.back_up_key("foo", &scan_key, &scanned).unwrap();
        assert_eq!(
            Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\invalid"): Entries(btree_map! {
                        s("dword"): Entry::Raw { kind: RegistryKind::Dword, data: vec![0, 0, 0, 0, 0, 0, 0, 0] },
                    })
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_store_key_from_full_path_of_leaf_key_without_values() {
        let scan_key = RegistryItem::from("HKEY_CURRENT_USER/Software/Ludusavi/other");
        let scanned = ScannedRegistry::new().change_as(ScanChange::New);
        let mut hives = Hives::default();
        hives.back_up_key("foo", &scan_key, &scanned).unwrap();
        assert_eq!(
            Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_store_key_from_full_path_of_parent_key_without_values() {
        let scan_key = RegistryItem::from("HKEY_CURRENT_USER/Software/Ludusavi");
        let scanned = ScannedRegistry::new().change_as(ScanChange::New);
        let mut hives = Hives::default();
        hives.back_up_key("foo", &scan_key, &scanned).unwrap();
        assert_eq!(
            Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi"): Entries::default(),
                })
            }),
            hives,
        );
    }
}
