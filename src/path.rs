use std::sync::{Arc, LazyLock, Mutex};

use filetime::FileTime;
use itertools::Itertools;

use crate::{
    prelude::{AnyError, SKIP},
    resource::manifest::{placeholder, Os},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Drive {
    Root,
    Windows(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Canonical {
    Valid(String),
    Unsupported,
    Inaccessible,
}

pub enum CommonPath {
    Config,
    Data,
    DataLocal,
    DataLocalLow,
    Document,
    Home,
    Public,
    SavedGames,
}

impl CommonPath {
    pub fn get(&self) -> Option<&str> {
        fn check_dir(path: Option<std::path::PathBuf>) -> Option<String> {
            Some(path?.to_string_lossy().to_string())
        }

        static CONFIG: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::config_dir()));
        static DATA: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::data_dir()));
        static DATA_LOCAL: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::data_local_dir()));
        static DOCUMENT: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::document_dir()));
        static HOME: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::home_dir()));
        static PUBLIC: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::public_dir()));

        #[cfg(windows)]
        static DATA_LOCAL_LOW: LazyLock<Option<String>> = LazyLock::new(|| {
            known_folders::get_known_folder_path(known_folders::KnownFolder::LocalAppDataLow)
                .map(|x| x.to_string_lossy().trim_end_matches(['/', '\\']).to_string())
        });
        #[cfg(not(windows))]
        static DATA_LOCAL_LOW: Option<String> = None;

        #[cfg(windows)]
        static SAVED_GAMES: LazyLock<Option<String>> = LazyLock::new(|| {
            known_folders::get_known_folder_path(known_folders::KnownFolder::SavedGames)
                .map(|x| x.to_string_lossy().trim_end_matches(['/', '\\']).to_string())
        });
        #[cfg(not(windows))]
        static SAVED_GAMES: Option<String> = None;

        match self {
            Self::Config => CONFIG.as_ref(),
            Self::Data => DATA.as_ref(),
            Self::DataLocal => DATA_LOCAL.as_ref(),
            Self::DataLocalLow => DATA_LOCAL_LOW.as_ref(),
            Self::Document => DOCUMENT.as_ref(),
            Self::Home => HOME.as_ref(),
            Self::Public => PUBLIC.as_ref(),
            Self::SavedGames => SAVED_GAMES.as_ref(),
        }
        .map(|x| x.as_str())
    }

    pub fn get_globbable(&self) -> Option<&str> {
        static CONFIG: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::Config.get().map(|x| StrictPath::new(x).globbable()));
        static DATA: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::Data.get().map(|x| StrictPath::new(x).globbable()));
        static DATA_LOCAL: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::DataLocal.get().map(|x| StrictPath::new(x).globbable()));
        static DATA_LOCAL_LOW: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::DataLocalLow.get().map(|x| StrictPath::new(x).globbable()));
        static DOCUMENT: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::Document.get().map(|x| StrictPath::new(x).globbable()));
        static HOME: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::Home.get().map(|x| StrictPath::new(x).globbable()));
        static PUBLIC: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::Public.get().map(|x| StrictPath::new(x).globbable()));
        static SAVED_GAMES: LazyLock<Option<String>> =
            LazyLock::new(|| CommonPath::SavedGames.get().map(|x| StrictPath::new(x).globbable()));

        match self {
            Self::Config => CONFIG.as_ref(),
            Self::Data => DATA.as_ref(),
            Self::DataLocal => DATA_LOCAL.as_ref(),
            Self::DataLocalLow => DATA_LOCAL_LOW.as_ref(),
            Self::Document => DOCUMENT.as_ref(),
            Self::Home => HOME.as_ref(),
            Self::Public => PUBLIC.as_ref(),
            Self::SavedGames => SAVED_GAMES.as_ref(),
        }
        .map(|x| x.as_str())
    }
}

#[derive(Debug)]
pub enum SetFileTimeError {
    Write(std::io::Error),
    InvalidTimestamp,
}

pub fn render_pathbuf(value: &std::path::Path) -> String {
    value.display().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrictPathError {
    Empty,
    Relative,
    Unmappable,
    Unsupported,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Analysis {
    drive: Option<Drive>,
    parts: Vec<String>,
}

impl Analysis {
    #[cfg(test)]
    fn new(drive: Option<Drive>, parts: Vec<String>) -> Self {
        Self { drive, parts }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnalysisMode {
    Normal,
    Glob,
}

/// This is a wrapper around paths to make it more obvious when we're
/// converting between different representations. This also handles
/// things like `~`.
#[derive(Clone, Default)]
pub struct StrictPath {
    raw: String,
    basis: Option<String>,
    canonical: Arc<Mutex<Option<Canonical>>>,
}

impl Eq for StrictPath {}

impl PartialEq for StrictPath {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.basis == other.basis
    }
}

impl Ord for StrictPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let raw = self.raw.cmp(&other.raw);
        if raw != std::cmp::Ordering::Equal {
            raw
        } else {
            self.basis.cmp(&other.basis)
        }
    }
}

impl PartialOrd for StrictPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for StrictPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
        self.basis.hash(state);
    }
}

impl std::fmt::Debug for StrictPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StrictPath {{ raw: {:?}, basis: {:?} }}", &self.raw, &self.basis)
    }
}

impl StrictPath {
    pub fn new(raw: impl Into<String>) -> Self {
        Self {
            raw: raw.into(),
            basis: None,
            canonical: Arc::new(Mutex::new(None)),
        }
    }

    pub fn relative(raw: impl Into<String>, basis: Option<impl Into<String>>) -> Self {
        Self {
            raw: raw.into(),
            basis: basis.map(|x| x.into()),
            canonical: Arc::new(Mutex::new(None)),
        }
    }

    pub fn rebase(&mut self, basis: &StrictPath) {
        self.basis = Some(basis.render());
        self.invalidate_cache();
    }

    pub fn cwd() -> Self {
        Self::from(std::env::current_dir().unwrap())
    }

    pub fn reset(&mut self, raw: String) {
        self.raw = raw;
        self.invalidate_cache();
    }

    pub fn equivalent(&self, other: &Self) -> bool {
        self.interpret() == other.interpret()
    }

    fn from_std_path_buf(path_buf: &std::path::Path) -> Self {
        Self::new(render_pathbuf(path_buf))
    }

