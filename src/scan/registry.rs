#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

#[cfg(target_os = "windows")]
pub mod win;

use std::collections::BTreeMap;

use crate::prelude::StrictPath;

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hives(pub BTreeMap<String, Keys>);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Keys(pub BTreeMap<String, Entries>);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entries(pub BTreeMap<String, Entry>);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Entry {
    Sz(String),
    ExpandSz(String),
    MultiSz(String),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
    Raw {
        kind: RegistryKind,
        data: Vec<u8>,
    },
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RegistryKind {
    #[default]
    None,
    Sz,
    ExpandSz,
    Binary,
    Dword,
    DwordBigEndian,
    Link,
    MultiSz,
    ResourceList,
    FullResourceDescriptor,
    ResourceRequirementsList,
    Qword,
}

impl Hives {
    pub fn load(file: &StrictPath) -> Option<Self> {
        if file.is_file() {
            let content = file.read()?;
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
            let _ = file.write_with_content(&self.serialize());
        }
    }

    pub fn serialize(&self) -> String {
        serde_yaml::to_string(self).unwrap()
    }

    pub fn deserialize(content: &str) -> Option<Self> {
        serde_yaml::from_str(content).ok()
    }

    pub fn sha1(&self) -> Option<String> {
        (!self.is_empty()).then(|| crate::prelude::sha1(self.serialize()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn get(&self, hive: &str, key: &str) -> Option<&Entries> {
        self.0.get(hive)?.0.get(key)
    }

    pub fn get_path(&self, path: &RegistryItem) -> Option<&Entries> {
        let (hive, key) = path.split_hive()?;
        self.get(&hive, &key)
    }
}

impl Entry {
    fn is_set(&self) -> bool {
        *self != Self::Unknown
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, schemars::JsonSchema)]
pub struct RegistryItem {
    raw: String,
}

impl RegistryItem {
    pub fn new(raw: String) -> Self {
        Self { raw }
    }

    #[allow(unused)]
    pub fn from_hive_and_key(hive: &str, key: &str) -> Self {
        Self {
            raw: format!("{}/{}", hive, key).replace('\\', "/"),
        }
    }

    pub fn reset(&mut self, raw: String) {
        self.raw = raw;
    }

    pub fn raw(&self) -> String {
        self.raw.to_string()
    }

    pub fn render(&self) -> String {
        self.raw.replace('\\', "/")
    }

    #[allow(dead_code)]
    pub fn rendered(&self) -> Self {
        Self { raw: self.render() }
    }

    pub fn interpret(&self) -> String {
        self.raw.replace('/', "\\")
    }

    #[allow(dead_code)]
    pub fn interpreted(&self) -> Self {
        Self { raw: self.interpret() }
    }

    pub fn split(&self) -> Vec<String> {
        self.interpret().split('\\').map(|x| x.to_string()).collect()
    }

    #[allow(dead_code)]
    pub fn split_hive(&self) -> Option<(String, String)> {
        let interpreted = self.interpret();
        let parts: Vec<_> = interpreted.splitn(2, '\\').collect();
        (parts.len() == 2).then(|| (parts[0].to_string(), parts[1].to_string()))
    }

    pub fn is_prefix_of(&self, other: &Self) -> bool {
        let us_components = self.split();
        let them_components = other.split();

        if us_components.len() >= them_components.len() {
            return false;
        }
        us_components
            .iter()
            .zip(them_components.iter())
            .all(|(us, them)| us == them)
    }

    pub fn nearest_prefix(&self, others: Vec<Self>) -> Option<Self> {
        let us_components = self.split();
        let us_count = us_components.len();

        let mut nearest = None;
        let mut nearest_len = 0;
        for other in others {
            let them_components = other.split();
            let them_len = them_components.len();

            if us_count <= them_len {
                continue;
            }
            if us_components
                .iter()
                .clone()
                .zip(them_components.iter())
                .all(|(us, them)| us == them)
                && them_len > nearest_len
            {
                nearest = Some(other.clone());
                nearest_len = them_len;
            }
        }
        nearest
    }
}

// Based on:
// https://github.com/serde-rs/serde/issues/751#issuecomment-277580700
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RegistryItemSerdeHelper(String);

impl serde::Serialize for RegistryItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        RegistryItemSerdeHelper(self.raw()).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for RegistryItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer).map(|RegistryItemSerdeHelper(raw)| RegistryItem { raw })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::btree_map;

    use super::*;
    use crate::testing::s;

    #[test]
    fn hives_can_be_serialized() {
        assert_eq!(
            r#"
---
HKEY_CURRENT_USER:
  "Software\\Ludusavi": {}
  "Software\\Ludusavi\\game3":
    binary:
      binary:
        - 1
        - 2
        - 3
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
  "Software\\Ludusavi\\invalid":
    dword:
      raw:
        kind: dword
        data:
          - 0
          - 0
          - 0
          - 0
          - 0
          - 0
          - 0
          - 0
  "Software\\Ludusavi\\other": {}
"#
            .trim(),
            serde_yaml::to_string(&Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi"): Entries::default(),
                    s("Software\\Ludusavi\\game3"): Entries(btree_map! {
                        s("sz"): Entry::Sz(s("foo")),
                        s("multiSz"): Entry::MultiSz(s("bar")),
                        s("expandSz"): Entry::ExpandSz(s("baz")),
                        s("dword"): Entry::Dword(1),
                        s("qword"): Entry::Qword(2),
                        s("binary"): Entry::Binary(vec![1, 2, 3]),
                    }),
                    s("Software\\Ludusavi\\invalid"): Entries(btree_map! {
                        s("dword"): Entry::Raw {
                            kind: RegistryKind::Dword,
                            data: vec![0, 0, 0, 0, 0, 0, 0, 0],
                        },
                    }),
                    s("Software\\Ludusavi\\other"): Entries::default(),
                })
            }))
            .unwrap()
            .trim()
        )
    }

    #[test]
    fn item_is_prefix_of() {
        assert!(RegistryItem::new(s(r#"HKCU"#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo"))));
        assert!(RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo/bar"))));
        assert!(!RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/f"))));
        assert!(!RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo"))));
        assert!(!RegistryItem::new(s(r#"HKCU/foo"#)).is_prefix_of(&RegistryItem::new(s("HKCU/bar"))));
        assert!(!RegistryItem::new(s(r#""#)).is_prefix_of(&RegistryItem::new(s("HKCU/foo"))));
    }

    #[test]
    fn item_nearest_prefix() {
        assert_eq!(
            Some(RegistryItem::new(s(r#"HKCU/foo/bar"#))),
            RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)).nearest_prefix(vec![
                RegistryItem::new(s(r#"HKCU/foo"#)),
                RegistryItem::new(s(r#"HKCU/foo/bar"#)),
                RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)),
            ])
        );
        assert_eq!(
            None,
            RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)).nearest_prefix(vec![
                RegistryItem::new(s(r#"HKCU/fo"#)),
                RegistryItem::new(s(r#"HKCU/fooo"#)),
                RegistryItem::new(s(r#"HKCU/foo/bar/baz"#)),
            ])
        );
    }
}
