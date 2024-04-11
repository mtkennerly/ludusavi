use std::{
    collections::{BTreeMap, HashMap, HashSet},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crate::{
    cloud::Remote,
    lang::{Language, TRANSLATOR},
    path::CommonPath,
    prelude::{app_dir, Error, StrictPath, AVAILABLE_PARALELLISM},
    resource::{
        manifest::{Manifest, Store},
        ResourceFile, SaveableResourceFile,
    },
    scan::registry_compat::RegistryItem,
};

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

fn default_backup_dir() -> StrictPath {
    StrictPath::new(format!("{}/ludusavi-backup", CommonPath::Home.get().unwrap()))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub runtime: Runtime,
    pub manifest: ManifestConfig,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme: Theme,
    pub roots: Vec<RootsConfig>,
    #[serde(default)]
    pub redirects: Vec<RedirectConfig>,
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
    #[serde(default)]
    pub scan: Scan,
    #[serde(default)]
    pub cloud: Cloud,
    #[serde(default)]
    pub apps: Apps,
    #[serde(default, rename = "customGames")]
    pub custom_games: Vec<CustomGame>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Runtime {
    #[serde(default)]
    pub threads: Option<NonZeroUsize>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestConfig {
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary: Vec<SecondaryManifestConfig>,
}

impl ManifestConfig {
    pub fn secondary_manifest_urls(&self) -> Vec<&str> {
        self.secondary
            .iter()
            .filter_map(|x| match x {
                SecondaryManifestConfig::Local { .. } => None,
                SecondaryManifestConfig::Remote { url } => Some(url.as_str()),
            })
            .collect()
    }

    pub fn load_secondary_manifests(&self) -> Vec<(StrictPath, Manifest)> {
        self.secondary
            .iter()
            .filter_map(|x| match x {
                SecondaryManifestConfig::Local { path } => {
                    let manifest = Manifest::load_from_existing(path);
                    if let Err(e) = &manifest {
                        log::error!("Cannot load secondary manifest: {:?} | {}", &path, e);
                    }
                    Some((path.clone(), manifest.ok()?))
                }
                SecondaryManifestConfig::Remote { url } => {
                    let path = Manifest::path_for(url, false);
                    let manifest = Manifest::load_from(&path);
                    if let Err(e) = &manifest {
                        log::error!("Cannot load manifest: {:?} | {}", &path, e);
                    }
                    Some((path.clone(), manifest.ok()?))
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SecondaryManifestConfigKind {
    Local,
    #[default]
    Remote,
}

impl SecondaryManifestConfigKind {
    pub const ALL: &'static [Self] = &[Self::Local, Self::Remote];
}

impl ToString for SecondaryManifestConfigKind {
    fn to_string(&self) -> String {
        match self {
            Self::Local => TRANSLATOR.file_label(),
            Self::Remote => TRANSLATOR.url_label(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum SecondaryManifestConfig {
    Local { path: StrictPath },
    Remote { url: String },
}

impl SecondaryManifestConfig {
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::Remote { url } => Some(url.as_str()),
        }
    }

    pub fn path(&self) -> Option<&StrictPath> {
        match self {
            Self::Local { path } => Some(path),
            Self::Remote { .. } => None,
        }
    }

    pub fn value(&self) -> String {
        match self {
            Self::Local { path } => path.raw(),
            Self::Remote { url } => url.to_string(),
        }
    }

    pub fn set(&mut self, value: String) {
        match self {
            Self::Local { path } => *path = StrictPath::new(value),
            Self::Remote { url } => *url = value,
        }
    }

    pub fn kind(&self) -> SecondaryManifestConfigKind {
        match self {
            Self::Local { .. } => SecondaryManifestConfigKind::Local,
            Self::Remote { .. } => SecondaryManifestConfigKind::Remote,
        }
    }

    pub fn convert(&mut self, kind: SecondaryManifestConfigKind) {
        match (&self, kind) {
            (Self::Local { path }, SecondaryManifestConfigKind::Remote) => {
                *self = Self::Remote { url: path.raw() };
            }
            (Self::Remote { url }, SecondaryManifestConfigKind::Local) => {
                *self = Self::Local {
                    path: StrictPath::new(url.clone()),
                };
            }
            _ => {}
        }
    }
}

impl Default for SecondaryManifestConfig {
    fn default() -> Self {
        Self::Remote { url: "".to_string() }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    #[default]
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

impl Theme {
    pub const ALL: &'static [Self] = &[Self::Light, Self::Dark];
}

impl ToString for Theme {
    fn to_string(&self) -> String {
        TRANSLATOR.theme_name(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
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

    pub fn find_secondary_manifests(&self) -> HashMap<StrictPath, Manifest> {
        self.path
            .joined(match self.store {
                Store::Steam => "steamapps/common/*/.ludusavi.yaml",
                _ => "*/.ludusavi.yaml",
            })
            .glob()
            .into_iter()
            .filter_map(|path| match Manifest::load_from(&path) {
                Ok(manifest) => {
                    log::info!("Loaded secondary manifest: {}", path.render());
                    log::trace!("Secondary manifest content: {:?}", &manifest);
                    Some((path, manifest))
                }
                Err(e) => {
                    log::error!("Failed to load secondary manifest: {} | {e}", path.render());
                    None
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedirectConfig {
    #[serde(default)]
    pub kind: RedirectKind,
    pub source: StrictPath,
    pub target: StrictPath,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RedirectKind {
    #[serde(rename = "backup")]
    Backup,
    #[default]
    #[serde(rename = "restore")]
    Restore,
    #[serde(rename = "bidirectional")]
    Bidirectional,
}

impl RedirectKind {
    pub const ALL: &'static [Self] = &[Self::Backup, Self::Restore, Self::Bidirectional];
}

impl ToString for RedirectKind {
    fn to_string(&self) -> String {
        TRANSLATOR.redirect_kind(self)
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BackupFilter {
    #[serde(default, rename = "excludeStoreScreenshots")]
    pub exclude_store_screenshots: bool,
    #[serde(default, rename = "ignoredPaths")]
    pub ignored_paths: Vec<StrictPath>,
    #[serde(default, rename = "ignoredRegistry")]
    pub ignored_registry: Vec<RegistryItem>,
    #[serde(skip)]
    pub path_globs: Arc<Mutex<Option<globset::GlobSet>>>,
}

impl std::fmt::Debug for BackupFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupFilter")
            .field("exclude_store_screenshots", &self.exclude_store_screenshots)
            .field("ignored_paths", &self.ignored_paths)
            .field("ignored_registry", &self.ignored_registry)
            .finish()
    }
}

impl Eq for BackupFilter {}

impl PartialEq for BackupFilter {
    fn eq(&self, other: &Self) -> bool {
        self.exclude_store_screenshots == other.exclude_store_screenshots
            && self.ignored_paths == other.ignored_paths
            && self.ignored_registry == other.ignored_registry
    }
}

impl BackupFilter {
    pub fn build_globs(&mut self) {
        let mut path_globs = self.path_globs.lock().unwrap();
        if self.ignored_paths.is_empty() {
            *path_globs = None;
            return;
        }

        let mut builder = globset::GlobSetBuilder::new();
        for item in &self.ignored_paths {
            let normalized = item.globbable();

            let variants = vec![
                normalized.to_string(),
                // If the user has specified a plain folder, we also want to include its children.
                format!("{}/**", &normalized),
            ];

            for variant in variants {
                if let Ok(glob) = globset::GlobBuilder::new(&variant)
                    .literal_separator(true)
                    .backslash_escape(false)
                    .case_insensitive(true)
                    .build()
                {
                    builder.add(glob);
                }
            }
        }

        *path_globs = builder.build().ok();
    }

    pub fn is_path_ignored(&self, item: &StrictPath) -> bool {
        if self.ignored_paths.is_empty() {
            return false;
        }

        let path_globs = self.path_globs.lock().unwrap();
        path_globs
            .as_ref()
            .map(|set| set.is_match(item.render()))
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn is_registry_ignored(&self, item: &RegistryItem) -> bool {
        if self.ignored_registry.is_empty() {
            return false;
        }
        let interpreted = item.interpret();
        self.ignored_registry
            .iter()
            .any(|x| x.is_prefix_of(item) || x.interpret() == interpreted)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToggledPaths(BTreeMap<String, BTreeMap<StrictPath, bool>>);

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToggledRegistry(BTreeMap<String, BTreeMap<RegistryItem, ToggledRegistryEntry>>);

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ToggledRegistryEntry {
    Unset,
    Key(bool),
    Complex {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<bool>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        values: BTreeMap<String, bool>,
    },
}

impl ToggledRegistryEntry {
    fn prune(&mut self) {
        if let Self::Complex { key, values } = self {
            if let Some(key) = key {
                let mut unnecessary = vec![];
                for (value_name, value) in values.iter() {
                    if value == key {
                        unnecessary.push(value_name.clone());
                    }
                }
                for item in unnecessary {
                    values.remove(&item);
                }
                if values.is_empty() {
                    *self = Self::Key(*key);
                }
            } else if values.is_empty() {
                *self = Self::Unset;
            }
        }
    }

    pub fn enable(&mut self, value: Option<&str>, enabled: bool) {
        match value {
            Some(value) => self.enable_value(value, enabled),
            None => self.enable_key(enabled),
        }
        self.prune();
    }

    fn enable_key(&mut self, enabled: bool) {
        match self {
            Self::Unset => *self = Self::Key(enabled),
            Self::Key(key) => *key = enabled,
            Self::Complex { key, .. } => *key = Some(enabled),
        }
    }

    fn enable_value(&mut self, value: &str, enabled: bool) {
        match self {
            Self::Unset => {
                let mut values = BTreeMap::<String, bool>::new();
                values.insert(value.to_string(), enabled);
                *self = Self::Complex { key: None, values };
            }
            Self::Key(key) => {
                let mut values = BTreeMap::<String, bool>::new();
                values.insert(value.to_string(), enabled);
                *self = Self::Complex {
                    key: Some(*key),
                    values,
                };
            }
            Self::Complex { values, .. } => {
                values.insert(value.to_string(), enabled);
            }
        }
    }

    pub fn key_enabled(&self) -> Option<bool> {
        match self {
            Self::Unset => None,
            Self::Key(enabled) => Some(*enabled),
            Self::Complex { key, .. } => *key,
        }
    }

    pub fn value_enabled(&self, name: &str) -> Option<bool> {
        match self {
            Self::Unset => None,
            Self::Key(_) => None,
            Self::Complex { values, .. } => values.get(name).copied(),
        }
    }

    pub fn fully_enabled(&self) -> bool {
        match self {
            Self::Unset => true,
            Self::Key(enabled) => *enabled,
            Self::Complex { values, .. } => values.iter().all(|x| *x.1),
        }
    }

    pub fn remove_value(&mut self, value: &str) {
        if let Self::Complex { key, values } = self {
            values.remove(value);
            if key.is_none() && values.is_empty() {
                *self = Self::Unset;
            }
        }
        self.prune();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SortKey {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "size")]
    Size,
    #[default]
    #[serde(rename = "status")]
    Status,
}

impl SortKey {
    pub const ALL: &'static [Self] = &[Self::Name, Self::Size, Self::Status];
}

impl ToString for SortKey {
    fn to_string(&self) -> String {
        TRANSLATOR.sort_key(self)
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
    #[serde(default, skip)]
    pub force_new_full: bool,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            full: 1,
            differential: 0,
            force_new_full: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BackupFormat {
    #[default]
    #[serde(rename = "simple")]
    Simple,
    #[serde(rename = "zip")]
    Zip,
}

impl BackupFormat {
    pub const ALL: &'static [Self] = &[Self::Simple, Self::Zip];
    pub const ALL_NAMES: &'static [&'static str] = &["simple", "zip"];
}

impl std::str::FromStr for BackupFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "simple" => Ok(Self::Simple),
            "zip" => Ok(Self::Zip),
            _ => Err(format!("invalid backup format: {}", s)),
        }
    }
}

impl ToString for BackupFormat {
    fn to_string(&self) -> String {
        TRANSLATOR.backup_format(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackupFormats {
    pub chosen: BackupFormat,
    pub zip: ZipConfig,
    #[serde(default)]
    pub compression: Compression,
}

impl BackupFormats {
    pub fn level(&self) -> Option<i32> {
        match self.chosen {
            BackupFormat::Simple => None,
            BackupFormat::Zip => match self.zip.compression {
                ZipCompression::None => None,
                ZipCompression::Deflate => Some(self.compression.deflate.level),
                ZipCompression::Bzip2 => Some(self.compression.bzip2.level),
                ZipCompression::Zstd => Some(self.compression.zstd.level),
            },
        }
    }

    pub fn set_level(&mut self, value: i32) {
        match self.chosen {
            BackupFormat::Simple => {}
            BackupFormat::Zip => match self.zip.compression {
                ZipCompression::None => {}
                ZipCompression::Deflate => {
                    self.compression.deflate.level = value;
                }
                ZipCompression::Bzip2 => {
                    self.compression.bzip2.level = value;
                }
                ZipCompression::Zstd => {
                    self.compression.zstd.level = value;
                }
            },
        }
    }

    pub fn range(&self) -> Option<std::ops::RangeInclusive<i32>> {
        match self.chosen {
            BackupFormat::Simple => None,
            BackupFormat::Zip => match self.zip.compression {
                ZipCompression::None => None,
                ZipCompression::Deflate => Some(DeflateCompression::RANGE),
                ZipCompression::Bzip2 => Some(Bzip2Compression::RANGE),
                ZipCompression::Zstd => Some(ZstdCompression::RANGE),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZipConfig {
    pub compression: ZipCompression,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
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

impl ZipCompression {
    pub const ALL: &'static [Self] = &[Self::None, Self::Deflate, Self::Bzip2, Self::Zstd];
    pub const ALL_NAMES: &'static [&'static str] = &["none", "deflate", "bzip2", "zstd"];
}

impl std::str::FromStr for ZipCompression {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "deflate" => Ok(Self::Deflate),
            "bzip2" => Ok(Self::Bzip2),
            "zstd" => Ok(Self::Zstd),
            _ => Err(format!("invalid compression method: {}", s)),
        }
    }
}

impl ToString for ZipCompression {
    fn to_string(&self) -> String {
        TRANSLATOR.backup_compression(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Compression {
    deflate: DeflateCompression,
    bzip2: Bzip2Compression,
    zstd: ZstdCompression,
}

impl Compression {
    pub fn set_level(&mut self, method: &ZipCompression, level: i32) {
        match method {
            ZipCompression::None => {}
            ZipCompression::Deflate => {
                self.deflate.level = level.clamp(*DeflateCompression::RANGE.start(), *DeflateCompression::RANGE.end());
            }
            ZipCompression::Bzip2 => {
                self.bzip2.level = level.clamp(*Bzip2Compression::RANGE.start(), *Bzip2Compression::RANGE.end());
            }
            ZipCompression::Zstd => {
                self.zstd.level = level.clamp(*ZstdCompression::RANGE.start(), *ZstdCompression::RANGE.end());
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeflateCompression {
    level: i32,
}

impl Default for DeflateCompression {
    fn default() -> Self {
        Self { level: 6 }
    }
}

impl DeflateCompression {
    pub const RANGE: std::ops::RangeInclusive<i32> = 1..=9;
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bzip2Compression {
    level: i32,
}

impl Default for Bzip2Compression {
    fn default() -> Self {
        Self { level: 6 }
    }
}

impl Bzip2Compression {
    pub const RANGE: std::ops::RangeInclusive<i32> = 1..=9;
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZstdCompression {
    level: i32,
}

impl Default for ZstdCompression {
    fn default() -> Self {
        Self { level: 10 }
    }
}

impl ZstdCompression {
    pub const RANGE: std::ops::RangeInclusive<i32> = -7..=22;
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackupConfig {
    pub path: StrictPath,
    #[serde(
        default,
        rename = "ignoredGames",
        serialize_with = "crate::serialization::ordered_set"
    )]
    pub ignored_games: HashSet<String>,
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
    pub ignored_games: HashSet<String>,
    #[serde(default, rename = "toggledPaths")]
    pub toggled_paths: ToggledPaths,
    #[serde(default, rename = "toggledRegistry")]
    pub toggled_registry: ToggledRegistry,
    #[serde(default)]
    pub sort: Sort,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    #[serde(default = "crate::serialization::default_true")]
    pub show_deselected_games: bool,
    #[serde(default = "crate::serialization::default_true")]
    pub show_unchanged_games: bool,
    #[serde(default = "crate::serialization::default_true")]
    pub show_unscanned_games: bool,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            show_deselected_games: true,
            show_unchanged_games: true,
            show_unscanned_games: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cloud {
    #[serde(default)]
    pub remote: Option<Remote>,
    #[serde(default)]
    pub path: String,
    #[serde(default = "crate::serialization::default_true")]
    pub synchronize: bool,
}

impl Default for Cloud {
    fn default() -> Self {
        Self {
            remote: Default::default(),
            path: "ludusavi-backup".to_string(),
            synchronize: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Apps {
    #[serde(default = "App::default_rclone")]
    pub rclone: App,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            rclone: App::default_rclone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    #[serde(default)]
    pub path: StrictPath,
    #[serde(default)]
    pub arguments: String,
}

impl App {
    pub fn is_valid(&self) -> bool {
        !self.path.raw().is_empty() && (self.path.is_file() || which::which(self.path.raw()).is_ok())
    }

    fn default_rclone() -> Self {
        Self {
            path: which::which("rclone").map(StrictPath::from).unwrap_or_default(),
            arguments: "--fast-list --ignore-checksum".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomGame {
    pub name: String,
    #[serde(default, skip_serializing_if = "crate::serialization::is_false")]
    pub ignore: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub prefer_alias: bool,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub registry: Vec<String>,
}

impl CustomGame {
    pub fn kind(&self) -> CustomGameKind {
        if self.alias.is_some() {
            CustomGameKind::Alias
        } else {
            CustomGameKind::Game
        }
    }

    pub fn convert(&mut self, kind: CustomGameKind) {
        match kind {
            CustomGameKind::Game => {
                self.alias = None;
            }
            CustomGameKind::Alias => {
                self.alias = Some("".to_string());
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomGameKind {
    Game,
    Alias,
}

impl CustomGameKind {
    pub const ALL: &'static [Self] = &[Self::Game, Self::Alias];
}

impl ToString for CustomGameKind {
    fn to_string(&self) -> String {
        TRANSLATOR.custom_game_kind(self)
    }
}

impl Default for ManifestConfig {
    fn default() -> Self {
        Self {
            url: MANIFEST_URL.to_string(),
            secondary: vec![],
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
            ignored_games: HashSet::new(),
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
            ignored_games: HashSet::new(),
            toggled_paths: Default::default(),
            toggled_registry: Default::default(),
            sort: Default::default(),
        }
    }
}

impl ResourceFile for Config {
    const FILE_NAME: &'static str = "config.yaml";

    fn initialize(mut self) -> Self {
        self.add_common_roots();
        self
    }

    fn migrate(mut self) -> Self {
        self.roots.retain(|x| !x.path.raw().trim().is_empty());
        self.manifest.secondary.retain(|x| !x.value().trim().is_empty());
        self.redirects
            .retain(|x| !x.source.raw().trim().is_empty() && !x.target.raw().trim().is_empty());
        self.backup.filter.ignored_paths.retain(|x| !x.raw().trim().is_empty());
        self.backup
            .filter
            .ignored_registry
            .retain(|x| !x.raw().trim().is_empty());
        for item in &mut self.custom_games {
            item.files.retain(|x| !x.trim().is_empty());
            item.registry.retain(|x| !x.trim().is_empty());
        }
        self.custom_games
            .retain(|x| !x.name.trim().is_empty() || !x.files.is_empty() || !x.registry.is_empty());

        if self.apps.rclone.path.raw().is_empty() {
            if let Ok(path) = which::which("rclone") {
                self.apps.rclone.path = StrictPath::from(path);
            }
        }

        self.backup.filter.build_globs();

        self
    }
}

impl SaveableResourceFile for Config {}

impl Config {
    fn file_archived_invalid() -> StrictPath {
        app_dir().joined("config.invalid.yaml")
    }

    pub fn load() -> Result<Self, Error> {
        ResourceFile::load().map_err(|e| Error::ConfigInvalid { why: format!("{}", e) })
    }

    pub fn archive_invalid() -> Result<(), Box<dyn std::error::Error>> {
        Self::path().move_to(&Self::file_archived_invalid())?;
        Ok(())
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

        let mut candidates = vec![
            // Steam:
            (format!("{}/Steam", pf32), Store::Steam),
            (format!("{}/Steam", pf64), Store::Steam),
            ("~/.steam/steam".to_string(), Store::Steam),
            (
                "~/.var/app/com.valvesoftware.Steam/.steam/steam".to_string(),
                Store::Steam,
            ),
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
            // Heroic:
            ("~/.config/heroic".to_string(), Store::Heroic),
            (
                "~/.var/app/com.heroicgameslauncher.hgl/config/heroic".to_string(),
                Store::Heroic,
            ),
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
            // EA app:
            (format!("{}/EA Games", pf32), Store::Ea),
            (format!("{}/EA Games", pf64), Store::Ea),
            // Lutris:
            ("~/.config/lutris".to_string(), Store::Lutris),
            ("~/.var/app/net.lutris.Lutris/config/lutris".to_string(), Store::Lutris),
        ];

        if let Some(data_dir) = CommonPath::Data.get() {
            candidates.push((format!("{}/heroic", data_dir), Store::Heroic));
        }

        let detected_steam = match steamlocate::SteamDir::locate() {
            Ok(steam_dir) => match steam_dir.library_paths() {
                Ok(libraries) => libraries
                    .into_iter()
                    .map(|pb| (pb.as_os_str().to_string_lossy().to_string(), Store::Steam))
                    .collect(),
                Err(e) => {
                    log::warn!("Unable to load Steam libraries: {:?}", e);
                    vec![]
                }
            },
            Err(e) => {
                log::warn!("Unable to locate Steam directory: {:?}", e);
                vec![]
            }
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
                for entry in std::fs::read_dir(manifest_dir)?.flatten() {
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

        let mut checked = HashSet::<StrictPath>::new();
        let mut roots = vec![];
        for (path, store) in [candidates, detected_steam, detected_epic].concat() {
            let Ok(sp) = StrictPath::new(path).interpreted() else {
                continue;
            };
            if self.roots.iter().any(|root| root.path.equivalent(&sp)) || checked.contains(&sp) {
                continue;
            }
            if sp.is_dir() {
                roots.push(RootsConfig {
                    path: sp.rendered(),
                    store,
                });
            }
            checked.insert(sp);
        }

        roots
    }

    pub fn add_common_roots(&mut self) {
        self.roots.extend(self.find_missing_roots());
    }

    pub fn is_game_enabled_for_operation(&self, name: &str, restoring: bool) -> bool {
        if restoring {
            self.is_game_enabled_for_restore(name)
        } else {
            self.is_game_enabled_for_backup(name)
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

    pub fn any_saves_ignored(&self, name: &str, restoring: bool) -> bool {
        if restoring {
            self.restore
                .toggled_paths
                .0
                .get(name)
                .map(|x| x.values().any(|x| !x))
                .unwrap_or(false)
                || self
                    .restore
                    .toggled_registry
                    .0
                    .get(name)
                    .map(|x| x.values().any(|x| !x.fully_enabled()))
                    .unwrap_or(false)
        } else {
            self.backup
                .toggled_paths
                .0
                .get(name)
                .map(|x| x.values().any(|x| !x))
                .unwrap_or(false)
                || self
                    .backup
                    .toggled_registry
                    .0
                    .get(name)
                    .map(|x| x.values().any(|x| !x.fully_enabled()))
                    .unwrap_or(false)
        }
    }

    pub fn add_redirect(&mut self, source: &StrictPath, target: &StrictPath) {
        let redirect = RedirectConfig {
            kind: Default::default(),
            source: source.clone(),
            target: target.clone(),
        };
        self.redirects.push(redirect);
    }

    pub fn get_redirects(&self) -> Vec<RedirectConfig> {
        self.redirects.to_vec()
    }

    pub fn add_custom_game(&mut self) {
        self.custom_games.push(CustomGame {
            name: "".to_string(),
            ignore: false,
            alias: None,
            prefer_alias: false,
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

    pub fn is_custom_game_individually_scannable(&self, index: usize) -> bool {
        self.is_custom_game_enabled(index) && self.custom_games[index].kind() == CustomGameKind::Game
    }

    pub fn are_all_custom_games_enabled(&self) -> bool {
        self.custom_games.iter().all(|x| !x.ignore)
    }

    pub fn expanded_roots(&self) -> Vec<RootsConfig> {
        for root in &self.roots {
            log::trace!(
                "Configured root ({:?}): {} | interpreted: {:?} | exists: {} | is dir: {}",
                &root.store,
                &root.path.raw(),
                &root.path.interpret(),
                root.path.exists(),
                root.path.is_dir()
            );
        }

        let expanded: Vec<RootsConfig> = self.roots.iter().flat_map(|x| x.glob()).collect();

        for root in &expanded {
            log::trace!(
                "Expanded root ({:?}): {} | interpreted: {:?} | exists: {} | is dir: {}",
                &root.store,
                &root.path.raw(),
                &root.path.interpret(),
                root.path.exists(),
                root.path.is_dir()
            );
        }

        expanded
    }

    pub fn should_show_game(&self, name: &str, restoring: bool, changed: bool, scanned: bool) -> bool {
        (self.scan.show_deselected_games || self.is_game_enabled_for_operation(name, restoring))
            && (self.scan.show_unchanged_games || changed || !scanned)
            && (self.scan.show_unscanned_games || scanned)
    }

    pub fn override_threads(&mut self, overridden: bool) {
        if overridden {
            self.runtime.threads = *AVAILABLE_PARALELLISM;
        } else {
            self.runtime.threads = None;
        }
    }

    pub fn set_threads(&mut self, threads: usize) {
        self.runtime.threads = NonZeroUsize::new(threads);
    }

    pub fn display_name<'a>(&'a self, official: &'a str) -> &'a str {
        let aliases: HashMap<_, _> = self
            .custom_games
            .iter()
            .filter_map(|game| {
                let alias = game.name.as_str();
                let target = game.alias.as_ref()?.as_str();
                if game.ignore || !game.prefer_alias || alias.is_empty() || target.is_empty() {
                    return None;
                }
                Some((target, alias))
            })
            .collect();

        let mut query = official;
        for _ in 0..10 {
            match aliases.get(query) {
                Some(mapped) => query = mapped,
                None => break,
            }
        }

        query
    }
}

impl ToggledPaths {
    #[cfg(test)]
    pub fn new(data: BTreeMap<String, BTreeMap<StrictPath, bool>>) -> Self {
        Self(data)
    }

    pub fn invalidate_path_caches(&self) {
        for inner in self.0.values() {
            for key in inner.keys() {
                key.invalidate_cache();
            }
        }
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
            .or_default()
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
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn new(data: BTreeMap<String, BTreeMap<RegistryItem, ToggledRegistryEntry>>) -> Self {
        Self(data)
    }

    fn prune(&mut self, game: &str, path: &RegistryItem) {
        if !self.0.contains_key(game) {
            return;
        }
        if let Some(entry) = self.0.get_mut(game) {
            if entry.get(path) == Some(&ToggledRegistryEntry::Unset) {
                entry.remove(path);
            }
        }
        if self.0[game].is_empty() {
            self.0.remove(game);
        }
    }

    pub fn is_ignored(&self, game: &str, path: &RegistryItem, value: Option<&str>) -> bool {
        let transitive = self.is_enabled_transitively(game, path);
        let specific = self.is_key_enabled_specifically(game, path);
        let by_value = value.and_then(|value| self.is_value_enabled_specifically(game, path, value));
        match (transitive, specific, by_value) {
            (_, _, Some(x)) => !x,
            (_, Some(x), _) => !x,
            (Some(x), _, _) => !x,
            _ => false,
        }
    }

    fn is_enabled_transitively(&self, game: &str, path: &RegistryItem) -> Option<bool> {
        self.0.get(game).and_then(|x| {
            path.nearest_prefix(x.keys().cloned().collect())
                .as_ref()
                .and_then(|prefix| x[prefix].key_enabled())
        })
    }

    fn is_key_enabled_specifically(&self, game: &str, path: &RegistryItem) -> Option<bool> {
        self.0.get(game).and_then(|x| match x.get(path) {
            Some(entry) => entry.key_enabled(),
            None => x
                .iter()
                .find(|(k, _)| path.interpret() == k.interpret())
                .and_then(|(_, v)| v.key_enabled()),
        })
    }

    fn is_value_enabled_specifically(&self, game: &str, path: &RegistryItem, value: &str) -> Option<bool> {
        self.0.get(game).and_then(|x| match x.get(path) {
            Some(entry) => entry.value_enabled(value),
            None => x
                .iter()
                .find(|(k, _)| path.interpret() == k.interpret())
                .and_then(|(_, v)| v.value_enabled(value)),
        })
    }

    fn set_enabled(&mut self, game: &str, path: &RegistryItem, value: Option<&str>, enabled: bool) {
        if value.is_none() {
            self.remove_children(game, path);
        }

        self.0
            .entry(game.to_string())
            .or_default()
            .entry(path.clone())
            .or_insert(ToggledRegistryEntry::Unset)
            .enable(value, enabled);
    }

    fn remove(&mut self, game: &str, path: &RegistryItem, value: Option<&str>) {
        match value {
            Some(value) => {
                self.0
                    .get_mut(game)
                    .map(|entry| entry.get_mut(path).map(|key| key.remove_value(value)));
            }
            None => {
                self.remove_children(game, path);
                self.0.get_mut(game).map(|entry| entry.remove(path));
            }
        }
        if let Some(entry) = self.0.get_mut(game) {
            if entry.get(path) == Some(&ToggledRegistryEntry::Unset) {
                entry.remove(path);
            }
        }
        if self.0[game].is_empty() {
            self.0.remove(game);
        }
    }

    fn remove_children(&mut self, game: &str, path: &RegistryItem) {
        let keys: Vec<_> = self
            .0
            .get(game)
            .map(|x| x.keys().cloned().collect())
            .unwrap_or_default();
        for key in keys {
            if path.is_prefix_of(&key) {
                self.0.get_mut(game).map(|entry| entry.remove(&key));
            }
        }
    }

    pub fn toggle_owned(&mut self, game: &str, path: &RegistryItem, value: Option<String>) {
        match value {
            Some(value) => self.toggle(game, path, Some(value.as_str())),
            None => self.toggle(game, path, None),
        }
    }

    pub fn toggle(&mut self, game: &str, path: &RegistryItem, value: Option<&str>) {
        let transitive = self.is_enabled_transitively(game, path);
        let specific = self.is_key_enabled_specifically(game, path);

        if value.is_some() {
            let by_value = value.and_then(|value| self.is_value_enabled_specifically(game, path, value));
            match (transitive, specific) {
                (None, None) => {
                    if by_value == Some(false) {
                        self.remove(game, path, value);
                    } else {
                        self.set_enabled(game, path, value, false);
                    }
                }
                (_, Some(inherited)) | (Some(inherited), None) => match by_value {
                    Some(own) if own != inherited => {
                        self.remove(game, path, value);
                    }
                    _ => {
                        self.set_enabled(game, path, value, !inherited);
                    }
                },
            }
            self.prune(game, path);
            return;
        }

        match (transitive, specific) {
            (None, None | Some(true)) => {
                self.set_enabled(game, path, value, false);
            }
            (None, Some(false)) => {
                self.remove(game, path, value);
            }
            (Some(x), None) => {
                self.set_enabled(game, path, value, !x);
            }
            (Some(x), Some(y)) if x == y => {
                self.set_enabled(game, path, value, !x);
            }
            (Some(_), Some(_)) => {
                self.remove(game, path, value);
            }
        }

        self.prune(game, path);
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, hash_set};

    use super::*;
    use crate::testing::s;

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
            apps:
              rclone:
                path: "rclone"
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                runtime: Default::default(),
                manifest: ManifestConfig {
                    url: s("example.com"),
                    secondary: vec![]
                },
                language: Language::English,
                theme: Theme::Light,
                roots: vec![],
                redirects: vec![],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: HashSet::new(),
                    filter: BackupFilter {
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
                    ignored_games: HashSet::new(),
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                },
                scan: Default::default(),
                apps: Apps {
                    rclone: App {
                        path: StrictPath::new("rclone".to_string()),
                        ..Default::default()
                    }
                },
                custom_games: vec![],
                ..Default::default()
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
              secondary:
                - url: example.com/2
            roots:
              - path: ~/steam
                store: steam
              - path: ~/other
                store: other
            redirects:
              - kind: restore
                source: ~/old
                target: ~/new
            backup:
              path: ~/backup
              ignoredGames:
                - Backup Game 1
                - Backup Game 2
                - Backup Game 2
              filter:
                excludeStoreScreenshots: true
            restore:
              path: ~/restore
              ignoredGames:
                - Restore Game 1
                - Restore Game 2
                - Restore Game 2
            scan:
              showDeselectedGames: false
              showUnchangedGames: false
              showUnscannedGames: false
            cloud:
              remote:
                GoogleDrive:
                  id: remote-id
              path: ludusavi-backup
              synchronize: false
            apps:
              rclone:
                path: rclone.exe
                arguments: ""
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
                runtime: Default::default(),
                manifest: ManifestConfig {
                    url: s("example.com"),
                    secondary: vec![SecondaryManifestConfig::Remote {
                        url: s("example.com/2")
                    }]
                },
                language: Language::English,
                theme: Theme::Light,
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
                redirects: vec![RedirectConfig {
                    kind: RedirectKind::Restore,
                    source: StrictPath::new(s("~/old")),
                    target: StrictPath::new(s("~/new")),
                }],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: hash_set! {
                        s("Backup Game 1"),
                        s("Backup Game 2"),
                    },
                    filter: BackupFilter {
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
                    ignored_games: hash_set! {
                        s("Restore Game 1"),
                        s("Restore Game 2"),
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                },
                scan: Scan {
                    show_deselected_games: false,
                    show_unchanged_games: false,
                    show_unscanned_games: false,
                },
                cloud: Cloud {
                    remote: Some(Remote::GoogleDrive {
                        id: "remote-id".to_string()
                    }),
                    path: "ludusavi-backup".to_string(),
                    synchronize: false,
                },
                apps: Apps {
                    rclone: App {
                        path: StrictPath::new("rclone.exe".to_string()),
                        arguments: "".to_string(),
                    },
                },
                custom_games: vec![
                    CustomGame {
                        name: s("Custom Game 1"),
                        ignore: false,
                        alias: None,
                        prefer_alias: false,
                        files: vec![],
                        registry: vec![],
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        ignore: false,
                        alias: None,
                        prefer_alias: false,
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
            apps:
              rclone:
                path: "rclone"
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                runtime: Default::default(),
                manifest: ManifestConfig {
                    url: s("example.com"),
                    secondary: vec![]
                },
                language: Language::English,
                theme: Theme::Light,
                roots: vec![RootsConfig {
                    path: StrictPath::new(s("~/other")),
                    store: Store::Other,
                }],
                redirects: vec![],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: HashSet::new(),
                    filter: BackupFilter {
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
                    ignored_games: HashSet::new(),
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                },
                scan: Default::default(),
                apps: Apps {
                    rclone: App {
                        path: StrictPath::new("rclone".to_string()),
                        ..Default::default()
                    },
                },
                custom_games: vec![],
                ..Default::default()
            },
            config,
        );
    }

    #[test]
    fn can_be_serialized() {
        assert_eq!(
            r#"
---
runtime:
  threads: ~
manifest:
  url: example.com
language: en-US
theme: light
roots:
  - path: ~/steam
    store: steam
  - path: ~/other
    store: other
redirects:
  - kind: restore
    source: ~/old
    target: ~/new
backup:
  path: ~/backup
  ignoredGames:
    - Backup Game 1
    - Backup Game 2
    - Backup Game 3
  filter:
    excludeStoreScreenshots: true
    ignoredPaths: []
    ignoredRegistry: []
  toggledPaths: {}
  toggledRegistry: {}
  sort:
    key: status
    reversed: false
  retention:
    full: 1
    differential: 0
  format:
    chosen: simple
    zip:
      compression: deflate
    compression:
      deflate:
        level: 6
      bzip2:
        level: 6
      zstd:
        level: 10
restore:
  path: ~/restore
  ignoredGames:
    - Restore Game 1
    - Restore Game 2
    - Restore Game 3
  toggledPaths: {}
  toggledRegistry: {}
  sort:
    key: status
    reversed: false
scan:
  showDeselectedGames: false
  showUnchangedGames: false
  showUnscannedGames: false
cloud:
  remote:
    GoogleDrive:
      id: remote-id
  path: ludusavi-backup
  synchronize: true
apps:
  rclone:
    path: rclone.exe
    arguments: ""
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
  - name: Alias
    alias: Other
    files: []
    registry: []
"#
            .trim(),
            serde_yaml::to_string(&Config {
                runtime: Default::default(),
                manifest: ManifestConfig {
                    url: s("example.com"),
                    secondary: vec![]
                },
                language: Language::English,
                theme: Theme::Light,
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
                redirects: vec![RedirectConfig {
                    kind: RedirectKind::Restore,
                    source: StrictPath::new(s("~/old")),
                    target: StrictPath::new(s("~/new")),
                }],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: hash_set! {
                        s("Backup Game 3"),
                        s("Backup Game 1"),
                        s("Backup Game 2"),
                    },
                    filter: BackupFilter {
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
                    ignored_games: hash_set! {
                        s("Restore Game 3"),
                        s("Restore Game 1"),
                        s("Restore Game 2"),
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                },
                scan: Scan {
                    show_deselected_games: false,
                    show_unchanged_games: false,
                    show_unscanned_games: false,
                },
                cloud: Cloud {
                    remote: Some(Remote::GoogleDrive {
                        id: "remote-id".to_string()
                    }),
                    path: "ludusavi-backup".to_string(),
                    synchronize: true,
                },
                apps: Apps {
                    rclone: App {
                        path: StrictPath::new("rclone.exe".to_string()),
                        arguments: "".to_string(),
                    }
                },
                custom_games: vec![
                    CustomGame {
                        name: s("Custom Game 1"),
                        ignore: false,
                        alias: None,
                        prefer_alias: false,
                        files: vec![],
                        registry: vec![],
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        ignore: false,
                        alias: None,
                        prefer_alias: false,
                        files: vec![s("Custom File 1"), s("Custom File 2"), s("Custom File 2"),],
                        registry: vec![s("Custom Registry 1"), s("Custom Registry 2"), s("Custom Registry 2"),],
                    },
                    CustomGame {
                        name: s("Alias"),
                        ignore: false,
                        alias: Some("Other".to_string()),
                        prefer_alias: false,
                        files: vec![],
                        registry: vec![],
                    },
                ],
            })
            .unwrap()
            .trim(),
        );
    }

    mod ignored_paths {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::testing::repo;

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
                ToggledPaths(btree_map! {
                    s("game"): btree_map! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")): false,
                    }
                }),
            );
        }

        #[test]
        fn transitively_unset_and_specifically_enabled() {
            verify_toggle_registry_sequential(
                ToggledPaths(btree_map! {
                    s("game"): btree_map! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")): true,
                    }
                }),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                true,
                vec![
                    ToggledPaths(btree_map! {
                        s("game"): btree_map! {
                            StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")): false,
                        }
                    }),
                    ToggledPaths::default(),
                ],
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_unset_or_enabled() {
            verify_toggle_registry_bouncing(
                ToggledPaths(btree_map! {
                    s("game"): btree_map! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir")): false,
                    }
                }),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                false,
                ToggledPaths(btree_map! {
                    s("game"): btree_map! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir")): false,
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")): true,
                    }
                }),
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_disabled() {
            verify_toggle_registry_sequential(
                ToggledPaths(btree_map! {
                    s("game"): btree_map! {
                        StrictPath::new(repo_path("tests/root1/game1/subdir")): false,
                        StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")): false,
                    }
                }),
                &repo_path("tests/root1/game1/subdir/file2.txt"),
                false,
                vec![
                    ToggledPaths(btree_map! {
                        s("game"): btree_map! {
                            StrictPath::new(repo_path("tests/root1/game1/subdir")): false,
                            StrictPath::new(repo_path("tests/root1/game1/subdir/file2.txt")): true,
                        }
                    }),
                    ToggledPaths(btree_map! {
                        s("game"): btree_map! {
                            StrictPath::new(repo_path("tests/root1/game1/subdir")): false,
                        }
                    }),
                ],
            );
        }
    }

    mod ignored_registry {
        use pretty_assertions::assert_eq;

        use super::*;

        fn verify_toggle_registry_bouncing(
            mut toggled: ToggledRegistry,
            path: &str,
            value: Option<&str>,
            initial: bool,
            after: ToggledRegistry,
        ) {
            let untoggled = toggled.clone();

            let path = RegistryItem::new(path.to_string());
            assert_eq!(initial, !toggled.is_ignored("game", &path, value));

            toggled.toggle("game", &path, value);
            assert_eq!(!initial, !toggled.is_ignored("game", &path, value));
            assert_eq!(after, toggled);

            toggled.toggle("game", &path, value);
            assert_eq!(initial, !toggled.is_ignored("game", &path, value));
            assert_eq!(untoggled, toggled);
        }

        fn verify_toggle_registry_sequential(
            mut toggled: ToggledRegistry,
            path: &str,
            value: Option<&str>,
            initial: bool,
            states: Vec<ToggledRegistry>,
        ) {
            let path = RegistryItem::new(path.to_string());
            assert_eq!(initial, !toggled.is_ignored("game", &path, value));

            let mut enabled = initial;
            for state in states {
                enabled = !enabled;
                toggled.toggle("game", &path, value);
                assert_eq!(enabled, !toggled.is_ignored("game", &path, value));
                assert_eq!(state, toggled);
            }
        }

        #[test]
        fn transitively_unset_and_specifically_unset_or_disabled() {
            verify_toggle_registry_bouncing(
                ToggledRegistry::default(),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                None,
                true,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(false),
                    }
                }),
            );
        }

        #[test]
        fn transitively_unset_and_specifically_enabled() {
            verify_toggle_registry_sequential(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(true),
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                None,
                true,
                vec![
                    ToggledRegistry(btree_map! {
                        s("game"): btree_map! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(false),
                        }
                    }),
                    ToggledRegistry::default(),
                ],
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_unset_or_enabled() {
            verify_toggle_registry_bouncing(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software")): ToggledRegistryEntry::Key(false),
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                None,
                false,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software")): ToggledRegistryEntry::Key(false),
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(true),
                    }
                }),
            );
        }

        #[test]
        fn transitively_disabled_and_specifically_disabled() {
            verify_toggle_registry_sequential(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software")): ToggledRegistryEntry::Key(false),
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(false),
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                None,
                false,
                vec![
                    ToggledRegistry(btree_map! {
                        s("game"): btree_map! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software")): ToggledRegistryEntry::Key(false),
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(true),
                        }
                    }),
                    ToggledRegistry(btree_map! {
                        s("game"): btree_map! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software")): ToggledRegistryEntry::Key(false),
                        }
                    }),
                ],
            );
        }

        #[test]
        fn value_is_unset_and_without_inheritance() {
            verify_toggle_registry_bouncing(
                ToggledRegistry::default(),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                Some("qword"),
                true,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Complex {
                            key: None,
                            values: btree_map! {
                                s("qword"): false,
                            },
                        },
                    }
                }),
            );
        }

        #[test]
        fn value_is_unset_and_inherits_specifically() {
            verify_toggle_registry_bouncing(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(false)
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                Some("qword"),
                false,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Complex {
                            key: Some(false),
                            values: btree_map! {
                                s("qword"): true,
                            },
                        },
                    }
                }),
            );
            verify_toggle_registry_bouncing(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(true)
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi",
                Some("qword"),
                true,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Complex {
                            key: Some(true),
                            values: btree_map! {
                                s("qword"): false,
                            },
                        },
                    }
                }),
            );
        }

        #[test]
        fn value_is_unset_and_inherits_transitively() {
            verify_toggle_registry_bouncing(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(false)
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi/other",
                Some("qword"),
                false,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(false),
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi/other")): ToggledRegistryEntry::Complex {
                            key: None,
                            values: btree_map! {
                                s("qword"): true,
                            },
                        },
                    }
                }),
            );
            verify_toggle_registry_bouncing(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(true)
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi/other",
                Some("qword"),
                true,
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi")): ToggledRegistryEntry::Key(true),
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi/other")): ToggledRegistryEntry::Complex {
                            key: None,
                            values: btree_map! {
                                s("qword"): false,
                            },
                        },
                    }
                }),
            );
        }

        #[test]
        fn value_is_set() {
            verify_toggle_registry_bouncing(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi/other")): ToggledRegistryEntry::Complex {
                            key: None,
                            values: btree_map! {
                                s("qword"): false,
                            },
                        }
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi/other",
                Some("qword"),
                false,
                ToggledRegistry::default(),
            );

            verify_toggle_registry_sequential(
                ToggledRegistry(btree_map! {
                    s("game"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi/other")): ToggledRegistryEntry::Complex {
                            key: None,
                            values: btree_map! {
                                s("qword"): true,
                            },
                        }
                    }
                }),
                "HKEY_CURRENT_USER/Software/Ludusavi/other",
                Some("qword"),
                true,
                vec![
                    ToggledRegistry(btree_map! {
                        s("game"): btree_map! {
                            RegistryItem::new(s("HKEY_CURRENT_USER/Software/Ludusavi/other")): ToggledRegistryEntry::Complex {
                                key: None,
                                values: btree_map! {
                                    s("qword"): false,
                                },
                            },
                        }
                    }),
                    ToggledRegistry::default(),
                ],
            );
        }
    }
}
