pub mod game_filter;
pub mod heroic;
pub mod layout;
pub mod registry_compat;

#[cfg(target_os = "windows")]
pub mod registry;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use fuzzy_matcher::FuzzyMatcher;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;

use crate::{
    path::StrictPath,
    prelude::{filter_map_walkdir, Error, INVALID_FILE_CHARS, SKIP},
    resource::{
        config::{BackupFilter, RedirectConfig, RedirectKind, RootsConfig, SortKey, ToggledPaths, ToggledRegistry},
        manifest::{Game, Manifest, Os, Store},
    },
    scan::{
        heroic::HeroicGames,
        layout::{Backup, BackupLayout, LatestBackup},
        registry_compat::RegistryItem,
    },
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize)]
pub enum ScanChange {
    New,
    Different,
    Removed,
    Same,
    #[default]
    Unknown,
}

impl ScanChange {
    pub fn normalize(&self, ignored: bool, restoring: bool) -> Self {
        match self {
            ScanChange::New if ignored => Self::Same,
            ScanChange::New => *self,
            ScanChange::Different if ignored && restoring => Self::Same,
            ScanChange::Different if ignored && !restoring => Self::Removed,
            ScanChange::Different => Self::Different,
            ScanChange::Removed => *self,
            ScanChange::Same if ignored && !restoring => Self::Removed,
            ScanChange::Same => *self,
            ScanChange::Unknown => *self,
        }
    }

    pub fn is_changed(&self) -> bool {
        match self {
            Self::New => true,
            Self::Different => true,
            Self::Removed => true,
            Self::Same => false,
            // This is because we want unchanged and unscanned games to be filtered differently:
            Self::Unknown => true,
        }
    }

    pub fn will_take_space(&self) -> bool {
        match self {
            Self::New => true,
            Self::Different => true,
            Self::Removed => false,
            Self::Same => true,
            Self::Unknown => true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize)]
pub struct ScanChangeCount {
    pub new: usize,
    pub different: usize,
    #[serde(skip)]
    pub removed: usize,
    pub same: usize,
}

impl ScanChangeCount {
    pub fn new() -> Self {
        Self {
            new: 0,
            different: 0,
            removed: 0,
            same: 0,
        }
    }

    pub fn add(&mut self, change: ScanChange) {
        match change {
            ScanChange::New => self.new += 1,
            ScanChange::Different => self.different += 1,
            ScanChange::Removed => self.removed += 1,
            ScanChange::Same => self.same += 1,
            ScanChange::Unknown => (),
        }
    }

    pub fn brand_new(&self) -> bool {
        self.only(ScanChange::New)
    }

    pub fn updated(&self) -> bool {
        !self.brand_new() && (self.new > 0 || self.different > 0 || self.removed > 0)
    }

    fn only(&self, change: ScanChange) -> bool {
        let total = self.new + self.different + self.removed + self.same;
        let only = |count: usize| count > 0 && count == total;
        match change {
            ScanChange::New => only(self.new),
            ScanChange::Different => only(self.different),
            ScanChange::Removed => only(self.removed),
            ScanChange::Same => only(self.same),
            ScanChange::Unknown => false,
        }
    }

