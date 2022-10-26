use std::collections::HashSet;

use crate::{
    config::{BackupFilter, ToggledRegistry},
    prelude::{Error, RegistryItem, ScanChange, ScanInfo, ScannedRegistry, StrictPath},
};
use winreg::types::{FromRegValue, ToRegValue};

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hives(
    #[serde(serialize_with = "crate::serialization::ordered_map")] pub std::collections::HashMap<String, Keys>,
);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Keys(
    #[serde(serialize_with = "crate::serialization::ordered_map")] pub std::collections::HashMap<String, Entries>,
);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entries(
    #[serde(serialize_with = "crate::serialization::ordered_map")] pub std::collections::HashMap<String, Entry>,
);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    #[serde(skip_serializing_if = "Option::is_none")]
    sz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "expandSz")]
    expand_sz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "multiSz")]
    multi_sz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dword: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    qword: Option<u64>,
}

pub fn scan_registry(
    game: &str,
    path: &str,
    filter: &BackupFilter,
    toggled: &ToggledRegistry,
    previous: &Option<Hives>,
) -> Result<Vec<ScannedRegistry>, Error> {
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
    previous: &Option<Hives>,
) -> Result<Vec<ScannedRegistry>, Error> {
    let mut found = vec![];
    let path = RegistryItem::new(format!("{}\\{}", hive_name, key));

    let subkey = winreg::RegKey::predef(hive)
        .open_subkey(key)
        .map_err(|_| Error::RegistryIssue)?;

    if !filter.is_registry_ignored(&path) {
        found.push(ScannedRegistry {
            path: path.rendered(),
            ignored: toggled.is_ignored(game, &path),
            change: match previous {
                None => ScanChange::New,
                Some(previous) => match previous.get(hive_name, key) {
                    None => ScanChange::New,
                    Some(entries) => {
                        if entries == &read_registry_key(&subkey) {
                            ScanChange::Same
                        } else {
                            ScanChange::Different
                        }
                    }
                },
            },
        });

        for name in subkey.enum_keys().filter_map(|x| x.ok()) {
            found.extend(
                scan_registry_key(
                    game,
                    hive,
                    hive_name,
                    &format!("{}\\{}", key, name),
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
        let entry = Entry::from(value);
        if entry.is_set() {
            entries.0.insert(name, entry);
        }
    }
    entries
}

impl Hives {
    pub fn load(file: &StrictPath) -> Option<Self> {
        if file.is_file() {
            let content = std::fs::read_to_string(&file.interpret()).ok()?;
            Self::deserialize(&content)
        } else {
            None
        }
    }

    pub fn save(&self, file: &StrictPath) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Some(old) = Self::load(file) {
            let old_content = serde_yaml::to_string(&old).unwrap();
            if old_content == new_content {
                return;
            }
        }

        if file.create_parent_dir().is_ok() {
            std::fs::write(file.interpret(), self.serialize().as_bytes()).unwrap();
        }
    }

    pub fn serialize(&self) -> String {
        serde_yaml::to_string(self).unwrap()
    }

    pub fn deserialize(content: &str) -> Option<Self> {
        serde_yaml::from_str(content).ok()
    }

    pub fn incorporate(&mut self, scan: &HashSet<ScannedRegistry>) -> (bool, HashSet<RegistryItem>) {
        let mut failed = HashSet::new();
        let mut found = false;

        for scanned in scan {
            if scanned.ignored {
                continue;
            }

            match self.store_key_from_full_path(&scanned.path.raw()) {
                Err(_) => {
                    failed.insert(scanned.path.clone());
                }
                Ok(_) => {
                    found = true;
                }
            }
        }

        (found, failed)
    }

    pub fn store_key_from_full_path(&mut self, path: &str) -> Result<(), Error> {
        let path = RegistryItem::new(path.to_string()).interpreted();

        let (hive_name, key) = path.split_hive().ok_or(Error::RegistryIssue)?;
        let hive = get_hkey_from_name(&hive_name).ok_or(Error::RegistryIssue)?;

        self.store_key(hive, &hive_name, &key)?;

        Ok(())
    }

    pub fn store_key(&mut self, hive: winreg::HKEY, hive_name: &str, key: &str) -> Result<(), Error> {
        let subkey = winreg::RegKey::predef(hive)
            .open_subkey(key)
            .map_err(|_| Error::RegistryIssue)?;

        self.0
            .entry(hive_name.to_string())
            .or_insert_with(Default::default)
            .0
            .entry(key.to_string())
            .or_insert_with(Default::default);
        for (name, value) in subkey.enum_values().filter_map(|x| x.ok()) {
            let entry = Entry::from(value);
            if entry.is_set() {
                self.0
                    .entry(hive_name.to_string())
                    .or_insert_with(Default::default)
                    .0
                    .entry(key.to_string())
                    .or_insert_with(Default::default)
                    .0
                    .entry(name.to_string())
                    .or_insert_with(|| entry);
            }
        }

        Ok(())
    }

    pub fn restore(&self) -> Result<(), Error> {
        let mut failed = false;

        for (hive_name, keys) in self.0.iter() {
            let hive = match get_hkey_from_name(hive_name) {
                Some(x) => winreg::RegKey::predef(x),
                None => {
                    failed = true;
                    continue;
                }
            };

            for (key_name, entries) in keys.0.iter() {
                let (key, _) = match hive.create_subkey(key_name) {
                    Ok(x) => x,
                    Err(_) => {
                        failed = true;
                        continue;
                    }
                };

                for (entry_name, entry) in entries.0.iter() {
                    if let Some(value) = Option::<winreg::RegValue>::from(entry) {
                        if key.set_raw_value(entry_name, &value).is_err() {
                            failed = true;
                        }
                    } else {
                        failed = true;
                    }
                }
            }
        }

        if failed {
            return Err(Error::RegistryIssue);
        }

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, hive: &str, key: &str) -> Option<&Entries> {
        self.0.get(hive)?.0.get(key)
    }
}

impl From<&HashSet<ScannedRegistry>> for Hives {
    fn from(source: &HashSet<ScannedRegistry>) -> Self {
        let mut hives = Self::default();
        hives.incorporate(source);
        hives
    }
}

impl From<&ScanInfo> for Hives {
    fn from(scan: &ScanInfo) -> Self {
        let mut hives = Self::default();
        hives.incorporate(&scan.found_registry_keys);
        hives
    }
}

impl Entry {
    pub fn is_set(&self) -> bool {
        self.sz.is_some()
            || self.expand_sz.is_some()
            || self.multi_sz.is_some()
            || self.dword.is_some()
            || self.qword.is_some()
    }
}

impl From<winreg::RegValue> for Entry {
    fn from(item: winreg::RegValue) -> Self {
        match item.vtype {
            winreg::enums::RegType::REG_SZ => Self {
                sz: Some(String::from_reg_value(&item).unwrap_or_default()),
                ..Default::default()
            },
            winreg::enums::RegType::REG_EXPAND_SZ => Self {
                expand_sz: Some(String::from_reg_value(&item).unwrap_or_default()),
                ..Default::default()
            },
            winreg::enums::RegType::REG_MULTI_SZ => Self {
                multi_sz: Some(String::from_reg_value(&item).unwrap_or_default()),
                ..Default::default()
            },
            winreg::enums::RegType::REG_DWORD => Self {
                dword: Some(u32::from_reg_value(&item).unwrap_or_default()),
                ..Default::default()
            },
            winreg::enums::RegType::REG_QWORD => Self {
                qword: Some(u64::from_reg_value(&item).unwrap_or_default()),
                ..Default::default()
            },
            _ => Default::default(),
        }
    }
}

impl From<&Entry> for Option<winreg::RegValue> {
    fn from(item: &Entry) -> Option<winreg::RegValue> {
        if let Some(x) = &item.sz {
            Some(x.to_reg_value())
        } else if let Some(x) = &item.multi_sz {
            Some(winreg::RegValue {
                bytes: x.to_reg_value().bytes,
                vtype: winreg::enums::RegType::REG_MULTI_SZ,
            })
        } else if let Some(x) = &item.expand_sz {
            Some(winreg::RegValue {
                bytes: x.to_reg_value().bytes,
                vtype: winreg::enums::RegType::REG_EXPAND_SZ,
            })
        } else if let Some(x) = &item.dword {
            Some(x.to_reg_value())
        } else {
            item.qword.as_ref().map(|x| x.to_reg_value())
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

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;
    use pretty_assertions::assert_eq;

    fn s(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn can_store_key_from_full_path_of_leaf_key_with_values() {
        let mut hives = Hives::default();
        hives
            .store_key_from_full_path("HKEY_CURRENT_USER/Software/Ludusavi/game3")
            .unwrap();
        assert_eq!(
            Hives(hashmap! {
                s("HKEY_CURRENT_USER") => Keys(hashmap! {
                    s("Software\\Ludusavi\\game3") => Entries(hashmap! {
                        s("sz") => Entry {
                            sz: Some(s("foo")),
                            ..Default::default()
                        },
                        s("multiSz") => Entry {
                            multi_sz: Some(s("bar")),
                            ..Default::default()
                        },
                        s("expandSz") => Entry {
                            expand_sz: Some(s("baz")),
                            ..Default::default()
                        },
                        s("dword") => Entry {
                            dword: Some(1),
                            ..Default::default()
                        },
                        s("qword") => Entry {
                            qword: Some(2),
                            ..Default::default()
                        },
                    })
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_store_key_from_full_path_of_leaf_key_without_values() {
        let mut hives = Hives::default();
        hives
            .store_key_from_full_path("HKEY_CURRENT_USER/Software/Ludusavi/other")
            .unwrap();
        assert_eq!(
            Hives(hashmap! {
                s("HKEY_CURRENT_USER") => Keys(hashmap! {
                    s("Software\\Ludusavi\\other") => Entries::default()
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_store_key_from_full_path_of_parent_key_without_values() {
        let mut hives = Hives::default();
        hives
            .store_key_from_full_path("HKEY_CURRENT_USER/Software/Ludusavi")
            .unwrap();
        assert_eq!(
            Hives(hashmap! {
                s("HKEY_CURRENT_USER") => Keys(hashmap! {
                    s("Software\\Ludusavi") => Entries::default(),
                })
            }),
            hives,
        );
    }

    #[test]
    fn can_be_serialized() {
        assert_eq!(
            r#"
---
HKEY_CURRENT_USER:
  "Software\\Ludusavi": {}
  "Software\\Ludusavi\\game3":
    dword:
      dword: 1
    expandSz:
      expandSz: baz
    multiSz:
      multiSz: bar
    qword:
      qword: 2
    sz:
      sz: foo
  "Software\\Ludusavi\\other": {}
"#
            .trim(),
            serde_yaml::to_string(&Hives(hashmap! {
                s("HKEY_CURRENT_USER") => Keys(hashmap! {
                    s("Software\\Ludusavi") => Entries::default(),
                    s("Software\\Ludusavi\\game3") => Entries(hashmap! {
                        s("sz") => Entry {
                            sz: Some(s("foo")),
                            ..Default::default()
                        },
                        s("multiSz") => Entry {
                            multi_sz: Some(s("bar")),
                            ..Default::default()
                        },
                        s("expandSz") => Entry {
                            expand_sz: Some(s("baz")),
                            ..Default::default()
                        },
                        s("dword") => Entry {
                            dword: Some(1),
                            ..Default::default()
                        },
                        s("qword") => Entry {
                            qword: Some(2),
                            ..Default::default()
                        },
                    }),
                    s("Software\\Ludusavi\\other") => Entries::default(),
                })
            }))
            .unwrap()
            .trim()
        )
    }
}
