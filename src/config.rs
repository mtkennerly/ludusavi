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
        let candidates = vec![
            // Steam:
            ("C:/Program Files/Steam", Store::Steam),
            ("C:/Program Files (x86)/Steam", Store::Steam),
            ("~/.steam/steam", Store::Steam),
            ("~/Library/Application Support/Steam", Store::Steam),
            // Epic:
            ("C:/Program Files/Epic Games", Store::Other),
            ("C:/Program Files (x86)/Epic Games", Store::Other),
            // GOG:
            ("C:/GOG Games", Store::Other),
            ("~/GOG Games", Store::Other),
            // Uplay:
            ("C:/Program Files/Ubisoft/Ubisoft Game Launcher/games", Store::Other),
            (
                "C:/Program Files (x86)/Ubisoft/Ubisoft Game Launcher/games",
                Store::Other,
            ),
            // Origin:
            ("C:/Program Files/Origin Games", Store::Other),
            ("C:/Program Files (x86)/Origin Games", Store::Other),
            // Microsoft:
            ("C:/Program Files/WindowsApps", Store::Other),
            ("C:/Program Files (x86)/WindowsApps", Store::Other),
        ];

        for (path, store) in candidates {
            if crate::path::is_dir(&path) {
                self.roots.push(RootsConfig {
                    path: crate::path::normalize(path),
                    store,
                });
            }
        }
    }
}
