use crate::manifest::Store;
use crate::prelude::Error;

const CONFIG_FILE: &str = "config.yaml";
const MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub manifest: ManifestConfig,
    pub roots: Vec<RootsConfig>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestConfig {
    pub url: String,
    pub etag: Option<String>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RootsConfig {
    pub path: String,
    pub store: Store,
}

impl Default for ManifestConfig {
    fn default() -> Self {
        Self {
            url: MANIFEST_URL.to_string(),
            etag: None,
        }
    }
}

impl Config {
    pub fn save(&self) {
        std::fs::write(CONFIG_FILE, serde_yaml::to_string(self).unwrap().as_bytes()).unwrap();
    }

    pub fn load() -> Result<Self, Error> {
        if !std::path::Path::new(CONFIG_FILE).exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(CONFIG_FILE).unwrap();
        serde_yaml::from_str(&content).map_err(|e| Error::ConfigInvalid { why: format!("{}", e) })
    }
}
