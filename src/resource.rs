pub mod cache;
pub mod config;
pub mod manifest;

use crate::prelude::{app_dir, AnyError, StrictPath};

pub trait ResourceFile
where
    Self: Default + serde::de::DeserializeOwned,
{
    const FILE_NAME: &'static str;

    fn path() -> StrictPath {
        app_dir().joined(Self::FILE_NAME)
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
        Self::load_from(&Self::path())
    }

    fn load_from(path: &StrictPath) -> Result<Self, AnyError> {
        if !path.exists() {
            return Ok(Self::default().initialize());
        }
        let content = Self::load_raw(path)?;
        Self::load_from_string(&content)
    }

    fn load_from_existing(path: &StrictPath) -> Result<Self, AnyError> {
        let content = Self::load_raw(path)?;
        Self::load_from_string(&content)
    }

    fn load_raw(path: &StrictPath) -> Result<String, AnyError> {
        path.try_read()
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

        if let Ok(old_content) = Self::load_raw(&Self::path()) {
            if old_content == new_content {
                return;
            }
        }

        if Self::path().create_parent_dir().is_ok() {
            let _ = Self::path().write_with_content(&new_content);
        }
    }
}
