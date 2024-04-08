use std::sync::{Arc, Mutex};

use filetime::FileTime;
use once_cell::sync::Lazy;

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
    Document,
    Home,
    Public,
}

impl CommonPath {
    pub fn get(&self) -> Option<&str> {
        fn check_dir(path: Option<std::path::PathBuf>) -> Option<String> {
            Some(path?.to_string_lossy().to_string())
        }

        static CONFIG: Lazy<Option<String>> = Lazy::new(|| check_dir(dirs::config_dir()));
        static DATA: Lazy<Option<String>> = Lazy::new(|| check_dir(dirs::data_dir()));
        static DATA_LOCAL: Lazy<Option<String>> = Lazy::new(|| check_dir(dirs::data_local_dir()));
        static DOCUMENT: Lazy<Option<String>> = Lazy::new(|| check_dir(dirs::document_dir()));
        static HOME: Lazy<Option<String>> = Lazy::new(|| check_dir(dirs::home_dir()));
        static PUBLIC: Lazy<Option<String>> = Lazy::new(|| check_dir(dirs::public_dir()));

        match self {
            Self::Config => CONFIG.as_ref(),
            Self::Data => DATA.as_ref(),
            Self::DataLocal => DATA_LOCAL.as_ref(),
            Self::Document => DOCUMENT.as_ref(),
            Self::Home => HOME.as_ref(),
            Self::Public => PUBLIC.as_ref(),
        }
        .map(|x| x.as_str())
    }

