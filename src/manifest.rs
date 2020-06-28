use crate::config::Config;
use crate::prelude::{app_dir, Error};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Os {
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "mac")]
    Mac,
    #[serde(other)]
    Other,
}

impl Default for Os {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Store {
    #[serde(rename = "steam")]
    Steam,
    #[serde(other)]
    Other,
}

impl Default for Store {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Tag {
    #[serde(rename = "steam")]
    Steam,
    #[serde(other)]
    Other,
}

impl Default for Tag {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Manifest(pub std::collections::HashMap<String, Game>);

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub files: Option<std::collections::HashMap<String, GameFileConstraint>>,
    #[serde(rename = "installDir")]
    pub install_dir: Option<std::collections::HashMap<String, GameInstallDirConstraint>>,
    pub registry: Option<std::collections::HashMap<String, GameRegistryConstraint>>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameFileConstraint {
    pub os: Option<Os>,
    pub store: Option<Store>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameInstallDirConstraint {}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameRegistryConstraint {
    pub store: Option<Store>,
}

impl Manifest {
    fn file() -> std::path::PathBuf {
        let mut path = app_dir();
        path.push("manifest.yaml");
        path
    }

    pub fn load(config: &mut Config) -> Result<Self, Error> {
        Self::update(config)?;
        let content = std::fs::read_to_string(Self::file()).unwrap();
        serde_yaml::from_str(&content).map_err(|e| Error::ManifestInvalid { why: format!("{}", e) })
    }

    pub fn update(config: &mut Config) -> Result<(), Error> {
        let mut req = reqwest::blocking::Client::new().get(&config.manifest.url);
        if let Some(etag) = &config.manifest.etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        let mut res = req.send().map_err(|_e| Error::ManifestCannotBeUpdated)?;
        match res.status() {
            reqwest::StatusCode::OK => {
                std::fs::create_dir_all(app_dir()).map_err(|_| Error::ManifestCannotBeUpdated)?;
                let mut file = std::fs::File::create(Self::file()).map_err(|_| Error::ManifestCannotBeUpdated)?;
                res.copy_to(&mut file).map_err(|_| Error::ManifestCannotBeUpdated)?;

                if let Some(etag) = res.headers().get(reqwest::header::ETAG) {
                    match &config.manifest.etag {
                        Some(old_etag) if etag == old_etag => (),
                        _ => {
                            config.manifest.etag = Some(String::from_utf8_lossy(etag.as_bytes()).to_string());
                            config.save();
                        }
                    }
                }

                Ok(())
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(()),
            _ => Err(Error::ManifestCannotBeUpdated),
        }
    }
}
