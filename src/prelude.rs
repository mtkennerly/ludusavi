use crate::{
    config::{BackupFilter, BackupFormats, RedirectConfig, RedirectKind, RootsConfig, ToggledPaths, ToggledRegistry},
    heroic::HeroicGames,
    layout::{Backup, BackupLayout, GameLayout, LatestBackup},
    manifest::{Game, Manifest, Os, Store},
};
use fuzzy_matcher::FuzzyMatcher;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;

pub use crate::{path::StrictPath, registry_compat::RegistryItem};

const WINDOWS: bool = cfg!(target_os = "windows");
const MAC: bool = cfg!(target_os = "macos");
const LINUX: bool = cfg!(target_os = "linux");
pub const CASE_INSENSITIVE_OS: bool = WINDOWS || MAC;
const SKIP: &str = "<skip>";
const APP_DIR_NAME: &str = "ludusavi";
const PORTABLE_FLAG_FILE_NAME: &str = "ludusavi.portable";

pub type AnyError = Box<dyn std::error::Error>;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("The manifest file is invalid: {why:?}")]
    ManifestInvalid { why: String },

    #[error("Unable to download an update to the manifest file")]
    ManifestCannotBeUpdated,

    #[error("The config file is invalid: {why:?}")]
    ConfigInvalid { why: String },

    #[error("Unrecognized games: {games:?}")]
    CliUnrecognizedGames { games: Vec<String> },

    #[error("Unable to request confirmation")]
    CliUnableToRequestConfirmation,

    #[error("Cannot specify backup ID when restoring multiple games")]
    CliBackupIdWithMultipleGames,

    #[error("Invalid backup ID")]
    CliInvalidBackupId,

    #[error("Some entries failed")]
    SomeEntriesFailed,

    #[error("Cannot prepare the backup target")]
    CannotPrepareBackupTarget { path: StrictPath },

    #[error("Cannot prepare the backup target")]
    RestorationSourceInvalid { path: StrictPath },

    #[allow(dead_code)]
    #[error("Error while working with the registry")]
    RegistryIssue,

    #[error("Unable to browse file system")]
    UnableToBrowseFileSystem,

    #[error("Unable to open directory")]
    UnableToOpenDir(StrictPath),

    #[error("Unable to open URL")]
    UnableToOpenUrl(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize)]
pub enum ScanChange {
    New,
    Different,
    Same,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize)]
pub struct ScanChangeCount {
    pub new: usize,
    pub different: usize,
    pub same: usize,
}

impl ScanChangeCount {
    pub fn new() -> Self {
        Self {
            new: 0,
            different: 0,
            same: 0,
        }
    }

    pub fn brand_new(&self) -> bool {
        self.new > 0 && self.different == 0 && self.same == 0
    }

    pub fn updated(&self) -> bool {
        !self.brand_new() && (self.new > 0 || self.different > 0)
    }

    pub fn overall(&self) -> ScanChange {
        if self.brand_new() {
            ScanChange::New
        } else if self.updated() {
            ScanChange::Different
        } else {
            ScanChange::Same
        }
    }
}

impl ScanChange {
    pub fn evaluate(current_hash: &str, previous_hash: Option<&&String>) -> Self {
        match previous_hash {
            None => Self::New,
            Some(&previous) => {
                if current_hash == previous {
                    Self::Same
                } else {
                    Self::Different
                }
            }
        }
    }