    pub fn get_or_skip(&self) -> &str {
        self.get().unwrap_or(SKIP)
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
    pub fn new(raw: String) -> Self {
        Self {
            raw,
            basis: None,
            canonical: Arc::new(Mutex::new(None)),
        }
    }

    pub fn relative(raw: String, basis: Option<String>) -> Self {
        Self {
            raw,
            basis,
            canonical: Arc::new(Mutex::new(None)),
        }
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

    pub fn raw(&self) -> String {
        self.raw.to_string()
    }

    /// For any paths that we store the entire time the GUI is running, like in the config,
    /// we sometimes want to refresh in case we have stale data.
    pub fn invalidate_cache(&self) {
        let mut cached = self.canonical.lock().unwrap();
        *cached = None;
    }

    fn analyze(&self) -> (Option<Drive>, Vec<String>) {
        use typed_path::{
            Utf8TypedComponent as Component, Utf8TypedPath as TypedPath, Utf8UnixComponent as UComponent,
            Utf8WindowsComponent as WComponent, Utf8WindowsPrefix as WindowsPrefix,
        };

        let mut drive = None;
        let mut parts = vec![];

        for (i, component) in TypedPath::derive(self.raw.trim()).components().enumerate() {
            match component {
                Component::Windows(WComponent::Prefix(prefix)) => {
                    let mapped = match prefix.kind() {
                        WindowsPrefix::Verbatim(id) => format!(r"\\?\{}", id),
                        WindowsPrefix::VerbatimUNC(server, share) => format!(r"\\?\UNC\{}\{}", server, share),
                        WindowsPrefix::VerbatimDisk(id) => format!("{}:", id.to_ascii_uppercase()),
                        WindowsPrefix::DeviceNS(id) => format!(r"\\.\{}", id),
                        WindowsPrefix::UNC(server, share) => format!(r"\\{}\{}", server, share),
                        WindowsPrefix::Disk(id) => format!("{}:", id.to_ascii_uppercase()),
                    };
                    drive = Some(Drive::Windows(mapped));
                }
                Component::Unix(UComponent::RootDir) | Component::Windows(WComponent::RootDir) => {
                    if i == 0 {
                        drive = Some(Drive::Root);
                    }
                }
                Component::Unix(UComponent::CurDir) | Component::Windows(WComponent::CurDir) => {
                    if i == 0 {
                        if let Some(basis) = &self.basis {
                            (drive, parts) = Self::new(basis.clone()).analyze();
                        }
                    }
                }
                Component::Unix(UComponent::ParentDir) | Component::Windows(WComponent::ParentDir) => {
                    if i == 0 {
                        if let Some(basis) = &self.basis {
                            (drive, parts) = Self::new(basis.clone()).analyze();
                        }
                    }
                    parts.pop();
                }
                Component::Unix(UComponent::Normal(part)) | Component::Windows(WComponent::Normal(part)) => {
                    let mut part = part.to_string();

                    if i == 0 {
                        let mapped = match part.as_str() {
                            "~" | placeholder::HOME => CommonPath::Home.get(),
                            placeholder::XDG_CONFIG => CommonPath::Config.get(),
                            placeholder::XDG_DATA | placeholder::WIN_APP_DATA => CommonPath::Data.get(),
                            placeholder::WIN_LOCAL_APP_DATA => CommonPath::DataLocal.get(),
                            placeholder::WIN_DOCUMENTS => CommonPath::Document.get(),
                            placeholder::WIN_PUBLIC => CommonPath::Public.get(),
                            placeholder::WIN_PROGRAM_DATA => Some("C:/ProgramData"),
                            placeholder::WIN_DIR => Some("C:/Windows"),
                            _ => None,
                        };

                        if let Some(mapped) = mapped {
                            (drive, parts) = Self::new(mapped.to_string()).analyze();
                            continue;
                        } else if let Some(basis) = &self.basis {
                            (drive, parts) = Self::new(basis.clone()).analyze();
                        }
                    }

                    if part == placeholder::OS_USER_NAME {
                        parts.push(crate::prelude::OS_USERNAME.to_string());
                        continue;
                    }

                    if part.contains(':') {
                        // This could happen if the user entered an invalid path like `C:\foo/C:\bar`
                        // or if the manifest contained a path like `<winDocuments>/<home>`.
                        // We escape it so that it (likely) just won't be found, rather than finding something irrelevant.
                        part = part.replace(':', "_");
                    }

                    // On Unix, Unix-style path segments may contain a backslash.
                    if part.contains('\\') {
                        for part in part.split('\\') {
                            if !part.trim().is_empty() {
                                parts.push(part.to_string());
                            }
                        }
                    } else {
                        parts.push(part);
                    }
                }
            }
        }

        (drive, parts)
    }

    fn display(&self) -> String {
        if self.raw.is_empty() {
            return "".to_string();
        }

        match self.analyze() {
            (Some(Drive::Root), parts) => format!("/{}", parts.join("/")),
            (Some(Drive::Windows(id)), parts) => {
                format!("{}/{}", id, parts.join("/"))
            }
            (None, parts) => parts.join("/"),
        }
    }

    fn access(&self) -> Result<String, StrictPathError> {
        if cfg!(target_os = "windows") {
            self.access_windows()
        } else {
            self.access_nonwindows()
        }
    }

    fn access_windows(&self) -> Result<String, StrictPathError> {
        if self.raw.is_empty() {
            return Err(StrictPathError::Empty);
        }

        match self.analyze() {
            (Some(Drive::Root), _) => Err(StrictPathError::Unsupported),
            (Some(Drive::Windows(id)), parts) => Ok(format!("{}\\{}", id, parts.join("\\"))),
            (None, parts) => match &self.basis {
                Some(basis) => Ok(format!("{}\\{}", basis, parts.join("\\"))),
                None => Err(StrictPathError::Relative),
            },
        }
    }

    pub fn access_nonwindows(&self) -> Result<String, StrictPathError> {
        if self.raw.is_empty() {
            return Err(StrictPathError::Empty);
        }

        match self.analyze() {
            (Some(Drive::Root), parts) => Ok(format!("/{}", parts.join("/"))),
            (Some(Drive::Windows(_)), _) => Err(StrictPathError::Unsupported),
            (None, parts) => match &self.basis {
                Some(basis) => Ok(format!("{}/{}", basis, parts.join("/"))),
                None => Err(StrictPathError::Relative),
            },
        }
    }

    // TODO: Better error reporting for incompatible UNC path variants.
    pub fn globbable(&self) -> String {
        self.display().trim().trim_end_matches(['/', '\\']).replace('\\', "/")
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
        match self.canonical() {
            Canonical::Valid(path) => Self::new(path).display(),
            Canonical::Unsupported | Canonical::Inaccessible => self.display(),
        }
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
            self.raw()
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
            Err(_) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get mtime in zip format",
            ))),
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

