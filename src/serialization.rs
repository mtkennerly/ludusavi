use crate::prelude::{app_dir, AnyError};
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap, HashSet};

#[allow(dead_code)]
pub fn ordered_map<S, V>(value: &HashMap<String, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    V: Serialize,
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

pub fn ordered_map_maybe<S, V>(value: &Option<HashMap<String, V>>, serializer: S) -> Result<S::Ok, S::Error>
where
    V: Serialize,
    S: Serializer,
{
    match value {
        Some(value) => ordered_map(value, serializer),
        None => Option::<HashMap<String, V>>::None.serialize(serializer),
    }
}

pub fn ordered_set<S>(value: &HashSet<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut ordered: Vec<_> = value.iter().collect();
    ordered.sort();
    ordered.serialize(serializer)
}

pub fn is_false(v: &bool) -> bool {
    !v
}

pub fn is_empty_set<T>(v: &std::collections::HashSet<T>) -> bool {
    v.is_empty()
}

pub const fn default_true() -> bool {
    true
}

pub trait ResourceFile
where
    Self: Default + serde::de::DeserializeOwned,
{
    const FILE_NAME: &'static str;

    fn path() -> std::path::PathBuf {
        let mut path = app_dir();
        path.push(Self::FILE_NAME);
        path
    }

    /// If the resource file does not exist, use default data and apply these modifications.
    fn initialize(self) -> Self {
        self
    }

    /// Update any legacy settings on load.
    fn migrate(self) -> Self {
        self
    }

    fn load() -> Result<Self, AnyError> {
        if !Self::path().exists() {
            return Ok(Self::default().initialize());
        }
        let content = Self::load_raw()?;
        Self::load_from_string(&content)
    }

    fn load_raw() -> Result<String, AnyError> {
        Ok(std::fs::read_to_string(Self::path())?)
    }

    fn load_from_string(content: &str) -> Result<Self, AnyError> {
        Ok(ResourceFile::migrate(serde_yaml::from_str(content)?))
    }
}

pub trait SaveableResourceFile
where
    Self: ResourceFile + serde::Serialize,
{
    fn save(&self) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Ok(old_content) = Self::load_raw() {
            if old_content == new_content {
                return;
            }
        }

        if std::fs::create_dir_all(app_dir()).is_ok() {
            let _ = std::fs::write(Self::path(), new_content.as_bytes());
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Demo {
    name: String,
    count: u32,
    flag: bool,
}

impl ResourceFile for Demo {
    const FILE_NAME: &'static str = "demo.yaml";
}

impl SaveableResourceFile for Demo {}