    pub fn as_std_path_buf(&self) -> Result<std::path::PathBuf, std::io::Error> {
        Ok(std::path::PathBuf::from(&self.interpret().map_err(|_| {
            std::io::Error::other(format!("Cannot interpret path: {:?}", &self))
        })?))
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// For any paths that we store the entire time the GUI is running, like in the config,
    /// we sometimes want to refresh in case we have stale data.
    pub fn invalidate_cache(&self) {
        let mut cached = self.canonical.lock().unwrap();
        *cached = None;
    }

    fn analyze(&self) -> Analysis {
        self.analyze_with_mode(AnalysisMode::Normal)
    }

    fn analyze_with_mode(&self, mode: AnalysisMode) -> Analysis {
        use typed_path::{
            Utf8TypedComponent as Component, Utf8TypedPath as TypedPath, Utf8UnixComponent as UComponent,
            Utf8WindowsComponent as WComponent, Utf8WindowsPrefix as WindowsPrefix,
        };

        let mut analysis = Analysis::default();

        // `\\?\UNC\server\share/foo` will end up with `share/foo` as the share name.
        macro_rules! correct_windows_slashes {
            ($start:expr, $server:expr, $share:expr) => {{
                let mut share_parts: Vec<_> = $share.split('/').collect();
                while share_parts.len() > 1 {
                    analysis.parts.push(share_parts.remove(1).to_string());
                }

                let share = share_parts.pop().unwrap_or($share);
                match $server {
                    Some(server) => format!(r"{}{}\{}", $start, server, share),
                    None => format!(r"{}{}", $start, share),
                }
            }};
        }

        for (i, component) in TypedPath::derive(self.raw.trim()).components().enumerate() {
            match component {
                Component::Windows(WComponent::Prefix(prefix)) => {
                    let mapped = match prefix.kind() {
                        WindowsPrefix::Verbatim(id) => match mode {
                            AnalysisMode::Normal => correct_windows_slashes!(r"\\?\", None::<&str>, id),
                            AnalysisMode::Glob => correct_windows_slashes!(r"\\", None::<&str>, id),
                        },
                        WindowsPrefix::VerbatimUNC(server, share) => match mode {
                            AnalysisMode::Normal => correct_windows_slashes!(r"\\?\UNC\", Some(server), share),
                            AnalysisMode::Glob => correct_windows_slashes!(r"\\", Some(server), share),
                        },
                        WindowsPrefix::VerbatimDisk(id) => format!("{}:", id.to_ascii_uppercase()),
                        WindowsPrefix::DeviceNS(id) => format!(r"\\.\{id}"),
                        WindowsPrefix::UNC(server, share) => correct_windows_slashes!(r"\\", Some(server), share),
                        WindowsPrefix::Disk(id) => format!("{}:", id.to_ascii_uppercase()),
                    };
                    analysis.drive = Some(Drive::Windows(mapped));
                }
                Component::Unix(UComponent::RootDir) | Component::Windows(WComponent::RootDir) => {
                    if i == 0 {
                        analysis.drive = Some(Drive::Root);
                    }
                }
                Component::Unix(UComponent::CurDir) | Component::Windows(WComponent::CurDir) => {
                    if i == 0 {
                        if let Some(basis) = &self.basis {
                            analysis = Self::new(basis.clone()).analyze_with_mode(mode);
                        }
                    }
                }
                Component::Unix(UComponent::ParentDir) | Component::Windows(WComponent::ParentDir) => {
                    if i == 0 {
                        if let Some(basis) = &self.basis {
                            analysis = Self::new(basis.clone()).analyze_with_mode(mode);
                        }
                    }
                    analysis.parts.pop();
                }
                Component::Unix(UComponent::Normal(part)) | Component::Windows(WComponent::Normal(part)) => {
                    if i == 0 {
                        let mapped = match part {
                            "~" | placeholder::HOME => CommonPath::Home.get(),
                            placeholder::XDG_CONFIG => CommonPath::Config.get(),
                            placeholder::XDG_DATA | placeholder::WIN_APP_DATA => CommonPath::Data.get(),
                            placeholder::WIN_LOCAL_APP_DATA => CommonPath::DataLocal.get(),
                            placeholder::WIN_LOCAL_APP_DATA_LOW => CommonPath::DataLocalLow.get(),
                            placeholder::WIN_DOCUMENTS => CommonPath::Document.get(),
                            placeholder::WIN_PUBLIC => CommonPath::Public.get(),
                            placeholder::WIN_PROGRAM_DATA => Some("C:/ProgramData"),
                            placeholder::WIN_DIR => Some("C:/Windows"),
                            _ => None,
                        };

                        if let Some(mapped) = mapped {
                            analysis = Self::new(mapped.to_string()).analyze_with_mode(mode);
                            continue;
                        } else if let Some(basis) = &self.basis {
                            analysis = Self::new(basis.clone()).analyze_with_mode(mode);
                        }
                    }

                    if part == placeholder::OS_USER_NAME {
                        analysis.parts.push(crate::prelude::OS_USERNAME.to_string());
                        continue;
                    }

                    // On Unix, Unix-style path segments may contain a backslash.
                    // On Windows, verbatim paths can end up with internal forward slashes.
                    if part.contains(['/', '\\']) {
                        for part in part.split(['/', '\\']) {
                            if !part.trim().is_empty() {
                                analysis.parts.push(part.to_string());
                            }
                        }
                    } else {
                        analysis.parts.push(part.to_string());
                    }
                }
            }
        }

        analysis
    }

    fn display(&self) -> String {
        if self.raw.is_empty() {
            return "".to_string();
        }

        match self.analyze() {
            Analysis {
                drive: Some(Drive::Root),
                parts,
            } => format!("/{}", parts.join("/")),
            Analysis {
                drive: Some(Drive::Windows(id)),
                parts,
            } => {
                format!("{}/{}", id, parts.join("/"))
            }
            Analysis { drive: None, parts } => parts.join("/"),
        }
    }

    fn access(&self) -> Result<String, StrictPathError> {
        self.access_with_mode(AnalysisMode::Normal)
    }

    fn access_with_mode(&self, mode: AnalysisMode) -> Result<String, StrictPathError> {
        if cfg!(target_os = "windows") {
            self.access_windows_with_mode(mode)
        } else {
            self.access_nonwindows_with_mode(mode)
        }
    }

    #[cfg(test)]
    fn access_windows(&self) -> Result<String, StrictPathError> {
        self.access_windows_with_mode(AnalysisMode::Normal)
    }

    fn access_windows_with_mode(&self, mode: AnalysisMode) -> Result<String, StrictPathError> {
        if self.raw.is_empty() {
            return Err(StrictPathError::Empty);
        }

        let analysis = self.analyze_with_mode(mode);
        if analysis.parts.iter().any(|x| x.contains(':')) {
            return Err(StrictPathError::Unsupported);
        }

        match analysis {
            Analysis {
                drive: Some(Drive::Root),
                ..
            } => Err(StrictPathError::Unsupported),
            Analysis {
                drive: Some(Drive::Windows(id)),
                parts,
            } => Ok(format!("{}\\{}", id, parts.join("\\"))),
            Analysis { drive: None, parts } => match &self.basis {
                Some(basis) => Ok(format!("{}\\{}", basis, parts.join("\\"))),
                None => Err(StrictPathError::Relative),
            },
        }
    }

    #[cfg(test)]
    fn access_nonwindows(&self) -> Result<String, StrictPathError> {
        self.access_nonwindows_with_mode(AnalysisMode::Normal)
    }

    fn access_nonwindows_with_mode(&self, mode: AnalysisMode) -> Result<String, StrictPathError> {
        if self.raw.is_empty() {
            return Err(StrictPathError::Empty);
        }

        match self.analyze_with_mode(mode) {
            Analysis {
                drive: Some(Drive::Root),
                parts,
            } => Ok(format!("/{}", parts.join("/"))),
            Analysis {
                drive: Some(Drive::Windows(_)),
                ..
            } => Err(StrictPathError::Unsupported),
            Analysis { drive: None, parts } => match &self.basis {
                Some(basis) => Ok(format!("{}/{}", basis, parts.join("/"))),
                None => Err(StrictPathError::Relative),
            },
        }
    }

    /// Simplify the drive segment to avoid `\\?` in Windows paths,
    /// and glob-escape the remainder of the path.
    pub fn globbable(&self) -> String {
        if self.raw.is_empty() {
            return "".to_string();
        }

        match self.analyze_with_mode(AnalysisMode::Glob) {
            Analysis {
                drive: Some(Drive::Root),
                parts,
            } => format!("/{}", globset::escape(&parts.join("/"))),
            Analysis {
                drive: Some(Drive::Windows(id)),
                parts,
            } => {
                format!("{}/{}", id, globset::escape(&parts.join("/")))
            }
            Analysis { drive: None, parts } => globset::escape(&parts.join("/")),
        }
    }

    fn canonical(&self) -> Canonical {
        let mut cached = self.canonical.lock().unwrap();

        match cached.as_ref() {
            Some(canonical) => canonical.clone(),
            None => match self.access() {
                Err(_) => Canonical::Unsupported,
                Ok(path) => match std::fs::canonicalize(path) {
                    Err(_) => Canonical::Inaccessible,
                    Ok(path) => {
                        let path = path.to_string_lossy().to_string();
                        *cached = Some(Canonical::Valid(path.clone()));
                        Canonical::Valid(path)
                    }
                },
            },
        }
    }

    pub fn interpret(&self) -> Result<String, StrictPathError> {
        match self.canonical() {
            Canonical::Valid(path) => match StrictPath::new(path).access() {
                Ok(path) => Ok(path),
                Err(_) => {
                    // This shouldn't be able to fail if we already have a canonical path,
                    // but we have a fallback just in case.
                    Ok(self.display())
                }
            },
            Canonical::Unsupported => Err(StrictPathError::Unsupported),
            Canonical::Inaccessible => self.access(),
        }
    }

    /// This is for a special case when we're scanning a dummy root.
    pub fn interpret_unless_skip(&self) -> Result<String, StrictPathError> {
        if self.raw == SKIP {
            Ok(SKIP.to_string())
        } else {
            self.interpret()
        }
    }

    pub fn interpreted(&self) -> Result<Self, StrictPathError> {
        Ok(Self {
            raw: self.interpret()?,
            basis: self.basis.clone(),
            canonical: self.canonical.clone(),
        })
    }

    pub fn render(&self) -> String {
        // We don't want to use `interpret` or `canonical` internally here,
        // because we may need to display a symlink path without traversing it.
        self.display()
    }

    pub fn rendered(&self) -> Self {
        Self {
            raw: self.render(),
            basis: self.basis.clone(),
            canonical: self.canonical.clone(),
        }
    }

    pub fn resolve(&self) -> String {
        if let Ok(access) = self.access() {
            access
        } else {
            self.raw().into()
        }
    }

    pub fn try_resolve(&self) -> Result<String, StrictPathError> {
        self.access()
    }

    pub fn is_file(&self) -> bool {
        self.as_std_path_buf().map(|x| x.is_file()).unwrap_or_default()
    }

    pub fn is_dir(&self) -> bool {
        self.as_std_path_buf().map(|x| x.is_dir()).unwrap_or_default()
    }

    pub fn exists(&self) -> bool {
        self.is_file() || self.is_dir()
    }

    pub fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
        self.as_std_path_buf()?.metadata()
    }

    pub fn get_mtime(&self) -> std::io::Result<std::time::SystemTime> {
        self.metadata()?.modified()
    }

    /// Zips don't store time zones, so we normalize to/from UTC.
    pub fn get_mtime_zip(&self) -> Result<zip::DateTime, AnyError> {
        use chrono::{Datelike, Timelike};

        let mtime: chrono::DateTime<chrono::Utc> = self.get_mtime()?.into();

        // Zip doesn't support years before 1980,
        // and this is probably just a default Unix timestamp anyway,
        // so we round up.
        if mtime.year() < 1980 {
            return Ok(zip::DateTime::default());
        }

        let converted = zip::DateTime::from_date_and_time(
            mtime.year() as u16,
            mtime.month() as u8,
            mtime.day() as u8,
            mtime.hour() as u8,
            mtime.minute() as u8,
            mtime.second() as u8,
        );

        match converted {
            Ok(x) => Ok(x),
            Err(_) => Err(Box::new(std::io::Error::other("Failed to get mtime in zip format"))),
        }
    }

    pub fn set_mtime(&self, mtime: std::time::SystemTime) -> Result<(), std::io::Error> {
        filetime::set_file_mtime(self.as_std_path_buf()?, FileTime::from_system_time(mtime))
    }

    /// Zips don't store time zones, so we normalize to/from UTC.
    pub fn set_mtime_zip(&self, mtime: zip::DateTime) -> Result<(), SetFileTimeError> {
        let naive_mtime = chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(mtime.year() as i32, mtime.month() as u32, mtime.day() as u32)
                .ok_or(SetFileTimeError::InvalidTimestamp)?,
            chrono::NaiveTime::from_hms_opt(mtime.hour() as u32, mtime.minute() as u32, mtime.second() as u32)
                .ok_or(SetFileTimeError::InvalidTimestamp)?,
        );
        self.set_mtime(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_mtime, chrono::Utc).into())
            .map_err(SetFileTimeError::Write)
    }