    pub fn joined(&self, other: &str) -> Self {
        Self {
            raw: format!("{}/{}", &self.raw, other).replace('\\', "/"),
            basis: self.basis.clone(),
            canonical: Arc::new(Mutex::new(None)),
        }
    }

    pub fn popped(&self) -> Self {
        let raw = match self.analyze() {
            (Some(Drive::Root), mut parts) => {
                parts.pop();
                format!("/{}", parts.join("/"))
            }
            (Some(Drive::Windows(id)), mut parts) => {
                parts.pop();
                format!("{}/{}", id, parts.join("/"))
            }
            (None, mut parts) => {
                parts.pop();
                match &self.basis {
                    Some(basis) => format!("{}/{}", basis, parts.join("/")),
                    None => parts.join("/"),
                }
            }
        };

        Self::new(raw)
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

    pub fn copy_to(&self, target: &StrictPath) -> std::io::Result<u64> {
        std::fs::copy(self.as_std_path_buf()?, target.as_std_path_buf()?)
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

    // TODO: Refactor to use `popped()`?
    pub fn parent(&self) -> Option<Self> {
        self.as_std_path_buf().ok()?.parent().map(Self::from)
    }

    // TODO: Refactor to use `popped()`?
    pub fn parent_if_file(&self) -> Result<Self, StrictPathError> {
        let resolved = self.try_resolve()?;
        let pathbuf = std::path::PathBuf::from(&resolved);
        if pathbuf.is_file() {
            match pathbuf.parent() {
                Some(parent) => Ok(Self::from(parent)),
                None => Ok(self.clone()),
            }
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
                "[{context}] unable to create parent directories: {} -> {} | {e}",
                self.raw(),
                target_file.raw()
            );
            return Err(e);
        }

        if let Err(e) = target_file.unset_readonly() {
            log::warn!(
                "[{context}] failed to unset read-only on target: {} | {e}",
                target_file.raw()
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to unset read-only",
            ));
        } else if let Err(e) = self.copy_to(target_file) {
            log::error!(
                "[{context}] unable to copy: {} -> {} | {e}",
                self.raw(),
                target_file.raw()
            );
            return Err(e);
        } else {
            let mtime = match self.get_mtime() {
                Ok(x) => x,
                Err(e) => {
                    log::error!(
                        "[{context}] unable to get modification time: {} -> {} | {e}",
                        self.raw(),
                        target_file.raw(),
                    );
                    return Err(e);
                }
            };
            if let Err(e) = target_file.set_mtime(mtime) {
                log::error!(
                    "[{context}] unable to set modification time: {} -> {} to {mtime:#?} | {e}",
                    self.raw(),
                    target_file.raw(),
                );
                return Err(e);
            }
        }
        Ok(())
    }

    /// This splits a path into a drive (e.g., `C:` or `\\?\D:`) and the remainder.
    /// This is only used during backups to record drives in mapping.yaml,
    /// so relative paths should have already been filtered out.
    pub fn split_drive(&self) -> (String, String) {
        match self.analyze() {
            (Some(Drive::Root), parts) => ("".to_string(), parts.join("/")),
            (Some(Drive::Windows(id)), parts) => (id, parts.join("/")),
            (None, _) => {
                log::error!("Unreachable state: unable to split drive of path: {}", &self.raw);
                unreachable!()
            }
        }
    }

