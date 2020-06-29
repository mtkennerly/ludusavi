use crate::config::RootsConfig;
use crate::manifest::{Game, Os, Store};

const CASE_INSENSITIVE_OS: bool = cfg!(target_os = "windows");
const SKIP: &str = "<skip>";

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("The manifest file is invalid: {why:?}")]
    ManifestInvalid { why: String },

    #[error("Unable to download an update to the manifest file")]
    ManifestCannotBeUpdated,

    #[error("The config file is invalid: {why:?}")]
    ConfigInvalid { why: String },

    #[error("Cannot prepare the backup target")]
    CannotPrepareBackupTarget,

    #[error("Cannot prepare the backup target")]
    RestorationSourceInvalid,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum OtherError {
    #[error("Cannot determine restoration target")]
    BadRestorationTarget,
}

#[derive(Clone, Debug)]
pub struct ScanInfo {
    pub found_files: std::collections::HashSet<String>,
    pub found_registry: bool,
}

#[derive(Clone, Debug)]
pub struct BackupInfo {
    pub failed_files: std::collections::HashSet<String>,
}

pub fn app_dir() -> std::path::PathBuf {
    let mut path = dirs::home_dir().unwrap();
    path.push(".config");
    path.push("ludusavi");
    path
}

pub fn get_os() -> Os {
    if cfg!(target_os = "linux") {
        Os::Linux
    } else if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
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
) -> std::collections::HashSet<String> {
    let mut paths = std::collections::HashSet::new();

    for install_dir in install_dirs {
        paths.insert(
            path.replace("<root>", &root.path)
                .replace("<game>", &install_dir)
                .replace(
                    "<base>",
                    &match root.store {
                        Store::Steam => format!("{}/steamapps/common/{}", root.path, install_dir),
                        Store::Other => format!("{}/**/{}", root.path, install_dir),
                    },
                )
                .replace(
                    "<home>",
                    &dirs::home_dir().unwrap_or_else(|| SKIP.into()).to_string_lossy(),
                )
                .replace("<storeUserId>", "*")
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
            let prefix = format!("{}/steamapps/compatdata/{}/pfx/drive_c", root.path, steam_id.unwrap());
            paths.insert(
                path.replace("<root>", &root.path)
                    .replace("<game>", &install_dir)
                    .replace("<base>", &format!("{}/steamapps/common/{}", root.path, install_dir))
                    .replace("<home>", &format!("{}/users/steamuser", prefix))
                    .replace("<storeUserId>", "*")
                    .replace("<osUserName>", "steamuser")
                    .replace("<winAppData>", &format!("{}/users/steamuser/AppData/Roaming", prefix))
                    .replace(
                        "<winLocalAppData>",
                        &format!("{}/users/steamuser/AppData/Local", prefix),
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
}

fn glob_any(path: &str, _base: &str) -> Result<glob::Paths, ()> {
    let options = glob::MatchOptions {
        case_sensitive: CASE_INSENSITIVE_OS,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    let entries = glob::glob_with(&path, options).map_err(|_| ())?;
    Ok(entries)
}

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[RootsConfig],
    manifest_dir: &str,
    steam_id: &Option<u32>,
) -> ScanInfo {
    let mut found_files = std::collections::HashSet::new();
    let found_registry = false;

    // Add a dummy root for checking paths without `<root>`.
    let mut roots_to_check: Vec<RootsConfig> = vec![RootsConfig {
        path: SKIP.to_string(),
        store: Store::Other,
    }];
    roots_to_check.extend(roots.iter().cloned());

    let mut paths_to_check = std::collections::HashSet::<String>::new();

    for root in roots_to_check {
        if root.path.trim().is_empty() {
            continue;
        }
        if let Some(files) = &game.files {
            let maybe_proton = get_os() == Os::Linux && root.store == Store::Steam && steam_id.is_some();
            let default_install_dir = name.to_string();
            let install_dirs: Vec<_> = match &game.install_dir {
                Some(x) => x.keys().collect(),
                _ => vec![&default_install_dir],
            };
            for (raw_path, constraint) in files {
                if let Some(os) = &constraint.os {
                    if os != &get_os() && !maybe_proton {
                        continue;
                    }
                }
                if let Some(store) = &constraint.store {
                    if store != &root.store {
                        continue;
                    }
                }
                let candidates = parse_paths(raw_path, &root, &install_dirs, &steam_id);
                for candidate in candidates {
                    if candidate.contains(SKIP) {
                        continue;
                    }
                    paths_to_check.insert(candidate);
                }
            }
        }
        if root.store == Store::Steam && steam_id.is_some() {
            paths_to_check.insert(format!("{}/userdata/*/{}/remote/", root.path, &steam_id.unwrap()));
            paths_to_check.insert(format!(
                "{}/userdata/*/760/remote/{}/screenshots/*.*",
                root.path,
                &steam_id.unwrap()
            ));
        }
    }

    for path in paths_to_check {
        let entries = match glob_any(&path, &manifest_dir) {
            Ok(x) => x,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|r| r.ok()) {
            let plain = entry.to_string_lossy().to_string().replace("\\", "/");
            let p = std::path::Path::new(&plain);
            if p.is_file() {
                found_files.insert(plain);
            } else if p.is_dir() {
                for child in walkdir::WalkDir::new(p)
                    .max_depth(100)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if child.file_type().is_file() {
                        found_files.insert(child.path().display().to_string().replace("\\", "/"));
                    }
                }
            }
        }
    }

    ScanInfo {
        found_files,
        found_registry,
    }
}

pub fn scan_game_for_restoration(name: &str, source: &str) -> ScanInfo {
    let mut found_files = std::collections::HashSet::new();
    let found_registry = false;

    let target_game: std::path::PathBuf = [source, &base64::encode(&name)].iter().collect();
    if target_game.as_path().is_dir() {
        for child in walkdir::WalkDir::new(target_game.as_path())
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if child.file_type().is_file() {
                let source = child.path().display().to_string().replace("\\", "/");
                found_files.insert(source);
            }
        }
    }

    ScanInfo {
        found_files,
        found_registry,
    }
}

pub fn prepare_backup_target(target: &str) -> Result<(), Error> {
    let p = std::path::Path::new(target);
    if p.is_file() {
        std::fs::remove_file(p).map_err(|_| Error::CannotPrepareBackupTarget)?;
    } else if p.is_dir() {
        std::fs::remove_dir_all(p).map_err(|_| Error::CannotPrepareBackupTarget)?;
    }
    std::fs::create_dir_all(p).map_err(|_| Error::CannotPrepareBackupTarget)?;

    Ok(())
}

pub fn back_up_game(info: &ScanInfo, target: &str, name: &str) -> BackupInfo {
    let mut failed_files = std::collections::HashSet::new();

    for file in &info.found_files {
        let target_game: std::path::PathBuf = [target, &base64::encode(&name)].iter().collect();
        if !target_game.as_path().is_dir() && std::fs::create_dir(target_game).is_err() {
            failed_files.insert(file.to_string());
            continue;
        }

        let target_file: std::path::PathBuf = [target, &base64::encode(&name), &base64::encode(&file)]
            .iter()
            .collect();
        if std::fs::copy(&file, &target_file).is_err() {
            failed_files.insert(file.to_string());
            continue;
        }
    }

    BackupInfo { failed_files }
}

pub fn get_target_from_backup_file(file: &str) -> Result<String, Box<dyn std::error::Error>> {
    let base_name = std::path::Path::new(file)
        .file_name()
        .ok_or(OtherError::BadRestorationTarget)?;
    let decoded = base64::decode(base_name.to_string_lossy().as_bytes())?;
    Ok(std::str::from_utf8(&decoded)?.to_string())
}

pub fn restore_game(info: &ScanInfo) -> BackupInfo {
    let mut failed_files = std::collections::HashSet::new();

    for file in &info.found_files {
        match get_target_from_backup_file(&file) {
            Err(_) => {
                failed_files.insert(file.to_string());
                continue;
            }
            Ok(target) => {
                let mut p = std::path::PathBuf::from(&target);
                p.pop();
                if std::fs::create_dir_all(&p.as_path().display().to_string()).is_err() {
                    failed_files.insert(file.to_string());
                    continue;
                }
                if std::fs::copy(file, target).is_err() {
                    failed_files.insert(file.to_string());
                    continue;
                }
            }
        }
    }

    BackupInfo { failed_files }
}