    pub fn remove(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_file() {
            std::fs::remove_file(self.as_std_path_buf()?)?;
        } else if self.is_dir() {
            std::fs::remove_dir_all(self.as_std_path_buf()?)?;
        }
        Ok(())
    }

    pub fn joined(&self, other: impl AsRef<str>) -> Self {
        Self {
            raw: format!("{}/{}", &self.raw, other.as_ref()).replace('\\', "/"),
            basis: self.basis.clone(),
            canonical: Arc::new(Mutex::new(None)),
        }
    }

    pub fn popped(&self) -> Self {
        let raw = match self.analyze() {
            Analysis {
                drive: Some(Drive::Root),
                mut parts,
            } => {
                parts.pop();
                format!("/{}", parts.join("/"))
            }
            Analysis {
                drive: Some(Drive::Windows(id)),
                mut parts,
            } => {
                parts.pop();
                format!("{}/{}", id, parts.join("/"))
            }
            Analysis { drive: None, mut parts } => {
                parts.pop();
                match &self.basis {
                    Some(basis) => format!("{}/{}", basis, parts.join("/")),
                    None => parts.join("/"),
                }
            }
        };

        Self::new(raw)
    }

    pub fn replace(&self, find: &Self, new: &Self) -> Self {
        if find.raw.trim().is_empty() || new.raw.trim().is_empty() {
            return self.clone();
        }

        let us = self.analyze();
        let find = find.analyze();

        if us.drive != find.drive {
            return self.clone();
        }

        let mut tail = vec![];
        for pair in us.parts.into_iter().zip_longest(find.parts.into_iter()) {
            match pair {
                itertools::EitherOrBoth::Both(old, find) => {
                    if old != find {
                        return self.clone();
                    }
                }
                itertools::EitherOrBoth::Left(old) => {
                    tail.push(old);
                }
                itertools::EitherOrBoth::Right(..) => {
                    return self.clone();
                }
            }
        }

        let mut new = new.analyze();
        new.parts.extend(tail);
        new.into()
    }

