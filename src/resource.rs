pub mod cache;
pub mod config;
pub mod manifest;

use crate::prelude::{app_dir, AnyError};

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
        Self::load_from(&Self::path())
    }

    fn load_from(path: &std::path::PathBuf) -> Result<Self, AnyError> {
        if !path.exists() {
            return Ok(Self::default().initialize());
        }
        let content = Self::load_raw(path)?;
        Self::load_from_string(&content)
    }

    fn load_from_existing(path: &std::path::PathBuf) -> Result<Self, AnyError> {
        let content = Self::load_raw(path)?;
        Self::load_from_string(&content)
    }

    fn load_raw(path: &std::path::PathBuf) -> Result<String, AnyError> {
        Ok(std::fs::read_to_string(path)?)
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

        if std::fs::create_dir_all(app_dir()).is_ok() {
            let _ = std::fs::write(Self::path(), new_content.as_bytes());
        }
    }
}
