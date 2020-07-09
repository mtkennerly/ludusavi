use crate::manifest::Store;
use crate::prelude::{app_dir, Error};

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

fn default_backup_dir() -> String {
    let mut path = dirs::home_dir().unwrap();
    path.push("ludusavi-backup");
    path.to_string_lossy().to_string()
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub manifest: ManifestConfig,
    pub roots: Vec<RootsConfig>,
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestConfig {
    pub url: String,
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RootsConfig {
    pub path: String,
    pub store: Store,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupConfig {
    pub path: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RestoreConfig {
    pub path: String,
}

impl Default for ManifestConfig {
    fn default() -> Self {
        Self {
            url: MANIFEST_URL.to_string(),
            etag: None,
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
        }
    }
}

impl Default for RestoreConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
        }
    }
}

impl Config {
    fn file() -> std::path::PathBuf {
        let mut path = app_dir();
        path.push("config.yaml");
        path
    }

    pub fn save(&self) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Ok(old) = Self::load() {
            let old_content = serde_yaml::to_string(&old).unwrap();
            if old_content == new_content {
                return;
            }
        }

        if std::fs::create_dir_all(app_dir()).is_ok() {
            std::fs::write(Self::file(), new_content.as_bytes()).unwrap();
        }
    }

    pub fn load() -> Result<Self, Error> {
        if !std::path::Path::new(&Self::file()).exists() {
            let mut starter = Self::default();
            starter.add_common_roots();
            return Ok(starter);
        }
        let content = std::fs::read_to_string(Self::file()).unwrap();
        serde_yaml::from_str(&content).map_err(|e| Error::ConfigInvalid { why: format!("{}", e) })
    }

    pub fn add_common_roots(&mut self) {
        let mut pf32 = "C:/Program Files (x86)".to_string();
        let mut pf64 = "C:/Program Files".to_string();
        if let Ok(x) = std::env::var("ProgramFiles(x86)") {
            pf32 = x.trim_end_matches("[\\/]").to_string();
        } else if let Ok(x) = std::env::var("PROGRAMFILES") {
            pf32 = x.trim_end_matches("[\\/]").to_string();
        }
        if let Ok(x) = std::env::var("ProgramW6432") {
            pf64 = x.trim_end_matches("[\\/]").to_string();
        }

        let candidates = vec![
            // Steam:
            (format!("{}/Steam", pf32), Store::Steam),
            (format!("{}/Steam", pf64), Store::Steam),
            ("~/.steam/steam".to_string(), Store::Steam),
            ("~/Library/Application Support/Steam".to_string(), Store::Steam),
            // Epic:
            (format!("{}/Epic Games", pf32), Store::Other),
            (format!("{}/Epic Games", pf64), Store::Other),
            // GOG:
            ("C:/GOG Games".to_string(), Store::Other),
            ("~/GOG Games".to_string(), Store::Other),
            // GOG Galaxy:
            (format!("{}/GOG Galaxy/Games", pf32), Store::Other),
            (format!("{}/GOG Galaxy/Games", pf64), Store::Other),
            // Uplay:
            (format!("{}/Ubisoft/Ubisoft Game Launcher/games", pf32), Store::Other),
            (format!("{}/Ubisoft/Ubisoft Game Launcher/games", pf64), Store::Other),
            // Origin:
            (format!("{}/Origin Games", pf32), Store::Other),
            (format!("{}/Origin Games", pf64), Store::Other),
            // Microsoft:
            (format!("{}/WindowsApps", pf32), Store::Other),
            (format!("{}/WindowsApps", pf64), Store::Other),
        ];

        let mut checked = std::collections::HashSet::<String>::new();
        for (path, store) in candidates {
            if checked.contains(&path) {
                continue;
            }
            if crate::path::is_dir(&path) {
                self.roots.push(RootsConfig {
                    path: crate::path::normalize(&path),
                    store,
                });
            }
            checked.insert(path);
        }
    }
}
