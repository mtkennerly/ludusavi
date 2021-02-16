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

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub manifest: ManifestConfig,
    pub roots: Vec<RootsConfig>,
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
    #[serde(default, rename = "customGames")]
    pub custom_games: Vec<CustomGame>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ManifestConfig {
    pub url: String,
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RootsConfig {
    pub path: StrictPath,
    pub store: Store,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RedirectConfig {
    pub source: StrictPath,
    pub target: StrictPath,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackupFilter {
    #[serde(
        default,
        skip_serializing_if = "crate::serialization::is_false",
        rename = "excludeOtherOsData"
    )]
    pub exclude_other_os_data: bool,
    #[serde(
        default,
        skip_serializing_if = "crate::serialization::is_false",
        rename = "excludeStoreScreenshots"
    )]
    pub exclude_store_screenshots: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackupConfig {
    pub path: StrictPath,
    #[serde(
        default,
        rename = "ignoredGames",
        serialize_with = "crate::serialization::ordered_set"
    )]
    pub ignored_games: std::collections::HashSet<String>,
    #[serde(default)]
    pub merge: bool,
    #[serde(default)]
    pub filter: BackupFilter,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RestoreConfig {
    pub path: StrictPath,
    #[serde(
        default,
        rename = "ignoredGames",
        serialize_with = "crate::serialization::ordered_set"
    )]
    pub ignored_games: std::collections::HashSet<String>,
    #[serde(default)]
    pub redirects: Vec<RedirectConfig>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
            merge: false,
            filter: BackupFilter::default(),
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
        Self::load_from_string(&content)
    }

    pub fn load_from_string(content: &str) -> Result<Self, Error> {
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

    pub fn is_game_customized(&self, name: &str) -> bool {
        self.custom_games.iter().any(|x| x.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashset;
    use pretty_assertions::assert_eq;

    fn s(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn can_parse_minimal_config() {
        let config = Config::load_from_string(
            r#"
            manifest:
              url: example.com
              etag: null
            roots: []
            backup:
              path: ~/backup
            restore:
              path: ~/restore
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                manifest: ManifestConfig {
                    url: s("example.com"),
                    etag: None,
                },
                roots: vec![],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: std::collections::HashSet::new(),
                    merge: false,
                    filter: BackupFilter {
                        exclude_other_os_data: false,
                        exclude_store_screenshots: false,
                    },
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: std::collections::HashSet::new(),
                    redirects: vec![],
                },
                custom_games: vec![],
            },
            config,
        );
    }

    #[test]
    fn can_parse_optional_fields_when_present_in_config() {
        let config = Config::load_from_string(
            r#"
            manifest:
              url: example.com
              etag: "foo"
            roots:
              - path: ~/steam
                store: steam
              - path: ~/other
                store: other
            backup:
              path: ~/backup
              ignoredGames:
                - Backup Game 1
                - Backup Game 2
                - Backup Game 2
              merge: true
              filter:
                excludeOtherOsData: true
                excludeStoreScreenshots: true
            restore:
              path: ~/restore
              ignoredGames:
                - Restore Game 1
                - Restore Game 2
                - Restore Game 2
              redirects:
                - source: ~/old
                  target: ~/new
            customGames:
              - name: Custom Game 1
              - name: Custom Game 2
                files:
                  - Custom File 1
                  - Custom File 2
                  - Custom File 2
                registry:
                  - Custom Registry 1
                  - Custom Registry 2
                  - Custom Registry 2
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                manifest: ManifestConfig {
                    url: s("example.com"),
                    etag: Some(s("foo")),
                },
                roots: vec![
                    RootsConfig {
                        path: StrictPath::new(s("~/steam")),
                        store: Store::Steam,
                    },
                    RootsConfig {
                        path: StrictPath::new(s("~/other")),
                        store: Store::Other,
                    },
                ],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: hashset! {
                        s("Backup Game 1"),
                        s("Backup Game 2"),
                    },
                    merge: true,
                    filter: BackupFilter {
                        exclude_other_os_data: true,
                        exclude_store_screenshots: true,
                    },
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: hashset! {
                        s("Restore Game 1"),
                        s("Restore Game 2"),
                    },
                    redirects: vec![RedirectConfig {
                        source: StrictPath::new(s("~/old")),
                        target: StrictPath::new(s("~/new")),
                    },],
                },
                custom_games: vec![
                    CustomGame {
                        name: s("Custom Game 1"),
                        files: vec![],
                        registry: vec![],
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        files: vec![s("Custom File 1"), s("Custom File 2"), s("Custom File 2"),],
                        registry: vec![s("Custom Registry 1"), s("Custom Registry 2"), s("Custom Registry 2"),],
                    },
                ],
            },
            config,
        );
    }

    /// There was a defect previously where `Store::Other` would be serialized
    /// as `store: Other` (capitalized). This test ensures that old config files
    /// with that issue will still be accepted.
    #[test]
    fn can_parse_legacy_capitalized_other_store_type() {
        let config = Config::load_from_string(
            r#"
            manifest:
              url: example.com
              etag: null
            roots:
              - path: ~/other
                store: Other
            backup:
              path: ~/backup
            restore:
              path: ~/restore
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                manifest: ManifestConfig {
                    url: s("example.com"),
                    etag: None,
                },
                roots: vec![RootsConfig {
                    path: StrictPath::new(s("~/other")),
                    store: Store::Other,
                }],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: std::collections::HashSet::new(),
                    merge: false,
                    filter: BackupFilter {
                        exclude_other_os_data: false,
                        exclude_store_screenshots: false,
                    },
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: std::collections::HashSet::new(),
                    redirects: vec![],
                },
                custom_games: vec![],
            },
            config,
        );
    }

    #[test]
    fn can_be_serialized() {
        assert_eq!(
            r#"
---
manifest:
  url: example.com
  etag: foo
roots:
  - path: ~/steam
    store: steam
  - path: ~/other
    store: other
backup:
  path: ~/backup
  ignoredGames:
    - Backup Game 1
    - Backup Game 2
    - Backup Game 3
  merge: true
  filter:
    excludeOtherOsData: true
    excludeStoreScreenshots: true
restore:
  path: ~/restore
  ignoredGames:
    - Restore Game 1
    - Restore Game 2
    - Restore Game 3
  redirects:
    - source: ~/old
      target: ~/new
customGames:
  - name: Custom Game 1
    files: []
    registry: []
  - name: Custom Game 2
    files:
      - Custom File 1
      - Custom File 2
      - Custom File 2
    registry:
      - Custom Registry 1
      - Custom Registry 2
      - Custom Registry 2
"#
            .trim(),
            serde_yaml::to_string(&Config {
                manifest: ManifestConfig {
                    url: s("example.com"),
                    etag: Some(s("foo")),
                },
                roots: vec![
                    RootsConfig {
                        path: StrictPath::new(s("~/steam")),
                        store: Store::Steam,
                    },
                    RootsConfig {
                        path: StrictPath::new(s("~/other")),
                        store: Store::Other,
                    },
                ],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: hashset! {
                        s("Backup Game 3"),
                        s("Backup Game 1"),
                        s("Backup Game 2"),
                    },
                    merge: true,
                    filter: BackupFilter {
                        exclude_other_os_data: true,
                        exclude_store_screenshots: true,
                    },
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: hashset! {
                        s("Restore Game 3"),
                        s("Restore Game 1"),
                        s("Restore Game 2"),
                    },
                    redirects: vec![RedirectConfig {
                        source: StrictPath::new(s("~/old")),
                        target: StrictPath::new(s("~/new")),
                    },],
                },
                custom_games: vec![
                    CustomGame {
                        name: s("Custom Game 1"),
                        files: vec![],
                        registry: vec![],
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        files: vec![s("Custom File 1"), s("Custom File 2"), s("Custom File 2"),],
                        registry: vec![s("Custom Registry 1"), s("Custom Registry 2"), s("Custom Registry 2"),],
                    },
                ],
            })
            .unwrap()
            .trim(),
        );
    }
}
