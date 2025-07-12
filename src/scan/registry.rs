#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

#[cfg(target_os = "windows")]
pub mod win;

use std::collections::BTreeMap;

use crate::prelude::StrictPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Reg,
    Yaml,
}

impl Format {
    pub const ALL: &[Self] = &[Self::Reg, Self::Yaml];

    pub fn filename(&self) -> &'static str {
        match self {
            Format::Reg => "registry.reg",
            Format::Yaml => "registry.yaml",
        }
    }
}

impl From<&StrictPath> for Format {
    fn from(path: &StrictPath) -> Self {
        if path.interpret().is_ok_and(|x| x.ends_with(".yaml")) {
            Self::Yaml
        } else {
            Self::Reg
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hives(pub BTreeMap<String, Keys>);

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Keys(pub BTreeMap<String, Entries>);

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entries(pub BTreeMap<String, Entry>);

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        let format = Format::from(file);
        Self::deserialize(&Self::load_raw(file)?, format)
    }

    fn load_raw(file: &StrictPath) -> Option<String> {
        if file.is_file() {
            file.read()
        } else {
            None
        }
    }

    pub fn save(&self, file: &StrictPath) {
        let new_content = self.serialize(Format::Reg);

        if let Some(old_content) = Self::load_raw(file) {
            if old_content == new_content {
                return;
            }
        }

        if file.create_parent_dir().is_ok() {
            let format = Format::from(file);
            let _ = file.write_with_content(&self.serialize(format));
        }
    }

    pub fn serialize(&self, format: Format) -> String {
        match format {
            Format::Reg => {
                let registry = regashii::Registry::from(self.clone());
                registry.serialize()
            }
            Format::Yaml => serde_yaml::to_string(self).unwrap(),
        }
    }

    pub fn deserialize(content: &str, format: Format) -> Option<Self> {
        match format {
            Format::Reg => regashii::Registry::deserialize(content).ok().map(Self::from),
            Format::Yaml => serde_yaml::from_str(content).ok(),
        }
    }

    pub fn sha1(&self, format: Format) -> Option<String> {
        (!self.is_empty()).then(|| crate::prelude::sha1(self.serialize(format)))
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

impl From<Hives> for regashii::Registry {
    fn from(hives: Hives) -> Self {
        use regashii::{Format, Key, Registry, Value};

        let mut registry = Registry::new(Format::Regedit5);

        for (hive_name, keys) in hives.0 {
            for (key_name, entries) in keys.0 {
                let mut key = Key::new();
                for (entry_name, entry) in entries.0 {
                    let Ok(value) = Value::try_from(entry) else { continue };
                    key.insert(entry_name.into(), value);
                }
                registry.insert(format!("{hive_name}\\{key_name}").into(), key);
            }
        }

        registry
    }
}

impl From<regashii::Registry> for Hives {
    fn from(registry: regashii::Registry) -> Self {
        use regashii::ValueName;

        let mut hives = Hives::default();

        for (key_name, key) in registry.keys().clone() {
            let mut parts = key_name.raw().splitn(2, '\\');
            let Some(hive_name) = parts.next() else { continue };
            let Some(key_name) = parts.next() else { continue };

            match key.kind() {
                regashii::KeyKind::Delete => continue,
                regashii::KeyKind::Add | regashii::KeyKind::Replace => {
                    let key_entry = hives
                        .0
                        .entry(hive_name.to_string())
                        .or_default()
                        .0
                        .entry(key_name.to_string())
                        .or_default();

                    for (value_name, value) in key.values().clone() {
                        let value_name = match value_name {
                            ValueName::Default => "".to_string(),
                            ValueName::Named(x) => x,
                        };
                        let Ok(entry) = Entry::try_from(value) else { continue };
                        key_entry.0.insert(value_name, entry);
                    }
                }
            }
        }

        hives
    }
}

impl TryFrom<Entry> for regashii::Value {
    type Error = ();

    fn try_from(value: Entry) -> Result<Self, Self::Error> {
        use regashii::Value;

        match value {
            Entry::Sz(x) => Ok(Value::Sz(x)),
            Entry::ExpandSz(x) => Ok(Value::ExpandSz(x)),
            Entry::MultiSz(x) => Ok(Value::MultiSz(x.split('\n').map(|x| x.to_string()).collect())),
            Entry::Dword(x) => Ok(Value::Dword(x)),
            Entry::Qword(x) => Ok(Value::Qword(x)),
            Entry::Binary(x) => Ok(Value::Binary(x)),
            Entry::Raw { kind, data } => Ok(Value::Hex {
                kind: kind.into(),
                bytes: data,
            }),
            Entry::Unknown => Err(()),
        }
    }
}

impl TryFrom<regashii::Value> for Entry {
    type Error = ();

    fn try_from(value: regashii::Value) -> Result<Self, Self::Error> {
        use regashii::Value;

        match value {
            Value::Delete => Err(()),
            Value::Sz(x) => Ok(Entry::Sz(x)),
            Value::ExpandSz(x) => Ok(Entry::ExpandSz(x)),
            Value::Binary(x) => Ok(Entry::Binary(x)),
            Value::Dword(x) => Ok(Entry::Dword(x)),
            Value::DwordBigEndian(x) => Ok(Entry::Dword(x)),
            Value::MultiSz(x) => Ok(Entry::MultiSz(x.join("\n"))),
            Value::Qword(x) => Ok(Entry::Qword(x)),
            Value::Hex { kind, bytes } => Ok(Entry::Raw {
                kind: kind.try_into()?,
                data: bytes,
            }),
        }
    }
}

impl From<RegistryKind> for regashii::Kind {
    fn from(kind: RegistryKind) -> Self {
        use regashii::Kind;

        match kind {
            RegistryKind::None => Kind::None,
            RegistryKind::Sz => Kind::Sz,
            RegistryKind::ExpandSz => Kind::ExpandSz,
            RegistryKind::Binary => Kind::Binary,
            RegistryKind::Dword => Kind::Dword,
            RegistryKind::DwordBigEndian => Kind::DwordBigEndian,
            RegistryKind::Link => Kind::Link,
            RegistryKind::MultiSz => Kind::MultiSz,
            RegistryKind::ResourceList => Kind::ResourceList,
            RegistryKind::FullResourceDescriptor => Kind::FullResourceList,
            RegistryKind::ResourceRequirementsList => Kind::ResourceRequirementsList,
            RegistryKind::Qword => Kind::Qword,
        }
    }
}

impl TryFrom<regashii::Kind> for RegistryKind {
    type Error = ();

    fn try_from(kind: regashii::Kind) -> Result<Self, Self::Error> {
        use regashii::Kind;

        match kind {
            Kind::None => Ok(RegistryKind::None),
            Kind::Sz => Ok(RegistryKind::Sz),
            Kind::ExpandSz => Ok(RegistryKind::ExpandSz),
            Kind::Binary => Ok(RegistryKind::Binary),
            Kind::Dword => Ok(RegistryKind::Dword),
            Kind::DwordBigEndian => Ok(RegistryKind::DwordBigEndian),
            Kind::Link => Ok(RegistryKind::Link),
            Kind::MultiSz => Ok(RegistryKind::MultiSz),
            Kind::ResourceList => Ok(RegistryKind::ResourceList),
            Kind::FullResourceList => Ok(RegistryKind::FullResourceDescriptor),
            Kind::ResourceRequirementsList => Ok(RegistryKind::ResourceRequirementsList),
            Kind::Qword => Ok(RegistryKind::Qword),
            Kind::Unknown(_) => Err(()),
        }
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

    pub fn from_hive_and_key(hive: &str, key: &str) -> Self {
        Self {
            raw: format!("{hive}/{key}").replace('\\', "/"),
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

    pub fn rendered(&self) -> Self {
        Self { raw: self.render() }
    }

    pub fn interpret(&self) -> String {
        self.raw.replace('/', "\\")
    }

    pub fn interpreted(&self) -> Self {
        Self { raw: self.interpret() }
    }

    pub fn split(&self) -> Vec<String> {
        self.interpret().split('\\').map(|x| x.to_string()).collect()
    }

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

impl From<String> for RegistryItem {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for RegistryItem {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
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
        let hives = Hives(btree_map! {
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
        });

        assert_eq!(
            r#"
Windows Registry Editor Version 5.00

[HKEY_CURRENT_USER\Software\Ludusavi]

[HKEY_CURRENT_USER\Software\Ludusavi\game3]
"binary"=hex:01,02,03
"dword"=dword:00000001
"expandSz"=hex(2):62,00,61,00,7a,00,00,00
"multiSz"=hex(7):62,00,61,00,72,00,00,00,00,00
"qword"=hex(b):02,00,00,00,00,00,00,00
"sz"="foo"

[HKEY_CURRENT_USER\Software\Ludusavi\invalid]
"dword"=hex(4):00,00,00,00,00,00,00,00

[HKEY_CURRENT_USER\Software\Ludusavi\other]
"#
            .trim(),
            hives.serialize(Format::Reg).trim()
        );

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
            hives.serialize(Format::Yaml).trim()
        );
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