    pub fn evaluate_restore(original_path: &StrictPath, previous_hash: &str) -> Self {
        match original_path.try_sha1() {
            Err(_) => ScanChange::New,
            Ok(current_hash) => {
                if current_hash == previous_hash {
                    Self::Same
                } else {
                    Self::Different
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedFile {
    /// The actual location on disk.
    /// When `container` is set, this is the path inside of the container
    /// and should be used in its raw form.
    pub path: StrictPath,
    pub size: u64,
    pub hash: String,
    /// This is the restoration target path, without redirects applied.
    pub original_path: Option<StrictPath>,
    pub ignored: bool,
    pub change: ScanChange,
    /// An enclosing archive file, if any, depending on the `BackupFormat`.
    pub container: Option<StrictPath>,
    pub redirected: Option<StrictPath>,
}

impl ScannedFile {
    #[cfg(test)]
    pub fn new<T: AsRef<str> + ToString, H: ToString>(path: T, size: u64, hash: H) -> Self {
        Self {
            path: StrictPath::new(path.to_string()),
            size,
            hash: hash.to_string(),
            original_path: None,
            ignored: false,
            change: Default::default(),
            container: None,
            redirected: None,
        }
    }

    #[cfg(test)]
    pub fn ignored(mut self) -> Self {
        self.ignored = true;
        self
    }

    #[cfg(test)]
    pub fn change(mut self, change: ScanChange) -> Self {
        self.change = change;
        self
    }

    #[cfg(test)]
    pub fn change_new(mut self) -> Self {
        self.change = ScanChange::New;
        self
    }

    pub fn original_path(&self) -> &StrictPath {
        match &self.original_path {
            Some(x) => x,
            None => &self.path,
        }
    }

    pub fn restoring(&self) -> bool {
        self.original_path.is_some()
    }

    /// This is stored in the mapping file and used for operations.
    pub fn effective(&self) -> &StrictPath {
        self.redirected.as_ref().unwrap_or_else(|| self.original_path())
    }

    /// This is the main path to show to the user.
    pub fn readable(&self, restoring: bool) -> String {
        if restoring {
            self.redirected
                .as_ref()
                .unwrap_or_else(|| self.original_path())
                .render()
        } else {
            self.original_path().render()
        }
    }

    /// This is shown in the GUI/CLI to annotate the `readable` path.
    pub fn alt(&self, restoring: bool) -> Option<&StrictPath> {
        if restoring {
            if self.redirected.is_some() {
                Some(self.original_path())
            } else {
                None
            }
        } else {
            self.redirected.as_ref()
        }
    }

    /// This is shown in the GUI/CLI to annotate the `readable` path.
    pub fn alt_readable(&self, restoring: bool) -> Option<String> {
        self.alt(restoring).map(|x| x.render())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedRegistry {
    pub path: RegistryItem,
    pub ignored: bool,
    pub change: ScanChange,
}

#[cfg(test)]
impl ScannedRegistry {
    pub fn new<T: AsRef<str> + ToString>(path: T) -> Self {
        Self {
            path: RegistryItem::new(path.to_string()),
            ignored: false,
            change: ScanChange::Unknown,
        }
    }

    pub fn ignored(mut self) -> Self {
        self.ignored = true;
        self
    }

    pub fn change(mut self, change: ScanChange) -> Self {
        self.change = change;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanInfo {
    pub game_name: String,
    pub found_files: std::collections::HashSet<ScannedFile>,
    pub found_registry_keys: std::collections::HashSet<ScannedRegistry>,
    /// Only populated by a restoration scan.
    pub available_backups: Vec<Backup>,
    /// Only populated by a restoration scan.
    pub backup: Option<Backup>,
}

impl ScanInfo {
    pub fn sum_bytes(&self, backup_info: &Option<BackupInfo>) -> u64 {
        let successful_bytes = self
            .found_files
            .iter()
            .filter(|x| !x.ignored)
            .map(|x| x.size)
            .sum::<u64>();
        let failed_bytes = if let Some(backup_info) = &backup_info {
            backup_info.failed_files.iter().map(|x| x.size).sum::<u64>()
        } else {
            0
        };
        successful_bytes - failed_bytes
    }

    pub fn total_possible_bytes(&self) -> u64 {
        self.found_files.iter().map(|x| x.size).sum::<u64>()
    }

    pub fn found_anything(&self) -> bool {
        !self.found_files.is_empty() || !self.found_registry_keys.is_empty()
    }

    pub fn found_anything_processable(&self) -> bool {
        self.found_files.iter().any(|x| !x.ignored) || self.found_registry_keys.iter().any(|x| !x.ignored)
    }

    pub fn update_ignored(&mut self, toggled_paths: &ToggledPaths, toggled_registry: &ToggledRegistry) {
        self.found_files = self
            .found_files
            .iter()
            .map(|x| {
                let mut y = x.clone();
                y.ignored = toggled_paths.is_ignored(&self.game_name, &x.path);
                y
            })
            .collect();
        self.found_registry_keys = self
            .found_registry_keys
            .iter()
            .map(|x| {
                let mut y = x.clone();
                y.ignored = toggled_registry.is_ignored(&self.game_name, &x.path);
                y
            })
            .collect();
    }

    pub fn all_ignored(&self) -> bool {
        if self.found_files.is_empty() && self.found_registry_keys.is_empty() {
            return false;
        }
        self.found_files.iter().all(|x| x.ignored) && self.found_registry_keys.iter().all(|x| x.ignored)
    }

    pub fn any_ignored(&self) -> bool {
        self.found_files.iter().any(|x| x.ignored) || self.found_registry_keys.iter().any(|x| x.ignored)
    }

    pub fn total_items(&self) -> usize {
        self.found_files.len() + self.found_registry_keys.len()
    }

    pub fn enabled_items(&self) -> usize {
        self.found_files.iter().filter(|x| !x.ignored).count()
            + self.found_registry_keys.iter().filter(|x| !x.ignored).count()
    }

    pub fn restoring(&self) -> bool {
        self.backup.is_some()
    }

    pub fn count_changes(&self) -> ScanChangeCount {
        let mut count = ScanChangeCount::new();

        for entry in &self.found_files {
            if entry.ignored {
                continue;
            }
            match entry.change {
                ScanChange::New => count.new += 1,
                ScanChange::Different => count.different += 1,
                ScanChange::Same => count.same += 1,
                ScanChange::Unknown => (),
            }
        }
        for entry in &self.found_registry_keys {
            if entry.ignored {
                continue;
            }
            match entry.change {
                ScanChange::New => count.new += 1,
                ScanChange::Different => count.different += 1,
                ScanChange::Same => count.same += 1,
                ScanChange::Unknown => (),
            }
        }

        count
    }
}

#[derive(Clone, Debug, Default)]
pub struct BackupInfo {
    pub failed_files: std::collections::HashSet<ScannedFile>,
    pub failed_registry: std::collections::HashSet<RegistryItem>,
}

impl BackupInfo {
    pub fn successful(&self) -> bool {
        self.failed_files.is_empty() && self.failed_registry.is_empty()
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct OperationStatus {
    #[serde(rename = "totalGames")]
    pub total_games: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "processedGames")]
    pub processed_games: usize,
    #[serde(rename = "processedBytes")]
    pub processed_bytes: u64,
    #[serde(rename = "changedGames")]
    pub changed_games: ScanChangeCount,
}

impl OperationStatus {
    pub fn add_game(&mut self, scan_info: &ScanInfo, backup_info: &Option<BackupInfo>, processed: bool) {
        self.total_games += 1;
        self.total_bytes += scan_info.total_possible_bytes();
        if processed {
            self.processed_games += 1;
            self.processed_bytes += scan_info.sum_bytes(backup_info);
        }

        let changes = scan_info.count_changes();
        if changes.brand_new() {
            self.changed_games.new += 1;
        } else if changes.updated() {
            self.changed_games.different += 1;
        } else {
            self.changed_games.same += 1;
        }
    }

    pub fn processed_all_games(&self) -> bool {
        self.total_games == self.processed_games
    }

    pub fn processed_all_bytes(&self) -> bool {
        self.total_bytes == self.processed_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum OperationStepDecision {
    Processed,
    Cancelled,
    Ignored,
}

impl Default for OperationStepDecision {
    fn default() -> Self {
        Self::Processed
    }
}

pub fn app_dir() -> std::path::PathBuf {
    if let Ok(mut flag) = std::env::current_exe() {
        flag.pop();
        flag.push(PORTABLE_FLAG_FILE_NAME);
        if flag.exists() {
            flag.pop();
            return flag;
        }
    }

    let mut path = dirs::config_dir().unwrap();
    path.push(APP_DIR_NAME);
    path
}

/// Returns the effective target, if different from the original
pub fn game_file_target(
    original_target: &StrictPath,
    redirects: &[RedirectConfig],
    restoring: bool,
) -> Option<StrictPath> {
    if redirects.is_empty() {
        return None;
    }

    let mut redirected_target = original_target.render();
    for redirect in redirects {
        if redirect.source.raw().trim().is_empty() || redirect.target.raw().trim().is_empty() {
            continue;
        }
        let (source, target) = if !restoring {
            match redirect.kind {
                RedirectKind::Backup | RedirectKind::Bidirectional => {
                    (redirect.source.render(), redirect.target.render())
                }
                RedirectKind::Restore => continue,
            }
        } else {
            match redirect.kind {
                RedirectKind::Backup => continue,
                RedirectKind::Restore => (redirect.source.render(), redirect.target.render()),
                RedirectKind::Bidirectional => (redirect.target.render(), redirect.source.render()),
            }
        };
        if !source.is_empty() && !target.is_empty() && redirected_target.starts_with(&source) {
            redirected_target = redirected_target.replacen(&source, &target, 1);
        }
    }

    let redirected_target = StrictPath::new(redirected_target);
    if original_target.render() != redirected_target.render() {
        Some(redirected_target)
    } else {
        None
    }
}

pub fn get_os() -> Os {
    if LINUX {
        Os::Linux
    } else if WINDOWS {
        Os::Windows
    } else if MAC {
        Os::Mac
    } else {
        Os::Other
    }
}

fn check_path(path: Option<std::path::PathBuf>) -> String {
    path.unwrap_or_else(|| SKIP.into()).to_string_lossy().to_string()
}

fn check_windows_path(path: Option<std::path::PathBuf>) -> String {
    match get_os() {
        Os::Windows => check_path(path),
        _ => SKIP.to_string(),
    }
}

fn check_windows_path_str(path: &str) -> &str {
    match get_os() {
        Os::Windows => path,
        _ => SKIP,
    }
}

fn check_nonwindows_path(path: Option<std::path::PathBuf>) -> String {
    match get_os() {
        Os::Windows => SKIP.to_string(),
        _ => check_path(path),
    }
}

fn check_nonwindows_path_str(path: &str) -> &str {
    match get_os() {
        Os::Windows => SKIP,
        _ => path,
    }
}

/// Returns paths to check and whether they require case-sensitive matching.
pub fn parse_paths(
    path: &str,
    root: &RootsConfig,
    install_dir: &Option<String>,
    full_install_dir: &Option<&StrictPath>,
    steam_id: &Option<u32>,
    manifest_dir: &StrictPath,
) -> std::collections::HashSet<(StrictPath, Option<bool>)> {
    let mut paths = std::collections::HashSet::new();

    let install_dir = match install_dir {
        Some(d) => d,
        None => SKIP,
    };

    let root_interpreted = root.path.interpret();
    let data_dir = check_path(dirs::data_dir());
    let data_local_dir = check_path(dirs::data_local_dir());
    let config_dir = check_path(dirs::config_dir());

    paths.insert((
        path.replace("<root>", &root_interpreted)
            .replace("<game>", install_dir)
            .replace(
                "<base>",
                &match root.store {
                    Store::Steam => format!("{}/steamapps/common/{}", &root_interpreted, install_dir),
                    Store::Heroic => full_install_dir
                        .map(|x| x.interpret())
                        .unwrap_or_else(|| SKIP.to_string()),
                    _ => format!("{}/{}", &root_interpreted, install_dir),
                },
            )
            .replace(
                "<home>",
                &dirs::home_dir().unwrap_or_else(|| SKIP.into()).to_string_lossy(),
            )
            .replace("<storeUserId>", "*")
            .replace("<osUserName>", &whoami::username())
            .replace("<winAppData>", check_windows_path_str(&data_dir))
            .replace("<winLocalAppData>", check_windows_path_str(&data_local_dir))
            .replace("<winDocuments>", &check_windows_path(dirs::document_dir()))
            .replace("<winPublic>", &check_windows_path(dirs::public_dir()))
            .replace("<winProgramData>", check_windows_path_str("C:/Windows/ProgramData"))
            .replace("<winDir>", check_windows_path_str("C:/Windows"))
            .replace("<xdgData>", check_nonwindows_path_str(&data_dir))
            .replace("<xdgConfig>", check_nonwindows_path_str(&config_dir))
            .replace("<regHkcu>", SKIP)
            .replace("<regHklm>", SKIP),
        None,
    ));
    if get_os() == Os::Windows {
        let (mut virtual_store, case_sensitive) = paths.iter().next().unwrap().clone();
        for virtualized in ["Program Files (x86)", "Program Files", "Windows", "ProgramData"] {
            for separator in ['/', '\\'] {
                virtual_store = virtual_store.replace(
                    &format!("C:{}{}", separator, virtualized),
                    &format!("{}/VirtualStore/{}", &data_local_dir, virtualized),
                );
            }
        }
        paths.insert((virtual_store, case_sensitive));
    }
    if root.store == Store::Gog && get_os() == Os::Linux {
        paths.insert((
            path.replace("<game>", &format!("{}/game", install_dir))
                .replace("<base>", &format!("{}/{}/game", root.path.interpret(), install_dir)),
            None,
        ));
    }

    // NOTE.2022-10-26 - Heroic flatpak installation detection
    //
    // flatpak wiki on filesystems
    // (https://github.com/flatpak/flatpak/wiki/Filesystem) as well as
    // https://docs.flatpak.org do not seem to mention an option to relocate
    // per-app data directories.  These are by default located in
    // $HOME/.var/app/$FLATPAK_ID, so we cat detect a flatpak installed heroic
    // by looking at the `root_interpreted` and check for
    // ".var/app/com.heroicgameslauncher.hgl/config/heroic"
    if root.store == Store::Heroic
        && get_os() == Os::Linux
        && root_interpreted.ends_with(".var/app/com.heroicgameslauncher.hgl/config/heroic")
    {
        paths.insert((
            path.replace(
                "<xdgData>",
                check_nonwindows_path_str(&format!("{}/../../data", &root_interpreted)),
            )
            .replace(
                "<xdgConfig>",
                check_nonwindows_path_str(&format!("{}/../../config", &root_interpreted)),
            ),
            None,
        ));
    }
    if root.store == Store::OtherHome {
        paths.insert((
            path.replace("<root>", &root_interpreted)
                .replace("<game>", install_dir)
                .replace("<base>", &format!("{}/{}", &root_interpreted, install_dir))
                .replace("<storeUserId>", SKIP)
                .replace("<osUserName>", &whoami::username())
                .replace("<winAppData>", check_windows_path_str("<home>/AppData/Roaming"))
                .replace("<winLocalAppData>", check_windows_path_str("<home>/AppData/Local"))
                .replace("<winDocuments>", check_windows_path_str("<home>/Documents"))
                .replace("<winPublic>", &check_windows_path(dirs::public_dir()))
                .replace("<winProgramData>", check_windows_path_str("C:/Windows/ProgramData"))
                .replace("<winDir>", check_windows_path_str("C:/Windows"))
                .replace("<xdgData>", check_nonwindows_path_str("<home>/.local/share"))
                .replace("<xdgConfig>", check_nonwindows_path_str("<home>/.config"))
                .replace("<regHkcu>", SKIP)
                .replace("<regHklm>", SKIP)
                .replace("<home>", &root_interpreted),
            None,
        ));
    }
    if get_os() == Os::Linux && root.store == Store::Steam && steam_id.is_some() {
        let prefix = format!(
            "{}/steamapps/compatdata/{}/pfx/drive_c",
            &root_interpreted,
            steam_id.unwrap()
        );
        let path2 = path
            .replace("<root>", &root_interpreted)
            .replace("<game>", install_dir)
            .replace(
                "<base>",
                &format!("{}/steamapps/common/{}", &root_interpreted, install_dir),
            )
            .replace("<home>", &format!("{}/users/steamuser", prefix))
            .replace("<storeUserId>", "*")
            .replace("<osUserName>", "steamuser")
            .replace("<winPublic>", &format!("{}/users/Public", prefix))
            .replace("<winProgramData>", &format!("{}/ProgramData", prefix))
            .replace("<winDir>", &format!("{}/windows", prefix))
            .replace("<xdgData>", &check_nonwindows_path(dirs::data_dir()))
            .replace("<xdgConfig>", &check_nonwindows_path(dirs::config_dir()))
            .replace("<regHkcu>", SKIP)
            .replace("<regHklm>", SKIP);
        paths.insert((
            path2
                .replace("<winDocuments>", &format!("{}/users/steamuser/Documents", prefix))
                .replace("<winAppData>", &format!("{}/users/steamuser/AppData/Roaming", prefix))
                .replace(
                    "<winLocalAppData>",
                    &format!("{}/users/steamuser/AppData/Local", prefix),
                ),
            Some(false),
        ));
        paths.insert((
            path2
                .replace("<winDocuments>", &format!("{}/users/steamuser/My Documents", prefix))
                .replace("<winAppData>", &format!("{}/users/steamuser/Application Data", prefix))
                .replace(
                    "<winLocalAppData>",
                    &format!("{}/users/steamuser/Local Settings/Application Data", prefix),
                ),
            Some(false),
        ));
    }
    if root.store == Store::OtherWine {
        let prefix = format!("{}/drive_*", &root_interpreted);
        let path2 = path
            .replace("<root>", &root_interpreted)
            .replace("<game>", install_dir)
            .replace("<base>", &format!("{}/{}", &root_interpreted, install_dir))
            .replace("<home>", &format!("{}/users/*", prefix))
            .replace("<storeUserId>", "*")
            .replace("<osUserName>", "*")
            .replace("<winPublic>", &format!("{}/users/Public", prefix))
            .replace("<winProgramData>", &format!("{}/ProgramData", prefix))
            .replace("<winDir>", &format!("{}/windows", prefix))
            .replace("<xdgData>", &check_nonwindows_path(dirs::data_dir()))
            .replace("<xdgConfig>", &check_nonwindows_path(dirs::config_dir()))
            .replace("<regHkcu>", SKIP)
            .replace("<regHklm>", SKIP);
        paths.insert((
            path2
                .replace("<winDocuments>", &format!("{}/users/*/Documents", prefix))
                .replace("<winAppData>", &format!("{}/users/*/AppData/Roaming", prefix))
                .replace("<winLocalAppData>", &format!("{}/users/*/AppData/Local", prefix)),
            Some(false),
        ));
        paths.insert((
            path2
                .replace("<winDocuments>", &format!("{}/users/*/My Documents", prefix))
                .replace("<winAppData>", &format!("{}/users/*/Application Data", prefix))
                .replace(
                    "<winLocalAppData>",
                    &format!("{}/users/*/Local Settings/Application Data", prefix),
                ),
            Some(false),
        ));
    }

    paths
        .iter()
        .map(|(x, y)| (StrictPath::relative(x.to_string(), Some(manifest_dir.interpret())), *y))
        .collect()
}

#[derive(Clone, Default)]
pub struct InstallDirRanking(std::collections::HashMap<(RootsConfig, String), (i64, String)>);

impl InstallDirRanking {
    /// Get the installation directory for some root/game combination.
    pub fn get(&self, root: &RootsConfig, name: &str) -> Option<String> {
        self.0.get(&(root.to_owned(), name.to_owned())).and_then(|candidate| {
            if candidate.0 == i64::MAX {
                return Some(candidate.1.to_owned());
            }
            for ((other_root, other_game), (other_score, other_subdir)) in &self.0 {
                if other_root == root && other_subdir == &candidate.1 && other_score > &candidate.0 {
                    log::info!("[{name}] outranked by '{other_game}' for subdir '{other_subdir}'");
                    return None;
                }
            }
            Some(candidate.1.to_owned())
        })
    }

    pub fn scan(roots: &[RootsConfig], manifest: &crate::manifest::Manifest, subjects: &[String]) -> Self {
        let mut ranking = Self::default();
        for root in roots {
            if root.store == Store::Heroic {
                // We handle this separately in the Heroic scan.
                continue;
            }
            ranking.scan_root(root, manifest, subjects);
        }
        ranking
    }

    fn scan_root(&mut self, root: &RootsConfig, manifest: &crate::manifest::Manifest, subjects: &[String]) {
        log::debug!("ranking installations for {:?}: {}", root.store, root.path.raw());

        let install_parent = match root.store {
            Store::Steam => root.path.joined("steamapps/common"),
            _ => root.path.clone(),
        };
        let matcher = make_fuzzy_matcher();

        let actual_dirs: Vec<_> = std::fs::read_dir(install_parent.interpret())
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| match entry.file_type() {
                        Ok(ft) if ft.is_dir() => Some(entry.file_name().to_string_lossy().to_string()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let scores: Vec<_> = subjects
            .into_par_iter()
            .filter_map(|name| {
                let manifest_install_dirs: Vec<_> = manifest.0[name]
                    .install_dir
                    .as_ref()
                    .map(|x| x.keys().collect())
                    .unwrap_or_default();
                let default_install_dir = name.to_string();
                let expected_install_dirs = &[manifest_install_dirs, vec![&default_install_dir]].concat();

                let mut best: Option<(i64, &String)> = None;
                'dirs: for expected_dir in expected_install_dirs {
                    log::trace!("[{name}] looking for install dir: {expected_dir}");
                    let ideal = matcher.fuzzy_match(expected_dir, expected_dir);
                    for actual_dir in &actual_dirs {
                        let score = fuzzy_match(&matcher, expected_dir, actual_dir, &ideal);
                        if let Some(score) = score {
                            if let Some((previous, _)) = best {
                                if score > previous {
                                    log::trace!("[{name}] score {score} beats previous {previous}: {actual_dir}");
                                    best = Some((score, actual_dir));
                                }
                            } else {
                                log::trace!("[{name}] new score {score}: {actual_dir}");
                                best = Some((score, actual_dir));
                            }
                        } else {
                            log::trace!("[{name}] irrelevant: {actual_dir}");
                        }
                        if score == Some(i64::MAX) {
                            break 'dirs;
                        }
                    }
                }
                best.map(|(score, subdir)| {
                    log::debug!("[{name}] selecting subdir with score {score}: {subdir}");
                    (score, name, subdir)
                })
            })
            .collect();

        for (score, name, subdir) in scores {
            self.0
                .insert((root.clone(), name.to_owned()), (score, subdir.to_owned()));
        }
    }
}

pub fn filter_map_walkdir(e: Result<walkdir::DirEntry, walkdir::Error>) -> Option<walkdir::DirEntry> {
    if let Err(e) = &e {
        log::warn!("failed to walk: {:?} | {e:?}", e.path());
    }
    e.ok()
}

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[RootsConfig],
    manifest_dir: &StrictPath,
    heroic_games: &HeroicGames,
    steam_id: &Option<u32>,
    filter: &BackupFilter,
    wine_prefix: &Option<StrictPath>,
    ranking: &InstallDirRanking,
    ignored_paths: &ToggledPaths,
    #[allow(unused_variables)] ignored_registry: &ToggledRegistry,
    previous: Option<LatestBackup>,
    redirects: &[RedirectConfig],
) -> ScanInfo {
    log::trace!("[{name}] beginning scan for backup");

    let mut found_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = std::collections::HashSet::new();

    let mut paths_to_check = std::collections::HashSet::<(StrictPath, Option<bool>)>::new();

    // Add a dummy root for checking paths without `<root>`.
    let mut roots_to_check: Vec<RootsConfig> = vec![RootsConfig {
        path: StrictPath::new(SKIP.to_string()),
        store: Store::Other,
    }];
    roots_to_check.extend(roots.iter().cloned());

    let manifest_dir_interpreted = manifest_dir.interpret();

    // We can add this for Wine prefixes from the CLI because they're
    // typically going to be used for only one or a few games at a time.
    // For other Wine roots, it would trigger for every game.
    if let Some(wp) = wine_prefix {
        log::trace!("[{name}] adding extra Wine prefix: {}", wp.raw());
        scan_game_for_backup_add_prefix(&mut roots_to_check, &mut paths_to_check, wp, &manifest_dir_interpreted);
    }

    // handle what was found for heroic
    for root in roots {
        if let Some(wp) = heroic_games.get_prefix(root, name) {
            scan_game_for_backup_add_prefix(&mut roots_to_check, &mut paths_to_check, wp, &manifest_dir_interpreted);
        }
    }

    for root in roots_to_check {
        log::trace!(
            "[{name}] adding candidates from {:?} root: {}",
            root.store,
            root.path.raw()
        );
        if root.path.raw().trim().is_empty() {
            continue;
        }
        let root_interpreted = root.path.interpret();

        if let Some(files) = &game.files {
            let install_dir = ranking.get(&root, name);
            let full_install_dir = heroic_games.get_install_dir(&root, name);

            for raw_path in files.keys() {
                log::trace!("[{name}] parsing candidates from: {}", raw_path);
                if raw_path.trim().is_empty() {
                    continue;
                }
                let candidates = parse_paths(raw_path, &root, &install_dir, &full_install_dir, steam_id, manifest_dir);
                for (candidate, case_sensitive) in candidates {
                    log::trace!("[{name}] parsed candidate: {}", candidate.raw());
                    if candidate.raw().contains('<') {
                        // This covers `SKIP` and any other unmatched placeholders.
                        continue;
                    }
                    paths_to_check.insert((candidate, case_sensitive));
                }
            }
        }
        if root.store == Store::Steam && steam_id.is_some() {
            // Cloud saves:
            paths_to_check.insert((
                StrictPath::relative(
                    format!("{}/userdata/*/{}/remote/", root_interpreted.clone(), &steam_id.unwrap()),
                    Some(manifest_dir_interpreted.clone()),
                ),
                None,
            ));

            // Screenshots:
            if !filter.exclude_store_screenshots {
                paths_to_check.insert((
                    StrictPath::relative(
                        format!(
                            "{}/userdata/*/760/remote/{}/screenshots/*.*",
                            &root_interpreted,
                            &steam_id.unwrap()
                        ),
                        Some(manifest_dir_interpreted.clone()),
                    ),
                    None,
                ));
            }

            // Registry:
            if game.registry.is_some() {
                let prefix = format!("{}/steamapps/compatdata/{}/pfx", &root_interpreted, steam_id.unwrap());
                paths_to_check.insert((
                    StrictPath::relative(format!("{}/*.reg", prefix), Some(manifest_dir_interpreted.clone())),
                    None,
                ));
            }
        }
    }

    let previous_files: std::collections::HashMap<&StrictPath, &String> = previous
        .as_ref()
        .map(|previous| {
            previous
                .scan
                .found_files
                .iter()
                .map(|x| (x.original_path(), &x.hash))
                .collect()
        })
        .unwrap_or_default();

    for (path, case_sensitive) in paths_to_check {
        log::trace!("[{name}] checking: {}", path.raw());
        if filter.is_path_ignored(&path) {
            log::debug!("[{name}] excluded: {}", path.raw());
            continue;
        }
        let paths = match case_sensitive {
            None => path.glob(),
            Some(cs) => path.glob_case_sensitive(cs),
        };
        for p in paths {
            let p = p.rendered();
            if p.is_file() {
                if filter.is_path_ignored(&p) {
                    log::debug!("[{name}] excluded: {}", p.raw());
                    continue;
                }
                let ignored = ignored_paths.is_ignored(name, &p);
                log::debug!("[{name}] found: {}", p.raw());
                let hash = p.sha1();
                let redirected = game_file_target(&p, redirects, false);
                found_files.insert(ScannedFile {
                    change: ScanChange::evaluate(&hash, previous_files.get(redirected.as_ref().unwrap_or(&p))),
                    size: p.size(),
                    hash,
                    redirected,
                    path: p,
                    original_path: None,
                    ignored,
                    container: None,
                });
            } else if p.is_dir() {
                log::trace!("[{name}] looking for files in: {}", p.raw());
                for child in walkdir::WalkDir::new(p.as_std_path_buf())
                    .max_depth(100)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(filter_map_walkdir)
                {
                    if child.file_type().is_file() {
                        let child = StrictPath::from(&child).rendered();
                        if filter.is_path_ignored(&child) {
                            log::debug!("[{name}] excluded: {}", child.raw());
                            continue;
                        }
                        let ignored = ignored_paths.is_ignored(name, &child);
                        log::debug!("[{name}] found: {}", child.raw());
                        let hash = child.sha1();
                        let redirected = game_file_target(&child, redirects, false);
                        found_files.insert(ScannedFile {
                            change: ScanChange::evaluate(
                                &hash,
                                previous_files.get(redirected.as_ref().unwrap_or(&child)),
                            ),
                            size: child.size(),
                            hash,
                            redirected,
                            path: child,
                            original_path: None,
                            ignored,
                            container: None,
                        });
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(registry) = &game.registry {
            let previous_registry = match previous.map(|x| x.registry_content) {
                Some(Some(content)) => crate::registry::Hives::deserialize(&content),
                _ => None,
            };

            for key in registry.keys() {
                if key.trim().is_empty() {
                    continue;
                }
                log::trace!("[{name}] checking registry: {key}");
                for scanned in crate::registry::scan_registry(name, key, filter, ignored_registry, &previous_registry)
                    .unwrap_or_default()
                {
                    log::debug!("[{name}] found registry: {}", scanned.path.raw());
                    found_registry_keys.insert(scanned);
                }
            }
        }
    }

    log::trace!("[{name}] completed scan for backup");

    ScanInfo {
        game_name: name.to_string(),
        found_files,
        found_registry_keys,
        ..Default::default()
    }
}

fn scan_game_for_backup_add_prefix(
    roots_to_check: &mut Vec<RootsConfig>,
    paths_to_check: &mut std::collections::HashSet<(StrictPath, Option<bool>)>,
    wp: &StrictPath,
    manifest_dir_interpreted: &str,
) {
    roots_to_check.push(RootsConfig {
        path: wp.clone(),
        store: Store::OtherWine,
    });
    paths_to_check.insert((
        StrictPath::relative(
            format!("{}/*.reg", wp.interpret()),
            Some(manifest_dir_interpreted.to_owned()),
        ),
        None,
    ));
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackupId {
    #[default]
    Latest,
    Named(String),
}

pub fn scan_game_for_restoration(
    name: &str,
    id: &BackupId,
    layout: &mut GameLayout,
    redirects: &[RedirectConfig],
) -> ScanInfo {
    log::trace!("[{name}] beginning scan for restore");

    let mut found_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut available_backups = vec![];
    let mut backup = None;

    let id = layout.verify_id(id);

    if layout.path.is_dir() {
        layout.migrate_legacy_backup();
        found_files = layout.restorable_files(&id, true, redirects);
        available_backups = layout.restorable_backups_flattened();
        backup = layout.find_by_id_flattened(&id);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(registry_content) = layout.registry_content(&id) {
            if let Some(hives) = crate::registry::Hives::deserialize(&registry_content) {
                for (hive_name, keys) in hives.0.iter() {
                    for (key_name, entries) in keys.0.iter() {
                        found_registry_keys.insert(ScannedRegistry {
                            path: RegistryItem::new(format!("{}/{}", hive_name, key_name).replace('\\', "/")),
                            ignored: false,
                            change: match crate::registry::try_read_registry_key(hive_name, key_name) {
                                None => ScanChange::New,
                                Some(current) => {
                                    if entries == &current {
                                        ScanChange::Same
                                    } else {
                                        ScanChange::Different
                                    }
                                }
                            },
                        });
                    }
                }
            }
        }
    }

    log::trace!("[{name}] completed scan for restore");

    ScanInfo {
        game_name: name.to_string(),
        found_files,
        found_registry_keys,
        available_backups,
        backup,
    }
}

pub fn prepare_backup_target(target: &StrictPath, merge: bool) -> Result<(), Error> {
    if !merge {
        target
            .remove()
            .map_err(|_| Error::CannotPrepareBackupTarget { path: target.clone() })?;
    } else if target.exists() && !target.is_dir() {
        return Err(Error::CannotPrepareBackupTarget { path: target.clone() });
    }

    let p = target.as_std_path_buf();
    std::fs::create_dir_all(&p).map_err(|_| Error::CannotPrepareBackupTarget { path: target.clone() })?;

    Ok(())
}

fn prepare_game_backup_target(target: &StrictPath, merge: bool) -> Result<(), AnyError> {
    if !merge {
        target.unset_readonly()?;
        target.remove()?;
    } else if target.exists() && !target.is_dir() {
        return Err("must merge into existing target, but target is not a directory".into());
    }

    std::fs::create_dir_all(target.interpret())?;
    Ok(())
}

pub fn back_up_game(
    info: &ScanInfo,
    mut layout: GameLayout,
    merge: bool,
    now: &chrono::DateTime<chrono::Utc>,
    format: &BackupFormats,
) -> BackupInfo {
    log::trace!("[{}] preparing for backup", &info.game_name);

    let able_to_prepare = if info.found_anything_processable() {
        match prepare_game_backup_target(&layout.path, merge) {
            Ok(_) => true,
            Err(e) => {
                log::error!(
                    "[{}] failed to prepare backup target: {} | {e}",
                    info.game_name,
                    layout.path.raw()
                );
                false
            }
        }
    } else {
        false
    };

    if able_to_prepare {
        layout.back_up(info, now, format)
    } else {
        let mut backup_info = BackupInfo::default();

        for file in &info.found_files {
            if file.ignored {
                continue;
            }
            backup_info.failed_files.insert(file.clone());
        }
        for reg_path in &info.found_registry_keys {
            if reg_path.ignored {
                continue;
            }
            backup_info.failed_registry.insert(reg_path.path.clone());
        }

        backup_info
    }
}

#[derive(Clone, Debug, Default)]
pub struct DuplicateDetector {
    files: std::collections::HashMap<StrictPath, std::collections::HashSet<String>>,
    registry: std::collections::HashMap<RegistryItem, std::collections::HashSet<String>>,
    game_files: std::collections::HashMap<String, std::collections::HashSet<StrictPath>>,
    game_registry: std::collections::HashMap<String, std::collections::HashSet<RegistryItem>>,
    game_duplicated_items: std::collections::HashMap<String, usize>,
}

impl DuplicateDetector {
    pub fn add_game(&mut self, scan_info: &ScanInfo) -> std::collections::HashSet<String> {
        let mut stale = self.remove_game_and_refresh(&scan_info.game_name, false);
        stale.insert(scan_info.game_name.clone());

        if scan_info.found_anything() {
            for item in scan_info.found_files.iter() {
                let path = self.pick_path(item);
                if let Some(existing) = self.files.get(&path) {
                    // Len 0: No games to update counts for.
                    // Len 2+: These games already include the item in their duplicate counts.
                    if existing.len() == 1 {
                        stale.extend(existing.clone());
                    }
                }
                self.files
                    .entry(path.clone())
                    .or_insert_with(Default::default)
                    .insert(scan_info.game_name.clone());
                self.game_files
                    .entry(scan_info.game_name.clone())
                    .or_insert_with(Default::default)
                    .insert(path);
            }
            for item in scan_info.found_registry_keys.iter() {
                let path = item.path.clone();
                if let Some(existing) = self.registry.get(&path) {
                    if existing.len() == 1 {
                        stale.extend(existing.clone());
                    }
                }
                self.registry
                    .entry(path.clone())
                    .or_insert_with(Default::default)
                    .insert(scan_info.game_name.clone());
                self.game_registry
                    .entry(scan_info.game_name.clone())
                    .or_insert_with(Default::default)
                    .insert(path);
            }
        }

        for game in &stale {
            self.game_duplicated_items
                .insert(game.clone(), self.count_duplicated_items_for(game));
        }

        stale.extend(self.duplicate_games(&scan_info.game_name));
        stale.remove(&scan_info.game_name);
        stale
    }

    pub fn remove_game(&mut self, game: &str) -> std::collections::HashSet<String> {
        self.remove_game_and_refresh(game, true)
    }

    fn remove_game_and_refresh(&mut self, game: &str, refresh: bool) -> std::collections::HashSet<String> {
        let mut stale = std::collections::HashSet::new();

        self.game_duplicated_items.remove(game);

        if let Some(files) = self.game_files.remove(game) {
            for file in files {
                if let Some(games) = self.files.get_mut(&file) {
                    games.remove(game);
                    for duplicate in games.iter() {
                        stale.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry.remove(game) {
            for registry in registry_keys {
                if let Some(games) = self.registry.get_mut(&registry) {
                    games.remove(game);
                    for duplicate in games.iter() {
                        stale.insert(duplicate.clone());
                    }
                }
            }
        }

        if refresh {
            for game in &stale {
                self.game_duplicated_items
                    .insert(game.clone(), self.count_duplicated_items_for(game));
            }
        }

        stale
    }

    pub fn is_game_duplicated(&self, scan_info: &ScanInfo) -> bool {
        self.count_duplicates_for(&scan_info.game_name) > 0
    }

    fn pick_path(&self, file: &ScannedFile) -> StrictPath {
        match &file.original_path {
            Some(op) => op.clone(),
            None => file.path.clone(),
        }
    }

    pub fn file(&self, file: &ScannedFile) -> std::collections::HashSet<String> {
        match self.files.get(&self.pick_path(file)) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_file_duplicated(&self, file: &ScannedFile) -> bool {
        self.file(file).len() > 1
    }

    pub fn registry(&self, path: &RegistryItem) -> std::collections::HashSet<String> {
        match self.registry.get(path) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_registry_duplicated(&self, path: &RegistryItem) -> bool {
        self.registry(path).len() > 1
    }

    pub fn clear(&mut self) {
        self.files.clear();
        self.registry.clear();
        self.game_duplicated_items.clear();
    }

    pub fn any_duplicates(&self) -> bool {
        for item in self.game_duplicated_items.values() {
            if *item > 0 {
                return true;
            }
        }
        false
    }

    fn count_duplicated_items_for(&self, game: &str) -> usize {
        let mut tally = 0;
        for item in self.files.values() {
            if item.contains(game) && item.len() > 1 {
                tally += 1;
            }
        }
        for item in self.registry.values() {
            if item.contains(game) && item.len() > 1 {
                tally += 1;
            }
        }
        tally
    }

    pub fn count_duplicates_for(&self, game: &str) -> usize {
        self.game_duplicated_items.get(game).copied().unwrap_or_default()
    }

    pub fn duplicate_games(&self, game: &str) -> std::collections::HashSet<String> {
        let mut duplicates = std::collections::HashSet::new();

        if let Some(files) = self.game_files.get(game) {
            for file in files {
                if let Some(games) = self.files.get(file) {
                    if games.len() < 2 {
                        continue;
                    }
                    for duplicate in games.iter() {
                        duplicates.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry.get(game) {
            for registry in registry_keys {
                if let Some(games) = self.registry.get(registry) {
                    if games.len() < 2 {
                        continue;
                    }
                    for duplicate in games.iter() {
                        duplicates.insert(duplicate.clone());
                    }
                }
            }
        }

        duplicates.remove(game);
        duplicates
    }
}

fn make_fuzzy_matcher() -> fuzzy_matcher::skim::SkimMatcherV2 {
    fuzzy_matcher::skim::SkimMatcherV2::default()
        .ignore_case()
        .score_config(fuzzy_matcher::skim::SkimScoreConfig {
            penalty_case_mismatch: 0,
            ..Default::default()
        })
}

pub fn fuzzy_match(
    matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    reference: &str,
    candidate: &str,
    ideal: &Option<i64>,
) -> Option<i64> {
    if reference == candidate {
        return Some(i64::MAX);
    }

    // A space-consolidating regex would be better, but is too much of a performance hit.
    let candidate = candidate
        .replace(['_', '-'], " ")
        .replace("    ", " ")
        .replace("   ", " ")
        .replace("  ", " ");

    let actual = matcher.fuzzy_match(reference, &candidate);
    if let (Some(ideal), Some(actual)) = (ideal, actual) {
        if actual == *ideal {
            return Some(i64::MAX);
        } else if actual > (ideal / 4 * 3) {
            return Some(actual);
        }
    }
    None
}

/// This covers any edition that is clearly separated by punctuation.
static RE_EDITION_PUNCTUATED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[™®©:-] .+ edition$"#).unwrap());
/// This covers specific, known editions that are not separated by punctuation.
static RE_EDITION_KNOWN: Lazy<Regex> = Lazy::new(|| Regex::new(r#" (game of the year) edition$"#).unwrap());
/// This covers any single-word editions that are not separated by punctuation.
/// We can't assume more than one word because it may be part of the main title.
static RE_EDITION_SHORT: Lazy<Regex> = Lazy::new(|| Regex::new(r#" [^ ]+ edition$"#).unwrap());
static RE_YEAR_SUFFIX: Lazy<Regex> = Lazy::new(|| Regex::new(r#" \(\d+\)$"#).unwrap());
static RE_SYMBOLS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[™®©:-]"#).unwrap());
static RE_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r#" {2,}"#).unwrap());

pub fn normalize_title(title: &str) -> String {
    let normalized = title.to_lowercase();
    let normalized = RE_YEAR_SUFFIX.replace_all(&normalized, "");
    let normalized = RE_EDITION_PUNCTUATED.replace_all(&normalized, "");
    let normalized = RE_EDITION_KNOWN.replace_all(&normalized, "");
    let normalized = RE_EDITION_SHORT.replace_all(&normalized, "");
    let normalized = RE_SYMBOLS.replace_all(&normalized, " ");
    let normalized = RE_SPACES.replace_all(&normalized, " ");
    normalized.trim().to_string()
}

#[cfg(target_os = "windows")]
pub fn sha1(content: String) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

pub struct TitleFinder {
    all_games: std::collections::HashSet<String>,
    can_backup: std::collections::HashSet<String>,
    can_restore: std::collections::HashSet<String>,
    steam_ids: std::collections::HashMap<u32, String>,
    normalized: std::collections::HashMap<String, String>,
}

impl TitleFinder {
    pub fn new(manifest: &Manifest, layout: &BackupLayout) -> Self {
        let can_backup: std::collections::HashSet<_> = manifest.0.keys().cloned().collect();
        let can_restore: std::collections::HashSet<_> = layout.restorable_games().into_iter().collect();
        let all_games: std::collections::HashSet<_> = can_backup.union(&can_restore).cloned().collect();
        let steam_ids = manifest.map_steam_ids_to_names();
        let normalized: std::collections::HashMap<_, _> = all_games
            .iter()
            .map(|title| (normalize_title(title), title.to_owned()))
            .collect();

        Self {
            all_games,
            can_backup,
            can_restore,
            steam_ids,
            normalized,
        }
    }

    fn eligible(&self, game: &str, backup: bool, restore: bool) -> bool {
        let can_backup = self.can_backup.contains(game);
        let can_restore = self.can_restore.contains(game);

        if backup && restore {
            can_backup && can_restore
        } else if backup {
            can_backup
        } else if restore {
            can_restore
        } else {
            true
        }
    }

    pub fn find_one(
        &self,
        names: &[String],
        steam_id: &Option<u32>,
        normalized: bool,
        backup: bool,
        restore: bool,
    ) -> Option<String> {
        let found = self.find(names, steam_id, normalized, backup, restore);
        found.iter().next().map(|x| x.to_owned())
    }

    pub fn find(
        &self,
        names: &[String],
        steam_id: &Option<u32>,
        normalized: bool,
        backup: bool,
        restore: bool,
    ) -> std::collections::BTreeSet<String> {
        let mut output = std::collections::BTreeSet::new();

        if let Some(steam_id) = steam_id {
            if let Some(found) = self.steam_ids.get(steam_id) {
                if self.eligible(found, backup, restore) {
                    output.insert(found.to_owned());
                    return output;
                }
            }
        }

        for name in names {
            if self.all_games.contains(name) && self.eligible(name, backup, restore) {
                output.insert(name.to_owned());
                return output;
            }
        }

        if normalized {
            for name in names {
                if let Some(found) = self.normalized.get(&normalize_title(name)) {
                    if self.eligible(found, backup, restore) {
                        output.insert((*found).to_owned());
                        return output;
                    }
                }
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    #[cfg(target_os = "windows")]
    use crate::layout::{BackupLayout, IndividualMappingRegistry};
    use crate::{
        config::{Config, Retention},
        layout::{FullBackup, IndividualMapping, IndividualMappingFile},
        manifest::Manifest,
        testing::*,
    };
    use maplit::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn fuzzy_matching() {
        let matcher = make_fuzzy_matcher();

        for (reference, candidate, output) in vec![
            ("a", "a", Some(i64::MAX)),
            ("a", "b", None),
            ("Something", "Something", Some(i64::MAX)),
            // Too short:
            ("ab", "a", None),
            ("ab", "b", None),
            ("abc", "ab", None),
            // Long enough:
            ("abcd", "abc", Some(71)),
            ("A Fun Game", "a fun game", Some(i64::MAX)),
            ("A Fun Game", "a  fun  game", Some(i64::MAX)),
            ("A Fun Game", "AFunGame", Some(171)),
            ("A Fun Game", "A_Fun_Game", Some(i64::MAX)),
            ("A Fun Game", "A _ Fun _ Game", Some(i64::MAX)),
            ("A Fun Game", "a-fun-game", Some(i64::MAX)),
            ("A Fun Game", "a - fun - game", Some(i64::MAX)),
            ("A Fun Game", "A FUN GAME", Some(i64::MAX)),
            ("A Fun Game!", "A Fun Game", Some(219)),
            ("A Funner Game", "A Fun Game", Some(209)),
            ("A Fun Game 2", "A Fun Game", Some(219)),
        ] {
            assert_eq!(
                output,
                fuzzy_match(
                    &matcher,
                    reference,
                    candidate,
                    &matcher.fuzzy_match(reference, reference)
                )
            );
        }
    }

    #[test]
    fn can_normalize_title() {
        // capitalization
        assert_eq!("foo bar", normalize_title("foo bar"));
        assert_eq!("foo bar", normalize_title("Foo Bar"));

        // punctuated editions
        assert_eq!("foo bar", normalize_title("Foo Bar: Any Arbitrary Edition"));
        assert_eq!("foo bar", normalize_title("Foo Bar - Any Arbitrary Edition"));
        assert_eq!("foo bar", normalize_title("Foo Bar™ Any Arbitrary Edition"));
        assert_eq!("foo bar", normalize_title("Foo Bar® - Any Arbitrary Edition"));

        // special cased editions
        assert_eq!("foo bar", normalize_title("Foo Bar Game of the Year Edition"));

        // short editions
        assert_eq!("foo bar", normalize_title("Foo Bar Special Edition"));

        // year suffixes
        assert_eq!("foo bar", normalize_title("Foo Bar (2000)"));

        // symbols
        assert_eq!("foo bar", normalize_title("Foo:Bar"));
        assert_eq!("foo bar", normalize_title("Foo: Bar"));

        // spaces
        assert_eq!("foo bar", normalize_title("  Foo  Bar  "));
    }

    fn s(text: &str) -> String {
        text.to_string()
    }

    fn repo() -> String {
        env!("CARGO_MANIFEST_DIR").replace('\\', "/")
    }

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::NaiveDate::from_ymd(2000, 1, 2)
            .and_hms(3, 4, 5)
            .and_local_timezone(chrono::Utc)
            .unwrap()
    }

    fn config() -> Config {
        Config::load_from_string(&format!(
            r#"
            manifest:
              url: example.com
              etag: null
            roots:
              - path: {0}/tests/root1
                store: other
              - path: {0}/tests/root2
                store: other
            backup:
              path: ~/backup
            restore:
              path: ~/restore
            "#,
            repo()
        ))
        .unwrap()
    }

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            game1:
              files:
                <base>/file1.txt: {}
                <base>/subdir: {}
            game 2:
              files:
                <root>/<game>: {}
              installDir:
                game2: {}
            game3:
              registry:
                HKEY_CURRENT_USER/Software/Ludusavi/game3: {}
                HKEY_CURRENT_USER/Software/Ludusavi/fake: {}
            game3-outer:
              registry:
                HKEY_CURRENT_USER/Software/Ludusavi: {}
            game4:
              files:
                <home>/data.txt: {}
                <winAppData>/winAppData.txt: {}
                <winLocalAppData>/winLocalAppData.txt: {}
                <winDocuments>/winDocuments.txt: {}
                <xdgConfig>/xdgConfig.txt: {}
                <xdgData>/xdgData.txt: {}
            game5:
              files:
                <base>: {}
            "#,
        )
        .unwrap()
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches() {
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2, "9d891e731f75deae56884d79e9816736b7488080").change_new(),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game1".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );

        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root2/game2/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game 2".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_deduplicating_symlinks() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/root3", repo())),
            store: Store::Other,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game5"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root3/game5/data/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game5"],
                "game5",
                roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game5".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_fuzzy_matched_install_dir() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/root3", repo())),
            store: Store::Other,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root3/game_2/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game 2".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_file_matches_in_custom_home_folder() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/home", repo())),
            store: Store::OtherHome,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/home/data.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/AppData/Roaming/winAppData.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/AppData/Local/winLocalAppData.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/Documents/winDocuments.txt", repo()), 0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game4".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn can_scan_game_for_backup_with_file_matches_in_custom_home_folder() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/home", repo())),
            store: Store::OtherHome,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/home/data.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/.config/xdgConfig.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/.local/share/xdgData.txt", repo()), 0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game4".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_in_wine_prefix() {
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/wine-prefix/drive_c/users/anyone/data.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/wine-prefix/user.reg", repo()), 37, "4a5b7e9de7d84ffb4bb3e9f38667f85741d5fbc0",).change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &Some(StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game4".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_and_ignores() {
        let cases = [
            (
                BackupFilter {
                    ignored_paths: vec![StrictPath::new(format!("{}\\tests/root1/game1/subdir", repo()))],
                    ..Default::default()
                },
                ToggledPaths::default(),
                hashset! {
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
            ),
            (
                BackupFilter::default(),
                ToggledPaths::new(btreemap! {
                    s("game1") => btreemap! {
                        StrictPath::new(format!("{}\\tests/root1/game1/subdir", repo())) => false
                    }
                }),
                hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2, "9d891e731f75deae56884d79e9816736b7488080").change_new().ignored(),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
            ),
            (
                BackupFilter::default(),
                ToggledPaths::new(btreemap! {
                    s("game1") => btreemap! {
                        StrictPath::new(format!("{}\\tests/root1/game1/subdir/file2.txt", repo())) => false
                    }
                }),
                hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2, "9d891e731f75deae56884d79e9816736b7488080").change_new().ignored(),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
            ),
        ];

        for (filter, ignored, found) in cases {
            assert_eq!(
                ScanInfo {
                    game_name: s("game1"),
                    found_files: found,
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                scan_game_for_backup(
                    &manifest().0["game1"],
                    "game1",
                    &config().roots,
                    &StrictPath::new(repo()),
                    &HeroicGames::default(),
                    &None,
                    &filter,
                    &None,
                    &InstallDirRanking::scan(&config().roots, &manifest(), &["game1".to_string()]),
                    &ignored,
                    &ToggledRegistry::default(),
                    None,
                    &[],
                ),
            );
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_leaf_key_with_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3"),
                found_files: hashset! {},
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New)
                },
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3"],
                "game3",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game3".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_parent_key_without_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3-outer"),
                found_files: hashset! {},
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change(ScanChange::New),
                },
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3-outer"],
                "game3-outer",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &None,
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game3-outer".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_and_ignores() {
        let cases = vec![
            (
                BackupFilter {
                    ignored_registry: vec![RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/other"))],
                    ..Default::default()
                },
                ToggledRegistry::default(),
                hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New),
                },
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btreemap! {
                    s("game3-outer") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi")) => false
                    }
                }),
                hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").ignored().change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").ignored().change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").ignored().change(ScanChange::New),
                },
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btreemap! {
                    s("game3-outer") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/other")) => false
                    }
                }),
                hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").ignored().change(ScanChange::New),
                },
            ),
        ];

        for (filter, ignored, found) in cases {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3-outer"),
                    found_files: hashset! {},
                    found_registry_keys: found,
                    ..Default::default()
                },
                scan_game_for_backup(
                    &manifest().0["game3-outer"],
                    "game3-outer",
                    &config().roots,
                    &StrictPath::new(repo()),
                    &HeroicGames::default(),
                    &None,
                    &filter,
                    &None,
                    &InstallDirRanking::scan(&config().roots, &manifest(), &["game1".to_string()]),
                    &ToggledPaths::default(),
                    &ignored,
                    None,
                    &[],
                ),
            );
        }
    }

    fn restorable_file_simple(backup: &str, file: &str) -> StrictPath {
        StrictPath::relative(
            format!(
                "{backup}/drive-{}/{file}",
                if cfg!(target_os = "windows") { "X" } else { "0" }
            ),
            Some(if cfg!(target_os = "windows") {
                format!("\\\\?\\{}\\tests\\backup\\game1", repo().replace('/', "\\"))
            } else {
                format!("{}/tests/backup/game1", repo())
            }),
        )
    }

    #[test]
    fn can_scan_game_for_restoration_with_files() {
        let mut layout = GameLayout::new(
            StrictPath::new(format!("{}/tests/backup/game1", repo())),
            IndividualMapping {
                name: "game1".to_string(),
                drives: drives_x(),
                backups: VecDeque::from(vec![FullBackup {
                    name: ".".into(),
                    when: now(),
                    files: btreemap! {
                        mapping_file_key("/file1.txt") => IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                        mapping_file_key("/file2.txt") => IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                    },
                    ..Default::default()
                }]),
            },
            Retention {
                full: 1,
                differential: 1,
            },
        );
        let backups = vec![Backup::Full(FullBackup {
            name: ".".to_string(),
            when: now(),
            files: btreemap! {
                mapping_file_key("/file1.txt") => IndividualMappingFile {
                    hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(),
                    size: 1,
                },
                mapping_file_key("/file2.txt") => IndividualMappingFile {
                    hash: "9d891e731f75deae56884d79e9816736b7488080".into(),
                    size: 2,
                },
            },
            ..Default::default()
        })];

        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hashset! {
                    ScannedFile {
                        path: restorable_file_simple(".", "file1.txt"),
                        size: 1,
                        hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        change: ScanChange::New,
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: restorable_file_simple(".", "file2.txt"),
                        size: 2,
                        hash: "9d891e731f75deae56884d79e9816736b7488080".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        change: ScanChange::New,
                        container: None,
                        redirected: None,
                    },
                },
                available_backups: backups.clone(),
                backup: Some(backups[0].clone()),
                ..Default::default()
            },
            scan_game_for_restoration("game1", &BackupId::Latest, &mut layout, &[]),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_restoration_with_registry() {
        let layout = BackupLayout::new(
            StrictPath::new(format!("{}/tests/backup", repo())),
            Retention::default(),
        );
        if cfg!(target_os = "windows") {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3"),
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::Same)
                    },
                    available_backups: vec![Backup::Full(FullBackup {
                        name: ".".to_string(),
                        when: now(),
                        registry: IndividualMappingRegistry {
                            hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into()),
                        },
                        ..Default::default()
                    })],
                    backup: Some(Backup::Full(FullBackup {
                        name: ".".to_string(),
                        when: now(),
                        registry: IndividualMappingRegistry {
                            hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into()),
                        },
                        ..Default::default()
                    })),
                    ..Default::default()
                },
                scan_game_for_restoration("game3", &BackupId::Latest, &mut layout.game_layout("game3"), &[]),
            );
        } else {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3"),
                    available_backups: vec![Backup::Full(FullBackup {
                        name: ".".to_string(),
                        when: now(),
                        registry: IndividualMappingRegistry {
                            hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into()),
                        },
                        ..Default::default()
                    })],
                    backup: Some(Backup::Full(FullBackup {
                        name: ".".to_string(),
                        when: now(),
                        ..Default::default()
                    })),
                    ..Default::default()
                },
                scan_game_for_restoration("game3", &BackupId::Latest, &mut layout.game_layout("game3"), &[]),
            );
        }
    }

    mod duplicate_detector {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn can_add_games_in_backup_mode() {
            let mut detector = DuplicateDetector::default();

            let game1 = s("game1");
            let game2 = s("game2");
            let file1 = ScannedFile::new("file1.txt", 1, "1");
            let file2 = ScannedFile::new("file2.txt", 2, "2");
            let reg1 = s("reg1");
            let reg2 = s("reg2");

            detector.add_game(&ScanInfo {
                game_name: game1.clone(),
                found_files: hashset! { file1.clone(), file2.clone() },
                found_registry_keys: hashset! { ScannedRegistry::new(&reg1) },
                ..Default::default()
            });
            detector.add_game(&ScanInfo {
                game_name: game2.clone(),
                found_files: hashset! { file1.clone() },
                found_registry_keys: hashset! { ScannedRegistry::new(&reg1), ScannedRegistry::new(&reg2) },
                ..Default::default()
            });

            assert!(detector.is_file_duplicated(&file1));
            assert_eq!(hashset! { game1.clone(), game2.clone() }, detector.file(&file1));

            assert!(!detector.is_file_duplicated(&file2));
            assert_eq!(hashset! { game1.clone() }, detector.file(&file2));

            assert!(detector.is_registry_duplicated(&RegistryItem::new(reg1.clone())));
            assert_eq!(
                hashset! { game1, game2.clone() },
                detector.registry(&RegistryItem::new(reg1))
            );

            assert!(!detector.is_registry_duplicated(&RegistryItem::new(reg2.clone())));
            assert_eq!(hashset! { game2 }, detector.registry(&RegistryItem::new(reg2)));
        }

        #[test]
        fn can_add_games_in_restore_mode() {
            let mut detector = DuplicateDetector::default();

            let game1 = s("game1");
            let game2 = s("game2");
            let file1a = ScannedFile {
                path: StrictPath::new(s("file1a.txt")),
                size: 1,
                hash: "1".to_string(),
                original_path: Some(StrictPath::new(s("file1.txt"))),
                ignored: false,
                change: Default::default(),
                container: None,
                redirected: None,
            };
            let file1b = ScannedFile {
                path: StrictPath::new(s("file1b.txt")),
                size: 1,
                hash: "1b".to_string(),
                original_path: Some(StrictPath::new(s("file1.txt"))),
                ignored: false,
                change: Default::default(),
                container: None,
                redirected: None,
            };

            detector.add_game(&ScanInfo {
                game_name: game1.clone(),
                found_files: hashset! { file1a.clone() },
                ..Default::default()
            });
            detector.add_game(&ScanInfo {
                game_name: game2.clone(),
                found_files: hashset! { file1b.clone() },
                ..Default::default()
            });

            assert!(detector.is_file_duplicated(&file1a));
            assert_eq!(hashset! { game1.clone(), game2.clone() }, detector.file(&file1a));
            assert!(!detector.is_file_duplicated(&ScannedFile {
                path: StrictPath::new(s("file1a.txt")),
                size: 1,
                hash: "1a".to_string(),
                original_path: None,
                ignored: false,
                change: Default::default(),
                container: None,
                redirected: None,
            }));

            assert!(detector.is_file_duplicated(&file1b));
            assert_eq!(hashset! { game1, game2 }, detector.file(&file1b));
            assert!(!detector.is_file_duplicated(&ScannedFile {
                path: StrictPath::new(s("file1b.txt")),
                size: 1,
                hash: "1b".to_string(),
                original_path: None,
                ignored: false,
                change: Default::default(),
                container: None,
                redirected: None,
            }));
        }
    }
}