    pub fn unset_readonly(&self) -> Result<(), AnyError> {
        let subject = self.as_std_path_buf()?;
        if self.is_file() {
            let mut perms = std::fs::metadata(&subject)?.permissions();
            if perms.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                std::fs::set_permissions(&subject, perms)?;
            }
        } else {
            for entry in walkdir::WalkDir::new(subject)
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .skip(1) // the base path itself
                .filter_map(crate::prelude::filter_map_walkdir)
                .filter(|x| x.file_type().is_file())
            {
                let file = &mut entry.path().display().to_string();
                let mut perms = std::fs::metadata(&file)?.permissions();
                if perms.readonly() {
                    #[allow(clippy::permissions_set_readonly_false)]
                    perms.set_readonly(false);
                    std::fs::set_permissions(&file, perms)?;
                }
            }
        }

        Ok(())
    }

    pub fn is_prefix_of(&self, other: &Self) -> bool {
        let (us_drive, us_parts) = self.analyze();
        let (them_drive, them_parts) = other.analyze();

        if us_drive != them_drive {
            return false;
        }

        if us_parts.len() >= them_parts.len() {
            return false;
        }

        us_parts.iter().zip(them_parts.iter()).all(|(us, them)| us == them)
    }

    pub fn nearest_prefix(&self, others: Vec<StrictPath>) -> Option<StrictPath> {
        let (us_drive, us_parts) = self.analyze();
        let us_count = us_parts.len();

        let mut nearest = None;
        let mut nearest_len = 0;
        for other in others {
            let (them_drive, them_parts) = other.analyze();
            let them_len = them_parts.len();

            if us_drive != them_drive || us_count <= them_len {
                continue;
            }
            if us_parts.iter().zip(them_parts.iter()).all(|(us, them)| us == them) && them_len > nearest_len {
                nearest = Some(other);
                nearest_len = them_len;
            }
        }
        nearest
    }

    pub fn glob(&self) -> Vec<StrictPath> {
        self.glob_case_sensitive(Os::HOST.is_case_sensitive())
    }

    pub fn glob_case_sensitive(&self, case_sensitive: bool) -> Vec<StrictPath> {
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
        serde::Deserialize::deserialize(deserializer).map(StrictPath::new)
    }
}

