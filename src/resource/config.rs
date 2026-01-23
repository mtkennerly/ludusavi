pub mod root;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crate::{
    cloud::Remote,
    lang::{Language, TRANSLATOR},
    path::CommonPath,
    prelude::{app_dir, EditAction, Error, RedirectEditActionField, Security, StrictPath, AVAILABLE_PARALELLISM},
    resource::{
        manifest::{self, CloudMetadata, Manifest, Store},
        ResourceFile, SaveableResourceFile,
    },
    scan::{registry::RegistryItem, ScanKind},
};

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

fn default_backup_dir() -> StrictPath {
    StrictPath::new(format!("{}/ludusavi-backup", CommonPath::Home.get().unwrap())).rendered()
}

#[derive(Debug, Clone)]
pub enum Event {
    Theme(Theme),
    Language(Language),
    CheckRelease(bool),
    BackupTarget(String),
    RestoreSource(String),
    Root(EditAction),
    RootLutrisDatabase(usize, String),
    SecondaryManifest(EditAction),
    RootStore(usize, Store),
    RedirectKind(usize, RedirectKind),
    SecondaryManifestKind(usize, SecondaryManifestConfigKind),
    CustomGameKind(usize, CustomGameKind),
    CustomGameIntegration(usize, Integration),
    Redirect(EditAction, Option<RedirectEditActionField>),
    ReverseRedirectsOnRestore(bool),
    CustomGame(EditAction),
    CustomGameAlias(usize, String),
    CustomGaleAliasDisplay(usize, bool),
    CustomGameFile(usize, EditAction),
    CustomGameRegistry(usize, EditAction),
    CustomGameInstallDir(usize, EditAction),
    CustomGameWinePrefix(usize, EditAction),
    ExcludeStoreScreenshots(bool),
    CloudFilter(CloudFilter),
    BackupFilterIgnoredPath(EditAction),
    BackupFilterIgnoredRegistry(EditAction),
    GameListEntryEnabled {
        name: String,
        enabled: bool,
        scan_kind: ScanKind,
    },
    ToggleSpecificGamePathIgnored {
        name: String,
        path: StrictPath,
        scan_kind: ScanKind,
    },
    ToggleSpecificGameRegistryIgnored {
        name: String,
        path: RegistryItem,
        value: Option<String>,
        scan_kind: ScanKind,
    },
    CustomGameEnabled {
        index: usize,
        enabled: bool,
    },
    PrimaryManifestEnabled {
        enabled: bool,
    },
    SecondaryManifestEnabled {
        index: usize,
        enabled: bool,
    },
    SortKey(SortKey),
    SortReversed(bool),
    FullRetention(u8),
    DiffRetention(u8),
    BackupFormat(BackupFormat),
    BackupCompression(ZipCompression),
    CompressionLevel(i32),
    ToggleCloudSynchronize,
    ShowDeselectedGames(bool),
    ShowUnchangedGames(bool),
    ShowUnscannedGames(bool),
    OverrideMaxThreads(bool),
    MaxThreads(usize),
    RcloneExecutable(String),
    RcloneArguments(String),
    CloudRemoteId(String),
    CloudPath(String),
    SortCustomGames,
    OnlyConstructiveBackups(bool),
}

