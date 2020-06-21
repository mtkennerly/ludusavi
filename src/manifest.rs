use crate::config::Config;
use crate::prelude::Error;

const MANIFEST_FILE: &str = "manifest.yaml";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub files: Option<std::collections::HashMap<String, GameFileConstraint>>,
    #[serde(rename = "installDir")]
    pub install_dir: Option<std::collections::HashMap<String, GameInstallDirConstraint>>,
    pub registry: Option<std::collections::HashMap<String, GameRegistryConstraint>>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameFileConstraint {
    pub os: Option<Os>,
    pub store: Option<Store>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameInstallDirConstraint {}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameRegistryConstraint {
    pub store: Option<Store>,
}

impl Manifest {
    pub fn load(config: &mut Config) -> Result<Self, Error> {
        Self::update(config)?;
        let content = std::fs::read_to_string(MANIFEST_FILE).unwrap();
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
                let mut file = std::fs::File::create(MANIFEST_FILE).map_err(|_| Error::ManifestCannotBeUpdated)?;
                res.copy_to(&mut file).map_err(|_| Error::ManifestCannotBeUpdated)?;

                if let Some(etag) = res.headers().get(reqwest::header::ETAG) {
                    config.manifest.etag = Some(String::from_utf8_lossy(etag.as_bytes()).to_string());
                    config.save();
                }

                Ok(())
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(()),
            _ => Err(Error::ManifestCannotBeUpdated),
        }
    }
}
