use crate::config::RootsConfig;
use crate::manifest::{Game, Os, Store};

const CASE_INSENSITIVE_OS: bool = cfg!(target_os = "windows");

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

pub fn parse_path(path: &str, root: &RootsConfig, install_dir: &str) -> String {
    let base = match root.store {
        Store::Steam => format!("{}/steamapps/common/{}", root.path, install_dir),
        Store::Other => format!("{}/**/{}", root.path, install_dir),
    };
    path.replace("<root>", &root.path)
        .replace("<game>", &install_dir)
        .replace("<base>", &base)
        .replace(
            "<home>",
            &dirs::home_dir().unwrap_or_else(|| "~".into()).to_string_lossy(),
        )
        .replace("<storeUserId>", "<skip>")
        .replace("<osUserName>", "<skip>")
        .replace("<winAppData>", "<skip>")
        .replace("<winLocalAppData>", "<skip>")
        .replace("<winPublic>", "<skip>")
        .replace("<winProgramData>", "<skip>")
        .replace("<winDir>", "<skip>")
        .replace("<xdgData>", "<skip>")
        .replace("<xdgConfig>", "<skip>")
        .replace("<regHkcu>", "<skip>")
        .replace("<regHklm>", "<skip>")
}

fn glob_any(path: &str, base: &str) -> Result<glob::Paths, ()> {
    let cwd = std::env::current_dir().map_err(|_| ())?;
    let options = glob::MatchOptions {
        case_sensitive: CASE_INSENSITIVE_OS,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    std::env::set_current_dir(base).map_err(|_| ())?;
    let entries = glob::glob_with(&path, options).map_err(|_| ())?;
    std::env::set_current_dir(cwd).map_err(|_| ())?;
    Ok(entries)
}

pub fn scan_game(game: &Game, name: &str, roots: &[RootsConfig], manifest_dir: &str) -> ScanInfo {
    let mut found_files = std::collections::HashSet::new();
    let found_registry = false;
    let mut roots_to_check: Vec<RootsConfig> = vec![RootsConfig {
        path: "<skip>".to_string(),
        store: Store::Other,
    }];
    roots_to_check.extend(roots.iter().cloned());
    let mut paths_to_check: Vec<String> = vec![];

    for root in roots_to_check {
        if root.path.trim().is_empty() {
            continue;
        }
        if let Some(files) = &game.files {
            let default_install_dir = name.to_string();
            let install_dirs: Vec<_> = match &game.install_dir {
                Some(x) => x.keys().collect(),
                _ => vec![&default_install_dir],
            };
            for (raw_path, constraint) in files {
                if let Some(os) = &constraint.os {
                    if os != &get_os() {
                        continue;
                    }
                }
                if let Some(store) = &constraint.store {
                    if store != &root.store {
                        continue;
                    }
                }
                for install_dir in &install_dirs {
                    let path = parse_path(raw_path, &root, install_dir);
                    if path.contains("<skip>") {
                        continue;
                    }
                    paths_to_check.push(path);
                }
            }
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
        if std::fs::copy(file, target_file).is_err() {
            failed_files.insert(file.to_string());
            continue;
        }
    }

    BackupInfo { failed_files }
}