    pub fn create(&self) -> std::io::Result<std::fs::File> {
        std::fs::File::create(self.as_std_path_buf()?)
    }

    pub fn open(&self) -> std::io::Result<std::fs::File> {
        std::fs::File::open(self.as_std_path_buf()?)
    }

    pub fn write_with_content(&self, content: &str) -> std::io::Result<()> {
        std::fs::write(self.as_std_path_buf()?, content.as_bytes())
    }

    pub fn move_to(&self, new_path: &StrictPath) -> std::io::Result<()> {
        std::fs::rename(self.as_std_path_buf()?, new_path.as_std_path_buf()?)
    }

    #[cfg(not(windows))]
    pub fn copy_to(&self, target: &StrictPath) -> std::io::Result<u64> {
        std::fs::copy(self.as_std_path_buf()?, target.as_std_path_buf()?)
    }

    // Reimplementation of standard library:
    // https://github.com/rust-lang/rust/blob/4d30011f6c616be074ba655a75e5d55441232bbb/library/std/src/sys/fs/windows.rs#L1515
    // but with `COPY_FILE_ALLOW_DECRYPTED_DESTINATION` flag set.
    #[cfg(target_os = "windows")]
    pub fn copy_to(&self, target: &StrictPath) -> std::io::Result<u64> {
        use windows::{
            core::PCWSTR,
            Win32::{Foundation::HANDLE, Storage::FileSystem::*},
        };

        fn prepare_verbatim_path(path: &StrictPath) -> Result<Vec<u16>, std::io::Error> {
            use typed_path::Utf8WindowsPrefix::*;
            let interpreted = path
                .interpret()
                .map_err(|_| std::io::Error::other(format!("Cannot interpret path: {path:?}")))?;
            let path_buf = typed_path::Utf8WindowsPath::new(&interpreted);
            let (trim, prefix) = match path_buf.components().prefix_kind() {
                Some(DeviceNS(_)) => (r"\\.\".len(), r"\\?\"),
                Some(UNC(_, _)) => (r"\\".len(), r"\\?\UNC\"),
                Some(Disk(_)) => (0, r"\\?\"),
                _ => (0, ""),
            };

            Ok(prefix
                .encode_utf16()
                .chain(interpreted[trim..].encode_utf16())
                .chain(std::iter::once(0))
                .collect())
        }
        let src = prepare_verbatim_path(self)?;
        let dst = prepare_verbatim_path(target)?;

        unsafe extern "system" fn callback(
            _totalfilesize: i64,
            _totalbytestransferred: i64,
            _streamsize: i64,
            streambytestransferred: i64,
            dwstreamnumber: u32,
            _dwcallbackreason: LPPROGRESS_ROUTINE_CALLBACK_REASON,
            _hsourcefile: HANDLE,
            _hdestinationfile: HANDLE,
            lpdata: *const core::ffi::c_void,
        ) -> COPYPROGRESSROUTINE_PROGRESS {
            if dwstreamnumber == 1 {
                *(lpdata as *mut i64) = streambytestransferred;
            }
            PROGRESS_CONTINUE
        }

        let mut size = 0i64;
        unsafe {
            CopyFileExW(
                PCWSTR(src.as_ptr()),
                PCWSTR(dst.as_ptr()),
                Some(callback),
                Some((&raw mut size) as *mut _),
                None,
                COPY_FILE_ALLOW_DECRYPTED_DESTINATION,
            )
            .map(|_| size as u64)
            .map_err(|_| std::io::Error::last_os_error())
        }
    }

    pub fn create_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.as_std_path_buf()?)?;
        Ok(())
    }

    pub fn create_parent_dir(&self) -> std::io::Result<()> {
        let mut pb = self.as_std_path_buf()?;
        pb.pop();
        std::fs::create_dir_all(&pb)?;
        Ok(())
    }

    pub fn read_dir(&self) -> std::io::Result<std::fs::ReadDir> {
        self.as_std_path_buf()?.read_dir()
    }

    pub fn file_stem(&self) -> Option<String> {
        self.as_std_path_buf()
            .ok()?
            .file_stem()
            .map(|x| x.to_string_lossy().to_string())
    }

    pub fn parent(&self) -> Option<Self> {
        let popped = self.popped();
        (self != &popped).then_some(popped)
    }

    pub fn parent_if_file(&self) -> Result<Self, StrictPathError> {
        if self.is_file() {
            Ok(self.popped())
        } else {
            Ok(self.clone())
        }
    }

    pub fn parent_raw(&self) -> Option<Self> {
        std::path::PathBuf::from(&self.raw).parent().map(Self::from)
    }

    pub fn leaf(&self) -> Option<String> {
        self.as_std_path_buf()
            .ok()?
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
    }

    pub fn is_absolute(&self) -> bool {
        use typed_path::{
            Utf8TypedComponent as Component, Utf8TypedPath as TypedPath, Utf8UnixComponent as UComponent,
            Utf8WindowsComponent as WComponent,
        };

        if let Some(component) = TypedPath::derive(&self.raw).components().next() {
            match component {
                Component::Windows(WComponent::Prefix(_) | WComponent::RootDir)
                | Component::Unix(UComponent::RootDir) => {
                    return true;
                }
                Component::Windows(WComponent::CurDir | WComponent::ParentDir)
                | Component::Unix(UComponent::CurDir | UComponent::ParentDir) => {
                    return false;
                }
                Component::Windows(WComponent::Normal(_)) | Component::Unix(UComponent::Normal(_)) => {}
            }
        }

        false
    }

    pub fn copy_to_path(&self, context: &str, target_file: &StrictPath) -> Result<(), std::io::Error> {
        log::trace!("[{context}] copy {:?} -> {:?}", &self, &target_file);

        if let Err(e) = target_file.create_parent_dir() {
            log::error!(
                "[{context}] unable to create parent directories: {:?} -> {:?} | {e}",
                &self,
                &target_file
            );
            return Err(e);
        }

        if let Err(e) = target_file.unset_readonly() {
            log::warn!(
                "[{context}] failed to unset read-only on target: {:?} | {e}",
                &target_file
            );
            return Err(std::io::Error::other("Failed to unset read-only"));
        }

        if let Err(e) = self.copy_to(target_file) {
            log::error!("[{context}] unable to copy: {:?} -> {:?} | {e}", &self, &target_file);
            return Err(e);
        }

        // We do this again in case the source file was also read-only.
        if let Err(e) = target_file.unset_readonly() {
            log::warn!(
                "[{context}] failed to unset read-only on target after copy: {:?} | {e}",
                &target_file
            );
            return Err(std::io::Error::other("Failed to unset read-only"));
        }

        let mtime = match self.get_mtime() {
            Ok(x) => x,
            Err(e) => {
                log::error!(
                    "[{context}] unable to get modification time: {:?} -> {:?} | {e}",
                    &self,
                    &target_file,
                );
                return Err(e);
            }
        };
        if let Err(e) = target_file.set_mtime(mtime) {
            log::error!(
                "[{context}] unable to set modification time: {:?} -> {:?} to {mtime:#?} | {e}",
                &self,
                &target_file,
            );
            return Err(e);
        }

        Ok(())
    }

    /// This splits a path into a drive (e.g., `C:` or `\\?\D:`) and the remainder.
    /// This is only used during backups to record drives in mapping.yaml,
    /// so relative paths should have already been filtered out.
    pub fn split_drive(&self) -> (String, String) {
        match self.analyze() {
            Analysis {
                drive: Some(Drive::Root),
                parts,
            } => ("".to_string(), parts.join("/")),
            Analysis {
                drive: Some(Drive::Windows(id)),
                parts,
            } => (id, parts.join("/")),
            Analysis { drive: None, .. } => {
                log::error!("Unreachable state: unable to split drive of path: {}", &self.raw);
                unreachable!()
            }
        }
    }

    pub fn unset_readonly(&self) -> Result<(), AnyError> {
        if !self.is_file() {
            return Ok(());
        }

        let subject = self.as_std_path_buf()?;
        let mut perms = std::fs::metadata(&subject)?.permissions();

        if perms.readonly() {
            #[cfg(windows)]
            {
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                perms.set_mode(perms.mode() | 0b110_000_000);
            }

            std::fs::set_permissions(&subject, perms)?;
        }

        Ok(())
    }

    pub fn is_prefix_of(&self, other: &Self) -> bool {
        let us = self.analyze();
        let them = other.analyze();

        if us.drive != them.drive {
            return false;
        }

        if us.parts.len() >= them.parts.len() {
            return false;
        }

        us.parts.iter().zip(them.parts.iter()).all(|(us, them)| us == them)
    }

    pub fn nearest_prefix(&self, others: Vec<StrictPath>) -> Option<StrictPath> {
        let us = self.analyze();
        let us_count = us.parts.len();

        let mut nearest = None;
        let mut nearest_len = 0;
        for other in others {
            let them = other.analyze();
            let them_len = them.parts.len();

            if us.drive != them.drive || us_count <= them_len {
                continue;
            }
            if us.parts.iter().zip(them.parts.iter()).all(|(us, them)| us == them) && them_len > nearest_len {
                nearest = Some(other);
                nearest_len = them_len;
            }
        }
        nearest
    }

    pub fn suffix_for(&self, other: &Self) -> Option<String> {
        self.is_prefix_of(other).then(|| {
            other
                .render()
                .replacen(&self.render(), "", 1)
                .trim_start_matches(['/', '\\'])
                .to_string()
        })
    }

    pub fn tail_for(&self, prefix: &Self) -> Option<Vec<String>> {
        let us = self.analyze();
        let them = prefix.analyze();

        if us.drive != them.drive {
            return None;
        }

        if us.parts.len() < them.parts.len() {
            return None;
        }

        let mut tail = vec![];
        for part in us.parts.iter().zip_longest(&them.parts) {
            match part {
                itertools::EitherOrBoth::Both(us, them) => {
                    if us != them {
                        return None;
                    }
                }
                itertools::EitherOrBoth::Left(part) => {
                    tail.push(part.clone());
                }
                itertools::EitherOrBoth::Right(_) => {
                    return None;
                }
            }
        }

        (!tail.is_empty()).then_some(tail)
    }

    pub fn glob(&self) -> Vec<StrictPath> {
        self.glob_case_sensitive(Os::HOST.is_case_sensitive())
    }

    pub fn glob_case_sensitive(&self, case_sensitive: bool) -> Vec<StrictPath> {
        if self.raw.trim().is_empty() {
            return vec![];
        }

        let options = globetter::MatchOptions {
            case_sensitive,
            require_literal_separator: true,
            require_literal_leading_dot: false,
            follow_links: true,
        };
        let rendered = self.render();
        match globetter::glob_with(&rendered, options) {
            Ok(xs) => xs
                .filter_map(|r| {
                    if let Err(e) = &r {
                        log::trace!("Glob error 2: {rendered} | {e}");
                    }
                    r.ok()
                })
                .map(StrictPath::from)
                .collect(),
            Err(e) => {
                log::trace!("Glob error 1: {rendered} | {e}");
                vec![]
            }
        }
    }

    pub fn same_content(&self, other: &StrictPath) -> bool {
        self.try_same_content(other).unwrap_or(false)
    }

    pub fn try_same_content(&self, other: &StrictPath) -> Result<bool, Box<dyn std::error::Error>> {
        use std::io::Read;

        let f1 = self.open()?;
        let mut f1r = std::io::BufReader::new(f1);
        let f2 = other.open()?;
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

    pub fn read(&self) -> Option<String> {
        self.try_read().ok()
    }

    pub fn try_read(&self) -> Result<String, AnyError> {
        Ok(std::fs::read_to_string(std::path::Path::new(&self.as_std_path_buf()?))?)
    }

    pub fn size(&self) -> u64 {
        match self.metadata() {
            Ok(m) => m.len(),
            _ => 0,
        }
    }

    pub fn sha1(&self) -> String {
        self.try_sha1().unwrap_or_default()
    }

    pub fn try_sha1(&self) -> Result<String, Box<dyn std::error::Error>> {
        use std::io::Read;

        use sha1::Digest;

        let mut hasher = sha1::Sha1::new();

        let file = self.open()?;
        let mut reader = std::io::BufReader::new(file);

        let mut buffer = [0; 1024];
        loop {
            let read = reader.read(&mut buffer[..])?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

impl From<&str> for StrictPath {
    fn from(source: &str) -> Self {
        StrictPath::new(source.to_string())
    }
}

impl From<String> for StrictPath {
    fn from(source: String) -> Self {
        StrictPath::new(source)
    }
}

impl From<&String> for StrictPath {
    fn from(source: &String) -> Self {
        StrictPath::new(source.clone())
    }
}

impl From<std::path::PathBuf> for StrictPath {
    fn from(source: std::path::PathBuf) -> Self {
        StrictPath::from_std_path_buf(&source)
    }
}

impl From<&std::path::Path> for StrictPath {
    fn from(source: &std::path::Path) -> Self {
        StrictPath::from_std_path_buf(source)
    }
}

impl From<&walkdir::DirEntry> for StrictPath {
    fn from(source: &walkdir::DirEntry) -> Self {
        StrictPath::from_std_path_buf(source.path())
    }
}

impl From<&StrictPath> for StrictPath {
    fn from(source: &StrictPath) -> Self {
        StrictPath::relative(source.raw.clone(), source.basis.clone())
    }
}

impl From<Analysis> for StrictPath {
    fn from(value: Analysis) -> Self {
        let raw = match value {
            Analysis {
                drive: Some(Drive::Root),
                parts,
            } => format!("/{}", parts.join("/")),
            Analysis {
                drive: Some(Drive::Windows(id)),
                parts,
            } => {
                format!("{}/{}", id, parts.join("/"))
            }
            Analysis { drive: None, parts } => parts.join("/"),
        };

        Self::new(raw)
    }
}

impl serde::Serialize for StrictPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for StrictPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer).map(|raw: String| StrictPath::new(raw))
    }
}

impl schemars::JsonSchema for StrictPath {
    fn schema_name() -> String {
        "FilePath".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }

    fn is_referenceable() -> bool {
        true
    }
}

#[cfg_attr(not(target_os = "windows"), allow(unused))]
pub fn is_raw_path_relative(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.is_empty() || path == "." || path == ".." || path.starts_with("./") || path.starts_with("../")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{repo, s};

    fn home() -> String {
        CommonPath::Home.get().unwrap().to_string()
    }

    mod strict_path {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn can_check_if_it_is_a_file() {
            assert!(StrictPath::new(format!("{}/README.md", repo())).is_file());
            assert!(!StrictPath::new(repo()).is_file());
        }

        #[test]
        fn can_check_if_it_is_a_directory() {
            assert!(StrictPath::new(repo()).is_dir());
            assert!(!StrictPath::new(format!("{}/README.md", repo())).is_dir());
        }

        #[test]
        fn can_check_if_it_exists() {
            assert!(StrictPath::new(repo()).exists());
            assert!(StrictPath::new(format!("{}/README.md", repo())).exists());
            assert!(!StrictPath::new(format!("{}/fake", repo())).exists());
        }

        #[test]
        fn can_split_drive_for_windows_path() {
            assert_eq!((s("C:"), s("foo/bar")), StrictPath::new(s("C:/foo/bar")).split_drive());
        }

        #[test]
        fn can_split_drive_for_nonwindows_path() {
            assert_eq!((s(""), s("foo/bar")), StrictPath::new(s("/foo/bar")).split_drive());
        }

        #[test]
        fn is_prefix_of() {
            assert!(StrictPath::new(s("/")).is_prefix_of(&StrictPath::new(s("/foo"))));
            assert!(StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/foo/bar"))));
            assert!(!StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/f"))));
            assert!(!StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/foo"))));
            assert!(!StrictPath::new(s("/foo")).is_prefix_of(&StrictPath::new(s("/bar"))));
            assert!(!StrictPath::new(s("")).is_prefix_of(&StrictPath::new(s("/foo"))));
        }

        #[test]
        fn is_prefix_of_with_windows_drive_letters() {
            assert!(StrictPath::new(s(r#"C:"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"C:/"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"C:\"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
        }

        #[test]
        fn is_prefix_of_with_unc_drives() {
            assert!(!StrictPath::new(s(r#"\\?\C:\foo"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"\\?\C:\foo"#)).is_prefix_of(&StrictPath::new(s("C:/foo/bar"))));
            assert!(!StrictPath::new(s(r#"\\remote\foo"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
            assert!(StrictPath::new(s(r#"C:\"#)).is_prefix_of(&StrictPath::new(s("C:/foo"))));
        }

        #[test]
        fn nearest_prefix() {
            assert_eq!(
                Some(StrictPath::new(s(r#"/foo/bar"#))),
                StrictPath::new(s(r#"/foo/bar/baz"#)).nearest_prefix(vec![
                    StrictPath::new(s(r#"/foo"#)),
                    StrictPath::new(s(r#"/foo/bar"#)),
                    StrictPath::new(s(r#"/foo/bar/baz"#)),
                ])
            );
            assert_eq!(
                None,
                StrictPath::new(s(r#"/foo/bar/baz"#)).nearest_prefix(vec![
                    StrictPath::new(s(r#"/fo"#)),
                    StrictPath::new(s(r#"/fooo"#)),
                    StrictPath::new(s(r#"/foo/bar/baz"#)),
                ])
            );
        }

        #[test]
        fn checks_if_files_are_identical() {
            assert!(StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo()))
                .same_content(&StrictPath::new(format!("{}/tests/root2/game2/file1.txt", repo()))));
            assert!(
                !StrictPath::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()))
                    .same_content(&StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo())))
            );
            assert!(!StrictPath::new(format!("{}/tests/root1/game1/file1.txt", repo()))
                .same_content(&StrictPath::new(format!("{}/nonexistent.txt", repo()))));
        }

        #[test]
        fn tries_to_check_if_files_are_identical() {
            assert!(StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo()))
                .try_same_content(&StrictPath::new(format!("{}/tests/root2/game2/file1.txt", repo())))
                .unwrap());
            assert!(
                !StrictPath::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()))
                    .try_same_content(&StrictPath::new(format!("{}/tests/root2/game1/file1.txt", repo())))
                    .unwrap()
            );
            assert!(StrictPath::new(format!("{}/tests/root1/game1/file1.txt", repo()))
                .try_same_content(&StrictPath::new(format!("{}/nonexistent.txt", repo())))
                .is_err());
        }

        #[test]
        fn can_replace() {
            // Identical
            assert_eq!(
                StrictPath::new("/foo"),
                StrictPath::new("/foo").replace(&StrictPath::new("/foo"), &StrictPath::new("/foo")),
            );

            // Match
            assert_eq!(
                StrictPath::new("/baz/bar"),
                StrictPath::new("/foo/bar").replace(&StrictPath::new("/foo"), &StrictPath::new("/baz")),
            );

            // Mismatch
            assert_eq!(
                StrictPath::new("/a"),
                StrictPath::new("/a").replace(&StrictPath::new("/ab"), &StrictPath::new("/ac")),
            );

            // Linux to Windows
            assert_eq!(
                StrictPath::new("C:/foo"),
                StrictPath::new("/foo").replace(&StrictPath::new("/"), &StrictPath::new("C:")),
            );

            // Windows to Linux
            assert_eq!(
                StrictPath::new("/foo"),
                StrictPath::new("C:/foo").replace(&StrictPath::new("C:/"), &StrictPath::new("/")),
            );

            // Empty - original
            assert_eq!(
                StrictPath::new(""),
                StrictPath::new("").replace(&StrictPath::new("/foo"), &StrictPath::new("/bar")),
            );

            // Empty - find
            assert_eq!(
                StrictPath::new("/foo"),
                StrictPath::new("/foo").replace(&StrictPath::new(""), &StrictPath::new("/bar")),
            );

            // Empty - new
            assert_eq!(
                StrictPath::new("/foo"),
                StrictPath::new("/foo").replace(&StrictPath::new("/foo"), &StrictPath::new("")),
            );
        }

        #[test]
        fn can_determine_tail_for() {
            let base = StrictPath::new("/foo/bar/baz");

            assert_eq!(
                Some(vec!["baz".to_string()]),
                base.tail_for(&StrictPath::new("/foo/bar")),
            );
            assert_eq!(
                Some(vec!["bar".to_string(), "baz".to_string()]),
                base.tail_for(&StrictPath::new("/foo")),
            );
            assert_eq!(
                Some(vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]),
                base.tail_for(&StrictPath::new("/")),
            );

            assert_eq!(None, base.tail_for(&StrictPath::new("C:/foo")));
            assert_eq!(None, base.tail_for(&StrictPath::new("/quux")));
            assert_eq!(None, base.tail_for(&StrictPath::new("")));
        }
    }

    mod strict_path_display_and_access {
        use super::*;

        use pretty_assertions::assert_eq;

        fn analysis(drive: Drive) -> Analysis {
            Analysis {
                drive: Some(drive),
                parts: vec!["foo".to_string(), "bar".to_string()],
            }
        }

        fn analysis_3(drive: Drive) -> Analysis {
            Analysis {
                drive: Some(drive),
                parts: vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
            }
        }

        #[test]
        fn linux_style() {
            let path = StrictPath::from("/foo/bar");

            assert_eq!(analysis(Drive::Root), path.analyze());
            assert_eq!("/foo/bar", path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/foo/bar".to_string()), path.access_nonwindows());
        }

        #[test]
        fn windows_style_verbatim() {
            let path = StrictPath::from(r"\\?\share\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"\\?\share".to_string())), path.analyze());
            assert_eq!(r"\\?\share/foo/bar", path.display());
            assert_eq!(Ok(r"\\?\share\foo\bar".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_verbatim_with_forward_slash() {
            let path = StrictPath::from(r"\\?\share/foo\bar/baz");

            assert_eq!(analysis_3(Drive::Windows(r"\\?\share".to_string())), path.analyze());
            assert_eq!(r"\\?\share/foo/bar/baz", path.display());
            assert_eq!(Ok(r"\\?\share\foo\bar\baz".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_verbatim_unc() {
            let path = StrictPath::from(r"\\?\UNC\server\share\foo\bar");

            assert_eq!(
                analysis(Drive::Windows(r"\\?\UNC\server\share".to_string())),
                path.analyze()
            );
            assert_eq!(r"\\?\UNC\server\share/foo/bar", path.display());
            assert_eq!(Ok(r"\\?\UNC\server\share\foo\bar".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_verbatim_unc_with_forward_slash() {
            let path = StrictPath::from(r"\\?\UNC\server\share/foo\bar/baz");

            assert_eq!(
                analysis_3(Drive::Windows(r"\\?\UNC\server\share".to_string())),
                path.analyze()
            );
            assert_eq!(r"\\?\UNC\server\share/foo/bar/baz", path.display());
            assert_eq!(
                Ok(r"\\?\UNC\server\share\foo\bar\baz".to_string()),
                path.access_windows()
            );
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_verbatim_disk() {
            let path = StrictPath::from(r"\\?\C:\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"C:".to_string())), path.analyze());
            assert_eq!(r"C:/foo/bar", path.display());
            assert_eq!(Ok(r"C:\foo\bar".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_verbatim_disk_with_forward_slash() {
            let path = StrictPath::from(r"\\?\C:/foo\bar/baz");

            assert_eq!(analysis_3(Drive::Windows(r"C:".to_string())), path.analyze());
            assert_eq!(r"C:/foo/bar/baz", path.display());
            assert_eq!(Ok(r"C:\foo\bar\baz".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_device_ns() {
            let path = StrictPath::from(r"\\.\COM42\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"\\.\COM42".to_string())), path.analyze());
            assert_eq!(r"\\.\COM42/foo/bar", path.display());
            assert_eq!(Ok(r"\\.\COM42\foo\bar".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_device_ns_with_forward_slash() {
            let path = StrictPath::from(r"\\.\COM42/foo\bar/baz");

            assert_eq!(analysis_3(Drive::Windows(r"\\.\COM42".to_string())), path.analyze());
            assert_eq!(r"\\.\COM42/foo/bar/baz", path.display());
            assert_eq!(Ok(r"\\.\COM42\foo\bar\baz".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_unc() {
            let path = StrictPath::from(r"\\server\share\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"\\server\share".to_string())), path.analyze());
            assert_eq!(r"\\server\share/foo/bar", path.display());
            assert_eq!(Ok(r"\\server\share\foo\bar".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_unc_with_forward_slash() {
            let path = StrictPath::from(r"\\server\share/foo\bar/baz");

            assert_eq!(
                analysis_3(Drive::Windows(r"\\server\share".to_string())),
                path.analyze()
            );
            assert_eq!(r"\\server\share/foo/bar/baz", path.display());
            assert_eq!(Ok(r"\\server\share\foo\bar\baz".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_disk() {
            let path = StrictPath::from(r"C:\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"C:".to_string())), path.analyze());
            assert_eq!(r"C:/foo/bar", path.display());
            assert_eq!(Ok(r"C:\foo\bar".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn windows_style_disk_with_forward_slash() {
            let path = StrictPath::from(r"C:/foo\bar/baz");

            assert_eq!(analysis_3(Drive::Windows(r"C:".to_string())), path.analyze());
            assert_eq!(r"C:/foo/bar/baz", path.display());
            assert_eq!(Ok(r"C:\foo\bar\baz".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn relative_plain() {
            let path = StrictPath::from("foo");
            assert_eq!(Analysis::new(None, vec!["foo".to_string()]), path.analyze());
            assert_eq!("foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Relative), path.access_windows());
            assert_eq!(Err(StrictPathError::Relative), path.access_nonwindows());

            let path = StrictPath::relative("foo".to_string(), Some("/tmp".to_string()));
            assert_eq!(
                Analysis::new(Some(Drive::Root), vec!["tmp".to_string(), "foo".to_string()]),
                path.analyze()
            );
            assert_eq!("/tmp/foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/tmp/foo".to_string()), path.access_nonwindows());

            let path = StrictPath::relative("foo".to_string(), Some("C:/tmp".to_string()));
            assert_eq!(
                Analysis::new(
                    Some(Drive::Windows("C:".to_string())),
                    vec!["tmp".to_string(), "foo".to_string()]
                ),
                path.analyze()
            );
            assert_eq!("C:/tmp/foo".to_string(), path.display());
            assert_eq!(Ok(r"C:\tmp\foo".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn relative_single_dot() {
            let path = StrictPath::from("./foo");
            assert_eq!(Analysis::new(None, vec!["foo".to_string()]), path.analyze());
            assert_eq!("foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Relative), path.access_windows());
            assert_eq!(Err(StrictPathError::Relative), path.access_nonwindows());

            let path = StrictPath::relative("./foo".to_string(), Some("/tmp".to_string()));
            assert_eq!(
                Analysis::new(Some(Drive::Root), vec!["tmp".to_string(), "foo".to_string()]),
                path.analyze()
            );
            assert_eq!("/tmp/foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/tmp/foo".to_string()), path.access_nonwindows());

            let path = StrictPath::relative("./foo".to_string(), Some("C:/tmp".to_string()));
            assert_eq!(
                Analysis::new(
                    Some(Drive::Windows("C:".to_string())),
                    vec!["tmp".to_string(), "foo".to_string()]
                ),
                path.analyze()
            );
            assert_eq!("C:/tmp/foo".to_string(), path.display());
            assert_eq!(Ok(r"C:\tmp\foo".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn relative_double_dot() {
            let path = StrictPath::from("../foo");
            assert_eq!(Analysis::new(None, vec!["foo".to_string()]), path.analyze());
            assert_eq!("foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Relative), path.access_windows());
            assert_eq!(Err(StrictPathError::Relative), path.access_nonwindows());

            let path = StrictPath::relative("../foo".to_string(), Some("/tmp/bar".to_string()));
            assert_eq!(
                Analysis::new(Some(Drive::Root), vec!["tmp".to_string(), "foo".to_string()]),
                path.analyze()
            );
            assert_eq!("/tmp/foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/tmp/foo".to_string()), path.access_nonwindows());

            let path = StrictPath::relative("../foo".to_string(), Some("C:/tmp/bar".to_string()));
            assert_eq!(
                Analysis::new(
                    Some(Drive::Windows("C:".to_string())),
                    vec!["tmp".to_string(), "foo".to_string()]
                ),
                path.analyze()
            );
            assert_eq!("C:/tmp/foo".to_string(), path.display());
            assert_eq!(Ok(r"C:\tmp\foo".to_string()), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
        }

        #[test]
        fn tilde() {
            let path = StrictPath::new("~".to_owned());
            assert_eq!(Ok(home()), path.access());
        }

        #[test]
        fn empty() {
            let path = StrictPath::from("");
            assert_eq!(Analysis::new(None, vec![]), path.analyze());
            assert_eq!("".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Empty), path.access_windows());
            assert_eq!(Err(StrictPathError::Empty), path.access_nonwindows());
        }

        #[test]
        fn extra_slashes() {
            let path = StrictPath::from(r"///foo\\bar/\baz");
            assert_eq!(
                Analysis::new(
                    Some(Drive::Root),
                    vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
                ),
                path.analyze()
            );
        }

        #[test]
        fn mixed_style() {
            let path = StrictPath::from(r"/foo\bar");
            assert_eq!(
                Analysis::new(Some(Drive::Root), vec!["foo".to_string(), "bar".to_string()]),
                path.analyze()
            );
        }

        #[test]
        fn linux_root_variations() {
            let path = StrictPath::from("/");

            assert_eq!(Analysis::new(Some(Drive::Root), vec![]), path.analyze());
            assert_eq!("/", path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/".to_string()), path.access_nonwindows());

            let path = StrictPath::from(r"\");

            assert_eq!(Analysis::new(Some(Drive::Root), vec![]), path.analyze());
            assert_eq!("/", path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/".to_string()), path.access_nonwindows());
        }

        #[test]
        fn windows_root_variations() {
            macro_rules! check {
                ($input:expr, $output:expr) => {
                    let path = StrictPath::from($input);
                    assert_eq!(
                        Analysis::new(Some(Drive::Windows($output.to_string())), vec![]),
                        path.analyze()
                    );
                };
            }

            // Verbatim
            check!(r"\\?\share", r"\\?\share");
            check!(r"//?/share", r"\\?\share");

            // Verbatim UNC
            check!(r"\\?\UNC\server\share", r"\\?\UNC\server\share");
            // TODO: Fix or remove this case?
            // check!(r"//?/UNC/server/share", r"\\?\UNC\server\share");

            // Verbatim disk
            check!(r"\\?\C:", r"C:");
            check!(r"\\?\C:\", r"C:");
            check!(r"//?/C:", r"C:");
            check!(r"//?/C:/", r"C:");

            // Device NS
            check!(r"\\.\COM42", r"\\.\COM42");
            check!(r"//./COM42", r"\\.\COM42");

            // UNC
            check!(r"\\server\share", r"\\server\share");
            check!(r"//server/share", r"\\server\share");

            // Disk
            check!(r"C:", r"C:");
            check!(r"C:\", r"C:");
            check!(r"C:/", r"C:");
        }

        #[test]
        fn handles_windows_classic_path_with_extra_colon() {
            // https://github.com/mtkennerly/ludusavi/issues/36
            // Test for: <winDocuments>/<home>

            let path = StrictPath::relative(
                r"C:\Users\Foo\Documents/C:\Users\Bar".to_string(),
                Some(r"\\?\C:\Users\Foo\.config\ludusavi".to_string()),
            );
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
            assert_eq!("C:/Users/Foo/Documents/C:/Users/Bar", path.display());
        }

        #[test]
        fn handles_windows_unc_path_with_extra_colon() {
            // https://github.com/mtkennerly/ludusavi/issues/36
            // Test for: <winDocuments>/<home>

            let path = StrictPath::relative(
                r"\\?\C:\Users\Foo\Documents\C:\Users\Bar".to_string(),
                Some(r"\\?\C:\Users\Foo\.config\ludusavi".to_string()),
            );
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_nonwindows());
            assert_eq!("C:/Users/Foo/Documents/C:/Users/Bar", path.display());
        }

        #[test]
        fn handles_nonwindows_path_with_extra_colon() {
            // https://github.com/mtkennerly/ludusavi/issues/351

            let path = StrictPath::new(r"/tmp/foo: bar.baz".to_string());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!("/tmp/foo: bar.baz", path.access_nonwindows().unwrap());
            assert_eq!("/tmp/foo: bar.baz", path.display());
        }
    }
}
