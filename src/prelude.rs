pub use crate::path::StrictPath;

use once_cell::sync::Lazy;
use std::{path::PathBuf, sync::Mutex};

pub static VERSION: Lazy<&'static str> =
    Lazy::new(|| option_env!("LUDUSAVI_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")));
pub static VARIANT: Option<&'static str> = option_env!("LUDUSAVI_VARIANT");

pub type AnyError = Box<dyn std::error::Error>;

pub const WINDOWS: bool = cfg!(target_os = "windows");
pub const MAC: bool = cfg!(target_os = "macos");
pub const LINUX: bool = cfg!(target_os = "linux");
pub const CASE_INSENSITIVE_OS: bool = WINDOWS || MAC;
pub const SKIP: &str = "<skip>";
pub const APP_DIR_NAME: &str = "ludusavi";
const PORTABLE_FLAG_FILE_NAME: &str = "ludusavi.portable";
pub const INVALID_FILE_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '\0'];

pub static STEAM_DECK: Lazy<bool> = Lazy::new(|| LINUX && StrictPath::new("/home/deck".to_string()).exists());

// NOTE.2022-11-04 not very pretty singleton like global variable
pub static CONFIG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    ManifestInvalid {
        why: String,
    },
    ManifestCannotBeUpdated,
    ConfigInvalid {
        why: String,
    },
    CliUnrecognizedGames {
        games: Vec<String>,
    },
    CliUnableToRequestConfirmation,
    CliBackupIdWithMultipleGames,
    CliInvalidBackupId,
    SomeEntriesFailed,
    CannotPrepareBackupTarget {
        path: StrictPath,
    },
    RestorationSourceInvalid {
        path: StrictPath,
    },
    #[allow(dead_code)]
    RegistryIssue,
    UnableToBrowseFileSystem,
    UnableToOpenDir(StrictPath),
    UnableToOpenUrl(String),
}

pub fn app_dir() -> std::path::PathBuf {
    if let Some(dir) = CONFIG_DIR.lock().unwrap().as_ref() {
        return dir.clone();
    }

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

pub fn filter_map_walkdir(e: Result<walkdir::DirEntry, walkdir::Error>) -> Option<walkdir::DirEntry> {
    if let Err(e) = &e {
        log::warn!("failed to walk: {:?} | {e:?}", e.path());
    }
    e.ok()
}

#[cfg(target_os = "windows")]
pub fn sha1(content: String) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}