    pub fn overall(&self) -> ScanChange {
        if self.brand_new() {
            ScanChange::New
        } else if self.only(ScanChange::Removed) {
            ScanChange::Removed
        } else if self.updated() {
            ScanChange::Different
        } else {
            ScanChange::Same
        }
    }
}

impl ScanChange {
    pub fn evaluate_backup(current_hash: &str, previous_hash: Option<&&String>) -> Self {
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
            Err(_) => Self::New,
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
    pub fn with_change<T: AsRef<str> + ToString, H: ToString>(path: T, size: u64, hash: H, change: ScanChange) -> Self {
        Self {
            path: StrictPath::new(path.to_string()),
            size,
            hash: hash.to_string(),
            original_path: None,
            ignored: false,
            change,
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

    pub fn will_take_space(&self) -> bool {
        !self.ignored && self.change.will_take_space()
    }

    pub fn is_changed(&self) -> bool {
        self.change.normalize(self.ignored, self.restoring()).is_changed()
    }

    pub fn effective_change(&self) -> ScanChange {
        self.change.normalize(self.ignored, self.restoring())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedRegistry {
    pub path: RegistryItem,
    pub ignored: bool,
    pub change: ScanChange,
    pub values: ScannedRegistryValues,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedRegistryValue {
    pub ignored: bool,
    pub change: ScanChange,
}

pub type ScannedRegistryValues = BTreeMap<String, ScannedRegistryValue>;

impl ScannedRegistry {
    #[cfg(test)]
    pub fn new<T: AsRef<str> + ToString>(path: T) -> Self {
        Self {
            path: RegistryItem::new(path.to_string()),
            ignored: false,
            change: ScanChange::Unknown,
            values: Default::default(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn ignored(mut self) -> Self {
        self.ignored = true;
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn change(mut self, change: ScanChange) -> Self {
        self.change = change;
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_value(mut self, value_name: &str, change: ScanChange, ignored: bool) -> Self {
        self.values
            .insert(value_name.to_string(), ScannedRegistryValue { change, ignored });
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_value_new(mut self, value_name: &str) -> Self {
        self.values.insert(
            value_name.to_string(),
            ScannedRegistryValue {
                change: ScanChange::New,
                ignored: false,
            },
        );
        self
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_value_same(mut self, value_name: &str) -> Self {
        self.values.insert(
            value_name.to_string(),
            ScannedRegistryValue {
                change: ScanChange::Same,
                ignored: false,
            },
        );
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanInfo {
    pub game_name: String,
    pub found_files: HashSet<ScannedFile>,
    pub found_registry_keys: HashSet<ScannedRegistry>,
    /// Only populated by a restoration scan.
    pub available_backups: Vec<Backup>,
    /// Only populated by a restoration scan.
    pub backup: Option<Backup>,
}

impl ScanInfo {
    pub fn sum_bytes(&self, backup_info: Option<&BackupInfo>) -> u64 {
        let successful_bytes = self
            .found_files
            .iter()
            .filter(|x| x.will_take_space())
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
        let can_report = match self.count_changes().overall() {
            ScanChange::New => true,
            ScanChange::Different => true,
            ScanChange::Removed => false,
            ScanChange::Same => true,
            ScanChange::Unknown => true,
        };
        can_report && (!self.found_files.is_empty() || !self.found_registry_keys.is_empty())
    }

    pub fn found_anything_processable(&self) -> bool {
        self.count_changes().overall().is_changed()
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
                y.ignored = toggled_registry.is_ignored(&self.game_name, &x.path, None);
                for (value_name, value) in &mut y.values {
                    value.ignored = toggled_registry.is_ignored(&self.game_name, &x.path, Some(value_name));
                }
                y
            })
            .collect();
    }

    pub fn all_ignored(&self) -> bool {
        if self.found_files.is_empty() && self.found_registry_keys.is_empty() {
            return false;
        }
        self.found_files.iter().all(|x| x.ignored)
            && self
                .found_registry_keys
                .iter()
                .all(|x| x.ignored && x.values.values().all(|y| y.ignored))
    }

    pub fn any_ignored(&self) -> bool {
        self.found_files.iter().any(|x| x.ignored)
            || self
                .found_registry_keys
                .iter()
                .any(|x| x.ignored || x.values.values().any(|y| y.ignored))
    }

    pub fn total_items(&self) -> usize {
        self.found_files.len()
            + self
                .found_registry_keys
                .iter()
                .map(|x| 1 + x.values.len())
                .sum::<usize>()
    }

    pub fn enabled_items(&self) -> usize {
        self.found_files.iter().filter(|x| !x.ignored).count()
            + self
                .found_registry_keys
                .iter()
                .map(|x| if x.ignored { 0 } else { 1 } + x.values.values().filter(|y| !y.ignored).count())
                .sum::<usize>()
    }

    pub fn restoring(&self) -> bool {
        self.backup.is_some()
    }

    pub fn is_changed(&self) -> bool {
        for entry in &self.found_files {
            if entry.ignored {
                continue;
            }
            if entry.change.is_changed() {
                return true;
            }
        }
        for entry in &self.found_registry_keys {
            if !entry.ignored && entry.change.is_changed() {
                return true;
            }
            for value in entry.values.values().filter(|x| !x.ignored) {
                if value.change.is_changed() {
                    return true;
                }
            }
        }

        false
    }

    pub fn count_changes(&self) -> ScanChangeCount {
        let mut count = ScanChangeCount::new();

        for entry in &self.found_files {
            count.add(entry.change.normalize(entry.ignored, self.restoring()));
        }
        for entry in &self.found_registry_keys {
            count.add(entry.change.normalize(entry.ignored, self.restoring()));
            for value in entry.values.values() {
                count.add(value.change.normalize(value.ignored, self.restoring()));
            }
        }

        count
    }
}

#[derive(Clone, Debug, Default)]
pub struct BackupInfo {
    pub failed_files: HashSet<ScannedFile>,
    pub failed_registry: HashSet<RegistryItem>,
}

impl BackupInfo {
    pub fn successful(&self) -> bool {
        self.failed_files.is_empty() && self.failed_registry.is_empty()
    }

    pub fn total_failure(scan: &ScanInfo) -> Self {
        let mut backup_info = Self::default();

        for file in &scan.found_files {
            if file.ignored {
                continue;
            }
            backup_info.failed_files.insert(file.clone());
        }
        for reg_path in &scan.found_registry_keys {
            if reg_path.ignored {
                continue;
            }
            backup_info.failed_registry.insert(reg_path.path.clone());
        }

        backup_info
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
            self.processed_bytes += scan_info.sum_bytes(backup_info.as_ref());
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

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub enum OperationStepDecision {
    #[default]
    Processed,
    Cancelled,
    Ignored,
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

fn check_path(path: Option<std::path::PathBuf>) -> String {
    path.unwrap_or_else(|| SKIP.into()).to_string_lossy().to_string()
}

fn check_windows_path(path: Option<std::path::PathBuf>) -> String {
    match Os::HOST {
        Os::Windows => check_path(path),
        _ => SKIP.to_string(),
    }
}

fn check_windows_path_str(path: &str) -> &str {
    match Os::HOST {
        Os::Windows => path,
        _ => SKIP,
    }
}

fn check_nonwindows_path(path: Option<std::path::PathBuf>) -> String {
    match Os::HOST {
        Os::Windows => SKIP.to_string(),
        _ => check_path(path),
    }
}

fn check_nonwindows_path_str(path: &str) -> &str {
    match Os::HOST {
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
    steam_shortcut: Option<&SteamShortcut>,
    platform: Os,
) -> HashSet<(StrictPath, bool)> {
    use crate::resource::manifest::placeholder::*;

    let mut paths = HashSet::new();

    let install_dir = match install_dir {
        Some(d) => d,
        None => SKIP,
    };

    let root_interpreted = root.path.interpret();
    let data_dir = check_path(dirs::data_dir());
    let data_local_dir = check_path(dirs::data_local_dir());
    let config_dir = check_path(dirs::config_dir());

    paths.insert((
        path.replace(ROOT, &root_interpreted)
            .replace(GAME, install_dir)
            .replace(
                BASE,
                &match root.store {
                    Store::Steam => format!("{}/steamapps/common/{}", &root_interpreted, install_dir),
                    Store::Heroic => full_install_dir
                        .map(|x| x.interpret())
                        .unwrap_or_else(|| SKIP.to_string()),
                    _ => format!("{}/{}", &root_interpreted, install_dir),
                },
            )
            .replace(HOME, &dirs::home_dir().unwrap_or_else(|| SKIP.into()).to_string_lossy())
            .replace(STORE_USER_ID, "*")
            .replace(OS_USER_NAME, &whoami::username())
            .replace(WIN_APP_DATA, check_windows_path_str(&data_dir))
            .replace(WIN_LOCAL_APP_DATA, check_windows_path_str(&data_local_dir))
            .replace(WIN_DOCUMENTS, &check_windows_path(dirs::document_dir()))
            .replace(WIN_PUBLIC, &check_windows_path(dirs::public_dir()))
            .replace(WIN_PROGRAM_DATA, check_windows_path_str("C:/ProgramData"))
            .replace(WIN_DIR, check_windows_path_str("C:/Windows"))
            .replace(XDG_DATA, check_nonwindows_path_str(&data_dir))
            .replace(XDG_CONFIG, check_nonwindows_path_str(&config_dir)),
        platform.is_case_sensitive(),
    ));
    if Os::HOST == Os::Windows {
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
    if root.store == Store::Gog && Os::HOST == Os::Linux {
        paths.insert((
            path.replace(GAME, &format!("{}/game", install_dir))
                .replace(BASE, &format!("{}/{}/game", root.path.interpret(), install_dir)),
            platform.is_case_sensitive(),
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
        && Os::HOST == Os::Linux
        && root_interpreted.ends_with(".var/app/com.heroicgameslauncher.hgl/config/heroic")
    {
        paths.insert((
            path.replace(
                XDG_DATA,
                check_nonwindows_path_str(&format!("{}/../../data", &root_interpreted)),
            )
            .replace(
                XDG_CONFIG,
                check_nonwindows_path_str(&format!("{}/../../config", &root_interpreted)),
            )
            .replace(STORE_USER_ID, "*"),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::OtherHome {
        paths.insert((
            path.replace(ROOT, &root_interpreted)
                .replace(GAME, install_dir)
                .replace(BASE, &format!("{}/{}", &root_interpreted, install_dir))
                .replace(STORE_USER_ID, SKIP)
                .replace(OS_USER_NAME, &whoami::username())
                .replace(WIN_APP_DATA, check_windows_path_str("<home>/AppData/Roaming"))
                .replace(WIN_LOCAL_APP_DATA, check_windows_path_str("<home>/AppData/Local"))
                .replace(WIN_DOCUMENTS, check_windows_path_str("<home>/Documents"))
                .replace(WIN_PUBLIC, &check_windows_path(dirs::public_dir()))
                .replace(WIN_PROGRAM_DATA, check_windows_path_str("C:/ProgramData"))
                .replace(WIN_DIR, check_windows_path_str("C:/Windows"))
                .replace(XDG_DATA, check_nonwindows_path_str("<home>/.local/share"))
                .replace(XDG_CONFIG, check_nonwindows_path_str("<home>/.config"))
                .replace(HOME, &root_interpreted),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::Steam {
        if let Some(steam_shortcut) = steam_shortcut {
            if let Some(start_dir) = &steam_shortcut.start_dir {
                paths.insert((path.replace(BASE, &start_dir.interpret()), platform.is_case_sensitive()));
            }
        }
    }
    if root.store == Store::Steam && Os::HOST == Os::Linux {
        let mut ids = vec![];
        if let Some(steam_id) = steam_id {
            ids.push(*steam_id);
        }
        if let Some(steam_shortcut) = steam_shortcut {
            ids.push(steam_shortcut.id);
        }

        for id in ids {
            let prefix = format!("{}/steamapps/compatdata/{}/pfx/drive_c", &root_interpreted, id);
            let path2 = path
                .replace(ROOT, &root_interpreted)
                .replace(GAME, install_dir)
                .replace(BASE, &format!("{}/steamapps/common/{}", &root_interpreted, install_dir))
                .replace(HOME, &format!("{}/users/steamuser", prefix))
                .replace(STORE_USER_ID, "*")
                .replace(OS_USER_NAME, "steamuser")
                .replace(WIN_PUBLIC, &format!("{}/users/Public", prefix))
                .replace(WIN_PROGRAM_DATA, &format!("{}/ProgramData", prefix))
                .replace(WIN_DIR, &format!("{}/windows", prefix))
                .replace(XDG_DATA, &check_nonwindows_path(dirs::data_dir()))
                .replace(XDG_CONFIG, &check_nonwindows_path(dirs::config_dir()));
            paths.insert((
                path2
                    .replace(WIN_DOCUMENTS, &format!("{}/users/steamuser/Documents", prefix))
                    .replace(WIN_APP_DATA, &format!("{}/users/steamuser/AppData/Roaming", prefix))
                    .replace(WIN_LOCAL_APP_DATA, &format!("{}/users/steamuser/AppData/Local", prefix)),
                false,
            ));
            paths.insert((
                path2
                    .replace(WIN_DOCUMENTS, &format!("{}/users/steamuser/My Documents", prefix))
                    .replace(WIN_APP_DATA, &format!("{}/users/steamuser/Application Data", prefix))
                    .replace(
                        WIN_LOCAL_APP_DATA,
                        &format!("{}/users/steamuser/Local Settings/Application Data", prefix),
                    ),
                false,
            ));
        }
    }
    if root.store == Store::OtherWine {
        let prefix = format!("{}/drive_*", &root_interpreted);
        let path2 = path
            .replace(ROOT, &root_interpreted)
            .replace(GAME, install_dir)
            .replace(BASE, &format!("{}/{}", &root_interpreted, install_dir))
            .replace(HOME, &format!("{}/users/*", prefix))
            .replace(STORE_USER_ID, "*")
            .replace(OS_USER_NAME, "*")
            .replace(WIN_PUBLIC, &format!("{}/users/Public", prefix))
            .replace(WIN_PROGRAM_DATA, &format!("{}/ProgramData", prefix))
            .replace(WIN_DIR, &format!("{}/windows", prefix))
            .replace(XDG_DATA, &check_nonwindows_path(dirs::data_dir()))
            .replace(XDG_CONFIG, &check_nonwindows_path(dirs::config_dir()));
        paths.insert((
            path2
                .replace(WIN_DOCUMENTS, &format!("{}/users/*/Documents", prefix))
                .replace(WIN_APP_DATA, &format!("{}/users/*/AppData/Roaming", prefix))
                .replace(WIN_LOCAL_APP_DATA, &format!("{}/users/*/AppData/Local", prefix)),
            false,
        ));
        paths.insert((
            path2
                .replace(WIN_DOCUMENTS, &format!("{}/users/*/My Documents", prefix))
                .replace(WIN_APP_DATA, &format!("{}/users/*/Application Data", prefix))
                .replace(
                    WIN_LOCAL_APP_DATA,
                    &format!("{}/users/*/Local Settings/Application Data", prefix),
                ),
            false,
        ));
    }

    paths
        .iter()
        .map(|(x, y)| (StrictPath::relative(x.to_string(), Some(manifest_dir.interpret())), *y))
        .collect()
}

#[derive(Clone, Default)]
pub struct InstallDirRanking(HashMap<(RootsConfig, String), (i64, String)>);

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

    pub fn scan(roots: &[RootsConfig], manifest: &Manifest, subjects: &[String]) -> Self {
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

    fn scan_root(&mut self, root: &RootsConfig, manifest: &Manifest, subjects: &[String]) {
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

#[derive(Clone, Default)]
pub struct SteamShortcuts(HashMap<String, SteamShortcut>);

#[derive(Clone, Default)]
pub struct SteamShortcut {
    id: u32,
    start_dir: Option<StrictPath>,
}

impl SteamShortcuts {
    pub fn scan() -> Self {
        let mut instance = Self::default();

        let mut steam = match steamlocate::SteamDir::locate() {
            Some(x) => x,
            None => return instance,
        };

        for shortcut in steam.shortcuts() {
            log::trace!(
                "Found Steam shortcut: name={}, id={}, start_dir={}",
                &shortcut.app_name,
                shortcut.appid,
                &shortcut.start_dir
            );
            let start_dir = std::path::Path::new(shortcut.start_dir.trim_start_matches('"').trim_end_matches('"'));
            instance.0.insert(
                shortcut.app_name.clone(),
                SteamShortcut {
                    id: shortcut.appid,
                    start_dir: if start_dir.is_absolute() {
                        Some(StrictPath::from(start_dir))
                    } else {
                        None
                    },
                },
            );
        }

        instance
    }

    pub fn get(&self, name: &str) -> Option<&SteamShortcut> {
        self.0.get(name)
    }
}

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[RootsConfig],
    manifest_dir: &StrictPath,
    heroic_games: &HeroicGames,
    filter: &BackupFilter,
    wine_prefix: &Option<StrictPath>,
    ranking: &InstallDirRanking,
    ignored_paths: &ToggledPaths,
    #[allow(unused_variables)] ignored_registry: &ToggledRegistry,
    previous: Option<LatestBackup>,
    redirects: &[RedirectConfig],
    steam_shortcuts: &SteamShortcuts,
) -> ScanInfo {
    log::trace!("[{name}] beginning scan for backup");

    let mut found_files = HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = HashSet::new();

    let mut paths_to_check = HashSet::<(StrictPath, Option<bool>)>::new();

    // Add a dummy root for checking paths without `<root>`.
    let mut roots_to_check: Vec<RootsConfig> = vec![RootsConfig {
        path: StrictPath::new(SKIP.to_string()),
        store: Store::Other,
    }];
    roots_to_check.extend(roots.iter().cloned());

    let manifest_dir_interpreted = manifest_dir.interpret();
    let steam_id = game.steam.as_ref().and_then(|x| x.id);

    // We can add this for Wine prefixes from the CLI because they're
    // typically going to be used for only one or a few games at a time.
    // For other Wine roots, it would trigger for every game.
    if let Some(wp) = wine_prefix {
        log::trace!("[{name}] adding extra Wine prefix: {}", wp.raw());
        scan_game_for_backup_add_prefix(
            &mut roots_to_check,
            &mut paths_to_check,
            wp,
            &manifest_dir_interpreted,
            game.registry.is_some(),
        );
    }

    // handle what was found for heroic
    for root in roots {
        if let Some(wp) = heroic_games.get_prefix(root, name) {
            let with_pfx = wp.joined("pfx");
            scan_game_for_backup_add_prefix(
                &mut roots_to_check,
                &mut paths_to_check,
                if with_pfx.exists() { &with_pfx } else { wp },
                &manifest_dir_interpreted,
                game.registry.is_some(),
            );
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

        let platform = heroic_games.get_platform(&root, name).unwrap_or(Os::HOST);

        if let Some(files) = &game.files {
            let install_dir = ranking.get(&root, name);
            let full_install_dir = heroic_games.get_install_dir(&root, name);

            for raw_path in files.keys() {
                log::trace!("[{name}] parsing candidates from: {}", raw_path);
                if raw_path.trim().is_empty() {
                    continue;
                }
                let candidates = parse_paths(
                    raw_path,
                    &root,
                    &install_dir,
                    &full_install_dir,
                    &steam_id,
                    manifest_dir,
                    steam_shortcuts.get(name),
                    platform,
                );
                for (candidate, case_sensitive) in candidates {
                    log::trace!("[{name}] parsed candidate: {}", candidate.raw());
                    if candidate.raw().contains('<') {
                        // This covers `SKIP` and any other unmatched placeholders.
                        continue;
                    }
                    paths_to_check.insert((candidate, Some(case_sensitive)));
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

    let previous_files: HashMap<&StrictPath, &String> = previous
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
                    change: ScanChange::evaluate_backup(&hash, previous_files.get(redirected.as_ref().unwrap_or(&p))),
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
                            change: ScanChange::evaluate_backup(
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

    // Mark removed files.
    let current_files: Vec<_> = found_files.iter().map(|x| x.path.interpret()).collect();
    for (previous_file, _) in previous_files {
        let previous_file_interpreted = previous_file.interpret();
        if !current_files.contains(&previous_file_interpreted) {
            found_files.insert(ScannedFile {
                change: ScanChange::Removed,
                size: 0,
                hash: "".to_string(),
                redirected: None,
                path: previous_file.to_owned(),
                original_path: None,
                ignored: ignored_paths.is_ignored(name, previous_file),
                container: None,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        let previous_registry = match previous.map(|x| x.registry_content) {
            Some(Some(content)) => registry::Hives::deserialize(&content),
            _ => None,
        };

        if let Some(registry) = &game.registry {
            for key in registry.keys() {
                if key.trim().is_empty() {
                    continue;
                }

                log::trace!("[{name}] computing candidates for registry: {key}");
                let mut candidates = vec![key.clone()];
                let normalized = key.replace('\\', "/").to_lowercase();
                if normalized.starts_with("hkey_local_machine/software/") && !normalized.contains("/wow6432node/") {
                    let tail = &key[28..];
                    candidates.push(format!("HKEY_LOCAL_MACHINE/SOFTWARE/Wow6432Node/{}", tail));
                    candidates.push(format!(
                        "HKEY_CURRENT_USER/Software/Classes/VirtualStore/MACHINE/SOFTWARE/{}",
                        tail
                    ));
                    candidates.push(format!(
                        "HKEY_CURRENT_USER/Software/Classes/VirtualStore/MACHINE/SOFTWARE/Wow6432Node/{}",
                        tail
                    ));
                }

                for candidate in candidates {
                    log::trace!("[{name}] checking registry: {candidate}");
                    for mut scanned in
                        registry::scan_registry(name, &candidate, filter, ignored_registry, &previous_registry)
                            .unwrap_or_default()
                    {
                        log::debug!("[{name}] found registry: {}", scanned.path.raw());

                        // Mark removed registry values.
                        let previous_values = previous_registry
                            .as_ref()
                            .and_then(|x| {
                                x.get_path(&scanned.path)
                                    .map(|y| y.0.keys().cloned().collect::<Vec<_>>())
                            })
                            .unwrap_or_default();
                        for previous_value in previous_values {
                            #[allow(clippy::map_entry)]
                            if !scanned.values.contains_key(&previous_value) {
                                let ignored = ignored_registry.is_ignored(name, &scanned.path, Some(&previous_value));
                                scanned.values.insert(
                                    previous_value,
                                    ScannedRegistryValue {
                                        ignored,
                                        change: ScanChange::Removed,
                                    },
                                );
                            }
                        }

                        found_registry_keys.insert(scanned);
                    }
                }
            }
        }

        // Mark removed registry keys.
        if let Some(previous_registry) = &previous_registry {
            let current_registry_keys: Vec<_> = found_registry_keys.iter().map(|x| x.path.interpret()).collect();
            for (previous_hive, previous_keys) in &previous_registry.0 {
                for previous_key in previous_keys.0.keys() {
                    let path = RegistryItem::from_hive_and_key(previous_hive, previous_key);
                    if !current_registry_keys.contains(&path.interpret()) {
                        let ignored = ignored_registry.is_ignored(name, &path, None);
                        found_registry_keys.insert(ScannedRegistry {
                            change: ScanChange::Removed,
                            path,
                            ignored,
                            values: Default::default(),
                        });
                    }
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
    paths_to_check: &mut HashSet<(StrictPath, Option<bool>)>,
    wp: &StrictPath,
    manifest_dir_interpreted: &str,
    has_registry: bool,
) {
    roots_to_check.push(RootsConfig {
        path: wp.clone(),
        store: Store::OtherWine,
    });
    if has_registry {
        paths_to_check.insert((
            StrictPath::relative(
                format!("{}/*.reg", wp.interpret()),
                Some(manifest_dir_interpreted.to_owned()),
            ),
            None,
        ));
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackupId {
    #[default]
    Latest,
    Named(String),
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
    std::fs::create_dir_all(p).map_err(|_| Error::CannotPrepareBackupTarget { path: target.clone() })?;

    Ok(())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Duplication {
    #[default]
    Unique,
    Resolved,
    Duplicate,
}

impl Duplication {
    pub fn unique(&self) -> bool {
        self == &Self::Unique
    }

    pub fn resolved(&self) -> bool {
        self == &Self::Resolved
    }

    pub fn evaluate<'a>(items: impl Iterator<Item = &'a DuplicateDetectorEntry> + Clone) -> Duplication {
        if items.clone().count() < 2 {
            Duplication::Unique
        } else if items.filter(|x| x.enabled).count() <= 1 {
            Duplication::Resolved
        } else {
            Duplication::Duplicate
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DuplicateDetectorEntry {
    enabled: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DuplicateDetectorCount {
    non_unique: u32,
    resolved: u32,
}

impl DuplicateDetectorCount {
    pub fn evaluate(&self) -> Duplication {
        if self.non_unique == 0 {
            Duplication::Unique
        } else if self.non_unique == self.resolved {
            Duplication::Resolved
        } else {
            Duplication::Duplicate
        }
    }

    pub fn add(&mut self, other: &Self) {
        self.non_unique += other.non_unique;
        self.resolved += other.resolved;
    }
}

#[derive(Clone, Debug, Default)]
pub struct DuplicateDetector {
    files: HashMap<StrictPath, HashMap<String, DuplicateDetectorEntry>>,
    registry: HashMap<RegistryItem, HashMap<String, DuplicateDetectorEntry>>,
    registry_values: HashMap<RegistryItem, HashMap<String, HashMap<String, DuplicateDetectorEntry>>>,
    game_files: HashMap<String, HashSet<StrictPath>>,
    game_registry: HashMap<String, HashSet<RegistryItem>>,
    game_registry_values: HashMap<String, HashMap<RegistryItem, HashSet<String>>>,
    game_duplicated_items: HashMap<String, DuplicateDetectorCount>,
}

impl DuplicateDetector {
    pub fn add_game(&mut self, scan_info: &ScanInfo, game_enabled: bool) -> HashSet<String> {
        let mut stale = self.remove_game_and_refresh(&scan_info.game_name, false);
        stale.insert(scan_info.game_name.clone());

        for item in scan_info.found_files.iter() {
            let path = self.pick_path(item);
            if let Some(existing) = self.files.get(&path).map(|x| x.keys()) {
                // Len 0: No games to update counts for.
                // Len 2+: These games already include the item in their duplicate counts.
                if existing.len() == 1 {
                    stale.extend(existing.cloned());
                }
            }
            self.files.entry(path.clone()).or_insert_with(Default::default).insert(
                scan_info.game_name.clone(),
                DuplicateDetectorEntry {
                    enabled: game_enabled && !item.ignored,
                },
            );
            self.game_files
                .entry(scan_info.game_name.clone())
                .or_insert_with(Default::default)
                .insert(path);
        }

        for item in scan_info.found_registry_keys.iter() {
            let path = item.path.clone();
            if let Some(existing) = self.registry.get(&path).map(|x| x.keys()) {
                if existing.len() == 1 {
                    stale.extend(existing.cloned());
                }
            }
            self.registry
                .entry(path.clone())
                .or_insert_with(Default::default)
                .insert(
                    scan_info.game_name.clone(),
                    DuplicateDetectorEntry {
                        enabled: game_enabled && !item.ignored,
                    },
                );
            self.game_registry
                .entry(scan_info.game_name.clone())
                .or_insert_with(Default::default)
                .insert(path.clone());

            for (value_name, value) in item.values.iter() {
                self.registry_values
                    .entry(path.clone())
                    .or_insert_with(Default::default)
                    .entry(value_name.to_string())
                    .or_insert_with(Default::default)
                    .insert(
                        scan_info.game_name.clone(),
                        DuplicateDetectorEntry {
                            enabled: game_enabled && !value.ignored,
                        },
                    );
                self.game_registry_values
                    .entry(scan_info.game_name.clone())
                    .or_insert_with(Default::default)
                    .entry(path.clone())
                    .or_insert_with(Default::default)
                    .insert(value_name.to_string());
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

    pub fn remove_game(&mut self, game: &str) -> HashSet<String> {
        self.remove_game_and_refresh(game, true)
    }

    fn remove_game_and_refresh(&mut self, game: &str, refresh: bool) -> HashSet<String> {
        let mut stale = HashSet::new();

        self.game_duplicated_items.remove(game);

        if let Some(files) = self.game_files.remove(game) {
            for file in files {
                if let Some(games) = self.files.get_mut(&file) {
                    games.remove(game);
                    for duplicate in games.keys() {
                        stale.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry.remove(game) {
            for registry in registry_keys {
                if let Some(games) = self.registry.get_mut(&registry) {
                    games.remove(game);
                    for duplicate in games.keys() {
                        stale.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry_values.remove(game) {
            for (registry_key, registry_values) in registry_keys {
                for registry_value in registry_values {
                    if let Some(games) = self
                        .registry_values
                        .get_mut(&registry_key)
                        .and_then(|x| x.get_mut(&registry_value))
                    {
                        games.remove(game);
                        for duplicate in games.keys() {
                            stale.insert(duplicate.clone());
                        }
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

    pub fn is_game_duplicated(&self, game: &str) -> Duplication {
        self.count_duplicates_for(game).evaluate()
    }

    fn pick_path(&self, file: &ScannedFile) -> StrictPath {
        match &file.original_path {
            Some(op) => op.clone(),
            None => file.path.clone(),
        }
    }

    pub fn file(&self, file: &ScannedFile) -> HashMap<String, DuplicateDetectorEntry> {
        match self.files.get(&self.pick_path(file)) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_file_duplicated(&self, file: &ScannedFile) -> Duplication {
        Duplication::evaluate(self.file(file).values())
    }

    pub fn registry(&self, path: &RegistryItem) -> HashMap<String, DuplicateDetectorEntry> {
        match self.registry.get(path) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_registry_duplicated(&self, path: &RegistryItem) -> Duplication {
        Duplication::evaluate(self.registry(path).values())
    }

    pub fn registry_value(&self, path: &RegistryItem, value: &str) -> HashMap<String, DuplicateDetectorEntry> {
        match self.registry_values.get(path).and_then(|key| key.get(value)) {
            Some(games) => games.clone(),
            None => Default::default(),
        }
    }

    pub fn is_registry_value_duplicated(&self, path: &RegistryItem, value: &str) -> Duplication {
        Duplication::evaluate(self.registry_value(path, value).values())
    }

    pub fn clear(&mut self) {
        self.files.clear();
        self.registry.clear();
        self.registry_values.clear();
        self.game_duplicated_items.clear();
    }

    pub fn overall(&self) -> Duplication {
        let mut count = DuplicateDetectorCount::default();

        for item in self.game_duplicated_items.values() {
            count.add(item);
        }

        count.evaluate()
    }

    fn count_duplicated_items_for(&self, game: &str) -> DuplicateDetectorCount {
        let mut tally = DuplicateDetectorCount::default();
        for item in self.files.values() {
            if item.contains_key(game) && item.len() > 1 {
                tally.non_unique += 1;
                if item.values().filter(|x| x.enabled).count() <= 1 {
                    tally.resolved += 1;
                }
            }
        }
        for item in self.registry.values() {
            if item.contains_key(game) && item.len() > 1 {
                tally.non_unique += 1;
                if item.values().filter(|x| x.enabled).count() <= 1 {
                    tally.resolved += 1;
                }
            }
        }
        for item in self.registry_values.values() {
            for item in item.values() {
                if item.contains_key(game) && item.len() > 1 {
                    tally.non_unique += 1;
                    if item.values().filter(|x| x.enabled).count() <= 1 {
                        tally.resolved += 1;
                    }
                }
            }
        }
        tally
    }

    fn count_duplicates_for(&self, game: &str) -> DuplicateDetectorCount {
        self.game_duplicated_items.get(game).copied().unwrap_or_default()
    }

    pub fn duplicate_games(&self, game: &str) -> HashSet<String> {
        let mut duplicates = HashSet::new();

        if let Some(files) = self.game_files.get(game) {
            for file in files {
                if let Some(games) = self.files.get(file) {
                    if games.len() < 2 {
                        continue;
                    }
                    for duplicate in games.keys() {
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
                    for duplicate in games.keys() {
                        duplicates.insert(duplicate.clone());
                    }
                }
            }
        }
        if let Some(registry_keys) = self.game_registry_values.get(game) {
            for (registry_key, registry_values) in registry_keys {
                for registry_value in registry_values {
                    if let Some(games) = self
                        .registry_values
                        .get(registry_key)
                        .and_then(|x| x.get(registry_value))
                    {
                        if games.len() < 2 {
                            continue;
                        }
                        for duplicate in games.keys() {
                            duplicates.insert(duplicate.clone());
                        }
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
    // Also, this is used for files/folders, so we can always ignore illegal characters.
    let candidate = candidate
        .replace(['_', '-'], " ")
        .replace(INVALID_FILE_CHARS, " ")
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

pub struct TitleFinder {
    all_games: HashSet<String>,
    can_backup: HashSet<String>,
    can_restore: HashSet<String>,
    steam_ids: HashMap<u32, String>,
    gog_ids: HashMap<u64, String>,
    normalized: HashMap<String, String>,
}

impl TitleFinder {
    pub fn new(manifest: &Manifest, layout: &BackupLayout) -> Self {
        let can_backup: HashSet<_> = manifest.0.keys().cloned().collect();
        let can_restore: HashSet<_> = layout.restorable_games().into_iter().collect();
        let all_games: HashSet<_> = can_backup.union(&can_restore).cloned().collect();
        let steam_ids = manifest.map_steam_ids_to_names();
        let gog_ids = manifest.map_gog_ids_to_names();
        let normalized: HashMap<_, _> = all_games
            .iter()
            .map(|title| (normalize_title(title), title.to_owned()))
            .collect();

        Self {
            all_games,
            can_backup,
            can_restore,
            steam_ids,
            gog_ids,
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
        gog_id: &Option<u64>,
        normalized: bool,
        backup: bool,
        restore: bool,
    ) -> Option<String> {
        let found = self.find(names, steam_id, gog_id, normalized, backup, restore);
        found.iter().next().map(|x| x.to_owned())
    }

    pub fn find(
        &self,
        names: &[String],
        steam_id: &Option<u32>,
        gog_id: &Option<u64>,
        normalized: bool,
        backup: bool,
        restore: bool,
    ) -> BTreeSet<String> {
        let mut output = BTreeSet::new();

        if let Some(steam_id) = steam_id {
            if let Some(found) = self.steam_ids.get(steam_id) {
                if self.eligible(found, backup, restore) {
                    output.insert(found.to_owned());
                    return output;
                }
            }
        }

        if let Some(gog_id) = gog_id {
            if let Some(found) = self.gog_ids.get(gog_id) {
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

pub fn compare_games(
    key: SortKey,
    scan_info1: &ScanInfo,
    backup_info1: Option<&BackupInfo>,
    scan_info2: &ScanInfo,
    backup_info2: Option<&BackupInfo>,
) -> std::cmp::Ordering {
    match key {
        SortKey::Name => compare_games_by_name(&scan_info1.game_name, &scan_info2.game_name),
        SortKey::Size => compare_games_by_size(scan_info1, backup_info1, scan_info2, backup_info2),
        SortKey::Status => compare_games_by_status(scan_info1, scan_info2),
    }
}

fn compare_games_by_name(name1: &str, name2: &str) -> std::cmp::Ordering {
    name1.to_lowercase().cmp(&name2.to_lowercase()).then(name1.cmp(name2))
}

fn compare_games_by_size(
    scan_info1: &ScanInfo,
    backup_info1: Option<&BackupInfo>,
    scan_info2: &ScanInfo,
    backup_info2: Option<&BackupInfo>,
) -> std::cmp::Ordering {
    scan_info1
        .sum_bytes(backup_info1)
        .cmp(&scan_info2.sum_bytes(backup_info2))
        .then_with(|| compare_games_by_name(&scan_info1.game_name, &scan_info2.game_name))
}

fn compare_games_by_status(scan_info1: &ScanInfo, scan_info2: &ScanInfo) -> std::cmp::Ordering {
    scan_info1
        .count_changes()
        .overall()
        .cmp(&scan_info2.count_changes().overall())
        .then_with(|| compare_games_by_name(&scan_info1.game_name, &scan_info2.game_name))
}

#[cfg(test)]
mod tests {
    use maplit::*;
    use pretty_assertions::assert_eq;

    use super::*;
    #[cfg(target_os = "windows")]
    use crate::resource::config::ToggledRegistryEntry;
    use crate::{
        resource::{config::Config, manifest::Manifest, ResourceFile},
        testing::{repo, s, EMPTY_HASH},
    };

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
            fake-registry:
              registry:
                HKEY_CURRENT_USER/Software/Ludusavi/fake: {}
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game1".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game 2".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game5".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game 2".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game4".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(roots, &manifest(), &["game4".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                &BackupFilter::default(),
                &Some(StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game4".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_registry_files_in_wine_prefix() {
        assert_eq!(
            ScanInfo {
                game_name: s("fake-registry"),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/wine-prefix/user.reg", repo()), 37, "4a5b7e9de7d84ffb4bb3e9f38667f85741d5fbc0",).change_new(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["fake-registry"],
                "fake-registry",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &BackupFilter::default(),
                &Some(StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &InstallDirRanking::scan(&config().roots, &manifest(), &["fake-registry".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                    &filter,
                    &None,
                    &InstallDirRanking::scan(&config().roots, &manifest(), &["game1".to_string()]),
                    &ignored,
                    &ToggledRegistry::default(),
                    None,
                    &[],
                    &Default::default(),
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
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz")
                },
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3"],
                "game3",
                &config().roots,
                &StrictPath::new(repo()),
                &HeroicGames::default(),
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game3".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz"),
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
                &BackupFilter::default(),
                &None,
                &InstallDirRanking::scan(&config().roots, &manifest(), &["game3-outer".to_string()]),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz"),
                },
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btreemap! {
                    s("game3-outer") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi")) => ToggledRegistryEntry::Key(false)
                    }
                }),
                hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").ignored().change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").ignored().change(ScanChange::New)
                        .with_value("binary", ScanChange::New, true)
                        .with_value("dword", ScanChange::New, true)
                        .with_value("expandSz", ScanChange::New, true)
                        .with_value("multiSz", ScanChange::New, true)
                        .with_value("qword", ScanChange::New, true)
                        .with_value("sz", ScanChange::New, true),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").ignored().change(ScanChange::New),
                },
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btreemap! {
                    s("game3-outer") => btreemap! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/game3")) => ToggledRegistryEntry::Complex {
                            key: None,
                            values: btreemap! {
                                s("qword") => false,
                            },
                        },
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/other")) => ToggledRegistryEntry::Key(false),
                    }
                }),
                hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value("qword", ScanChange::New, true)
                        .with_value_new("sz"),
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
                    &filter,
                    &None,
                    &InstallDirRanking::scan(&config().roots, &manifest(), &["game1".to_string()]),
                    &ToggledPaths::default(),
                    &ignored,
                    None,
                    &[],
                    &Default::default(),
                ),
            );
        }
    }

    mod duplicate_detector {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn can_add_games_in_backup_mode() {
            let mut detector = DuplicateDetector::default();

            let game1 = s("game1");
            let game2 = s("game2");
            let file1 = ScannedFile::new("file1.txt", 1, "1");
            let file2 = ScannedFile::new("file2.txt", 2, "2");
            let reg1 = s("reg1");
            let reg2 = s("reg2");

            detector.add_game(
                &ScanInfo {
                    game_name: game1.clone(),
                    found_files: hashset! { file1.clone(), file2.clone() },
                    found_registry_keys: hashset! { ScannedRegistry::new(&reg1) },
                    ..Default::default()
                },
                true,
            );
            detector.add_game(
                &ScanInfo {
                    game_name: game2.clone(),
                    found_files: hashset! { file1.clone() },
                    found_registry_keys: hashset! { ScannedRegistry::new(&reg1), ScannedRegistry::new(&reg2) },
                    ..Default::default()
                },
                true,
            );

            assert_eq!(Duplication::Duplicate, detector.is_file_duplicated(&file1));
            assert_eq!(
                hashmap! {
                    game1.clone() => DuplicateDetectorEntry { enabled: true },
                    game2.clone() => DuplicateDetectorEntry { enabled: true }
                },
                detector.file(&file1)
            );

            assert_eq!(Duplication::Unique, detector.is_file_duplicated(&file2));
            assert_eq!(
                hashmap! {
                    game1.clone() => DuplicateDetectorEntry { enabled: true }
                },
                detector.file(&file2)
            );

            assert_eq!(
                Duplication::Duplicate,
                detector.is_registry_duplicated(&RegistryItem::new(reg1.clone()))
            );
            assert_eq!(
                hashmap! {
                    game1 => DuplicateDetectorEntry { enabled: true },
                    game2.clone() => DuplicateDetectorEntry { enabled: true }
                },
                detector.registry(&RegistryItem::new(reg1))
            );

            assert_eq!(
                Duplication::Unique,
                detector.is_registry_duplicated(&RegistryItem::new(reg2.clone()))
            );
            assert_eq!(
                hashmap! {
                    game2 => DuplicateDetectorEntry { enabled: true }
                },
                detector.registry(&RegistryItem::new(reg2))
            );
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

            detector.add_game(
                &ScanInfo {
                    game_name: game1.clone(),
                    found_files: hashset! { file1a.clone() },
                    ..Default::default()
                },
                true,
            );
            detector.add_game(
                &ScanInfo {
                    game_name: game2.clone(),
                    found_files: hashset! { file1b.clone() },
                    ..Default::default()
                },
                true,
            );

            assert_eq!(Duplication::Duplicate, detector.is_file_duplicated(&file1a));
            assert_eq!(
                hashmap! {
                    game1.clone() => DuplicateDetectorEntry { enabled: true },
                    game2.clone() => DuplicateDetectorEntry { enabled: true }
                },
                detector.file(&file1a)
            );
            assert_eq!(
                Duplication::Unique,
                detector.is_file_duplicated(&ScannedFile {
                    path: StrictPath::new(s("file1a.txt")),
                    size: 1,
                    hash: "1a".to_string(),
                    original_path: None,
                    ignored: false,
                    change: Default::default(),
                    container: None,
                    redirected: None,
                })
            );

            assert_eq!(Duplication::Duplicate, detector.is_file_duplicated(&file1b));
            assert_eq!(
                hashmap! {
                    game1 => DuplicateDetectorEntry { enabled: true },
                    game2 => DuplicateDetectorEntry { enabled: true }
                },
                detector.file(&file1b)
            );
            assert_eq!(
                Duplication::Unique,
                detector.is_file_duplicated(&ScannedFile {
                    path: StrictPath::new(s("file1b.txt")),
                    size: 1,
                    hash: "1b".to_string(),
                    original_path: None,
                    ignored: false,
                    change: Default::default(),
                    container: None,
                    redirected: None,
                })
            );
        }
    }
}