/// Settings for `config.yaml`
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub runtime: Runtime,
    pub release: Release,
    pub manifest: ManifestConfig,
    pub language: Language,
    pub theme: Theme,
    pub roots: Vec<Root>,
    pub redirects: Vec<RedirectConfig>,
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
    pub scan: Scan,
    pub cloud: Cloud,
    pub apps: Apps,
    pub custom_games: Vec<CustomGame>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Runtime {
    /// How many threads to use for parallel scanning.
    pub threads: Option<NonZeroUsize>,
    /// Control certificate and hostname validation when performing downloads.
    pub network_security: Security,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Release {
    /// Whether to check for new releases.
    /// If enabled, Ludusavi will check at most once every 24 hours.
    pub check: bool,
}

impl Default for Release {
    fn default() -> Self {
        Self { check: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ManifestConfig {
    /// Where to download the primary manifest.
    /// Default: https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub enable: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secondary: Vec<SecondaryManifestConfig>,
}

impl ManifestConfig {
    pub fn url(&self) -> &str {
        self.url.as_deref().unwrap_or(MANIFEST_URL)
    }

    pub fn secondary_manifest_urls(&self, force: bool) -> Vec<&str> {
        self.secondary
            .iter()
            .filter_map(|x| match x {
                SecondaryManifestConfig::Local { .. } => None,
                SecondaryManifestConfig::Remote { url, enable } => (*enable || force).then_some(url.as_str()),
            })
            .collect()
    }

    pub fn load_secondary_manifests(&self) -> Vec<manifest::Secondary> {
        self.secondary
            .iter()
            .filter_map(|x| match x {
                SecondaryManifestConfig::Local { path, enable } => {
                    if !enable {
                        return None;
                    }

                    let manifest = Manifest::load_from_existing(path);
                    if let Err(e) = &manifest {
                        log::error!("Cannot load secondary manifest: {:?} | {}", &path, e);
                    }
                    Some(manifest::Secondary {
                        id: path.render(),
                        path: path.clone(),
                        data: manifest.ok()?,
                    })
                }
                SecondaryManifestConfig::Remote { url, enable } => {
                    if !enable {
                        return None;
                    }

                    let path = Manifest::path_for(url, false);
                    let manifest = Manifest::load_from(&path);
                    if let Err(e) = &manifest {
                        log::error!("Cannot load manifest: {:?} | {}", &path, e);
                    }
                    Some(manifest::Secondary {
                        id: url.to_string(),
                        path: path.clone(),
                        data: manifest.ok()?,
                    })
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SecondaryManifestConfig {
    Local {
        path: StrictPath,
        #[serde(default = "crate::serialization::default_true")]
        enable: bool,
    },
    Remote {
        url: String,
        #[serde(default = "crate::serialization::default_true")]
        enable: bool,
    },
}

impl SecondaryManifestConfig {
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::Remote { url, .. } => Some(url.as_str()),
        }
    }

    pub fn path(&self) -> Option<&StrictPath> {
        match self {
            Self::Local { path, .. } => Some(path),
            Self::Remote { .. } => None,
        }
    }

    pub fn value(&self) -> String {
        match self {
            Self::Local { path, .. } => path.raw().into(),
            Self::Remote { url, .. } => url.to_string(),
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Local { enable, .. } => *enable,
            Self::Remote { enable, .. } => *enable,
        }
    }

    pub fn set(&mut self, value: String) {
        match self {
            Self::Local { path, .. } => *path = StrictPath::new(value),
            Self::Remote { url, .. } => *url = value,
        }
    }

    pub fn enable(&mut self, enabled: bool) {
        match self {
            Self::Local { enable, .. } => *enable = enabled,
            Self::Remote { enable, .. } => *enable = enabled,
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
            (Self::Local { path, enable }, SecondaryManifestConfigKind::Remote) => {
                *self = Self::Remote {
                    url: path.raw().into(),
                    enable: *enable,
                };
            }
            (Self::Remote { url, enable }, SecondaryManifestConfigKind::Local) => {
                *self = Self::Local {
                    path: StrictPath::new(url.clone()),
                    enable: *enable,
                };
            }
            _ => {}
        }
    }
}

impl Default for SecondaryManifestConfig {
    fn default() -> Self {
        Self::Remote {
            url: "".to_string(),
            enable: true,
        }
    }
}

/// Visual theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    Light,
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

#[derive(
    Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "store", rename_all = "camelCase")]
pub enum Root {
    Ea(root::Ea),
    Epic(root::Epic),
    Gog(root::Gog),
    GogGalaxy(root::GogGalaxy),
    Heroic(root::Heroic),
    Legendary(root::Legendary),
    Lutris(root::Lutris),
    Microsoft(root::Microsoft),
    Origin(root::Origin),
    Prime(root::Prime),
    Steam(root::Steam),
    Uplay(root::Uplay),
    OtherHome(root::OtherHome),
    OtherWine(root::OtherWine),
    OtherWindows(root::OtherWindows),
    OtherLinux(root::OtherLinux),
    OtherMac(root::OtherMac),
    Other(root::Other),
}

impl Default for Root {
    fn default() -> Self {
        Self::Other(Default::default())
    }
}

impl Root {
    pub fn new(path: impl Into<StrictPath>, store: Store) -> Self {
        match store {
            Store::Ea => Self::Ea(root::Ea { path: path.into() }),
            Store::Epic => Self::Epic(root::Epic { path: path.into() }),
            Store::Gog => Self::Gog(root::Gog { path: path.into() }),
            Store::GogGalaxy => Self::GogGalaxy(root::GogGalaxy { path: path.into() }),
            Store::Heroic => Self::Heroic(root::Heroic { path: path.into() }),
            Store::Legendary => Self::Legendary(root::Legendary { path: path.into() }),
            Store::Lutris => Self::Lutris(root::Lutris {
                path: path.into(),
                database: None,
            }),
            Store::Microsoft => Self::Microsoft(root::Microsoft { path: path.into() }),
            Store::Origin => Self::Origin(root::Origin { path: path.into() }),
            Store::Prime => Self::Prime(root::Prime { path: path.into() }),
            Store::Steam => Self::Steam(root::Steam { path: path.into() }),
            Store::Uplay => Self::Uplay(root::Uplay { path: path.into() }),
            Store::OtherHome => Self::OtherHome(root::OtherHome { path: path.into() }),
            Store::OtherWine => Self::OtherWine(root::OtherWine { path: path.into() }),
            Store::OtherWindows => Self::OtherWindows(root::OtherWindows { path: path.into() }),
            Store::OtherLinux => Self::OtherLinux(root::OtherLinux { path: path.into() }),
            Store::OtherMac => Self::OtherMac(root::OtherMac { path: path.into() }),
            Store::Other => Self::Other(root::Other { path: path.into() }),
        }
    }

    pub fn store(&self) -> Store {
        match self {
            Self::Ea(_) => Store::Ea,
            Self::Epic(_) => Store::Epic,
            Self::Gog(_) => Store::Gog,
            Self::GogGalaxy(_) => Store::GogGalaxy,
            Self::Heroic(_) => Store::Heroic,
            Self::Legendary(_) => Store::Legendary,
            Self::Lutris(_) => Store::Lutris,
            Self::Microsoft(_) => Store::Microsoft,
            Self::Origin(_) => Store::Origin,
            Self::Prime(_) => Store::Prime,
            Self::Steam(_) => Store::Steam,
            Self::Uplay(_) => Store::Uplay,
            Self::OtherHome(_) => Store::OtherHome,
            Self::OtherWine(_) => Store::OtherWine,
            Self::OtherWindows(_) => Store::OtherWindows,
            Self::OtherLinux(_) => Store::OtherLinux,
            Self::OtherMac(_) => Store::OtherMac,
            Self::Other(_) => Store::Other,
        }
    }

    pub fn path(&self) -> &StrictPath {
        match self {
            Self::Ea(root::Ea { path }) => path,
            Self::Epic(root::Epic { path }) => path,
            Self::Gog(root::Gog { path }) => path,
            Self::GogGalaxy(root::GogGalaxy { path }) => path,
            Self::Heroic(root::Heroic { path }) => path,
            Self::Legendary(root::Legendary { path }) => path,
            Self::Lutris(root::Lutris { path, .. }) => path,
            Self::Microsoft(root::Microsoft { path }) => path,
            Self::Origin(root::Origin { path }) => path,
            Self::Prime(root::Prime { path }) => path,
            Self::Steam(root::Steam { path }) => path,
            Self::Uplay(root::Uplay { path }) => path,
            Self::OtherHome(root::OtherHome { path }) => path,
            Self::OtherWine(root::OtherWine { path }) => path,
            Self::OtherWindows(root::OtherWindows { path }) => path,
            Self::OtherLinux(root::OtherLinux { path }) => path,
            Self::OtherMac(root::OtherMac { path }) => path,
            Self::Other(root::Other { path }) => path,
        }
    }

    pub fn path_mut(&mut self) -> &mut StrictPath {
        match self {
            Self::Ea(root::Ea { path }) => path,
            Self::Epic(root::Epic { path }) => path,
            Self::Gog(root::Gog { path }) => path,
            Self::GogGalaxy(root::GogGalaxy { path }) => path,
            Self::Heroic(root::Heroic { path }) => path,
            Self::Legendary(root::Legendary { path }) => path,
            Self::Lutris(root::Lutris { path, .. }) => path,
            Self::Microsoft(root::Microsoft { path }) => path,
            Self::Origin(root::Origin { path }) => path,
            Self::Prime(root::Prime { path }) => path,
            Self::Steam(root::Steam { path }) => path,
            Self::Uplay(root::Uplay { path }) => path,
            Self::OtherHome(root::OtherHome { path }) => path,
            Self::OtherWine(root::OtherWine { path }) => path,
            Self::OtherWindows(root::OtherWindows { path }) => path,
            Self::OtherLinux(root::OtherLinux { path }) => path,
            Self::OtherMac(root::OtherMac { path }) => path,
            Self::Other(root::Other { path }) => path,
        }
    }

    pub fn with_path(&self, path: StrictPath) -> Self {
        match self {
            Self::Lutris(root::Lutris { database, .. }) => Self::Lutris(root::Lutris {
                path,
                database: database.clone(),
            }),
            _ => Self::new(path, self.store()),
        }
    }

    pub fn games_path(&self) -> StrictPath {
        match self.store() {
            Store::Steam => self.path().joined("steamapps/common"),
            _ => self.path().clone(),
        }
    }

    pub fn lutris_database(&self) -> Option<&StrictPath> {
        match self {
            Self::Lutris(root) => root.database.as_ref(),
            _ => None,
        }
    }

    pub fn set_store(&mut self, store: Store) {
        if self.store() != store {
            *self = Self::new(self.path().clone(), store);
        }
    }

    pub fn glob(&self) -> Vec<Self> {
        self.path()
            .glob()
            .into_iter()
            .map(|path| self.with_path(path))
            .collect()
    }

    pub fn find_secondary_manifests(&self) -> HashMap<StrictPath, Manifest> {
        self.path()
            .joined(match self.store() {
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

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct RedirectConfig {
    /// When and how to apply the redirect.
    pub kind: RedirectKind,
    /// The original location when the backup was performed.
    pub source: StrictPath,
    /// The new location.
    pub target: StrictPath,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RedirectKind {
    Backup,
    #[default]
    Restore,
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

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct CloudFilter {
    /// If true, don't back up games with cloud support
    /// on the stores indicated in the other options here.
    pub exclude: bool,
    /// If this and `exclude` are true, don't back up games with cloud support on Epic.
    pub epic: bool,
    /// If this and `exclude` are true, don't back up games with cloud support on GOG.
    pub gog: bool,
    /// If this and `exclude` are true, don't back up games with cloud support on Origin / EA App.
    pub origin: bool,
    /// If this and `exclude` are true, don't back up games with cloud support on Steam.
    pub steam: bool,
    /// If this and `exclude` are true, don't back up games with cloud support on Uplay / Ubisoft Connect.
    pub uplay: bool,
}

impl CloudFilter {
    pub fn excludes(&self, info: &CloudMetadata) -> bool {
        let CloudFilter {
            exclude,
            epic,
            gog,
            origin,
            steam,
            uplay,
        } = self;

        if !exclude {
            return false;
        }

        (*epic && info.epic)
            || (*gog && info.gog)
            || (*origin && info.origin)
            || (*steam && info.steam)
            || (*uplay && info.uplay)
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupFilter {
    /// If true, then the backup should exclude screenshots from stores like Steam.
    pub exclude_store_screenshots: bool,
    pub cloud: CloudFilter,
    /// Globally ignored paths.
    pub ignored_paths: Vec<StrictPath>,
    /// Globally ignored registry keys.
    pub ignored_registry: Vec<RegistryItem>,
    #[serde(skip)]
    pub path_globs: Arc<Mutex<Option<globset::GlobSet>>>,
}

impl std::fmt::Debug for BackupFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupFilter")
            .field("exclude_store_screenshots", &self.exclude_store_screenshots)
            .field("cloud", &self.cloud)
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
            let normalized = item.render();

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

    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn is_registry_ignored(&self, item: &RegistryItem) -> bool {
        if self.ignored_registry.is_empty() {
            return false;
        }
        let interpreted = item.interpret();
        self.ignored_registry
            .iter()
            .any(|x| x.is_prefix_of(item) || x.interpret() == interpreted)
    }

    pub fn excludes(&self, explicit: bool, has_backup: bool, info: &CloudMetadata) -> bool {
        !explicit && self.cloud.excludes(info) && !has_backup
    }
}

/// Allows including/excluding specific file paths.
/// Each outer key is a game name,
/// and each nested key is a file path.
/// Boolean true means that a file should be included.
/// Settings on child paths override settings on parent paths.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ToggledPaths(BTreeMap<String, BTreeMap<StrictPath, bool>>);

/// Allows including/excluding specific registry keys.
/// Each outer key is a game name,
/// and each nested key is a registry key path.
/// Settings on child paths override settings on parent paths.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ToggledRegistry(BTreeMap<String, BTreeMap<RegistryItem, ToggledRegistryEntry>>);

/// Whether an individual registry key and its values should be included/excluded.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ToggledRegistryEntry {
    /// Follow default behavior.
    Unset,
    /// Control inclusion of a key and all of its values.
    Key(bool),
    /// Control inclusion of specific values.
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    Name,
    Size,
    #[default]
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

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Sort {
    /// Main sorting criteria.
    pub key: SortKey,
    /// If true, sort reverse alphabetical or from the largest size.
    pub reversed: bool,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Retention {
    /// Full backups to keep. Range: 1-255.
    pub full: u8,
    /// Differential backups to keep. Range: 0-255.
    pub differential: u8,
    #[serde(skip)]
    pub force_new_full: bool,
}

impl Retention {
    #[cfg(test)]
    pub fn new(full: u8, differential: u8) -> Self {
        Self {
            full,
            differential,
            ..Default::default()
        }
    }

    pub fn with_limits(self, full: Option<u8>, differential: Option<u8>) -> Self {
        Self {
            full: full.unwrap_or(self.full),
            differential: differential.unwrap_or(self.differential),
            ..self
        }
    }

    pub fn with_force_new_full(self, force: bool) -> Self {
        Self {
            force_new_full: force,
            ..self
        }
    }
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

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BackupFormat {
    #[default]
    Simple,
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
            _ => Err(format!("invalid backup format: {s}")),
        }
    }
}

impl ToString for BackupFormat {
    fn to_string(&self) -> String {
        TRANSLATOR.backup_format(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupFormats {
    /// Active format.
    pub chosen: BackupFormat,
    /// Settings for the zip format.
    pub zip: ZipConfig,
    /// Settings for specific compression methods.
    /// In compression levels, higher numbers are slower, but save more space.
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

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ZipConfig {
    /// Preferred compression method.
    pub compression: ZipCompression,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ZipCompression {
    None,
    #[default]
    Deflate,
    Bzip2,
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
            _ => Err(format!("invalid compression method: {s}")),
        }
    }
}

impl ToString for ZipCompression {
    fn to_string(&self) -> String {
        TRANSLATOR.backup_compression(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Compression {
    /// Preferences when using deflate compression.
    deflate: DeflateCompression,
    /// Preferences when using bzip2 compression.
    bzip2: Bzip2Compression,
    /// Preferences when using zstd compression.
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct DeflateCompression {
    /// Range: 1 to 9.
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Bzip2Compression {
    /// Range: 1 to 9.
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ZstdCompression {
    /// Range: -7 to 22.
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupConfig {
    /// Full path to a directory in which to save backups.
    pub path: StrictPath,
    /// Names of games to skip when backing up.
    pub ignored_games: BTreeSet<String>,
    pub filter: BackupFilter,
    pub toggled_paths: ToggledPaths,
    pub toggled_registry: ToggledRegistry,
    pub sort: Sort,
    pub retention: Retention,
    pub format: BackupFormats,
    /// Don't create a new backup if there are only removed saves and no new/edited ones.
    pub only_constructive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct RestoreConfig {
    /// Full path to a directory from which to restore data.
    pub path: StrictPath,
    /// Names of games to skip when restoring.
    pub ignored_games: BTreeSet<String>,
    pub toggled_paths: ToggledPaths,
    pub toggled_registry: ToggledRegistry,
    pub sort: Sort,
    pub reverse_redirects: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Scan {
    /// In the GUI, show games that have been deselected.
    pub show_deselected_games: bool,
    /// In the GUI, show games that have been scanned, but do not have any changed saves.
    pub show_unchanged_games: bool,
    /// In the GUI, show recent games that have not been scanned yet.
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Cloud {
    /// Rclone remote.
    /// You should use the GUI or the `cloud set` command to modify this,
    /// since any changes need to be synchronized with Rclone to take effect.
    pub remote: Option<Remote>,
    /// Cloud folder to use for backups.
    pub path: String,
    /// If true, upload changes automatically after backing up,
    /// as long as there aren't any conflicts.
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Apps {
    /// Settings for  Rclone.
    pub rclone: App,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            rclone: App::default_rclone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct App {
    /// Path to `rclone.exe`.
    pub path: StrictPath,
    /// Any global flags (space-separated) to include in Rclone commands.
    pub arguments: String,
}

impl App {
    pub fn is_valid(&self) -> bool {
        !self.path.raw().is_empty() && (self.path.is_file() || which::which(self.path.raw()).is_ok())
    }

    fn default_rclone() -> Self {
        Self {
            path: which::which("rclone")
                .map(|x| StrictPath::from(x).rendered())
                .unwrap_or_default(),
            arguments: "--fast-list --ignore-checksum".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomGame {
    /// Name of the game.
    pub name: String,
    /// Whether to disable this game.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ignore: bool,
    pub integration: Integration,
    /// If set to the title of another game,
    /// then when Ludusavi displays that other game,
    /// Ludusavi will display this custom game's `name` instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub prefer_alias: bool,
    /// Any files or directories you want to back up.
    pub files: Vec<String>,
    /// Any registry keys you want to back up.
    pub registry: Vec<String>,
    /// Bare folder names where the game has been installed.
    pub install_dir: Vec<String>,
    /// Any Wine prefixes that Ludusavi wouldn't be able to determine from your roots.
    pub wine_prefix: Vec<String>,
    #[serde(skip)]
    pub expanded: bool,
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

    pub fn effective_integration(&self) -> Integration {
        if self.alias.is_some() {
            Integration::Override
        } else {
            self.integration
        }
    }

    pub fn is_empty(&self) -> bool {
        let Self {
            name,
            ignore: _,
            integration: _,
            alias,
            prefer_alias: _,
            files,
            registry,
            install_dir,
            wine_prefix,
            expanded: _,
        } = self;

        name.trim().is_empty()
            && alias.is_none()
            && files.is_empty()
            && registry.is_empty()
            && install_dir.is_empty()
            && wine_prefix.is_empty()
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

#[derive(Clone, Debug, Default, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Integration {
    #[default]
    Override,
    Extend,
}

impl Integration {
    pub const ALL: &'static [Self] = &[Self::Override, Self::Extend];
}

impl ToString for Integration {
    fn to_string(&self) -> String {
        match self {
            Self::Override => TRANSLATOR.override_manifest_button(),
            Self::Extend => TRANSLATOR.extend_manifest_button(),
        }
    }
}

impl Default for ManifestConfig {
    fn default() -> Self {
        Self {
            url: None,
            enable: true,
            secondary: vec![],
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
            ignored_games: BTreeSet::new(),
            filter: BackupFilter::default(),
            toggled_paths: Default::default(),
            toggled_registry: Default::default(),
            sort: Default::default(),
            retention: Retention::default(),
            format: Default::default(),
            only_constructive: Default::default(),
        }
    }
}

impl Default for RestoreConfig {
    fn default() -> Self {
        Self {
            path: default_backup_dir(),
            ignored_games: BTreeSet::new(),
            toggled_paths: Default::default(),
            toggled_registry: Default::default(),
            sort: Default::default(),
            reverse_redirects: false,
        }
    }
}

impl ResourceFile for Config {
    const FILE_NAME: &'static str = "config.yaml";

    fn initialize(mut self) -> Self {
        self.add_common_roots();
        self.rebase_paths();
        self
    }

    fn migrate(mut self) -> Self {
        self.roots.retain(|x| !x.path().raw().trim().is_empty());
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
            item.install_dir.retain(|x| !x.trim().is_empty());
            item.wine_prefix.retain(|x| !x.trim().is_empty());
        }
        self.custom_games.retain(|x| !x.is_empty());

        if self.apps.rclone.path.raw().is_empty() {
            self.apps.rclone.path = App::default_rclone().path;
        }

        self.backup.filter.build_globs();
        self.rebase_paths();

        self
    }
}

impl SaveableResourceFile for Config {}

impl Config {
    fn file_archived_invalid() -> StrictPath {
        app_dir().joined("config.invalid.yaml")
    }

    pub fn load() -> Result<Self, Error> {
        ResourceFile::load().map_err(|e| Error::ConfigInvalid { why: format!("{e}") })
    }

    pub fn archive_invalid() -> Result<(), Box<dyn std::error::Error>> {
        Self::path().move_to(&Self::file_archived_invalid())?;
        Ok(())
    }

    pub fn find_missing_roots(&self) -> Vec<Root> {
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
            (format!("{pf32}/Steam"), Store::Steam),
            (format!("{pf64}/Steam"), Store::Steam),
            ("~/.steam/steam".to_string(), Store::Steam),
            (
                "~/.var/app/com.valvesoftware.Steam/.steam/steam".to_string(),
                Store::Steam,
            ),
            ("~/Library/Application Support/Steam".to_string(), Store::Steam),
            // Epic:
            (format!("{pf32}/Epic Games"), Store::Epic),
            (format!("{pf64}/Epic Games"), Store::Epic),
            // GOG:
            ("C:/GOG Games".to_string(), Store::Gog),
            ("~/GOG Games".to_string(), Store::Gog),
            // GOG Galaxy:
            (format!("{pf32}/GOG Galaxy/Games"), Store::GogGalaxy),
            (format!("{pf64}/GOG Galaxy/Games"), Store::GogGalaxy),
            // Heroic:
            ("~/.config/heroic".to_string(), Store::Heroic),
            (
                "~/.var/app/com.heroicgameslauncher.hgl/config/heroic".to_string(),
                Store::Heroic,
            ),
            // Uplay:
            (format!("{pf32}/Ubisoft/Ubisoft Game Launcher"), Store::Uplay),
            (format!("{pf64}/Ubisoft/Ubisoft Game Launcher"), Store::Uplay),
            // Origin:
            (format!("{pf32}/Origin Games"), Store::Origin),
            (format!("{pf64}/Origin Games"), Store::Origin),
            // Microsoft:
            (format!("{pf32}/WindowsApps"), Store::Microsoft),
            (format!("{pf64}/WindowsApps"), Store::Microsoft),
            // Prime Gaming:
            ("C:/Amazon Games/Library".to_string(), Store::Prime),
            // EA app:
            (format!("{pf32}/EA Games"), Store::Ea),
            (format!("{pf64}/EA Games"), Store::Ea),
        ];

        if let Some(data_dir) = CommonPath::Data.get() {
            candidates.push((format!("{data_dir}/heroic"), Store::Heroic));
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
                log::info!("Unable to locate Steam directory: {:?}", e);
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
            if self.roots.iter().any(|root| root.path().equivalent(&sp)) || checked.contains(&sp) {
                continue;
            }
            if sp.is_dir() {
                roots.push(Root::new(sp.rendered(), store));
            }
            checked.insert(sp);
        }

        let lutris = vec![
            ("~/.config/lutris", "~/.local/share/lutris"),
            (
                "~/.var/app/net.lutris.Lutris/config/lutris",
                "~/.var/app/net.lutris.Lutris/data/lutris",
            ),
        ];
        'lutris: for (config_dir, data_dir) in lutris {
            let config_dir = StrictPath::new(config_dir.to_string());
            let data_dir = StrictPath::new(data_dir.to_string());

            let (path, db_candidate) = 'inner: {
                for (candidate, db_candidate) in [(&config_dir, Some(&data_dir)), (&data_dir, None)] {
                    if !candidate.joined("games/*.y*ml").glob().is_empty() {
                        break 'inner (candidate.rendered(), db_candidate);
                    }
                }
                continue 'lutris;
            };

            let database = db_candidate.and_then(|candidate| {
                let candidate = candidate.joined("pga.db");
                candidate.is_file().then(|| candidate.rendered())
            });

            for root in &self.roots {
                if let Root::Lutris(stored) = root {
                    if stored.path.equivalent(&path) && (stored.database.is_some() || database.is_none()) {
                        continue 'lutris;
                    }
                }
            }

            roots.push(Root::Lutris(root::Lutris {
                path: path.clone(),
                database,
            }));
            checked.insert(path);
        }

        roots
    }

    pub fn add_common_roots(&mut self) {
        self.roots.extend(self.find_missing_roots());
    }

    pub fn merge_root(&mut self, candidate: &Root) -> Option<usize> {
        for (i, root) in self.roots.iter_mut().enumerate() {
            match (root, candidate) {
                (Root::Lutris(root), Root::Lutris(candidate)) => {
                    if root.path.equivalent(&candidate.path) && root.database.is_none() && candidate.database.is_some()
                    {
                        root.database.clone_from(&candidate.database);
                        return Some(i);
                    }
                }
                _ => continue,
            }
        }

        None
    }

    pub fn is_game_enabled_for_operation(&self, name: &str, scan_kind: ScanKind) -> bool {
        match scan_kind {
            ScanKind::Backup => self.is_game_enabled_for_backup(name),
            ScanKind::Restore => self.is_game_enabled_for_restore(name),
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

    pub fn any_saves_ignored(&self, name: &str, scan_kind: ScanKind) -> bool {
        match scan_kind {
            ScanKind::Backup => {
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
            ScanKind::Restore => {
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
            }
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
            expanded: true,
            ..Default::default()
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
        self.is_custom_game_enabled(index)
            && self.custom_games[index].kind() == CustomGameKind::Game
            && !self.custom_games[index].name.trim().is_empty()
    }

    pub fn expanded_roots(&self) -> Vec<Root> {
        for root in &self.roots {
            log::trace!(
                "Configured root: {:?} | interpreted: {:?} | exists: {} | is dir: {}",
                &root,
                root.path().interpret(),
                root.path().exists(),
                root.path().is_dir()
            );
        }

        let expanded: Vec<Root> = self.roots.iter().flat_map(|x| x.glob()).collect();

        for root in &expanded {
            log::trace!(
                "Expanded root: {:?} | interpreted: {:?} | exists: {} | is dir: {}",
                &root,
                root.path().interpret(),
                root.path().exists(),
                root.path().is_dir()
            );
        }

        expanded
    }

    pub fn should_show_game(&self, name: &str, scan_kind: ScanKind, changed: bool, scanned: bool) -> bool {
        (self.scan.show_deselected_games || self.is_game_enabled_for_operation(name, scan_kind))
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

    fn rebase_paths(&mut self) {
        let cwd = StrictPath::cwd();
        self.backup.path.rebase(&cwd);
        self.restore.path.rebase(&cwd);
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
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
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
    use velcro::{btree_map, btree_set};

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
                    url: Some(s("example.com")),
                    enable: true,
                    secondary: vec![]
                },
                language: Language::English,
                theme: Theme::Light,
                roots: vec![],
                redirects: vec![],
                backup: BackupConfig {
                    path: StrictPath::relative(s("~/backup"), Some(StrictPath::cwd().render())),
                    ignored_games: BTreeSet::new(),
                    filter: BackupFilter {
                        exclude_store_screenshots: false,
                        ..Default::default()
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    retention: Retention::default(),
                    format: Default::default(),
                    only_constructive: false,
                },
                restore: RestoreConfig {
                    path: StrictPath::relative(s("~/restore"), Some(StrictPath::cwd().render())),
                    ignored_games: BTreeSet::new(),
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    reverse_redirects: false,
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
            release:
              check: true
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
              onlyConstructive: true
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
                installDir:
                  - Custom Install Dir 1
                  - Custom Install Dir 2
                  - Custom Install Dir 2
                winePrefix:
                  - Wine Prefix 1
                  - Wine Prefix 2
                  - Wine Prefix 2
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                runtime: Default::default(),
                release: Release { check: true },
                manifest: ManifestConfig {
                    url: Some(s("example.com")),
                    enable: true,
                    secondary: vec![SecondaryManifestConfig::Remote {
                        url: s("example.com/2"),
                        enable: true,
                    }]
                },
                language: Language::English,
                theme: Theme::Light,
                roots: vec![Root::new("~/steam", Store::Steam), Root::new("~/other", Store::Other),],
                redirects: vec![RedirectConfig {
                    kind: RedirectKind::Restore,
                    source: StrictPath::new(s("~/old")),
                    target: StrictPath::new(s("~/new")),
                }],
                backup: BackupConfig {
                    path: StrictPath::relative(s("~/backup"), Some(StrictPath::cwd().render())),
                    ignored_games: btree_set! {
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
                    only_constructive: true,
                },
                restore: RestoreConfig {
                    path: StrictPath::relative(s("~/restore"), Some(StrictPath::cwd().render())),
                    ignored_games: btree_set! {
                        s("Restore Game 1"),
                        s("Restore Game 2"),
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    reverse_redirects: false,
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
                        integration: Integration::Override,
                        alias: None,
                        prefer_alias: false,
                        files: vec![],
                        registry: vec![],
                        install_dir: vec![],
                        wine_prefix: vec![],
                        expanded: false,
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        ignore: false,
                        integration: Integration::Override,
                        alias: None,
                        prefer_alias: false,
                        files: vec![s("Custom File 1"), s("Custom File 2"), s("Custom File 2")],
                        registry: vec![s("Custom Registry 1"), s("Custom Registry 2"), s("Custom Registry 2")],
                        install_dir: vec![
                            s("Custom Install Dir 1"),
                            s("Custom Install Dir 2"),
                            s("Custom Install Dir 2")
                        ],
                        wine_prefix: vec![s("Wine Prefix 1"), s("Wine Prefix 2"), s("Wine Prefix 2")],
                        expanded: false,
                    },
                ],
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
  networkSecurity: safe
release:
  check: true
manifest:
  url: example.com
  enable: true
language: en-US
theme: light
roots:
  - store: steam
    path: ~/steam
  - store: other
    path: ~/other
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
    cloud:
      exclude: false
      epic: false
      gog: false
      origin: false
      steam: false
      uplay: false
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
  onlyConstructive: false
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
  reverseRedirects: false
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
    integration: override
    files: []
    registry: []
    installDir: []
    winePrefix: []
  - name: Custom Game 2
    integration: extend
    files:
      - Custom File 1
      - Custom File 2
      - Custom File 2
    registry:
      - Custom Registry 1
      - Custom Registry 2
      - Custom Registry 2
    installDir:
      - Custom Install Dir 1
      - Custom Install Dir 2
      - Custom Install Dir 2
    winePrefix:
      - Wine Prefix 1
      - Wine Prefix 2
      - Wine Prefix 2
  - name: Alias
    integration: override
    alias: Other
    files: []
    registry: []
    installDir: []
    winePrefix: []
"#
            .trim(),
            serde_yaml::to_string(&Config {
                runtime: Default::default(),
                release: Default::default(),
                manifest: ManifestConfig {
                    url: Some(s("example.com")),
                    enable: true,
                    secondary: vec![]
                },
                language: Language::English,
                theme: Theme::Light,
                roots: vec![Root::new("~/steam", Store::Steam), Root::new("~/other", Store::Other),],
                redirects: vec![RedirectConfig {
                    kind: RedirectKind::Restore,
                    source: StrictPath::new(s("~/old")),
                    target: StrictPath::new(s("~/new")),
                }],
                backup: BackupConfig {
                    path: StrictPath::new(s("~/backup")),
                    ignored_games: btree_set! {
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
                    only_constructive: false,
                },
                restore: RestoreConfig {
                    path: StrictPath::new(s("~/restore")),
                    ignored_games: btree_set! {
                        s("Restore Game 3"),
                        s("Restore Game 1"),
                        s("Restore Game 2"),
                    },
                    toggled_paths: Default::default(),
                    toggled_registry: Default::default(),
                    sort: Default::default(),
                    reverse_redirects: false,
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
                        integration: Integration::Override,
                        alias: None,
                        prefer_alias: false,
                        files: vec![],
                        registry: vec![],
                        install_dir: vec![],
                        wine_prefix: vec![],
                        expanded: false,
                    },
                    CustomGame {
                        name: s("Custom Game 2"),
                        ignore: false,
                        integration: Integration::Extend,
                        alias: None,
                        prefer_alias: false,
                        files: vec![s("Custom File 1"), s("Custom File 2"), s("Custom File 2")],
                        registry: vec![s("Custom Registry 1"), s("Custom Registry 2"), s("Custom Registry 2")],
                        install_dir: vec![
                            s("Custom Install Dir 1"),
                            s("Custom Install Dir 2"),
                            s("Custom Install Dir 2")
                        ],
                        wine_prefix: vec![s("Wine Prefix 1"), s("Wine Prefix 2"), s("Wine Prefix 2")],
                        expanded: false,
                    },
                    CustomGame {
                        name: s("Alias"),
                        ignore: false,
                        integration: Integration::Override,
                        alias: Some("Other".to_string()),
                        prefer_alias: false,
                        files: vec![],
                        registry: vec![],
                        install_dir: vec![],
                        wine_prefix: vec![],
                        expanded: false,
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