#[allow(dead_code)]
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
    }

    mod strict_path_display_and_access {
        use super::*;

        use pretty_assertions::assert_eq;

        fn analysis(drive: Drive) -> (Option<Drive>, Vec<String>) {
            (Some(drive), vec!["foo".to_string(), "bar".to_string()])
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
        fn windows_style_verbatim_disk() {
            let path = StrictPath::from(r"\\?\C:\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"C:".to_string())), path.analyze());
            assert_eq!(r"C:/foo/bar", path.display());
            assert_eq!(Ok(r"C:\foo\bar".to_string()), path.access_windows());
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
        fn windows_style_unc() {
            let path = StrictPath::from(r"\\server\share\foo\bar");

            assert_eq!(analysis(Drive::Windows(r"\\server\share".to_string())), path.analyze());
            assert_eq!(r"\\server\share/foo/bar", path.display());
            assert_eq!(Ok(r"\\server\share\foo\bar".to_string()), path.access_windows());
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
        fn relative_plain() {
            let path = StrictPath::from("foo");
            assert_eq!((None, vec!["foo".to_string()]), path.analyze());
            assert_eq!("foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Relative), path.access_windows());
            assert_eq!(Err(StrictPathError::Relative), path.access_nonwindows());

            let path = StrictPath::relative("foo".to_string(), Some("/tmp".to_string()));
            assert_eq!(
                (Some(Drive::Root), vec!["tmp".to_string(), "foo".to_string()]),
                path.analyze()
            );
            assert_eq!("/tmp/foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/tmp/foo".to_string()), path.access_nonwindows());

            let path = StrictPath::relative("foo".to_string(), Some("C:/tmp".to_string()));
            assert_eq!(
                (
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
            assert_eq!((None, vec!["foo".to_string()]), path.analyze());
            assert_eq!("foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Relative), path.access_windows());
            assert_eq!(Err(StrictPathError::Relative), path.access_nonwindows());

            let path = StrictPath::relative("./foo".to_string(), Some("/tmp".to_string()));
            assert_eq!(
                (Some(Drive::Root), vec!["tmp".to_string(), "foo".to_string()]),
                path.analyze()
            );
            assert_eq!("/tmp/foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/tmp/foo".to_string()), path.access_nonwindows());

            let path = StrictPath::relative("./foo".to_string(), Some("C:/tmp".to_string()));
            assert_eq!(
                (
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
            assert_eq!((None, vec!["foo".to_string()]), path.analyze());
            assert_eq!("foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Relative), path.access_windows());
            assert_eq!(Err(StrictPathError::Relative), path.access_nonwindows());

            let path = StrictPath::relative("../foo".to_string(), Some("/tmp/bar".to_string()));
            assert_eq!(
                (Some(Drive::Root), vec!["tmp".to_string(), "foo".to_string()]),
                path.analyze()
            );
            assert_eq!("/tmp/foo".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/tmp/foo".to_string()), path.access_nonwindows());

            let path = StrictPath::relative("../foo".to_string(), Some("C:/tmp/bar".to_string()));
            assert_eq!(
                (
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
            assert_eq!((None, vec![]), path.analyze());
            assert_eq!("".to_string(), path.display());
            assert_eq!(Err(StrictPathError::Empty), path.access_windows());
            assert_eq!(Err(StrictPathError::Empty), path.access_nonwindows());
        }

        #[test]
        fn extra_slashes() {
            let path = StrictPath::from(r"///foo\\bar/\baz");
            assert_eq!(
                (
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
                (Some(Drive::Root), vec!["foo".to_string(), "bar".to_string()]),
                path.analyze()
            );
        }

        #[test]
        fn linux_root_variations() {
            let path = StrictPath::from("/");

            assert_eq!((Some(Drive::Root), vec![]), path.analyze());
            assert_eq!("/", path.display());
            assert_eq!(Err(StrictPathError::Unsupported), path.access_windows());
            assert_eq!(Ok("/".to_string()), path.access_nonwindows());

            let path = StrictPath::from(r"\");

            assert_eq!((Some(Drive::Root), vec![]), path.analyze());
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
                        (Some(Drive::Windows($output.to_string())), vec![]),
                        path.analyze()
                    );
                };
            }

            // Verbatim
            check!(r"\\?\share", r"\\?\share");
            check!(r"//?/share", r"\\?\share");

            // Verbatim UNC
            check!(r"\\?\UNC\server\share", r"\\?\UNC\server\share");
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
        #[cfg(target_os = "windows")]
        fn does_not_truncate_path_up_to_drive_letter_in_windows_classic_path() {
            // https://github.com/mtkennerly/ludusavi/issues/36
            // Test for: <winDocuments>/<home>

            let path = StrictPath::relative(
                r"C:\Users\Foo\Documents/C:\Users\Bar".to_string(),
                Some(r"\\?\C:\Users\Foo\.config\ludusavi".to_string()),
            );
            assert_eq!(r"C:\Users\Foo\Documents\C_\Users\Bar", path.interpret().unwrap());
            assert_eq!("C:/Users/Foo/Documents/C_/Users/Bar", path.render());
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn does_not_truncate_path_up_to_drive_letter_in_windows_unc_path() {
            // https://github.com/mtkennerly/ludusavi/issues/36
            // Test for: <winDocuments>/<home>

            let path = StrictPath::relative(
                r"\\?\C:\Users\Foo\Documents\C:\Users\Bar".to_string(),
                Some(r"\\?\C:\Users\Foo\.config\ludusavi".to_string()),
            );
            assert_eq!(r"C:\Users\Foo\Documents\C_\Users\Bar", path.interpret().unwrap());
            assert_eq!("C:/Users/Foo/Documents/C_/Users/Bar", path.render());
        }
    }
}
