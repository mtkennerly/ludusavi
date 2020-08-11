use crate::{
    config::{BackupFilter, RedirectConfig, RootsConfig},
    layout::{BackupLayout, IndividualMapping},
    manifest::{Game, GameFileConstraint, Os, Store},
};
use std::io::Read;

pub use crate::path::StrictPath;

const WINDOWS: bool = cfg!(target_os = "windows");
const MAC: bool = cfg!(target_os = "macos");
const LINUX: bool = cfg!(target_os = "linux");
const CASE_INSENSITIVE_OS: bool = WINDOWS || MAC;
const SKIP: &str = "<skip>";

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("The manifest file is invalid: {why:?}")]
    ManifestInvalid { why: String },

    #[error("Unable to download an update to the manifest file")]
    ManifestCannotBeUpdated,

    #[error("The config file is invalid: {why:?}")]
    ConfigInvalid { why: String },

    #[error("Target already exists")]
    CliBackupTargetExists { path: StrictPath },

    #[error("Target already exists")]
    CliUnrecognizedGames { games: Vec<String> },

    #[error("Unable to request confirmation")]
    CliUnableToRequestConfirmation,

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
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedFile {
    pub path: StrictPath,
    pub size: u64,
    /// This is the restoration target path, without redirects applied.
    pub original_path: Option<StrictPath>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScanInfo {
    pub game_name: String,
    pub found_files: std::collections::HashSet<ScannedFile>,
    pub found_registry_keys: std::collections::HashSet<String>,
    pub registry_file: Option<StrictPath>,
}

impl ScanInfo {
    pub fn sum_bytes(&self, backup_info: &Option<BackupInfo>) -> u64 {
        let successful_bytes = self.found_files.iter().map(|x| x.size).sum::<u64>();
        let failed_bytes = if let Some(backup_info) = &backup_info {
            backup_info.failed_files.iter().map(|x| x.size).sum::<u64>()
        } else {
            0
        };
        successful_bytes - failed_bytes
    }

    pub fn found_anything(&self) -> bool {
        !self.found_files.is_empty() || !self.found_registry_keys.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BackupInfo {
    pub failed_files: std::collections::HashSet<ScannedFile>,
    pub failed_registry: std::collections::HashSet<String>,
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
}

impl OperationStatus {
    pub fn clear(&mut self) {
        self.total_games = 0;
        self.total_bytes = 0;
        self.processed_games = 0;
        self.processed_bytes = 0;
    }

    pub fn add_game(&mut self, scan_info: &ScanInfo, backup_info: &Option<BackupInfo>, processed: bool) {
        self.total_games += 1;
        self.total_bytes += scan_info.sum_bytes(&None);
        if processed {
            self.processed_games += 1;
            self.processed_bytes += scan_info.sum_bytes(&backup_info);
        }
    }

    pub fn completed(&self) -> bool {
        self.total_games == self.processed_games && self.total_bytes == self.processed_bytes
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
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

// This helps for unit tests when comparing StrictPaths.
fn reslashed(path: &str) -> String {
    path.replace("\\", "/")
}

pub fn app_dir() -> std::path::PathBuf {
    let mut path = dirs::home_dir().unwrap();
    path.push(".config");
    path.push("ludusavi");
    path
}

/// Returns the effective target and the original target (if different)
pub fn game_file_restoration_target(
    original_target: &StrictPath,
    redirects: &[RedirectConfig],
) -> (StrictPath, Option<StrictPath>) {
    let mut redirected_target = original_target.render();
    for redirect in redirects {
        if redirect.source.raw().trim().is_empty() || redirect.target.raw().trim().is_empty() {
            continue;
        }
        let source = redirect.source.render();
        let target = redirect.target.render();
        if !source.is_empty() && !target.is_empty() && redirected_target.starts_with(&source) {
            redirected_target = redirected_target.replacen(&source, &target, 1);
        }
    }

    let redirected_target = StrictPath::new(redirected_target);
    if original_target.render() != redirected_target.render() {
        (redirected_target, Some(original_target.clone()))
    } else {
        (original_target.clone(), None)
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

fn check_nonwindows_path(path: Option<std::path::PathBuf>) -> String {
    match get_os() {
        Os::Windows => SKIP.to_string(),
        _ => check_path(path),
    }
}

pub fn parse_paths(
    path: &str,
    root: &RootsConfig,
    install_dirs: &[&String],
    steam_id: &Option<u32>,
    manifest_dir: &StrictPath,
) -> std::collections::HashSet<StrictPath> {
    let mut paths = std::collections::HashSet::new();

    for install_dir in install_dirs {
        paths.insert(
            path.replace("<root>", &root.path.interpret())
                .replace("<game>", &install_dir)
                .replace(
                    "<base>",
                    &match root.store {
                        Store::Steam => format!("{}/steamapps/common/{}", root.path.interpret(), install_dir),
                        Store::Other => format!("{}/{}", root.path.interpret(), install_dir),
                    },
                )
                .replace(
                    "<home>",
                    &dirs::home_dir().unwrap_or_else(|| SKIP.into()).to_string_lossy(),
                )
                .replace(
                    "<storeUserId>",
                    match root.store {
                        Store::Steam => "[0-9]*",
                        Store::Other => "*",
                    },
                )
                .replace("<osUserName>", &whoami::username())
                .replace("<winAppData>", &check_windows_path(dirs::data_dir()))
                .replace("<winLocalAppData>", &check_windows_path(dirs::data_local_dir()))
                .replace("<winDocuments>", &check_windows_path(dirs::document_dir()))
                .replace("<winPublic>", &check_windows_path(dirs::public_dir()))
                .replace(
                    "<winProgramData>",
                    &check_windows_path(Some(std::path::PathBuf::from("C:/Windows/ProgramData"))),
                )
                .replace(
                    "<winDir>",
                    &check_windows_path(Some(std::path::PathBuf::from("C:/Windows"))),
                )
                .replace("<xdgData>", &check_nonwindows_path(dirs::data_dir()))
                .replace("<xdgConfig>", &check_nonwindows_path(dirs::config_dir()))
                .replace("<regHkcu>", SKIP)
                .replace("<regHklm>", SKIP),
        );
        if get_os() == Os::Linux && root.store == Store::Steam && steam_id.is_some() {
            let prefix = format!(
                "{}/steamapps/compatdata/{}/pfx/drive_c",
                root.path.interpret(),
                steam_id.unwrap()
            );
            paths.insert(
                path.replace("<root>", &root.path.interpret())
                    .replace("<game>", &install_dir)
                    .replace(
                        "<base>",
                        &format!("{}/steamapps/common/{}", root.path.interpret(), install_dir),
                    )
                    .replace("<home>", &format!("{}/users/steamuser", prefix))
                    .replace("<storeUserId>", "*")
                    .replace("<osUserName>", "steamuser")
                    .replace("<winAppData>", &format!("{}/users/steamuser/Application Data", prefix))
                    .replace(
                        "<winLocalAppData>",
                        &format!("{}/users/steamuser/Application Data", prefix),
                    )
                    .replace("<winDocuments>", &format!("{}/users/steamuser/My Documents", prefix))
                    .replace("<winPublic>", &format!("{}/users/Public", prefix))
                    .replace("<winProgramData>", &format!("{}/ProgramData", prefix))
                    .replace("<winDir>", &format!("{}/windows", prefix))
                    .replace("<xdgData>", &check_nonwindows_path(dirs::data_dir()))
                    .replace("<xdgConfig>", &check_nonwindows_path(dirs::config_dir()))
                    .replace("<regHkcu>", SKIP)
                    .replace("<regHklm>", SKIP),
            );
        }
    }

    paths
        .iter()
        .map(|x| StrictPath::relative(x.to_string(), Some(manifest_dir.interpret())))
        .collect()
}

fn glob_any(path: &StrictPath) -> Result<glob::Paths, ()> {
    let options = glob::MatchOptions {
        case_sensitive: CASE_INSENSITIVE_OS,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    let entries = glob::glob_with(&path.render(), options).map_err(|_| ())?;
    Ok(entries)
}

fn should_exclude_as_other_os_data(constraints: &[GameFileConstraint], host: Os, maybe_proton: bool) -> bool {
    let constrained = !constraints.is_empty();
    let unconstrained_by_os = constraints.iter().any(|x| x.os == None);
    let matches_os = constraints.iter().any(|x| x.os == Some(host.clone()));
    let suitable_for_proton = maybe_proton && constraints.iter().any(|x| x.os == Some(Os::Windows));
    constrained && !unconstrained_by_os && !matches_os && !suitable_for_proton
}

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[RootsConfig],
    manifest_dir: &StrictPath,
    steam_id: &Option<u32>,
    filter: &BackupFilter,
) -> ScanInfo {
    let mut found_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = std::collections::HashSet::new();

    // Add a dummy root for checking paths without `<root>`.
    let mut roots_to_check: Vec<RootsConfig> = vec![RootsConfig {
        path: StrictPath::new(SKIP.to_string()),
        store: Store::Other,
    }];
    roots_to_check.extend(roots.iter().cloned());

    let mut paths_to_check = std::collections::HashSet::<StrictPath>::new();

    for root in &roots_to_check {
        if root.path.raw().trim().is_empty() {
            continue;
        }
        if let Some(files) = &game.files {
            let maybe_proton = get_os() == Os::Linux && root.store == Store::Steam && steam_id.is_some();
            let default_install_dir = name.to_string();
            let install_dirs: Vec<_> = match &game.install_dir {
                Some(x) => x.keys().collect(),
                _ => vec![&default_install_dir],
            };
            for (raw_path, path_info) in files {
                if raw_path.trim().is_empty() {
                    continue;
                }
                if filter.exclude_other_os_data {
                    if let Some(constraints) = &path_info.when {
                        if should_exclude_as_other_os_data(&constraints, get_os(), maybe_proton) {
                            continue;
                        }
                    }
                }
                let candidates = parse_paths(raw_path, &root, &install_dirs, &steam_id, &manifest_dir);
                for candidate in candidates {
                    if candidate.raw().contains(SKIP) {
                        continue;
                    }
                    paths_to_check.insert(candidate);
                }
            }
        }
        if root.store == Store::Steam && steam_id.is_some() {
            // Cloud saves:
            paths_to_check.insert(StrictPath::relative(
                format!("{}/userdata/*/{}/remote/", root.path.interpret(), &steam_id.unwrap()),
                Some(manifest_dir.interpret()),
            ));

            // Screenshots:
            if !filter.exclude_store_screenshots {
                paths_to_check.insert(StrictPath::relative(
                    format!(
                        "{}/userdata/*/760/remote/{}/screenshots/*.*",
                        root.path.interpret(),
                        &steam_id.unwrap()
                    ),
                    Some(manifest_dir.interpret()),
                ));
            }

            // Registry:
            if game.registry.is_some() {
                let prefix = format!(
                    "{}/steamapps/compatdata/{}/pfx",
                    root.path.interpret(),
                    steam_id.unwrap()
                );
                paths_to_check.insert(StrictPath::relative(
                    format!("{}/*.reg", prefix),
                    Some(manifest_dir.interpret()),
                ));
            }
        }
    }

    for path in paths_to_check {
        let entries = match glob_any(&path) {
            Ok(x) => x,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|r| r.ok()) {
            let plain = entry.to_string_lossy().to_string();
            let p = std::path::Path::new(&plain);
            if p.is_file() {
                found_files.insert(ScannedFile {
                    path: StrictPath::new(reslashed(&plain)),
                    size: match p.metadata() {
                        Ok(m) => m.len(),
                        _ => 0,
                    },
                    original_path: None,
                });
            } else if p.is_dir() {
                for child in walkdir::WalkDir::new(p)
                    .max_depth(100)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if child.file_type().is_file() {
                        found_files.insert(ScannedFile {
                            path: StrictPath::new(reslashed(&child.path().display().to_string())),
                            size: match child.metadata() {
                                Ok(m) => m.len(),
                                _ => 0,
                            },
                            original_path: None,
                        });
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut hives = crate::registry::Hives::default();
        if let Some(registry) = &game.registry {
            for key in registry.keys() {
                if key.trim().is_empty() {
                    continue;
                }
                if let Ok(info) = hives.store_key_from_full_path(&key) {
                    if info.found {
                        found_registry_keys.insert(key.to_string());
                    }
                }
            }
        }
    }

    ScanInfo {
        game_name: name.to_string(),
        found_files,
        found_registry_keys,
        registry_file: None,
    }
}

pub fn scan_game_for_restoration(name: &str, layout: &BackupLayout) -> ScanInfo {
    let mut found_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut registry_file = None;

    let target_game = layout.game_folder(&name);
    if target_game.is_dir() {
        found_files = layout.restorable_files(&name, &target_game);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(hives) = crate::registry::Hives::load(&layout.game_registry_file(&target_game)) {
            registry_file = Some(layout.game_registry_file(&target_game));
            for (hive_name, keys) in hives.0.iter() {
                for (key_name, _) in keys.0.iter() {
                    found_registry_keys.insert(format!("{}/{}", hive_name, key_name).replace("\\", "/"));
                }
            }
        }
    }

    ScanInfo {
        game_name: name.to_string(),
        found_files,
        found_registry_keys,
        registry_file,
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

fn are_files_identical(file1: &StrictPath, file2: &StrictPath) -> Result<bool, Box<dyn std::error::Error>> {
    let f1 = std::fs::File::open(file1.interpret())?;
    let mut f1r = std::io::BufReader::new(f1);
    let f2 = std::fs::File::open(file2.interpret())?;
    let mut f2r = std::io::BufReader::new(f2);

    let mut f1b = [0; 1024];
    let mut f2b = [0; 1024];
    loop {
        let f1n = f1r.read(&mut f1b[..])?;
        let f2n = f2r.read(&mut f2b[..])?;

        if f1n != f2n || f1b.iter().zip(f2b.iter()).any(|(a, b)| a != b) {
            return Ok(false);
        }
        if f1n == 0 || f2n == 0 {
            break;
        }
    }
    Ok(true)
}

pub fn back_up_game(info: &ScanInfo, name: &str, layout: &BackupLayout, merge: bool) -> BackupInfo {
    let mut failed_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut failed_registry = std::collections::HashSet::new();

    let target_game = layout.game_folder(&name);

    let able_to_prepare = info.found_anything()
        && (merge || (target_game.unset_readonly().is_ok() && target_game.remove().is_ok()))
        && std::fs::create_dir_all(target_game.interpret()).is_ok();

    let mut mapping = match IndividualMapping::load(&layout.game_mapping_file(&target_game)) {
        Ok(x) => x,
        Err(_) => IndividualMapping::new(name.to_string()),
    };

    let mut relevant_backup_files = Vec::<StrictPath>::new();
    for file in &info.found_files {
        if !able_to_prepare {
            failed_files.insert(file.clone());
            continue;
        }

        let target_file = layout.game_file(&target_game, &file.path, &mut mapping);
        relevant_backup_files.push(target_file.clone());

        if target_file.exists() {
            match are_files_identical(&file.path, &target_file) {
                Ok(true) => continue,
                Ok(false) => (),
                Err(_) => {
                    failed_files.insert(file.clone());
                    continue;
                }
            }
        }
        if target_file.create_parent_dir().is_err() {
            failed_files.insert(file.clone());
            continue;
        }
        if std::fs::copy(&file.path.interpret(), &target_file.interpret()).is_err() {
            failed_files.insert(file.clone());
            continue;
        }
    }

    if able_to_prepare && merge {
        layout.remove_irrelevant_backup_files(&target_game, &relevant_backup_files);
    }

    #[cfg(target_os = "windows")]
    {
        let mut hives = crate::registry::Hives::default();
        let mut found_some_registry = false;

        for reg_path in &info.found_registry_keys {
            if !able_to_prepare {
                failed_registry.insert(reg_path.to_string());
                continue;
            }

            match hives.store_key_from_full_path(&reg_path) {
                Err(_) => {
                    failed_registry.insert(reg_path.to_string());
                }
                Ok(x) if !x.found => {
                    failed_registry.insert(reg_path.to_string());
                }
                _ => {
                    found_some_registry = true;
                }
            }
        }

        let target_registry_file = layout.game_registry_file(&target_game);
        if found_some_registry {
            hives.save(&target_registry_file);
        } else {
            let _ = target_registry_file.remove();
        }
    }

    if info.found_anything() && able_to_prepare {
        mapping.save(&layout.game_mapping_file(&target_game));
    }

    BackupInfo {
        failed_files,
        failed_registry,
    }
}

pub fn restore_game(info: &ScanInfo, redirects: &[RedirectConfig]) -> BackupInfo {
    let mut failed_files = std::collections::HashSet::new();
    let failed_registry = std::collections::HashSet::new();

    'outer: for file in &info.found_files {
        let original_path = match &file.original_path {
            Some(x) => x,
            None => continue,
        };
        let (target, _) = game_file_restoration_target(&original_path, &redirects);

        if target.exists() {
            match are_files_identical(&file.path, &target) {
                Ok(true) => continue,
                Ok(false) => (),
                Err(_) => {
                    failed_files.insert(file.clone());
                    continue;
                }
            }
        }

        if target.create_parent_dir().is_err() {
            failed_files.insert(file.clone());
            continue;
        }
        for i in 0..99 {
            if target.unset_readonly().is_ok() && std::fs::copy(&file.path.interpret(), &target.interpret()).is_ok() {
                continue 'outer;
            }
            // File might be busy, especially if multiple games share a file,
            // like in a collection, so retry after a delay:
            std::thread::sleep(std::time::Duration::from_millis(i * info.game_name.len() as u64));
        }
        failed_files.insert(file.clone());
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(registry_file) = &info.registry_file {
            if let Some(hives) = crate::registry::Hives::load(&registry_file) {
                // TODO: Track failed keys.
                let _ = hives.restore();
            }
        }
    }

    BackupInfo {
        failed_files,
        failed_registry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::manifest::Manifest;
    use maplit::hashset;
    use pretty_assertions::assert_eq;

    fn s(text: &str) -> String {
        text.to_string()
    }

    fn repo() -> String {
        reslashed(env!("CARGO_MANIFEST_DIR"))
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
            "#,
        )
        .unwrap()
    }

    #[test]
    fn should_not_exclude_as_other_os_data_when_os_matches() {
        assert_eq!(
            false,
            should_exclude_as_other_os_data(
                &[GameFileConstraint {
                    os: Some(Os::Windows),
                    store: None
                }],
                Os::Windows,
                false
            )
        );
    }

    #[test]
    fn should_exclude_as_other_os_data_when_os_does_not_match() {
        assert_eq!(
            true,
            should_exclude_as_other_os_data(
                &[GameFileConstraint {
                    os: Some(Os::Linux),
                    store: None
                }],
                Os::Windows,
                false
            )
        );
    }

    #[test]
    fn should_not_exclude_as_other_os_data_when_no_os_constraint() {
        assert_eq!(
            false,
            should_exclude_as_other_os_data(
                &[GameFileConstraint {
                    os: None,
                    store: Some(Store::Steam)
                }],
                Os::Windows,
                false
            )
        );
    }

    #[test]
    fn should_not_exclude_as_other_os_data_when_any_constraint_lacks_os() {
        assert_eq!(
            false,
            should_exclude_as_other_os_data(
                &[
                    GameFileConstraint {
                        os: Some(Os::Linux),
                        store: None
                    },
                    GameFileConstraint {
                        os: None,
                        store: Some(Store::Steam)
                    }
                ],
                Os::Windows,
                false
            )
        );
    }

    #[test]
    fn should_exclude_as_other_os_data_when_constraint_has_store_and_other_os() {
        assert_eq!(
            true,
            should_exclude_as_other_os_data(
                &[GameFileConstraint {
                    os: Some(Os::Linux),
                    store: Some(Store::Steam)
                }],
                Os::Windows,
                false
            )
        );
    }

    #[test]
    fn should_not_exclude_as_other_os_data_when_no_constraints() {
        assert_eq!(false, should_exclude_as_other_os_data(&[], Os::Windows, false));
    }

    #[test]
    fn should_not_exclude_as_other_os_data_when_suitable_for_proton() {
        assert_eq!(
            false,
            should_exclude_as_other_os_data(
                &[GameFileConstraint {
                    os: Some(Os::Windows),
                    store: Some(Store::Steam)
                }],
                Os::Linux,
                true
            )
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches() {
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo())),
                        size: 2,
                        original_path: None,
                    },
                    ScannedFile {
                        path: StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo())),
                        size: 1,
                        original_path: None,
                    },
                },
                found_registry_keys: hashset! {},
                registry_file: None,
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &None,
                &BackupFilter::default(),
            ),
        );

        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(format!("{}/tests/root2/game2/file1.txt", repo())),
                        size: 1,
                        original_path: None,
                    },
                },
                found_registry_keys: hashset! {},
                registry_file: None,
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                &config().roots,
                &StrictPath::new(repo()),
                &None,
                &BackupFilter::default(),
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_leaf_key_with_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3"),
                found_files: hashset! {},
                found_registry_keys: hashset! {
                    s("HKEY_CURRENT_USER/Software/Ludusavi/game3")
                },
                registry_file: None,
            },
            scan_game_for_backup(
                &manifest().0["game3"],
                "game3",
                &config().roots,
                &StrictPath::new(repo()),
                &None,
                &BackupFilter::default(),
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
                    s("HKEY_CURRENT_USER/Software/Ludusavi")
                },
                registry_file: None,
            },
            scan_game_for_backup(
                &manifest().0["game3-outer"],
                "game3-outer",
                &config().roots,
                &StrictPath::new(repo()),
                &None,
                &BackupFilter::default(),
            ),
        );
    }

    #[test]
    fn can_scan_game_for_restoration_with_files() {
        let make_path = |x| {
            if cfg!(target_os = "windows") {
                StrictPath::new(format!(
                    "\\\\?\\{}\\tests\\backup\\game1\\drive-X\\{}",
                    repo().replace("/", "\\"),
                    x
                ))
            } else {
                StrictPath::new(format!("{}/tests/backup/game1/drive-X/{}", repo(), x))
            }
        };

        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hashset! {
                    ScannedFile { path: make_path("file1.txt"), size: 1, original_path: Some(StrictPath::new(s(if cfg!(target_os = "windows") { "X:\\file1.txt" } else { "X:/file1.txt" }))) },
                    ScannedFile { path: make_path("file2.txt"), size: 2, original_path: Some(StrictPath::new(s(if cfg!(target_os = "windows") { "X:\\file2.txt" } else { "X:/file2.txt" }))) },
                },
                ..Default::default()
            },
            scan_game_for_restoration(
                "game1",
                &BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo())))
            ),
        );
    }

    #[test]
    fn can_scan_game_for_restoration_with_registry() {
        if cfg!(target_os = "windows") {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3"),
                    found_registry_keys: hashset! {
                        s("HKEY_CURRENT_USER/Software/Ludusavi/game3")
                    },
                    registry_file: Some(StrictPath::new(format!(
                        "\\\\?\\{}\\tests\\backup\\game3-renamed/registry.yaml",
                        repo().replace("/", "\\")
                    ))),
                    ..Default::default()
                },
                scan_game_for_restoration(
                    "game3",
                    &BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo())))
                ),
            );
        } else {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3"),
                    ..Default::default()
                },
                scan_game_for_restoration(
                    "game3",
                    &BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo())))
                ),
            );
        }
    }
}
