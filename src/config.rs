use crate::{
    manifest::Store,
    prelude::{app_dir, Error, StrictPath},
};

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

fn default_backup_dir() -> StrictPath {
    let mut path = dirs::home_dir().unwrap();
    path.push("ludusavi-backup");
    StrictPath::from_std_path_buf(&path)
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub manifest: ManifestConfig,
    pub roots: Vec<RootsConfig>,
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
    #[serde(default, rename = "customGames")]
    pub custom_games: Vec<CustomGame>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestConfig {
    pub url: String,
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RootsConfig {
    pub path: StrictPath,
    pub store: Store,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RedirectConfig {
    pub source: StrictPath,
    pub target: StrictPath,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupConfig {
    pub path: StrictPath,
    #[serde(default, rename = "ignoredGames")]
    pub ignored_games: std::collections::HashSet<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RestoreConfig {
    pub path: StrictPath,
    #[serde(default, rename = "ignoredGames")]
    pub ignored_games: std::collections::HashSet<String>,
    #[serde(default)]
    pub redirects: Vec<RedirectConfig>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomGame {
    pub name: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub registry: Vec<String>,
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
            ignored_games: std::collections::HashSet::new(),
        }
    }
}

impl Default for RestoreConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
            ignored_games: std::collections::HashSet::new(),
            redirects: vec![],
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

        let mut checked = std::collections::HashSet::<StrictPath>::new();
        for (path, store) in candidates {
            let sp = StrictPath::new(path);
            if checked.contains(&sp) {
                continue;
            }
            if sp.is_dir() {
                self.roots.push(RootsConfig {
                    path: sp.clone(),
                    store,
                });
            }
            checked.insert(sp);
        }
    }

    pub fn is_game_enabled_for_backup(&self, name: &str) -> bool {
        !self.backup.ignored_games.contains(name)
    }

    pub fn enable_game_for_backup(&mut self, name: &str) {
        self.backup.ignored_games.remove(name);
    }

    pub fn disable_game_for_backup(&mut self, name: &str) {
        self.backup.ignored_games.insert(name.to_owned());
    }

    pub fn is_game_enabled_for_restore(&self, name: &str) -> bool {
        !self.restore.ignored_games.contains(name)
    }

    pub fn enable_game_for_restore(&mut self, name: &str) {
        self.restore.ignored_games.remove(name);
    }

    pub fn disable_game_for_restore(&mut self, name: &str) {
        self.restore.ignored_games.insert(name.to_owned());
    }

    pub fn add_redirect(&mut self, source: &StrictPath, target: &StrictPath) {
        let redirect = RedirectConfig {
            source: source.clone(),
            target: target.clone(),
        };
        self.restore.redirects.push(redirect);
    }

    pub fn get_redirects(&self) -> Vec<RedirectConfig> {
        self.restore.redirects.to_vec()
    }

    pub fn add_custom_game(&mut self) {
        self.custom_games.push(CustomGame {
            name: "".to_string(),
            files: vec![],
            registry: vec![],
        });
    }
}
