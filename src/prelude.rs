use crate::{
    config::{RedirectConfig, RootsConfig},
    manifest::{Game, Os, Store},
};

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

#[derive(Clone, Debug, thiserror::Error)]
pub enum OtherError {
    #[error("Cannot determine restoration target")]
    BadRestorationTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScannedFile {
    pub path: StrictPath,
    pub size: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ScanInfo {
    pub game_name: String,
    pub found_files: std::collections::HashSet<ScannedFile>,
    pub found_registry_keys: std::collections::HashSet<String>,
    pub registry_file: Option<std::path::PathBuf>,
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

#[derive(Clone, Debug, Default)]
pub struct OperationStatus {
    pub total_games: usize,
    pub total_bytes: u64,
    pub processed_games: usize,
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

#[derive(Debug, Clone, PartialEq)]
pub enum OperationStepDecision {
    Processed,
    Cancelled,
    Ignored,
}

pub fn app_dir() -> std::path::PathBuf {
    let mut path = dirs::home_dir().unwrap();
    path.push(".config");
    path.push("ludusavi");
    path
}

pub fn game_backup_dir(start: &StrictPath, game: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    path.push(start.interpret());
    path.push(base64::encode(game));
    path
}

pub fn game_file_backup_target(start: &StrictPath, game: &str, original_path: &StrictPath) -> std::path::PathBuf {
    let mut path = game_backup_dir(&start, &game);
    path.push(base64::encode(original_path.render()));
    path
}

pub fn game_file_restoration_target(
    file: &StrictPath,
    redirects: &[RedirectConfig],
) -> Result<(StrictPath, StrictPath), Box<dyn std::error::Error>> {
    let file_pb = file.as_std_path_buf();
    let base_name = file_pb.file_name().ok_or(OtherError::BadRestorationTarget)?;
    let decoded = base64::decode(base_name.to_string_lossy().as_bytes())?;
    let original_target = std::str::from_utf8(&decoded)?.to_string();

    let mut redirected_target = original_target.clone();
    for redirect in redirects {
        let source = redirect.source.render();
        let target = redirect.target.render();
        if !source.is_empty() && !target.is_empty() && redirected_target.starts_with(&source) {
            redirected_target = redirected_target.replacen(&source, &target, 1);
        }
    }

    Ok((StrictPath::new(original_target), StrictPath::new(redirected_target)))
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

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[RootsConfig],
    manifest_dir: &StrictPath,
    steam_id: &Option<u32>,
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
            let default_install_dir = name.to_string();
            let install_dirs: Vec<_> = match &game.install_dir {
                Some(x) => x.keys().collect(),
                _ => vec![&default_install_dir],
            };
            for raw_path in files.keys() {
                if raw_path.trim().is_empty() {
                    continue;
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
            paths_to_check.insert(StrictPath::relative(
                format!(
                    "{}/userdata/*/760/remote/{}/screenshots/*.*",
                    root.path.interpret(),
                    &steam_id.unwrap()
                ),
                Some(manifest_dir.interpret()),
            ));

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
                    path: StrictPath::new(plain.clone()),
                    size: match p.metadata() {
                        Ok(m) => m.len(),
                        _ => 0,
                    },
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
                            path: StrictPath::new(child.path().display().to_string()),
                            size: match child.metadata() {
                                Ok(m) => m.len(),
                                _ => 0,
                            },
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

pub fn scan_dir_for_restorable_games(source: &StrictPath) -> Vec<(String, StrictPath)> {
    let mut games = vec![];
    for subdir in walkdir::WalkDir::new(source.interpret())
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .skip(1) // the restore path itself
        .filter_map(|e| e.ok())
    {
        let path = &subdir.path();
        let base_name = match path.file_name() {
            None => continue,
            Some(x) => x,
        };

        let decoded_base_name = match base64::decode(base_name.to_string_lossy().as_bytes()) {
            Err(_) => continue,
            Ok(x) => x,
        };
        let name = match std::str::from_utf8(&decoded_base_name) {
            Err(_) => continue,
            Ok(x) => x.to_string(),
        };

        games.push((name, StrictPath::from_std_path_buf(&subdir.into_path())));
    }
    games
}

pub fn get_restore_name_and_parent(source: &StrictPath) -> Option<(String, StrictPath)> {
    let path = source.as_std_path_buf();
    let base_name = match path.file_name() {
        None => return None,
        Some(x) => x,
    };
    let parent = match path.parent() {
        None => return None,
        Some(x) => x.to_string_lossy(),
    };

    let decoded_base_name = match base64::decode(base_name.to_string_lossy().as_bytes()) {
        Err(_) => return None,
        Ok(x) => x,
    };
    let name = match std::str::from_utf8(&decoded_base_name) {
        Err(_) => return None,
        Ok(x) => x.to_string(),
    };

    Some((name, StrictPath::new(parent.to_string())))
}

pub fn scan_dir_for_restoration(source: &StrictPath) -> ScanInfo {
    match get_restore_name_and_parent(&source) {
        None => ScanInfo::default(),
        Some((name, parent)) => scan_game_for_restoration(&name, &parent),
    }
}

pub fn scan_game_for_restoration(name: &str, source: &StrictPath) -> ScanInfo {
    let mut found_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut registry_file = None;

    let target_game = game_backup_dir(&source, &name);
    if target_game.as_path().is_dir() {
        for child in walkdir::WalkDir::new(target_game.as_path())
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if child.file_type().is_file() {
                let source = StrictPath::new(child.path().display().to_string());
                found_files.insert(ScannedFile {
                    path: source,
                    size: match child.metadata() {
                        Ok(m) => m.len(),
                        _ => 0,
                    },
                });
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(hives) = crate::registry::Hives::load(&crate::registry::game_registry_backup_file(&source, &name)) {
            registry_file = Some(crate::registry::game_registry_backup_file(&source, &name));
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

pub fn prepare_backup_target(target: &StrictPath) -> Result<(), Error> {
    target
        .remove()
        .map_err(|_| Error::CannotPrepareBackupTarget { path: target.clone() })?;
    let p = target.as_std_path_buf();
    std::fs::create_dir_all(&p).map_err(|_| Error::CannotPrepareBackupTarget { path: target.clone() })?;

    Ok(())
}

pub fn back_up_game(info: &ScanInfo, target: &StrictPath, name: &str) -> BackupInfo {
    let mut failed_files = std::collections::HashSet::new();
    #[allow(unused_mut)]
    let mut failed_registry = std::collections::HashSet::new();

    for file in &info.found_files {
        let target_game = game_backup_dir(&target, &name);
        if !target_game.as_path().is_dir() && std::fs::create_dir(target_game).is_err() {
            failed_files.insert(file.clone());
            continue;
        }

        let target_file = game_file_backup_target(&target, &name, &file.path);
        if std::fs::copy(&file.path.interpret(), &target_file).is_err() {
            failed_files.insert(file.clone());
            continue;
        }
    }

    #[cfg(target_os = "windows")]
    {
        for reg_path in &info.found_registry_keys {
            let mut hives = crate::registry::Hives::default();
            match hives.store_key_from_full_path(&reg_path) {
                Err(_) => {
                    failed_registry.insert(reg_path.to_string());
                }
                Ok(x) if !x.found => {
                    failed_registry.insert(reg_path.to_string());
                }
                _ => {
                    hives.save(&crate::registry::game_registry_backup_file(&target, &name));
                }
            }
        }
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
        match game_file_restoration_target(&file.path, &redirects) {
            Err(_) => {
                failed_files.insert(file.clone());
                continue;
            }
            Ok((_, target)) => {
                let mut p = std::path::PathBuf::from(&target.interpret());
                p.pop();
                if std::fs::create_dir_all(&p.as_path().display().to_string()).is_err() {
                    failed_files.insert(file.clone());
                    continue;
                }
                for i in 0..99 {
                    if std::fs::copy(&file.path.interpret(), &target.interpret()).is_ok() {
                        continue 'outer;
                    }
                    // File might be busy, especially if multiple games share a file,
                    // like in a collection, so retry after a delay:
                    std::thread::sleep(std::time::Duration::from_millis(i * info.game_name.len() as u64));
                }
                failed_files.insert(file.clone());
            }
        }
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
