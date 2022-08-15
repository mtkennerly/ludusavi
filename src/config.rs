use crate::{
    lang::Language,
    manifest::Store,
    prelude::{app_dir, Error, RegistryItem, StrictPath},
};

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

fn default_backup_dir() -> StrictPath {
    let mut path = dirs::home_dir().unwrap();
    path.push("ludusavi-backup");
    StrictPath::from_std_path_buf(&path)
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub manifest: ManifestConfig,
    #[serde(default)]
    pub language: Language,
    pub roots: Vec<RootsConfig>,
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
    #[serde(default, rename = "customGames")]
    pub custom_games: Vec<CustomGame>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestConfig {
    pub url: String,
    pub etag: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RootsConfig {
    pub path: StrictPath,
    pub store: Store,
}

impl RootsConfig {
    pub fn glob(&self) -> Vec<Self> {
        self.path
            .glob()
            .iter()
            .cloned()
            .map(|path| RootsConfig {
                path,
                store: self.store,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedirectConfig {
    pub source: StrictPath,
    pub target: StrictPath,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    #[serde(default, rename = "ignoredPaths")]
    pub ignored_paths: Vec<StrictPath>,
    #[serde(default, rename = "ignoredRegistry")]
    pub ignored_registry: Vec<RegistryItem>,
}

impl BackupFilter {
    pub fn is_path_ignored(&self, item: &StrictPath) -> bool {
        let interpreted = item.interpret();
        self.ignored_paths
            .iter()
            .any(|x| x.is_prefix_of(item) || x.interpret() == interpreted)
    }

    #[allow(dead_code)]
    pub fn is_registry_ignored(&self, item: &RegistryItem) -> bool {
        let interpreted = item.interpret();
        self.ignored_registry
            .iter()
            .any(|x| x.is_prefix_of(item) || x.interpret() == interpreted)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToggledPaths(std::collections::BTreeMap<String, std::collections::BTreeMap<StrictPath, bool>>);

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToggledRegistry(std::collections::BTreeMap<String, std::collections::BTreeMap<RegistryItem, bool>>);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SortKey {
    #[default]
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "size")]
    Size,
}

impl SortKey {
    pub const ALL: &'static [Self] = &[Self::Name, Self::Size];
}

impl std::fmt::Display for SortKey {
    // This is needed for Iced's PickList.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        // TODO: Use display adapter wrapper struct to respect the active language.
        writeln!(f, "{}", crate::lang::Translator::default().sort_key(self))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sort {
    pub key: SortKey,
    pub reversed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Retention {
    pub full: u8,
    pub differential: u8,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            full: 1,
            differential: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BackupFormat {
    #[default]
    #[serde(rename = "simple")]
    Simple,
    #[serde(rename = "zip")]
    Zip,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackupFormats {
    pub chosen: BackupFormat,
    pub zip: ZipConfig,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZipConfig {
    pub compression: ZipCompression,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ZipCompression {
    #[serde(rename = "none")]
    None,
    #[default]
    #[serde(rename = "deflate")]
    Deflate,
    #[serde(rename = "bzip2")]
    Bzip2,
    #[serde(rename = "zstd")]
    Zstd,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackupConfig {
    pub path: StrictPath,
    #[serde(
        default,
        rename = "ignoredGames",
        serialize_with = "crate::serialization::ordered_set"
    )]
    pub ignored_games: std::collections::HashSet<String>,
    #[serde(default = "crate::serialization::default_true")]
    pub merge: bool,
    #[serde(default)]
    pub filter: BackupFilter,
    #[serde(default, rename = "toggledPaths")]
    pub toggled_paths: ToggledPaths,
    #[serde(default, rename = "toggledRegistry")]
    pub toggled_registry: ToggledRegistry,
    #[serde(default)]
    pub sort: Sort,
    #[serde(default)]
    pub retention: Retention,
    #[serde(default)]
    pub format: BackupFormats,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    #[serde(default)]
    pub sort: Sort,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomGame {
    pub name: String,
    #[serde(default, skip_serializing_if = "crate::serialization::is_false")]
    pub ignore: bool,
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
            merge: true,
            filter: BackupFilter::default(),
            toggled_paths: Default::default(),
            toggled_registry: Default::default(),
            sort: Default::default(),
            retention: Retention::default(),
            format: Default::default(),
        }
    }
}

impl Default for RestoreConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
            ignored_games: std::collections::HashSet::new(),
            redirects: vec![],
            sort: Default::default(),
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
        serde_yaml::from_str(content).map_err(|e| Error::ConfigInvalid { why: format!("{}", e) })
    }

    pub fn find_missing_roots(&self) -> Vec<RootsConfig> {
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
            (format!("{}/Epic Games", pf32), Store::Epic),
            (format!("{}/Epic Games", pf64), Store::Epic),
            // GOG:
            ("C:/GOG Games".to_string(), Store::Gog),
            ("~/GOG Games".to_string(), Store::Gog),
            // GOG Galaxy:
            (format!("{}/GOG Galaxy/Games", pf32), Store::GogGalaxy),
            (format!("{}/GOG Galaxy/Games", pf64), Store::GogGalaxy),
            // Uplay:
            (format!("{}/Ubisoft/Ubisoft Game Launcher", pf32), Store::Uplay),
            (format!("{}/Ubisoft/Ubisoft Game Launcher", pf64), Store::Uplay),
            // Origin:
            (format!("{}/Origin Games", pf32), Store::Origin),
            (format!("{}/Origin Games", pf64), Store::Origin),
            // Microsoft:
            (format!("{}/WindowsApps", pf32), Store::Microsoft),
            (format!("{}/WindowsApps", pf64), Store::Microsoft),
            // Prime Gaming:
            ("C:/Amazon Games/Library".to_string(), Store::Prime),
        ];

        let detected_steam = match steamlocate::SteamDir::locate() {
            Some(mut steam_dir) => steam_dir
                .libraryfolders()
                .paths
                .iter()
                .cloned()
                .map(|mut pb| {
                    // Remove "/steamapps" suffix:
                    pb.pop();
                    pb
                })
                .map(|pb| (pb.as_os_str().to_string_lossy().to_string(), Store::Steam))
                .collect(),
            None => vec![],
        };

        #[cfg(target_os = "windows")]
        let detected_epic: Vec<(String, Store)> = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey(r"SOFTWARE\Epic Games\EOS")
            .and_then(|subkey| {
                #[derive(serde::Deserialize)]
                struct EpicManifest {
                    #[serde(rename = "InstallLocation")]
                    install_location: String,
                }

                let mut install_dirs = vec![];
                let manifest_dir = subkey.get_value::<String, &str>("ModSdkMetadataDir")?;
                for entry in std::fs::read_dir(&manifest_dir)?.flatten() {
                    if !entry.file_type()?.is_file() {
                        continue;
                    }
                    let content = std::fs::read_to_string(entry.path())?;
                    let manifest = serde_json::from_str::<EpicManifest>(&content)?;
                    let normalized = manifest.install_location.replace('\\', "/");
                    if let Some((prefix, _)) = normalized.rsplit_once('/') {
                        let prefix = prefix.trim();
                        if crate::path::is_raw_path_relative(prefix) {
                            continue;
                        }
                        install_dirs.push(prefix.to_string());
                    }
                }
                Ok(install_dirs.iter().cloned().map(|x| (x, Store::Epic)).collect())
            })
            .unwrap_or_default();
        #[cfg(not(target_os = "windows"))]
        let detected_epic = vec![];

        let mut checked = std::collections::HashSet::<StrictPath>::new();
        let mut roots = vec![];
        for (path, store) in [candidates, detected_steam, detected_epic].concat() {
            let sp = StrictPath::new(path);
            if self.roots.iter().any(|root| root.path.interpret() == sp.interpret())
                || checked.contains(&sp.interpreted())
            {
                continue;
            }
            if sp.is_dir() {
                roots.push(RootsConfig {
                    path: sp.rendered(),
                    store,
                });
            }
            checked.insert(sp.interpreted());
        }

        roots
    }

    pub fn add_common_roots(&mut self) {
        self.roots.extend(self.find_missing_roots());
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
            ignore: false,
            files: vec![],
            registry: vec![],
        });
    }

    pub fn is_game_customized(&self, name: &str) -> bool {
        self.custom_games.iter().any(|x| x.name == name)
    }

    pub fn enable_custom_game(&mut self, index: usize) {
        self.custom_games[index].ignore = false;
    }

    pub fn disable_custom_game(&mut self, index: usize) {
        self.custom_games[index].ignore = true;
    }

    pub fn is_custom_game_enabled(&self, index: usize) -> bool {
        !self.custom_games[index].ignore
    }

    pub fn are_all_custom_games_enabled(&self) -> bool {
        self.custom_games.iter().all(|x| !x.ignore)
    }
}

impl ToggledPaths {
    #[cfg(test)]
    pub fn new(data: std::collections::BTreeMap<String, std::collections::BTreeMap<StrictPath, bool>>) -> Self {
        Self(data)
    }

    pub fn is_ignored(&self, game: &str, path: &StrictPath) -> bool {
        let transitive = self.is_enabled_transitively(game, path);
        let specific = self.is_enabled_specifically(game, path);
        match (transitive, specific) {
            (_, Some(x)) => !x,
            (Some(x), _) => !x,
            _ => false,
        }
    }

    fn is_enabled_transitively(&self, game: &str, path: &StrictPath) -> Option<bool> {
        self.0.get(game).and_then(|x| {
            path.nearest_prefix(x.keys().cloned().collect())
                .as_ref()
                .map(|prefix| x[prefix])
        })
    }

    fn is_enabled_specifically(&self, game: &str, path: &StrictPath) -> Option<bool> {
        self.0.get(game).and_then(|x| match x.get(path) {
            Some(enabled) => Some(*enabled),
            None => x
                .iter()
                .find(|(k, _)| path.interpret() == k.interpret())
                .map(|(_, v)| *v),
        })
    }

    fn set_enabled(&mut self, game: &str, path: &StrictPath, enabled: bool) {
        self.remove_with_children(game, path);
        self.0
            .entry(game.to_string())
            .or_insert_with(Default::default)
            .insert(path.clone(), enabled);
    }

    fn remove(&mut self, game: &str, path: &StrictPath) {
        self.remove_with_children(game, path);
        if self.0[game].is_empty() {
            self.0.remove(game);
        }
    }

    fn remove_with_children(&mut self, game: &str, path: &StrictPath) {
        let keys: Vec<_> = self
            .0
            .get(game)
            .map(|x| x.keys().cloned().collect())
            .unwrap_or_default();
        for key in keys {
            if path.is_prefix_of(&key) || key.interpret() == path.interpret() {
                self.0.get_mut(game).map(|entry| entry.remove(&key));
            }
        }
    }

    pub fn toggle(&mut self, game: &str, path: &StrictPath) {
        let transitive = self.is_enabled_transitively(game, path);
        let specific = self.is_enabled_specifically(game, path);
        match (transitive, specific) {
            (None, None | Some(true)) => {
                self.set_enabled(game, path, false);
            }
            (None, Some(false)) => {
                self.remove(game, path);
            }
            (Some(x), None) => {
                self.set_enabled(game, path, !x);
            }
            (Some(x), Some(y)) if x == y => {
                self.set_enabled(game, path, !x);
            }
            (Some(_), Some(_)) => {
                self.remove(game, path);
            }
        }
    }
}

impl ToggledRegistry {
    #[cfg(test)]
    pub fn new(data: std::collections::BTreeMap<String, std::collections::BTreeMap<RegistryItem, bool>>) -> Self {
        Self(data)
    }

    pub fn is_ignored(&self, game: &str, path: &RegistryItem) -> bool {
        let transitive = self.is_enabled_transitively(game, path);
        let specific = self.is_enabled_specifically(game, path);
        match (transitive, specific) {
            (_, Some(x)) => !x,
            (Some(x), _) => !x,
            _ => false,
        }
    }

    fn is_enabled_transitively(&self, game: &str, path: &RegistryItem) -> Option<bool> {
        self.0.get(game).and_then(|x| {
            path.nearest_prefix(x.keys().cloned().collect())
                .as_ref()
                .map(|prefix| x[prefix])
        })
    }

    fn is_enabled_specifically(&self, game: &str, path: &RegistryItem) -> Option<bool> {
        self.0.get(game).and_then(|x| match x.get(path) {
            Some(enabled) => Some(*enabled),
            None => x
                .iter()
                .find(|(k, _)| path.interpret() == k.interpret())
                .map(|(_, v)| *v),
        })
    }

    fn set_enabled(&mut self, game: &str, path: &RegistryItem, enabled: bool) {
        self.remove_with_children(game, path);
        self.0
            .entry(game.to_string())
            .or_insert_with(Default::default)
            .insert(path.clone(), enabled);
    }

    fn remove(&mut self, game: &str, path: &RegistryItem) {
        self.remove_with_children(game, path);
        self.0.get_mut(game).map(|entry| entry.remove(path));
        if self.0[game].is_empty() {
            self.0.remove(game);
        }
    }

    fn remove_with_children(&mut self, game: &str, path: &RegistryItem) {
        let keys: Vec<_> = self
            .0
            .get(game)
            .map(|x| x.keys().cloned().collect())
            .unwrap_or_default();
        for key in keys {
            if path.is_prefix_of(&key) || key.interpret() == path.interpret() {
                self.0.get_mut(game).map(|entry| entry.remove(&key));
            }
        }
    }

    pub fn toggle(&mut self, game: &str, path: &RegistryItem) {
        let transitive = self.is_enabled_transitively(game, path);
        let specific = self.is_enabled_specifically(game, path);
        match (transitive, specific) {
            (None, None | Some(true)) => {
                self.set_enabled(game, path, false);
            }
            (None, Some(false)) => {
                self.remove(game, path);
            }
            (Some(x), None) => {
                self.set_enabled(game, path, !x);
            }
            (Some(x), Some(y)) if x == y => {
                self.set_enabled(game, path, !x);
            }
            (Some(_), Some(_)) => {
                self.remove(game, path);
            }
        }
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
                language: Language::English,
                roots: vec![],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: std::collections::HashSet::new(),
                    merge: true,
                    filter: BackupFilter {
                        exclude_other_os_data: false,
                        exclude_store_screenshots: false,
                        ..Default::default()
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    retention: Retention::default(),
                    format: Default::default(),
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: std::collections::HashSet::new(),
                    redirects: vec![],
                    sort: Default::default(),
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
                language: Language::English,
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
                        ..Default::default()
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    retention: Retention::default(),
                    format: Default::default(),
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
                    }],
                    sort: Default::default(),
                },
                custom_games: vec![
                    CustomGame {
                        name: s("Custom Game 1"),
                        ignore: false,
                        files: vec![],
                        registry: vec![],
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        ignore: false,
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
                language: Language::English,
                roots: vec![RootsConfig {
                    path: StrictPath::new(s("~/other")),
                    store: Store::Other,
                }],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: std::collections::HashSet::new(),
                    merge: true,
                    filter: BackupFilter {
                        exclude_other_os_data: false,
                        exclude_store_screenshots: false,
                        ..Default::default()
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    retention: Retention::default(),
                    format: Default::default(),
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: std::collections::HashSet::new(),
                    redirects: vec![],
                    sort: Default::default(),
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
language: en-US
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
    ignoredPaths: []
    ignoredRegistry: []
  toggledPaths: {}
  toggledRegistry: {}
  sort:
    key: name
    reversed: false
  retention:
    full: 1
    differential: 0
  format:
    chosen: simple
    zip:
      compression: deflate
restore:
  path: ~/restore
  ignoredGames:
    - Restore Game 1
    - Restore Game 2
    - Restore Game 3
  redirects:
    - source: ~/old
      target: ~/new
  sort:
    key: name
    reversed: false
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
                language: Language::English,
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
                        ..Default::default()
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    retention: Retention::default(),
                    format: Default::default(),
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
                    }],
                    sort: Default::default(),
                },
                custom_games: vec![
                    CustomGame {
                        name: s("Custom Game 1"),
                        ignore: false,
                        files: vec![],
                        registry: vec![],
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        ignore: false,
                        files: vec![s("Custom File 1"), s("Custom File 2"), s("Custom File 2"),],
                        registry: vec![s("Custom Registry 1"), s("Custom Registry 2"), s("Custom Registry 2"),],
                    },
                ],
            })
            .unwrap()
            .trim(),
        );
    }

    mod ignored_paths {
        use super::*;
        use maplit::*;
        use pretty_assertions::assert_eq;

        fn repo() -> String {
            env!("CARGO_MANIFEST_DIR").to_string()
        }

        fn repo_path(path: &str) -> String {
            format!("{}/{}", repo(), path)
        }

        fn verify_toggle_registry_bouncing(mut toggled: ToggledPaths, path: &str, initial: bool, after: ToggledPaths) {
            let untoggled = toggled.clone();

            let path = StrictPath::new(path.to_string());
            assert_eq!(initial, !toggled.is_ignored("game", &path));

            toggled.toggle("game", &path);
            assert_eq!(!initial, !toggled.is_ignored("game", &path));
            assert_eq!(after, toggled);

            toggled.toggle("game", &path);
            assert_eq!(initial, !toggled.is_ignored("game", &path));
            assert_eq!(untoggled, toggled);
        }

        fn verify_toggle_registry_sequential(
            mut toggled: ToggledPaths,
            path: &str,
            initial: bool,
            states: Vec<ToggledPaths>,
        ) {
            let path = StrictPath::new(path.to_string());
            assert_eq!(initial, !toggled.is_ignored("game", &path));

            let mut enabled = initial;
            for state in states {
                enabled = !enabled;
                toggled.toggle("game", &path);
                assert_eq!(enabled, !toggled.is_ignored("game", &path));
                assert_eq!(state, toggled);
            }
        }

        #[test]
        fn transitively_unset_and_specifically_unset_or_disabled() {
            verify_toggle_registry_bouncing(
                ToggledPaths::default(),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                true,
                ToggledPaths(btreemap! {
                    s("game") => btreemap! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")) => false,
                    }
                }),
            );
        }

        #[test]
        fn transitively_unset_and_specifically_enabled() {
            verify_toggle_registry_sequential(
                ToggledPaths(btreemap! {
                    s("game") => btreemap! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")) => true,
                    }
                }),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                true,
                vec![
                    ToggledPaths(btreemap! {
                        s("game") => btreemap! {
                            StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")) => false,
                        }
                    }),
                    ToggledPaths::default(),
                ],
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_unset_or_enabled() {
            verify_toggle_registry_bouncing(
                ToggledPaths(btreemap! {
                    s("game") => btreemap! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir")) => false,
                    }
                }),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                false,
                ToggledPaths(btreemap! {
                    s("game") => btreemap! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir")) => false,
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")) => true,
                    }
                }),
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_disabled() {
            verify_toggle_registry_sequential(
                ToggledPaths(btreemap! {
                    s("game") => btreemap! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir")) => false,
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")) => false,
                    }
                }),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                false,
                vec![
                    ToggledPaths(btreemap! {
                        s("game") => btreemap! {
                            StrictPath::new(repo_path("tests/root1/game1/subdir")) => false,
                            StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")) => true,
                        }
                    }),
                    ToggledPaths(btreemap! {
                        s("game") => btreemap! {
                            StrictPath::new(repo_path("tests/root1/game1/subdir")) => false,
                        }
                    }),
                ],
            );
        }
    }

    mod ignored_registry {
        use super::*;
        use maplit::*;
        use pretty_assertions::assert_eq;

        fn verify_toggle_registry_bouncing(
            mut toggled: ToggledRegistry,
            path: &str,
            initial: bool,
            after: ToggledRegistry,
        ) {
            let untoggled = toggled.clone();

            let path = RegistryItem::new(path.to_string());
            assert_eq!(initial, !toggled.is_ignored("game", &path));

            toggled.toggle("game", &path);
            assert_eq!(!initial, !toggled.is_ignored("game", &path));
            assert_eq!(after, toggled);

            toggled.toggle("game", &path);
            assert_eq!(initial, !toggled.is_ignored("game", &path));
            assert_eq!(untoggled, toggled);
        }

        fn verify_toggle_registry_sequential(
            mut toggled: ToggledRegistry,
            path: &str,
            initial: bool,
            states: Vec<ToggledRegistry>,
        ) {
            let path = RegistryItem::new(path.to_string());
            assert_eq!(initial, !toggled.is_ignored("game", &path));

            let mut enabled = initial;
            for state in states {
                enabled = !enabled;
                toggled.toggle("game", &path);
                assert_eq!(enabled, !toggled.is_ignored("game", &path));
                assert_eq!(state, toggled);
            }
        }

        #[test]
        fn transitively_unset_and_specifically_unset_or_disabled() {
            verify_toggle_registry_bouncing(
                ToggledRegistry::default(),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                true,
                ToggledRegistry(btreemap! {
                    s("game") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")) => false,
                    }
                }),
            );
        }

        #[test]
        fn transitively_unset_and_specifically_enabled() {
            verify_toggle_registry_sequential(
                ToggledRegistry(btreemap! {
                    s("game") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")) => true,
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                true,
                vec![
                    ToggledRegistry(btreemap! {
                        s("game") => btreemap! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")) => false,
                        }
                    }),
                    ToggledRegistry::default(),
                ],
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_unset_or_enabled() {
            verify_toggle_registry_bouncing(
                ToggledRegistry(btreemap! {
                    s("game") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software")) => false,
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                false,
                ToggledRegistry(btreemap! {
                    s("game") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software")) => false,
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")) => true,
                    }
                }),
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_disabled() {
            verify_toggle_registry_sequential(
                ToggledRegistry(btreemap! {
                    s("game") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software")) => false,
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")) => false,
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                false,
                vec![
                    ToggledRegistry(btreemap! {
                        s("game") => btreemap! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software")) => false,
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")) => true,
                        }
                    }),
                    ToggledRegistry(btreemap! {
                        s("game") => btreemap! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software")) => false,
                        }
                    }),
                ],
            );
        }
    }
}
