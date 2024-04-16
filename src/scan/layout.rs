use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    io::Write,
};

use chrono::{Datelike, Timelike};

use crate::{
    path::StrictPath,
    prelude::{AnyError, INVALID_FILE_CHARS},
    resource::{
        config::{
            BackupFormat, BackupFormats, RedirectConfig, Retention, ToggledPaths, ToggledRegistry, ZipCompression,
        },
        manifest::Os,
    },
    scan::{
        game_file_target, prepare_backup_target, BackupError, BackupId, BackupInfo, ScanChange, ScanInfo, ScannedFile,
        ScannedRegistry,
    },
};

const SAFE: &str = "_";

macro_rules! some_or_continue {
    ($maybe:expr) => {
        match $maybe {
            None => continue,
            Some(x) => x,
        }
    };
}

fn encode_base64_for_folder(name: &str) -> String {
    use base64::prelude::*;

    BASE64_STANDARD.encode(name).replace('/', SAFE)
}

pub fn escape_folder_name(name: &str) -> String {
    let mut escaped = String::from(name);

    // Technically, dots should be fine as long as the folder name isn't
    // exactly `.` or `..`. However, leading dots will often cause items
    // to be hidden by default, which could be confusing for users, so we
    // escape those. And Windows Explorer has a fun bug where, if you try
    // to open a folder whose name ends with a dot, then it will say that
    // the folder no longer exists at that location, so we also escape dots
    // at the end of the name. The combination of these two rules also
    // happens to cover the `.` and `..` cases.
    if escaped.starts_with('.') {
        escaped.replace_range(..1, SAFE);
    }
    if escaped.ends_with('.') {
        escaped.replace_range(escaped.len() - 1.., SAFE);
    }

    escaped.replace(INVALID_FILE_CHARS, SAFE)
}

pub struct LatestBackup {
    pub scan: ScanInfo,
    pub registry_content: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Backup {
    Full(FullBackup),
    Differential(DifferentialBackup),
}

impl Backup {
    pub fn name(&self) -> &str {
        match self {
            Self::Full(x) => &x.name,
            Self::Differential(x) => &x.name,
        }
    }

    pub fn when(&self) -> &chrono::DateTime<chrono::Utc> {
        match self {
            Self::Full(x) => &x.when,
            Self::Differential(x) => &x.when,
        }
    }

    pub fn when_local(&self) -> chrono::DateTime<chrono::Local> {
        chrono::DateTime::<chrono::Local>::from(*self.when())
    }

    pub fn os(&self) -> Option<Os> {
        match self {
            Self::Full(x) => x.os,
            Self::Differential(x) => x.os,
        }
    }

    pub fn comment(&self) -> Option<&String> {
        match self {
            Self::Full(x) => x.comment.as_ref(),
            Self::Differential(x) => x.comment.as_ref(),
        }
    }

    pub fn set_comment(&mut self, comment: String) {
        let comment = if comment.is_empty() { None } else { Some(comment) };

        match self {
            Self::Full(x) => x.comment = comment,
            Self::Differential(x) => x.comment = comment,
        }
    }

    pub fn locked(&self) -> bool {
        match self {
            Self::Full(x) => x.locked,
            Self::Differential(x) => x.locked,
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        match self {
            Self::Full(x) => x.locked = locked,
            Self::Differential(x) => x.locked = locked,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Full(x) => x.label(),
            Self::Differential(x) => x.label(),
        }
    }

    pub fn id(&self) -> BackupId {
        match self {
            Self::Full(x) => BackupId::Named(x.name.clone()),
            Self::Differential(x) => BackupId::Named(x.name.clone()),
        }
    }

    pub fn kind(&self) -> BackupKind {
        match self {
            Self::Full(_) => BackupKind::Full,
            Self::Differential(_) => BackupKind::Differential,
        }
    }

    pub fn full(&self) -> bool {
        self.kind() == BackupKind::Full
    }

    /// File path must be in rendered form.
    pub fn includes_file(&self, file: String) -> bool {
        match self {
            Self::Full(backup) => backup.files.contains_key(&file),
            Self::Differential(backup) => backup.files.get(&file).map(|x| x.is_some()).unwrap_or_default(),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn includes_registry(&self) -> bool {
        match self {
            Self::Full(backup) => backup.registry.hash.is_some(),
            Self::Differential(backup) => backup.registry.as_ref().map(|x| x.hash.is_some()).unwrap_or_default(),
        }
    }

    /// In this case, we just need to update the mapping file,
    /// but we don't want to end up creating an empty folder/archive.
    pub fn only_inherits_and_overrides(&self) -> bool {
        match self {
            Self::Full(_) => false,
            Self::Differential(backup) => backup.files.values().all(|x| x.is_none()) && backup.registry.is_none(),
        }
    }

    pub fn prune_failures(&mut self, backup_info: &BackupInfo) {
        match self {
            Self::Full(backup) => {
                let mut failed = vec![];
                for file in backup.files.keys() {
                    if backup_info.failed_files.keys().any(|x| &x.path.raw() == file) {
                        failed.push(file.to_string());
                    }
                }
                for file in failed {
                    backup.files.remove(&file);
                }

                // TODO: Registry failures are currently ignored during backup.
                // If that changes, then make sure this logic is still appropriate.
                if !backup_info.failed_registry.is_empty() {
                    backup.registry.hash = None;
                }
            }
            Self::Differential(backup) => {
                let mut failed = vec![];
                for file in backup.files.keys() {
                    if backup_info.failed_files.keys().any(|x| &x.path.raw() == file) {
                        failed.push(file.to_string());
                    }
                }
                for file in failed {
                    backup.files.remove(&file);
                }

                if !backup_info.failed_registry.is_empty() {
                    backup.registry = None;
                }
            }
        }
    }

    /// Use this after pruning failures to check if the backup is still useful.
    pub fn needed(&self) -> bool {
        match self {
            Backup::Full(backup) => !backup.files.is_empty() || backup.registry.hash.is_some(),
            Backup::Differential(backup) => !backup.files.is_empty() || backup.registry.is_some(),
        }
    }
}

impl ToString for Backup {
    fn to_string(&self) -> String {
        self.label()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FullBackup {
    pub name: String,
    pub when: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<Os>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Locked backups do not count toward retention limits and are never deleted.
    #[serde(default, skip_serializing_if = "crate::serialization::is_false")]
    pub locked: bool,
    #[serde(default)]
    pub files: BTreeMap<String, IndividualMappingFile>,
    #[serde(default)]
    pub registry: IndividualMappingRegistry,
    pub children: VecDeque<DifferentialBackup>,
}

impl FullBackup {
    pub fn label(&self) -> String {
        chrono::DateTime::<chrono::Local>::from(self.when)
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string()
    }

    pub fn format(&self) -> BackupFormat {
        if self.name.ends_with(".zip") {
            BackupFormat::Zip
        } else {
            BackupFormat::Simple
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupInclusion {
    Included,
    Inherited,
    Excluded,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DifferentialBackup {
    pub name: String,
    pub when: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<Os>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Locked backups do not count toward retention limits and are never deleted.
    #[serde(default, skip_serializing_if = "crate::serialization::is_false")]
    pub locked: bool,
    #[serde(default)]
    pub files: BTreeMap<String, Option<IndividualMappingFile>>,
    #[serde(default)]
    pub registry: Option<IndividualMappingRegistry>,
}

impl DifferentialBackup {
    /// File path must be in rendered form.
    pub fn file(&self, file: String) -> BackupInclusion {
        match self.files.get(&file) {
            None => BackupInclusion::Inherited,
            Some(info) => match info {
                None => BackupInclusion::Excluded,
                Some(_) => BackupInclusion::Included,
            },
        }
    }

    pub fn omits_registry(&self) -> bool {
        self.registry.as_ref().map(|x| x.hash.is_none()).unwrap_or_default()
    }

    pub fn label(&self) -> String {
        chrono::DateTime::<chrono::Local>::from(self.when)
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string()
    }

    pub fn format(&self) -> BackupFormat {
        if self.name.ends_with(".zip") {
            BackupFormat::Zip
        } else {
            BackupFormat::Simple
        }
    }
}

fn default_backup_list() -> VecDeque<FullBackup> {
    VecDeque::from(vec![FullBackup {
        name: ".".to_string(),
        ..Default::default()
    }])
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
pub struct IndividualMappingFile {
    pub hash: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndividualMappingRegistry {
    pub hash: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndividualMapping {
    pub name: String,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    pub drives: HashMap<String, String>,
    #[serde(default = "default_backup_list")]
    pub backups: VecDeque<FullBackup>,
}

impl IndividualMapping {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    fn reversed_drives(&self) -> HashMap<String, String> {
        self.drives.iter().map(|(k, v)| (v.to_owned(), k.to_owned())).collect()
    }

    fn new_drive_folder_name(drive: &str) -> String {
        if drive.is_empty() {
            "drive-0".to_string()
        } else {
            // Simplify "C:" to "drive-C" instead of "drive-C_" for the common case.
            format!("drive-{}", escape_folder_name(&drive.replace(':', "")))
        }
    }

    pub fn drive_folder_name(&mut self, drive: &str) -> String {
        let reversed = self.reversed_drives();
        match reversed.get::<str>(drive) {
            Some(mapped) => mapped.to_string(),
            None => {
                let key = Self::new_drive_folder_name(drive);
                self.drives.insert(key.to_string(), drive.to_string());
                key
            }
        }
    }

    pub fn drive_folder_name_immutable(&self, drive: &str) -> String {
        let reversed = self.reversed_drives();
        match reversed.get::<str>(drive) {
            Some(mapped) => mapped.to_string(),
            None => Self::new_drive_folder_name(drive),
        }
    }

    pub fn game_file(&mut self, base: &StrictPath, original_file: &StrictPath, backup: &str) -> StrictPath {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name(&drive);
        StrictPath::relative(
            format!("{}/{}/{}", backup, drive_folder, plain_path),
            base.interpret().ok(),
        )
    }

    pub fn game_file_immutable(&self, base: &StrictPath, original_file: &StrictPath, backup: &str) -> StrictPath {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name_immutable(&drive);
        StrictPath::relative(
            format!("{}/{}/{}", backup, drive_folder, plain_path),
            base.interpret().ok(),
        )
    }

    fn game_file_for_zip(&mut self, original_file: &StrictPath) -> String {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name(&drive);
        format!("{}/{}", drive_folder, plain_path).replace('\\', "/")
    }

    fn game_file_for_zip_immutable(&self, original_file: &StrictPath) -> String {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name_immutable(&drive);
        format!("{}/{}", drive_folder, plain_path).replace('\\', "/")
    }

    fn latest_backup(&self) -> Option<(&FullBackup, Option<&DifferentialBackup>)> {
        let full = self.backups.back();
        full.map(|x| (x, x.children.back()))
    }

    pub fn save(&self, file: &StrictPath) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Ok(old_content) = Self::load_raw(file) {
            if old_content == new_content {
                return;
            }
        }

        if file.create_parent_dir().is_ok() {
            let _ = file.write_with_content(&self.serialize());
        }
    }

    pub fn serialize(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }

    pub fn load(file: &StrictPath) -> Result<Self, AnyError> {
        if !file.is_file() {
            return Err("File does not exist".into());
        }
        let content = Self::load_raw(file)?;
        let mut parsed = Self::load_from_string(&content)?;

        // Handle legacy files without backup timestamps.
        for full in parsed.backups.iter_mut() {
            if full.name == "." && full.when == chrono::DateTime::<chrono::Utc>::default() {
                full.when = file
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok().map(chrono::DateTime::<chrono::Utc>::from))
                    .unwrap_or_default();
            }
        }

        Ok(parsed)
    }

    fn load_raw(file: &StrictPath) -> Result<String, AnyError> {
        file.try_read()
    }

    pub fn load_from_string(content: &str) -> Result<Self, AnyError> {
        match serde_yaml::from_str(content) {
            Ok(x) => Ok(x),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn has_backup(&self, name: &str) -> bool {
        self.backups
            .iter()
            .any(|full| full.name == name || full.children.iter().any(|diff| diff.name == name))
    }

    pub fn irrelevant_parents(&self, base: &StrictPath) -> Vec<StrictPath> {
        let mut irrelevant = vec![];
        let relevant = self.backups.iter().map(|x| x.name.clone()).chain(
            self.backups
                .iter()
                .flat_map(|x| x.children.iter().map(|y| y.name.clone())),
        );

        if !self.has_backup(".") {
            irrelevant.push(base.joined("registry.yaml"));
        }

        let Ok(base) = base.interpret() else {
            return vec![];
        };

        for child in walkdir::WalkDir::new(base)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(crate::scan::filter_map_walkdir)
        {
            let name = child.file_name().to_string_lossy();

            if name.starts_with("drive-") && !self.has_backup(".") {
                irrelevant.push(StrictPath::from(&child));
            }
            if name.starts_with("backup-") && !relevant.clone().any(|x| x == name) {
                irrelevant.push(StrictPath::from(&child));
            }
        }

        irrelevant
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BackupPlan {
    backup: Backup,
    files: HashSet<ScannedFile>,
    registry: HashSet<ScannedRegistry>,
}

#[derive(Clone, Debug, Default)]
pub struct GameLayout {
    pub path: StrictPath,
    mapping: IndividualMapping,
    #[allow(dead_code)]
    retention: Retention,
}

impl GameLayout {
    #[cfg(test)]
    pub fn new(path: StrictPath, mapping: IndividualMapping, retention: Retention) -> Self {
        Self {
            path,
            mapping,
            retention,
        }
    }

    pub fn load(path: StrictPath, retention: Retention) -> Result<Self, AnyError> {
        let mapping = Self::mapping_file(&path);
        Ok(Self {
            path,
            mapping: IndividualMapping::load(&mapping).map_err(|e| {
                log::error!("Unable to load mapping: {:?} | {:?}", &mapping, e);
                e
            })?,
            retention,
        })
    }

    pub fn save(&self) {
        self.mapping.save(&Self::mapping_file(&self.path))
    }

    pub fn verify_id(&self, id: &BackupId) -> BackupId {
        match id {
            BackupId::Latest => id.clone(),
            BackupId::Named(name) => {
                for full in &self.mapping.backups {
                    for diff in &full.children {
                        if diff.name == *name {
                            return id.clone();
                        }
                    }
                    if full.name == *name {
                        return id.clone();
                    }
                }
                BackupId::Latest
            }
        }
    }

    pub fn find_by_id(&self, id: &BackupId) -> Option<(&FullBackup, Option<&DifferentialBackup>)> {
        match id {
            BackupId::Latest => self.mapping.latest_backup(),
            BackupId::Named(id) => {
                let mut full = None;
                let mut diff = None;

                'outer: for full_candidate in &self.mapping.backups {
                    if full_candidate.name == *id {
                        full = Some(full_candidate);
                        break 'outer;
                    }
                    for diff_candidate in &full_candidate.children {
                        if diff_candidate.name == *id {
                            full = Some(full_candidate);
                            diff = Some(diff_candidate);
                            break 'outer;
                        }
                    }
                }

                match (full, diff) {
                    (None, _) => None,
                    (Some(full), None) => Some((full, None)),
                    (Some(full), Some(diff)) => Some((full, Some(diff))),
                }
            }
        }
    }

    pub fn find_by_id_flattened(&self, id: &BackupId) -> Option<Backup> {
        match self.find_by_id(id) {
            None => None,
            Some((full, None)) => Some(Backup::Full(full.clone())),
            Some((_, Some(diff))) => Some(Backup::Differential(diff.clone())),
        }
    }

    /// When `restoring` is false, we don't check for entries' ScanChange,
    /// because the backup scan will do that separately.
    pub fn latest_backup(
        &self,
        restoring: bool,
        redirects: &[RedirectConfig],
        toggled_paths: &ToggledPaths,
    ) -> Option<ScanInfo> {
        if self.mapping.backups.is_empty() {
            None
        } else {
            Some(ScanInfo {
                game_name: self.mapping.name.clone(),
                found_files: self.restorable_files(&BackupId::Latest, restoring, redirects, toggled_paths),
                // Registry is handled separately.
                found_registry_keys: Default::default(),
                available_backups: vec![],
                backup: None,
            })
        }
    }

    pub fn restorable_backups_flattened(&self) -> Vec<Backup> {
        let mut backups = vec![];

        for full in &self.mapping.backups {
            backups.push(Backup::Full(full.clone()));
            for diff in &full.children {
                backups.push(Backup::Differential(diff.clone()));
            }
        }

        backups
    }

    pub fn restorable_files(
        &self,
        id: &BackupId,
        restoring: bool,
        redirects: &[RedirectConfig],
        toggled_paths: &ToggledPaths,
    ) -> HashSet<ScannedFile> {
        let mut files = HashSet::new();

        match self.find_by_id(id) {
            None => {}
            Some((full, None)) => {
                files.extend(self.restorable_files_from_full_backup(full, restoring, redirects, toggled_paths));
            }
            Some((full, Some(diff))) => {
                files.extend(self.restorable_files_from_diff_backup(diff, restoring, redirects, toggled_paths));

                for full_file in self.restorable_files_from_full_backup(full, restoring, redirects, toggled_paths) {
                    let original_path = full_file.original_path.as_ref().unwrap().render();
                    if diff.file(original_path) == BackupInclusion::Inherited {
                        files.insert(full_file);
                    }
                }
            }
        }

        files
    }

    fn restorable_files_from_full_backup(
        &self,
        backup: &FullBackup,
        restoring: bool,
        redirects: &[RedirectConfig],
        toggled_paths: &ToggledPaths,
    ) -> HashSet<ScannedFile> {
        let mut restorables = HashSet::new();

        for (k, v) in &backup.files {
            let original_path = StrictPath::new(k.to_string());
            let redirected = game_file_target(&original_path, redirects, true);
            let ignorable_path = redirected.as_ref().unwrap_or(&original_path);
            match backup.format() {
                BackupFormat::Simple => {
                    restorables.insert(ScannedFile {
                        change: if restoring {
                            ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                        } else {
                            ScanChange::Unknown
                        },
                        path: self
                            .mapping
                            .game_file_immutable(&self.path, &original_path, &backup.name),
                        size: v.size,
                        hash: v.hash.clone(),
                        ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                        redirected,
                        original_path: Some(original_path),
                        container: None,
                    });
                }
                BackupFormat::Zip => {
                    restorables.insert(ScannedFile {
                        change: if restoring {
                            ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                        } else {
                            ScanChange::Unknown
                        },
                        path: StrictPath::new(self.mapping.game_file_for_zip_immutable(&original_path)),
                        size: v.size,
                        hash: v.hash.clone(),
                        ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                        redirected,
                        original_path: Some(original_path),
                        container: Some(self.path.joined(&backup.name)),
                    });
                }
            }
        }

        restorables
    }

    fn restorable_files_from_diff_backup(
        &self,
        backup: &DifferentialBackup,
        restoring: bool,
        redirects: &[RedirectConfig],
        toggled_paths: &ToggledPaths,
    ) -> HashSet<ScannedFile> {
        let mut restorables = HashSet::new();

        for (k, v) in &backup.files {
            let v = some_or_continue!(v);
            let original_path = StrictPath::new(k.to_string());
            let redirected = game_file_target(&original_path, redirects, true);
            let ignorable_path = redirected.as_ref().unwrap_or(&original_path);
            match backup.format() {
                BackupFormat::Simple => {
                    restorables.insert(ScannedFile {
                        change: if restoring {
                            ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                        } else {
                            ScanChange::Unknown
                        },
                        path: self
                            .mapping
                            .game_file_immutable(&self.path, &original_path, &backup.name),
                        size: v.size,
                        hash: v.hash.clone(),
                        ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                        redirected,
                        original_path: Some(original_path),
                        container: None,
                    });
                }
                BackupFormat::Zip => {
                    restorables.insert(ScannedFile {
                        change: if restoring {
                            ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                        } else {
                            ScanChange::Unknown
                        },
                        path: StrictPath::new(self.mapping.game_file_for_zip_immutable(&original_path)),
                        size: v.size,
                        hash: v.hash.clone(),
                        ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                        redirected,
                        original_path: Some(original_path),
                        container: Some(self.path.joined(&backup.name)),
                    });
                }
            }
        }

        restorables
    }

    // Since this is only used for a specific migration use case,
    // we don't need to fill out all of the `ScannedFile` info.
    fn restorable_files_in_simple(&self, backup: &str) -> HashSet<ScannedFile> {
        let Ok(path) = self.path.joined(backup).interpret() else {
            return HashSet::new();
        };

        let mut files = HashSet::new();
        for drive_dir in walkdir::WalkDir::new(path)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(crate::scan::filter_map_walkdir)
        {
            let raw_drive_dir = drive_dir.path().display().to_string();
            let drive_mapping =
                some_or_continue!(self.mapping.drives.get::<str>(&drive_dir.file_name().to_string_lossy()));

            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(crate::scan::filter_map_walkdir)
                .filter(|x| x.file_type().is_file())
            {
                let raw_file = file.path().display().to_string();
                let original_path = Some(StrictPath::new(raw_file.replace(&raw_drive_dir, drive_mapping)));
                let path = StrictPath::new(raw_file);
                files.insert(ScannedFile {
                    change: crate::scan::ScanChange::Unknown,
                    size: path.size(),
                    hash: path.sha1(),
                    path,
                    original_path,
                    ignored: false,
                    container: None,
                    redirected: None,
                });
            }
        }
        files
    }

    #[allow(dead_code)]
    pub fn registry_content(&self, id: &BackupId) -> Option<String> {
        match self.find_by_id(id) {
            None => None,
            Some((full, None)) => self.registry_content_in(&full.name, &full.format()),
            Some((full, Some(diff))) => {
                let diff_reg = self.registry_content_in(&diff.name, &diff.format());
                if diff_reg.is_some() {
                    diff_reg
                } else if diff.omits_registry() {
                    None
                } else {
                    self.registry_content_in(&full.name, &full.format())
                }
            }
        }
    }

    fn registry_content_in(&self, backup: &str, format: &BackupFormat) -> Option<String> {
        match format {
            BackupFormat::Simple => self.path.joined(backup).joined("registry.yaml").read(),
            BackupFormat::Zip => {
                let handle = self.path.joined(backup).open().ok()?;
                let mut archive = zip::ZipArchive::new(handle).ok()?;
                let mut file = archive.by_name("registry.yaml").ok()?;

                let mut buffer = vec![];
                std::io::copy(&mut file, &mut buffer).ok()?;

                String::from_utf8(buffer).ok()
            }
        }
    }

    #[allow(dead_code)]
    pub fn registry_file(&self, id: &BackupId) -> StrictPath {
        match self.find_by_id(id) {
            None => self.registry_file_in("."),
            Some((full, None)) => self.registry_file_in(&full.name),
            Some((full, Some(diff))) => {
                let diff_reg = self.registry_file_in(&diff.name);
                if diff_reg.exists() || diff.omits_registry() {
                    diff_reg
                } else {
                    self.registry_file_in(&full.name)
                }
            }
        }
    }

    #[allow(dead_code)]
    fn registry_file_in(&self, backup: &str) -> StrictPath {
        self.path.joined(backup).joined("registry.yaml")
    }

    fn generate_file_friendly_timestamp(now: &chrono::DateTime<chrono::Utc>) -> String {
        format!(
            "{}{:02}{:02}T{:02}{:02}{:02}Z",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
        )
    }

    fn generate_backup_name(
        &self,
        kind: &BackupKind,
        now: &chrono::DateTime<chrono::Utc>,
        format: &BackupFormats,
    ) -> String {
        if *kind == BackupKind::Full
            && self.retention.full == 1
            && format.chosen == BackupFormat::Simple
            && self.mapping.backups.iter().all(|x| !x.locked)
        {
            ".".to_string()
        } else {
            let timestamp = Self::generate_file_friendly_timestamp(now);
            let name = match *kind {
                BackupKind::Full => format!("backup-{}", timestamp),
                BackupKind::Differential => format!("backup-{}-diff", timestamp),
            };
            match format.chosen {
                BackupFormat::Simple => name,
                BackupFormat::Zip => format!("{name}.zip"),
            }
        }
    }

    fn plan_backup(
        &self,
        scan: &ScanInfo,
        now: &chrono::DateTime<chrono::Utc>,
        format: &BackupFormats,
    ) -> Option<Backup> {
        if !scan.found_anything_processable() && !self.retention.force_new_full {
            return None;
        }

        let kind = self.plan_backup_kind();

        let backup = match kind {
            BackupKind::Full => Backup::Full(self.plan_full_backup(scan, now, format)),
            BackupKind::Differential => Backup::Differential(self.plan_differential_backup(scan, now, format)),
        };

        backup.needed().then_some(backup)
    }

    fn plan_backup_kind(&self) -> BackupKind {
        if self.retention.force_new_full {
            return BackupKind::Full;
        }

        let fulls = self.mapping.backups.iter().filter(|full| !full.locked).count() as u8;
        let diffs = self
            .mapping
            .backups
            .back()
            .map(|x| x.children.iter().filter(|diff| !diff.locked).count())
            .unwrap_or(0) as u8;

        if fulls > 0
            && (diffs < self.retention.differential || (self.retention.full == 1 && self.retention.differential > 0))
        {
            BackupKind::Differential
        } else {
            BackupKind::Full
        }
    }

    fn plan_full_backup(
        &self,
        scan: &ScanInfo,
        now: &chrono::DateTime<chrono::Utc>,
        format: &BackupFormats,
    ) -> FullBackup {
        let mut files = BTreeMap::new();
        #[allow(unused_mut)]
        let mut registry = IndividualMappingRegistry::default();

        for file in scan.found_files.iter().filter(|x| !x.ignored) {
            match file.change() {
                ScanChange::New | ScanChange::Different | ScanChange::Same => {
                    files.insert(
                        file.mapping_key(),
                        IndividualMappingFile {
                            hash: file.hash.clone(),
                            size: file.size,
                        },
                    );
                }
                ScanChange::Removed | ScanChange::Unknown => (),
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::registry::Hives;
            let mut hives = Hives::default();
            let _ = hives.back_up(&scan.game_name, &scan.found_registry_keys);
            registry.hash = hives.sha1();
        }

        FullBackup {
            name: self.generate_backup_name(&BackupKind::Full, now, format),
            when: *now,
            os: Some(Os::HOST),
            comment: None,
            locked: false,
            files,
            registry,
            children: VecDeque::new(),
        }
    }

    fn plan_differential_backup(
        &self,
        scan: &ScanInfo,
        now: &chrono::DateTime<chrono::Utc>,
        format: &BackupFormats,
    ) -> DifferentialBackup {
        let mut files = BTreeMap::new();
        #[allow(unused_mut)]
        let mut registry = Some(IndividualMappingRegistry::default());

        for file in scan.found_files.iter() {
            match file.change() {
                ScanChange::New | ScanChange::Different | ScanChange::Same => {
                    files.insert(
                        file.mapping_key(),
                        Some(IndividualMappingFile {
                            hash: file.hash.clone(),
                            size: file.size,
                        }),
                    );
                }
                ScanChange::Removed => {
                    files.insert(file.mapping_key(), None);
                }
                ScanChange::Unknown => (),
            };
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::registry::Hives;
            let mut hives = Hives::default();
            let _ = hives.back_up(&scan.game_name, &scan.found_registry_keys);
            if !hives.is_empty() {
                registry = Some(IndividualMappingRegistry { hash: hives.sha1() });
            }
        }

        // Individual saves' ScanChange are relative to the latest full + differential composite.
        // If the latest full backup has file 1 version 1, the latest diff has file 1 version 2,
        // and our new scan is back to version 1, then we don't want to duplicate the file content.
        if let Some((full, _)) = self.mapping.latest_backup() {
            for (file, prior) in &full.files {
                if let Some(current) = files.get(file) {
                    if Some(&prior.hash) == current.as_ref().map(|x| &x.hash) {
                        files.remove(file);
                    }
                } else {
                    files.insert(file.clone(), None);
                }
            }
            if let Some(current_registry) = &registry {
                if &full.registry == current_registry {
                    registry = None;
                }
            }
        }

        DifferentialBackup {
            name: self.generate_backup_name(&BackupKind::Differential, now, format),
            when: *now,
            os: Some(Os::HOST),
            comment: None,
            locked: false,
            files,
            registry,
        }
    }

    fn execute_backup_as_simple(&mut self, backup: &Backup, scan: &ScanInfo) -> BackupInfo {
        let mut backup_info = BackupInfo::default();

        let mut relevant_files = vec![];
        for file in &scan.found_files {
            if !backup.includes_file(file.mapping_key()) {
                log::debug!("[{}] skipped: {}", self.mapping.name, file.path.raw());
                continue;
            }

            let target_file = self.mapping.game_file(&self.path, file.effective(), backup.name());
            if file.path.same_content(&target_file) {
                log::info!(
                    "[{}] already matches: {:?} -> {:?}",
                    self.mapping.name,
                    &file.path,
                    &target_file
                );
                relevant_files.push(target_file);
                continue;
            }
            if let Err(e) = file.path.copy_to_path(&self.mapping.name, &target_file) {
                backup_info
                    .failed_files
                    .insert(file.clone(), BackupError::Raw(e.to_string()));
                continue;
            }
            log::info!(
                "[{}] backed up: {:?} -> {:?}",
                self.mapping.name,
                file.path,
                target_file
            );
            relevant_files.push(target_file);
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::registry::Hives;
            let target_registry_file = self.registry_file_in(backup.name());

            if backup.includes_registry() {
                let mut hives = Hives::default();
                if let Err(failed) = hives.back_up(&scan.game_name, &scan.found_registry_keys) {
                    backup_info.failed_registry.extend(failed);
                }
                hives.save(&target_registry_file);
            } else {
                let _ = target_registry_file.remove();
            }
        }

        if backup.full() {
            self.remove_irrelevant_backup_files(backup.name(), &relevant_files);
        }

        backup_info
    }

    fn execute_backup_as_zip(&mut self, backup: &Backup, scan: &ScanInfo, format: &BackupFormats) -> BackupInfo {
        let mut backup_info = BackupInfo::default();

        let fail_file = |file: &ScannedFile, backup_info: &mut BackupInfo, error: String| {
            backup_info.failed_files.insert(file.clone(), BackupError::Raw(error))
        };
        let fail_all = |backup_info: &mut BackupInfo, error: String| {
            for file in &scan.found_files {
                backup_info
                    .failed_files
                    .insert(file.clone(), BackupError::Raw(error.clone()));
            }
        };

        let archive_path = self.path.joined(backup.name());
        let archive_file = match archive_path.create() {
            Ok(x) => x,
            Err(e) => {
                log::error!(
                    "[{}] unable to create zip file: {:?} | {e}",
                    self.mapping.name,
                    &archive_path
                );
                fail_all(&mut backup_info, e.to_string());
                return backup_info;
            }
        };
        let mut zip = zip::ZipWriter::new(archive_file);
        let options = zip::write::FileOptions::default()
            .compression_method(match format.zip.compression {
                ZipCompression::None => zip::CompressionMethod::Stored,
                ZipCompression::Deflate => zip::CompressionMethod::Deflated,
                ZipCompression::Bzip2 => zip::CompressionMethod::Bzip2,
                ZipCompression::Zstd => zip::CompressionMethod::Zstd,
            })
            .compression_level(format.level())
            .large_file(true);

        'item: for file in &scan.found_files {
            if !backup.includes_file(file.mapping_key()) {
                log::debug!("[{}] skipped: {:?}", self.mapping.name, &file.path);
                continue;
            }

            let target_file_id = self.mapping.game_file_for_zip(file.effective());

            let mtime = match file.path.get_mtime_zip() {
                Ok(x) => x,
                Err(e) => {
                    log::error!(
                        "[{}] unable to get mtime: {:?} -> {} | {e}",
                        self.mapping.name,
                        &file.path,
                        &target_file_id
                    );
                    fail_file(file, &mut backup_info, e.to_string());
                    continue;
                }
            };

            #[cfg(target_os = "windows")]
            let mode: Option<u32> = None;
            #[cfg(not(target_os = "windows"))]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                file.path.metadata().map(|metadata| metadata.permissions().mode()).ok()
            };

            let local_options = match mode {
                Some(mode) => options.last_modified_time(mtime).unix_permissions(mode),
                None => options.last_modified_time(mtime),
            };

            if let Err(e) = zip.start_file(&target_file_id, local_options) {
                log::error!(
                    "[{}] unable to start zip file record: {:?} -> {} | {e}",
                    self.mapping.name,
                    &file.path,
                    &target_file_id
                );
                fail_file(file, &mut backup_info, e.to_string());
                continue;
            }

            use std::io::Read;
            let handle = match file.path.open() {
                Ok(x) => x,
                Err(e) => {
                    log::error!("[{}] unable to open source: {:?} | {e}", self.mapping.name, &file.path);
                    fail_file(file, &mut backup_info, e.to_string());
                    continue;
                }
            };
            let mut reader = std::io::BufReader::new(handle);
            let mut buffer = [0; 1024];

            loop {
                let read = match reader.read(&mut buffer[..]) {
                    Ok(x) => x,
                    Err(e) => {
                        log::error!("[{}] unable to read source: {:?} | {e}", self.mapping.name, &file.path);
                        fail_file(file, &mut backup_info, e.to_string());
                        continue 'item;
                    }
                };
                if read == 0 {
                    log::info!(
                        "[{}] backed up: {:?} -> {}",
                        self.mapping.name,
                        &file.path,
                        &target_file_id
                    );
                    break;
                }
                if let Err(e) = zip.write_all(&buffer[0..read]) {
                    log::error!(
                        "[{}] unable to write target: {:?} -> {} | {e}",
                        self.mapping.name,
                        &file.path,
                        &target_file_id
                    );
                    fail_file(file, &mut backup_info, e.to_string());
                    continue 'item;
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::registry::Hives;

            if backup.includes_registry() {
                let mut hives = Hives::default();
                if let Err(failed) = hives.back_up(&scan.game_name, &scan.found_registry_keys) {
                    backup_info.failed_registry.extend(failed);
                }
                if zip.start_file("registry.yaml", options).is_ok() {
                    let _ = zip.write_all(hives.serialize().as_bytes());
                }
            }
        }

        if let Err(e) = zip.finish() {
            fail_all(&mut backup_info, e.to_string());
        }

        backup_info
    }

    fn insert_backup(&mut self, backup: Backup) {
        match backup {
            Backup::Full(backup) => {
                self.mapping.backups.push_back(backup);
            }
            Backup::Differential(backup) => {
                if let Some(parent) = self.mapping.backups.back_mut() {
                    parent.children.push_back(backup);
                }
            }
        }
    }

    fn forget_excess_backups(&mut self) {
        // We need to track by index rather than by ID.
        // If we're merging into a single existing backup (like the special ID `.`),
        // then we may have two of them before pruning the older one.
        let mut excess = vec![];

        let unlocked_fulls = self
            .mapping
            .backups
            .iter()
            .filter(|full| !full.locked && full.children.iter().all(|diff| !diff.locked))
            .count();
        let mut excess_fulls = unlocked_fulls.saturating_sub(self.retention.full as usize);

        for (i, full) in self.mapping.backups.iter_mut().enumerate() {
            let locked = full.locked || full.children.iter().any(|diff| diff.locked);
            if !locked && excess_fulls > 0 {
                excess.push((i, None));
                excess_fulls -= 1;
            }

            let unlocked_diffs = full.children.iter().filter(|diff| !diff.locked).count();
            let mut excess_diffs = unlocked_diffs.saturating_sub(self.retention.differential as usize);

            for (j, diff) in full.children.iter_mut().enumerate() {
                let locked = diff.locked;
                if !locked && excess_diffs > 0 {
                    excess.push((i, Some(j)));
                    excess_diffs -= 1;
                }
            }
        }

        log::debug!("[{}] Excess backups: {:?}", &self.mapping.name, excess);

        if !excess.is_empty() {
            // Remove indices from biggest to smallest so that the order is stable.
            excess.sort();
            excess.reverse();

            for (full, diff) in excess {
                if let Some(diff) = diff {
                    self.mapping.backups[full].children.remove(diff);
                } else {
                    self.mapping.backups.remove(full);
                }
            }
        }
    }

    fn execute_backup(&mut self, backup: &Backup, scan: &ScanInfo, format: &BackupFormats) -> BackupInfo {
        if backup.only_inherits_and_overrides() {
            BackupInfo::default()
        } else {
            match format.chosen {
                BackupFormat::Simple => self.execute_backup_as_simple(backup, scan),
                BackupFormat::Zip => self.execute_backup_as_zip(backup, scan, format),
            }
        }
    }

    fn prune_irrelevant_parents(&self) {
        for irrelevant_parent in self.mapping.irrelevant_parents(&self.path) {
            log::debug!(
                "[{}] Removing irrelevant parent: {:?}",
                &self.mapping.name,
                &irrelevant_parent
            );
            let _ = irrelevant_parent.remove();
        }
    }

    /// Handle legacy backups from before multi-backup support.
    /// In this case, a default backup with name "." has already been inserted.
    pub fn migrate_legacy_backup(&mut self) {
        if self.mapping.backups.len() != 1 {
            return;
        }

        let backup = self.mapping.backups.back().unwrap();
        if backup.name != "." || !backup.files.is_empty() || backup.registry.hash.is_some() {
            return;
        }

        let mut files = BTreeMap::new();
        #[allow(unused_mut)]
        let mut registry = IndividualMappingRegistry::default();

        log::info!("[{}] migrating legacy backup", &self.mapping.name);

        for file in self.restorable_files_in_simple(&backup.name) {
            files.insert(
                file.mapping_key(),
                IndividualMappingFile {
                    hash: file.path.sha1(),
                    size: file.path.size(),
                },
            );
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(content) = self.registry_content_in(&backup.name, &BackupFormat::Simple) {
                registry = IndividualMappingRegistry {
                    hash: Some(crate::prelude::sha1(content)),
                };
            }
        }

        if !files.is_empty() || registry.hash.is_some() {
            let backup = self.mapping.backups.back_mut().unwrap();
            backup.files = files;
            backup.registry = registry;
            self.save();
        }
    }

    pub fn back_up(
        &mut self,
        scan: &ScanInfo,
        now: &chrono::DateTime<chrono::Utc>,
        format: &BackupFormats,
    ) -> BackupInfo {
        if !scan.found_anything() {
            log::trace!("[{}] nothing to back up", &scan.game_name);
            return BackupInfo::default();
        }

        log::trace!("[{}] preparing for backup", &scan.game_name);
        if let Err(e) = prepare_backup_target(&self.path) {
            log::error!(
                "[{}] failed to prepare backup target: {:?} | {e:?}",
                scan.game_name,
                &self.path
            );
            return BackupInfo::total_failure(scan, BackupError::App(e));
        }

        self.migrate_legacy_backup();
        match self.plan_backup(scan, now, format) {
            None => {
                log::info!("[{}] no need for new backup", &scan.game_name);
                BackupInfo::default()
            }
            Some(mut backup) => {
                log::info!(
                    "[{}] creating a {:?} backup: {}",
                    &scan.game_name,
                    backup.kind(),
                    backup.name()
                );
                let backup_info = self.execute_backup(&backup, scan, format);
                backup.prune_failures(&backup_info);
                if backup.needed() {
                    self.insert_backup(backup.clone());
                    self.forget_excess_backups();
                    self.save();
                }
                self.prune_irrelevant_parents();
                backup_info
            }
        }
    }

    pub fn get_backups(&mut self) -> Vec<Backup> {
        let mut available_backups = vec![];

        if self.path.is_dir() {
            self.migrate_legacy_backup();
            available_backups = self.restorable_backups_flattened();
        }

        available_backups
    }

    pub fn has_backups(&self) -> bool {
        !self.mapping.backups.is_empty()
    }

    pub fn scan_for_restoration(
        &mut self,
        name: &str,
        id: &BackupId,
        redirects: &[RedirectConfig],
        toggled_paths: &ToggledPaths,
        #[allow(unused)] toggled_registry: &ToggledRegistry,
    ) -> ScanInfo {
        log::trace!("[{name}] beginning scan for restore");

        let mut found_files = HashSet::new();
        #[allow(unused_mut)]
        let mut found_registry_keys = HashSet::new();
        #[allow(unused_mut)]
        let mut available_backups = vec![];
        let mut backup = None;

        let id = self.verify_id(id);

        if self.path.is_dir() {
            self.migrate_legacy_backup();
            found_files = self.restorable_files(&id, true, redirects, toggled_paths);
            available_backups = self.restorable_backups_flattened();
            backup = self.find_by_id_flattened(&id);
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::{registry, RegistryItem, ScannedRegistryValue, ScannedRegistryValues};

            if let Some(registry_content) = self.registry_content(&id) {
                if let Some(hives) = registry::Hives::deserialize(&registry_content) {
                    for (hive_name, keys) in hives.0.iter() {
                        for (key_name, entries) in keys.0.iter() {
                            let live_entries = registry::try_read_registry_key(hive_name, key_name);
                            let mut live_values = ScannedRegistryValues::new();

                            let path = RegistryItem::from_hive_and_key(hive_name, key_name);

                            for (entry_name, entry) in entries.0.iter() {
                                live_values.insert(
                                    entry_name.clone(),
                                    ScannedRegistryValue {
                                        ignored: toggled_registry.is_ignored(name, &path, Some(entry_name)),
                                        change: live_entries
                                            .as_ref()
                                            .and_then(|x| x.0.get(entry_name))
                                            .map(|live_entry| {
                                                if entry == live_entry {
                                                    ScanChange::Same
                                                } else {
                                                    ScanChange::Different
                                                }
                                            })
                                            .unwrap_or(ScanChange::New),
                                    },
                                );
                            }

                            found_registry_keys.insert(ScannedRegistry {
                                ignored: toggled_registry.is_ignored(name, &path, None)
                                    && entries
                                        .0
                                        .keys()
                                        .all(|x| toggled_registry.is_ignored(name, &path, Some(x))),
                                path,
                                change: match &live_entries {
                                    None => ScanChange::New,
                                    Some(_) => ScanChange::Same,
                                },
                                values: live_values,
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

    pub fn restore(&self, scan: &ScanInfo, #[allow(unused)] toggled: &ToggledRegistry) -> BackupInfo {
        log::trace!("[{}] beginning restore", &scan.game_name);

        let mut failed_files = HashMap::new();
        #[allow(unused_mut)]
        let mut failed_registry = HashMap::new();

        let mut containers: HashMap<StrictPath, zip::ZipArchive<std::fs::File>> = HashMap::new();
        let mut failed_containers: HashMap<StrictPath, BackupError> = HashMap::new();

        for file in &scan.found_files {
            let target = file.effective();

            if !file.change().is_changed() || file.ignored {
                log::info!(
                    "[{}] skipping file; change={:?}, ignored={}: {:?} -> {:?}",
                    self.mapping.name,
                    file.change,
                    file.ignored,
                    &file.path,
                    &target
                );
                continue;
            }

            if let Some(container) = file.container.as_ref() {
                if let Some(e) = failed_containers.get(container) {
                    log::warn!(
                        "[{}] skipping file because container had failed to load: {:?} -> {:?} -> {:?}",
                        self.mapping.name,
                        &container,
                        &file.path,
                        &target,
                    );
                    failed_files.insert(file.clone(), e.clone());
                    continue;
                }

                if !containers.contains_key(container) {
                    log::debug!("[{}] loading zip archive: {:?}", &self.mapping.name, &container);
                    let handle = match container.open() {
                        Ok(handle) => handle,
                        Err(e) => {
                            log::error!(
                                "[{}] failed to open zip archive: {:?} | {e:?}",
                                &self.mapping.name,
                                &container
                            );
                            failed_containers.insert(container.clone(), BackupError::Raw(e.to_string()));
                            failed_files.insert(file.clone(), BackupError::Raw(e.to_string()));
                            continue;
                        }
                    };
                    let archive = match zip::ZipArchive::new(handle) {
                        Ok(archive) => archive,
                        Err(e) => {
                            log::error!(
                                "[{}] failed to parse zip archive: {:?} | {e:?}",
                                &self.mapping.name,
                                &container
                            );
                            failed_containers.insert(container.clone(), BackupError::Raw(e.to_string()));
                            failed_files.insert(file.clone(), BackupError::Raw(e.to_string()));
                            continue;
                        }
                    };
                    log::debug!("[{}] loaded zip archive: {:?}", &self.mapping.name, &container);
                    containers.insert(container.clone(), archive);
                }
            }

            let outcome = match &file.container {
                None => self.restore_file_from_simple(target, file),
                Some(container) => {
                    let Some(archive) = containers.get_mut(container) else {
                        continue;
                    };
                    self.restore_file_from_zip(target, file, archive)
                }
            };

            match outcome {
                Ok(_) => {
                    log::info!("[{}] restored: {:?} -> {:?}", &self.mapping.name, &file.path, &target);
                }
                Err(e) => {
                    log::error!(
                        "[{}] failed to restore: {:?} -> {:?} | {e}",
                        self.mapping.name,
                        &file.path,
                        &target
                    );
                    failed_files.insert(file.clone(), BackupError::Raw(e.to_string()));
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::registry::Hives;

            if let Some(backup) = scan.backup.as_ref() {
                if let Some(registry_content) = self.registry_content(&backup.id()) {
                    if let Some(hives) = Hives::deserialize(&registry_content) {
                        if let Err(failed) = hives.restore(&scan.game_name, toggled) {
                            failed_registry.extend(failed);
                        }
                    }
                }
            }
        }

        log::trace!("[{}] completed restore", &scan.game_name);

        BackupInfo {
            failed_files,
            failed_registry,
        }
    }

    fn restore_file_from_simple(&self, target: &StrictPath, file: &ScannedFile) -> Result<(), AnyError> {
        log::trace!(
            "[{}] about to restore (simple): {:?} -> {:?}",
            self.mapping.name,
            &file.path,
            &target
        );

        Ok(file.path.copy_to_path(&self.mapping.name, target)?)
    }

    fn restore_file_from_zip(
        &self,
        target: &StrictPath,
        file: &ScannedFile,
        archive: &mut zip::ZipArchive<std::fs::File>,
    ) -> Result<(), AnyError> {
        log::debug!(
            "[{}] about to restore (zip): {:?} -> {:?}",
            self.mapping.name,
            &file.path,
            &target
        );

        if let Err(e) = target.create_parent_dir() {
            log::error!(
                "[{}] unable to create parent directories: {:?} | {e}",
                self.mapping.name,
                &target
            );
            return Err(Box::new(e));
        }
        if let Err(e) = target.unset_readonly() {
            log::warn!(
                "[{}] failed to unset read-only on target: {:?} | {e}",
                self.mapping.name,
                &target
            );
            return Err(e);
        }
        let mut target_handle = match target.create() {
            Ok(x) => x,
            Err(e) => {
                log::warn!(
                    "[{}] failed to get handle: {:?} -> {:?} | {e}",
                    self.mapping.name,
                    &file.path,
                    &target
                );
                return Err(Box::new(e));
            }
        };
        let mut source_file = archive.by_name(&file.path.raw())?;
        if let Err(e) = std::io::copy(&mut source_file, &mut target_handle) {
            log::warn!(
                "[{}] failed to copy to target: {:?} -> {:?} | {e}",
                self.mapping.name,
                &file.path,
                &target,
            );
            return Err(Box::new(e));
        }

        let mtime = source_file.last_modified();
        if let Err(e) = target.set_mtime_zip(mtime) {
            log::error!(
                "[{}] unable to set modification time: {:?} -> {:?} to {:#?} | {e:?}",
                self.mapping.name,
                &file.path,
                &target,
                mtime
            );
            return Err("unable to set modification time".into());
        }

        Ok(())
    }

    fn mapping_file(path: &StrictPath) -> StrictPath {
        path.joined("mapping.yaml")
    }

    fn find_irrelevant_backup_files(&self, backup: &str, relevant_files: &[StrictPath]) -> Vec<StrictPath> {
        #[allow(clippy::needless_collect)]
        let relevant_files: Vec<_> = relevant_files.iter().filter_map(|x| x.interpret().ok()).collect();
        let mut irrelevant_files = vec![];

        let Ok(walk_path) = self.path.joined(backup).interpret() else {
            return vec![];
        };

        for drive_dir in walkdir::WalkDir::new(walk_path)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(crate::scan::filter_map_walkdir)
            .filter(|x| x.file_name().to_string_lossy().starts_with("drive-"))
        {
            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(crate::scan::filter_map_walkdir)
                .filter(|x| x.file_type().is_file())
            {
                let backup_file = StrictPath::new(file.path().display().to_string());
                let Ok(backup_path) = backup_file.interpret() else {
                    continue;
                };
                if !relevant_files.contains(&backup_path) {
                    irrelevant_files.push(backup_file);
                }
            }
        }

        irrelevant_files
    }

    pub fn remove_irrelevant_backup_files(&self, backup: &str, relevant_files: &[StrictPath]) {
        log::trace!(
            "[{}] looking for irrelevant backup files in {}",
            self.mapping.name,
            backup
        );
        for file in self.find_irrelevant_backup_files(backup, relevant_files) {
            log::debug!("[{}] removing irrelevant backup file: {:?}", self.mapping.name, &file);
            let _ = file.remove();
        }
        log::trace!("[{}] done removing irrelevant backup files", self.mapping.name);
    }

    pub fn set_backup_comment(&mut self, backup_name: &str, comment: &str) {
        let comment = if comment.is_empty() {
            None
        } else {
            Some(comment.to_string())
        };

        'outer: for backup in &mut self.mapping.backups {
            if backup.name == backup_name {
                backup.comment = comment;
                break 'outer;
            }
            for child in &mut backup.children {
                if child.name == backup_name {
                    child.comment = comment;
                    break 'outer;
                }
            }
        }
    }

    pub fn set_backup_locked(&mut self, backup_name: &str, locked: bool) {
        'outer: for backup in &mut self.mapping.backups {
            if backup.name == backup_name {
                backup.locked = locked;
                break 'outer;
            }
            for child in &mut backup.children {
                if child.name == backup_name {
                    child.locked = locked;
                    break 'outer;
                }
            }
        }
    }

    /// Checks the latest backup (full + diff) only.
    /// Returns whether backup is valid.
    pub fn validate(&self, backup_id: BackupId) -> bool {
        if let Some((backup, diff)) = self.find_by_id(&backup_id) {
            match backup.format() {
                BackupFormat::Simple => {
                    for file in backup.files.keys() {
                        let original_path = StrictPath::new(file.to_string());
                        let stored = self
                            .mapping
                            .game_file_immutable(&self.path, &original_path, &backup.name);
                        if !stored.is_file() {
                            #[cfg(test)]
                            eprintln!("can't find {}", stored.render());
                            return false;
                        }
                    }
                }
                BackupFormat::Zip => {
                    let Ok(handle) = self.path.joined(&backup.name).open() else {
                        return false;
                    };
                    let Ok(mut archive) = zip::ZipArchive::new(handle) else {
                        return false;
                    };

                    for file in backup.files.keys() {
                        let original_path = StrictPath::new(file.to_string());
                        let stored = self.mapping.game_file_for_zip_immutable(&original_path);
                        if archive.by_name(&stored).is_err() {
                            #[cfg(test)]
                            eprintln!("can't find {}", stored);
                            return false;
                        }
                    }
                }
            }

            if let Some(backup) = diff {
                match backup.format() {
                    BackupFormat::Simple => {
                        for (file, data) in &backup.files {
                            if data.is_none() {
                                // File is deliberately omitted.
                                continue;
                            }

                            let original_path = StrictPath::new(file.to_string());
                            let stored = self
                                .mapping
                                .game_file_immutable(&self.path, &original_path, &backup.name);
                            if !stored.is_file() {
                                #[cfg(test)]
                                eprintln!("can't find {}", stored.render());
                                return false;
                            }
                        }
                    }
                    BackupFormat::Zip => {
                        let Ok(handle) = self.path.joined(&backup.name).open() else {
                            return false;
                        };
                        let Ok(mut archive) = zip::ZipArchive::new(handle) else {
                            return false;
                        };

                        for (file, data) in &backup.files {
                            if data.is_none() {
                                // File is deliberately omitted.
                                continue;
                            }

                            let original_path = StrictPath::new(file.to_string());
                            let stored = self.mapping.game_file_for_zip_immutable(&original_path);
                            if archive.by_name(&stored).is_err() {
                                #[cfg(test)]
                                eprintln!("can't find {}", stored);
                                return false;
                            }
                        }
                    }
                }
            }
        }

        true
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackupKind {
    #[default]
    Full,
    Differential,
}

#[derive(Clone, Debug, Default)]
pub struct BackupLayout {
    pub base: StrictPath,
    games: HashMap<String, StrictPath>,
    games_lowercase: HashMap<String, StrictPath>,
    retention: Retention,
}

impl BackupLayout {
    pub fn new(base: StrictPath, retention: Retention) -> Self {
        let games = Self::load(&base);
        let games_lowercase = games.iter().map(|(k, v)| (k.to_lowercase(), v.clone())).collect();
        Self {
            base,
            games,
            games_lowercase,
            retention,
        }
    }

    pub fn load(base: &StrictPath) -> HashMap<String, StrictPath> {
        let mut overall = HashMap::new();

        let Ok(base_interpreted) = base.interpret() else {
            return HashMap::new();
        };

        for game_dir in walkdir::WalkDir::new(base_interpreted)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .skip(1) // the base path itself
            .filter_map(crate::scan::filter_map_walkdir)
            .filter(|x| x.file_type().is_dir())
        {
            let game_dir = StrictPath::from(&game_dir);
            let mapping_file = game_dir.joined("mapping.yaml");
            if mapping_file.is_file() {
                match IndividualMapping::load(&mapping_file) {
                    Ok(mapping) => {
                        overall.insert(mapping.name.clone(), game_dir);
                    }
                    Err(e) => {
                        log::warn!("Ignoring unloadable mapping: {:?} | {:?}", &mapping_file, e);
                    }
                }
            }
        }

        overall
    }

    pub fn game_layout(&self, name: &str) -> GameLayout {
        let path = self.game_folder(name);

        match GameLayout::load(path.clone(), self.retention.clone()) {
            Ok(mut x) => {
                if x.mapping.name != name {
                    // This can happen if the game name changed in the manifest,
                    // but differs only by capitalization when we're on a case-insensitive OS.
                    // If we don't adjust it, it'll always show up as a new game.
                    log::info!("Updating renamed game: {} -> {}", &x.mapping.name, name);
                    x.mapping.name = name.to_string();
                }
                x
            }
            Err(_) => GameLayout {
                path,
                mapping: IndividualMapping::new(name.to_string()),
                retention: self.retention.clone(),
            },
        }
    }

    pub fn try_game_layout(&self, name: &str) -> Option<GameLayout> {
        let path = self.game_folder(name);

        GameLayout::load(path, self.retention.clone()).ok().map(|mut x| {
            if x.mapping.name != name {
                // This can happen if the game name changed in the manifest,
                // but differs only by capitalization when we're on a case-insensitive OS.
                // If we don't adjust it, it'll always show up as a new game.
                log::info!("Updating renamed game: {} -> {}", &x.mapping.name, name);
                x.mapping.name = name.to_string();
            }
            x
        })
    }

    fn contains_game(&self, name: &str) -> bool {
        self.games.contains_key(name)
            || (!Os::HOST.is_case_sensitive() && self.games_lowercase.contains_key(&name.to_lowercase()))
    }

    pub fn latest_backup(
        &self,
        name: &str,
        restoring: bool,
        redirects: &[RedirectConfig],
        toggled_paths: &ToggledPaths,
    ) -> Option<LatestBackup> {
        if self.contains_game(name) {
            let game_layout = self.game_layout(name);
            let scan = game_layout.latest_backup(restoring, redirects, toggled_paths);
            scan.map(|scan| LatestBackup {
                scan,
                registry_content: if cfg!(target_os = "windows") {
                    game_layout.registry_content(&BackupId::Latest)
                } else {
                    None
                },
            })
        } else {
            None
        }
    }

    fn generate_total_rename(original_name: &str) -> String {
        format!("ludusavi-renamed-{}", encode_base64_for_folder(original_name))
    }

    pub fn game_folder(&self, game_name: &str) -> StrictPath {
        match self.games.get::<str>(game_name) {
            Some(game) => game.clone(),
            None => {
                let mut safe_name = escape_folder_name(game_name);

                if safe_name.matches(SAFE).count() == safe_name.len() {
                    // It's unreadable now, so do a total rename.
                    safe_name = Self::generate_total_rename(game_name);
                }

                self.base.joined(&safe_name)
            }
        }
    }

    pub fn restorable_games(&self) -> Vec<String> {
        self.games.keys().cloned().collect()
    }

    pub fn restorable_game_set(&self) -> BTreeSet<String> {
        self.games.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use velcro::{btree_map, hash_map, hash_set};

    use super::*;
    use crate::testing::{drives_x, make_original_path, mapping_file_key, repo, repo_raw, s};

    mod individual_mapping {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn can_generate_drive_folder_name() {
            let mut mapping = IndividualMapping::new("foo".to_owned());
            assert_eq!("drive-0", mapping.drive_folder_name(""));
            assert_eq!("drive-C", mapping.drive_folder_name("C:"));
            assert_eq!("drive-D", mapping.drive_folder_name("D:"));
            assert_eq!("drive-____C", mapping.drive_folder_name(r#"\\?\C:"#));
            assert_eq!("drive-__remote", mapping.drive_folder_name(r#"\\remote"#));
        }
    }

    mod backup_layout {
        use pretty_assertions::assert_eq;

        use crate::testing::{repo_file_raw, repo_path, repo_path_raw};

        use super::*;

        fn layout() -> BackupLayout {
            BackupLayout::new(
                StrictPath::new(format!("{}/tests/backup", repo_raw())),
                Retention::default(),
            )
        }

        fn game_layout(name: &str, path: &str) -> GameLayout {
            GameLayout {
                path: StrictPath::new(path.to_string()),
                mapping: IndividualMapping::new(name.to_string()),
                retention: Retention::default(),
            }
        }

        fn drives() -> HashMap<String, String> {
            let (drive, _) = StrictPath::cwd().split_drive();
            let folder = IndividualMapping::new_drive_folder_name(&drive);
            hash_map! { folder: drive }
        }

        #[test]
        fn can_find_existing_game_folder_with_matching_name() {
            assert_eq!(repo_path_raw("tests/backup/game1"), layout().game_folder("game1"));
        }

        #[test]
        fn can_find_existing_game_folder_with_rename() {
            assert_eq!(
                repo_path_raw("tests/backup/game3-renamed"),
                layout().game_folder("game3")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_without_rename() {
            assert_eq!(
                repo_path("tests/backup/nonexistent"),
                layout().game_folder("nonexistent")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_with_partial_rename() {
            assert_eq!(repo_path("tests/backup/foo_bar"), layout().game_folder("foo:bar"));
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_with_total_rename() {
            assert_eq!(
                repo_path("tests/backup/ludusavi-renamed-Kioq"),
                layout().game_folder("***")
            );
        }

        #[test]
        fn can_determine_game_folder_by_escaping_dots_at_start_and_end() {
            assert_eq!(repo_path("tests/backup/_._"), layout().game_folder("..."));
        }

        #[test]
        fn can_find_irrelevant_backup_files() {
            assert_eq!(
                vec![repo_path_raw("tests/backup/game1/drive-X/file2.txt")],
                game_layout("game1", &repo_file_raw("tests/backup/game1"))
                    .find_irrelevant_backup_files(".", &[repo_path("tests/backup/game1/drive-X/file1.txt")])
            );
            assert_eq!(
                Vec::<StrictPath>::new(),
                game_layout("game1", &repo_file("tests/backup/game1")).find_irrelevant_backup_files(
                    ".",
                    &[
                        repo_path("tests/backup/game1/drive-X/file1.txt"),
                        repo_path("tests/backup/game1/drive-X/file2.txt"),
                    ]
                )
            );
        }

        fn past() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd_opt(2000, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 1)
                .unwrap()
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn past2() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd_opt(2000, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 2)
                .unwrap()
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn now() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd_opt(2000, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 5)
                .unwrap()
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn now_str() -> String {
            "20000102T030405Z".to_string()
        }

        fn repo_file(path: &str) -> String {
            format!("{}/{}", repo_raw(), path)
        }

        #[test]
        fn can_plan_backup_when_empty() {
            let scan = ScanInfo::default();
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                mapping: IndividualMapping::new("game1".to_string()),
                retention: Retention::default(),
            };
            assert_eq!(None, layout.plan_backup(&scan, &now(), &BackupFormats::default()));
        }

        #[test]
        fn can_plan_backup_kind_when_first_time() {
            let layout = GameLayout::default();
            assert_eq!(BackupKind::Full, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_merged_single_full() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup::default()]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 0,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_locked_single_full() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        locked: true,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 0,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_multiple_full() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup::default()]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 2,
                    differential: 0,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_single_full_with_differential() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup::default()]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_single_full_with_differential_rollover() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        children: VecDeque::from(vec![DifferentialBackup::default()]),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_multiple_full_with_differential_room_remaining() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![
                        FullBackup {
                            children: VecDeque::from(vec![
                                DifferentialBackup::default(),
                                DifferentialBackup::default(),
                            ]),
                            ..Default::default()
                        },
                        FullBackup {
                            children: VecDeque::from(vec![DifferentialBackup::default()]),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 2,
                    differential: 2,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_multiple_full_with_differential_at_limit() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![
                        FullBackup {
                            children: VecDeque::from(vec![
                                DifferentialBackup::default(),
                                DifferentialBackup::default(),
                            ]),
                            ..Default::default()
                        },
                        FullBackup {
                            children: VecDeque::from(vec![
                                DifferentialBackup::default(),
                                DifferentialBackup::default(),
                            ]),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 2,
                    differential: 2,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_backup_kind_when_single_full_with_differential_at_limit_but_locked() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        children: VecDeque::from(vec![
                            DifferentialBackup::default(),
                            DifferentialBackup {
                                locked: true,
                                ..Default::default()
                            },
                        ]),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 2,
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind());
        }

        #[test]
        fn can_plan_full_backup_with_files() {
            let scan = ScanInfo {
                found_files: hash_set! {
                    ScannedFile::with_change(repo_file("new"), 1, "n", ScanChange::New),
                    ScannedFile::with_change(repo_file("different"), 2, "d", ScanChange::Different),
                    ScannedFile::with_change(repo_file("removed"), 3, "r", ScanChange::Removed),
                    ScannedFile::with_change(repo_file("same"), 5, "s", ScanChange::Same),
                    ScannedFile::with_change(repo_file("unknown"), 6, "u", ScanChange::Unknown),
                },
                ..Default::default()
            };
            let layout = GameLayout::default();
            assert_eq!(
                FullBackup {
                    name: ".".to_string(),
                    when: now(),
                    os: Some(Os::HOST),
                    files: btree_map! {
                        StrictPath::new(repo_file("new")).render(): IndividualMappingFile { hash: "n".into(), size: 1 },
                        StrictPath::new(repo_file("different")).render(): IndividualMappingFile { hash: "d".into(), size: 2 },
                        StrictPath::new(repo_file("same")).render(): IndividualMappingFile { hash: "s".into(), size: 5 },
                    },
                    ..Default::default()
                },
                layout.plan_full_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_full_backup_with_registry() {
            use crate::scan::registry::{Entries, Entry, Hives, Keys};

            // `Hives` only loads values that actually exist.
            // Realistically, if a value is marked as removed`, then it won't exist, so `Hives` won't load it.
            // The removed value here only makes it into the plan because it actually does exist.
            let scan = ScanInfo {
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change_as(ScanChange::New).ignored(),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change_as(ScanChange::Different)
                        .with_value("binary", ScanChange::New, false)
                        .with_value("dword", ScanChange::Different, false)
                        .with_value("expandSz", ScanChange::Removed, false)
                        .with_value("multiSz", ScanChange::Same, false)
                        .with_value("qword", ScanChange::Same, true)
                        .with_value("sz", ScanChange::Unknown, false),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change_as(ScanChange::Removed)
                },
                ..Default::default()
            };
            let layout = GameLayout::default();
            let hives = Hives(hash_map! {
                s("HKEY_CURRENT_USER"): Keys(hash_map! {
                    s("Software\\Ludusavi\\game3"): Entries(hash_map! {
                        s("sz"): Entry::Sz("foo".into()),
                        s("multiSz"): Entry::MultiSz("bar".into()),
                        s("expandSz"): Entry::ExpandSz("baz".into()),
                        s("dword"): Entry::Dword(1),
                        s("binary"): Entry::Binary(vec![65]),
                    }),
                })
            });
            assert_eq!(
                FullBackup {
                    name: ".".to_string(),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: IndividualMappingRegistry {
                        hash: Some(crate::prelude::sha1(hives.serialize()))
                    },
                    ..Default::default()
                },
                layout.plan_full_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_differential_backup_with_files() {
            let scan = ScanInfo {
                found_files: hash_set! {
                    ScannedFile::with_change(repo_file("new"), 1, "n", ScanChange::New),
                    ScannedFile::with_change(repo_file("different"), 2, "d+", ScanChange::Different),
                    ScannedFile::with_change(repo_file("removed"), 0, "", ScanChange::Removed),
                    ScannedFile::with_change(repo_file("same"), 5, "s", ScanChange::Same),
                    ScannedFile::with_change(repo_file("unknown"), 6, "u", ScanChange::Unknown),
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btree_map! {
                            StrictPath::new(repo_file("different")).render(): IndividualMappingFile { hash: "d".into(), size: 2 },
                            StrictPath::new(repo_file("removed")).render(): IndividualMappingFile { hash: "r".into(), size: 3 },
                            StrictPath::new(repo_file("same")).render(): IndividualMappingFile { hash: "s".into(), size: 5 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    files: btree_map! {
                        StrictPath::new(repo_file("new")).render(): Some(IndividualMappingFile { hash: "n".into(), size: 1 }),
                        StrictPath::new(repo_file("different")).render(): Some(IndividualMappingFile { hash: "d+".into(), size: 2 }),
                        StrictPath::new(repo_file("removed")).render(): None,
                    },
                    registry: None,
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_new() {
            use crate::scan::registry::{Entries, Hives, Keys};

            let scan = ScanInfo {
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change_as(ScanChange::New)
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        registry: IndividualMappingRegistry { hash: None },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            };
            let hives = Hives(hash_map! {
                s("HKEY_CURRENT_USER"): Keys(hash_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            });
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: Some(IndividualMappingRegistry {
                        hash: Some(crate::prelude::sha1(hives.serialize()))
                    }),
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_changed() {
            use crate::scan::registry::{Entries, Hives, Keys};

            let scan = ScanInfo {
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change_as(ScanChange::Different)
                        .with_value("removed", ScanChange::Removed, false)
                        // Fake registry values are ignored because `Hives` re-reads the actual registry.
                        .with_value("fake", ScanChange::New, false)
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        registry: IndividualMappingRegistry {
                            hash: Some("foo".into()),
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            };
            let hives = Hives(hash_map! {
                s("HKEY_CURRENT_USER"): Keys(hash_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            });
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: Some(IndividualMappingRegistry {
                        hash: Some(crate::prelude::sha1(hives.serialize()))
                    }),
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_unchanged() {
            use crate::scan::registry::{Entries, Hives, Keys};

            let scan = ScanInfo {
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change_as(ScanChange::Same)
                },
                ..Default::default()
            };
            let hives = Hives(hash_map! {
                s("HKEY_CURRENT_USER"): Keys(hash_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            });
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        registry: IndividualMappingRegistry {
                            hash: Some(crate::prelude::sha1(hives.serialize())),
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: None,
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_removed() {
            let scan = ScanInfo {
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change_as(ScanChange::Removed)
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        registry: IndividualMappingRegistry {
                            hash: Some("foo".into()),
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: Some(IndividualMappingRegistry { hash: None }),
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_forget_excess_backups_without_locks() {
            let mut layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![
                        FullBackup {
                            name: "1".to_string(),
                            children: VecDeque::from_iter(vec![DifferentialBackup {
                                name: "1-a".to_string(),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                        FullBackup {
                            name: "2".to_string(),
                            children: VecDeque::from_iter(vec![
                                DifferentialBackup {
                                    name: "2-a".to_string(),
                                    ..Default::default()
                                },
                                DifferentialBackup {
                                    name: "2-b".to_string(),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            layout.forget_excess_backups();
            assert_eq!(
                VecDeque::from_iter(vec![FullBackup {
                    name: "2".to_string(),
                    children: VecDeque::from_iter(vec![DifferentialBackup {
                        name: "2-b".to_string(),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },]),
                layout.mapping.backups,
            );
        }

        #[test]
        fn can_forget_excess_backups_without_locks_using_duplicate_name() {
            let mut layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![
                        FullBackup {
                            name: ".".to_string(),
                            comment: Some("old".to_string()),
                            ..Default::default()
                        },
                        FullBackup {
                            name: ".".to_string(),
                            comment: Some("new".to_string()),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 0,
                    ..Default::default()
                },
                ..Default::default()
            };

            layout.forget_excess_backups();
            assert_eq!(
                VecDeque::from_iter(vec![FullBackup {
                    name: ".".to_string(),
                    comment: Some("new".to_string()),
                    ..Default::default()
                },]),
                layout.mapping.backups,
            );
        }

        #[test]
        fn can_forget_excess_backups_with_locks() {
            let mut layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![
                        FullBackup {
                            name: "1".to_string(),
                            locked: true,
                            children: VecDeque::from_iter(vec![
                                DifferentialBackup {
                                    name: "1-a".to_string(),
                                    ..Default::default()
                                },
                                DifferentialBackup {
                                    name: "1-b".to_string(),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        },
                        FullBackup {
                            name: "2".to_string(),
                            children: VecDeque::from_iter(vec![
                                DifferentialBackup {
                                    name: "2-a".to_string(),
                                    ..Default::default()
                                },
                                DifferentialBackup {
                                    name: "2-b".to_string(),
                                    locked: true,
                                    ..Default::default()
                                },
                                DifferentialBackup {
                                    name: "2-c".to_string(),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        },
                        FullBackup {
                            name: "3".to_string(),
                            ..Default::default()
                        },
                        FullBackup {
                            name: "4".to_string(),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            layout.forget_excess_backups();
            assert_eq!(
                VecDeque::from_iter(vec![
                    FullBackup {
                        name: "1".to_string(),
                        locked: true,
                        children: VecDeque::from_iter(vec![DifferentialBackup {
                            name: "1-b".to_string(),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    },
                    FullBackup {
                        name: "2".to_string(),
                        children: VecDeque::from_iter(vec![
                            DifferentialBackup {
                                name: "2-b".to_string(),
                                locked: true,
                                ..Default::default()
                            },
                            DifferentialBackup {
                                name: "2-c".to_string(),
                                ..Default::default()
                            }
                        ]),
                        ..Default::default()
                    },
                    FullBackup {
                        name: "4".to_string(),
                        ..Default::default()
                    },
                ]),
                layout.mapping.backups,
            );
        }

        fn make_path(file: &str) -> StrictPath {
            repo_path(&format!("tests/backup/game1/{}", file))
        }

        fn make_restorable_path(backup: &str, file: &str) -> StrictPath {
            StrictPath::relative(
                format!(
                    "{backup}/drive-{}/{file}",
                    if cfg!(target_os = "windows") { "X" } else { "0" }
                ),
                Some(repo_file_raw("tests/backup/game1")),
            )
        }

        fn make_restorable_path_zip(file: &str) -> StrictPath {
            StrictPath::relative(
                format!("drive-{}/{file}", if cfg!(target_os = "windows") { "X" } else { "0" }),
                None,
            )
        }

        #[test]
        fn can_report_restorable_files_for_full_backup_in_simple_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1".into(),
                        when: past(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "old".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "old".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
            };
            assert_eq!(
                hash_set! {
                    ScannedFile {
                        path: make_restorable_path("backup-1", "file1.txt"),
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: make_restorable_path("backup-1", "file2.txt"),
                        size: 2,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, false, &[], &Default::default()),
            );
        }

        #[test]
        fn can_report_restorable_files_for_full_backup_in_zip_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1.zip".into(),
                        when: past(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "old".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "old".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
            };
            assert_eq!(
                hash_set! {
                    ScannedFile {
                        path: make_restorable_path_zip("file1.txt"),
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-1.zip")),
                        redirected: None,
                    },
                    ScannedFile {
                        path: make_restorable_path_zip("file2.txt"),
                        size: 2,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-1.zip")),
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, false, &[], &Default::default()),
            );
        }

        #[test]
        fn can_report_restorable_files_for_differential_backup_in_simple_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1".into(),
                        when: past(),
                        files: btree_map! {
                            mapping_file_key("/unchanged.txt"): IndividualMappingFile { hash: "old".into(), size: 1 },
                            mapping_file_key("/changed.txt"): IndividualMappingFile { hash: "old".into(), size: 2 },
                            mapping_file_key("/delete.txt"): IndividualMappingFile { hash: "old".into(), size: 3 },
                        },
                        children: VecDeque::from([DifferentialBackup {
                            name: "backup-2".into(),
                            when: past2(),
                            files: btree_map! {
                                mapping_file_key("/changed.txt"): Some(IndividualMappingFile { hash: "new".into(), size: 2 }),
                                mapping_file_key("/delete.txt"): None,
                                mapping_file_key("/added.txt"): Some(IndividualMappingFile { hash: "new".into(), size: 5 }),
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
            };
            assert_eq!(
                hash_set! {
                    ScannedFile {
                        path: make_restorable_path("backup-1", "unchanged.txt"),
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/unchanged.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: make_restorable_path("backup-2", "changed.txt"),
                        size: 2,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/changed.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: make_restorable_path("backup-2", "added.txt"),
                        size: 5,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/added.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, false, &[], &Default::default()),
            );
        }

        #[test]
        fn can_report_restorable_files_for_differential_backup_in_zip_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1.zip".into(),
                        when: past(),
                        files: btree_map! {
                            mapping_file_key("/unchanged.txt"): IndividualMappingFile { hash: "old".into(), size: 1 },
                            mapping_file_key("/changed.txt"): IndividualMappingFile { hash: "old".into(), size: 2 },
                            mapping_file_key("/delete.txt"): IndividualMappingFile { hash: "old".into(), size: 3 },
                        },
                        children: VecDeque::from([DifferentialBackup {
                            name: "backup-2.zip".into(),
                            when: past2(),
                            files: btree_map! {
                                mapping_file_key("/changed.txt"): Some(IndividualMappingFile { hash: "new".into(), size: 2 }),
                                mapping_file_key("/delete.txt"): None,
                                mapping_file_key("/added.txt"): Some(IndividualMappingFile { hash: "new".into(), size: 5 }),
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
            };
            assert_eq!(
                hash_set! {
                    ScannedFile {
                        path: make_restorable_path_zip("unchanged.txt"),
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/unchanged.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-1.zip")),
                        redirected: None,
                    },
                    ScannedFile {
                        path: make_restorable_path_zip("changed.txt"),
                        size: 2,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/changed.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-2.zip")),
                        redirected: None,
                    },
                    ScannedFile {
                        path: make_restorable_path_zip("added.txt"),
                        size: 5,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/added.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-2.zip")),
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, false, &[], &Default::default()),
            );
        }
    }

    mod game_layout {
        use pretty_assertions::assert_eq;

        use crate::testing::{drives_x_always, repo_file_raw};

        use super::*;

        fn now() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd_opt(2000, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 5)
                .unwrap()
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn restorable_file_simple(backup: &str, file: &str) -> StrictPath {
            StrictPath::relative(
                format!(
                    "{backup}/drive-{}/{file}",
                    if cfg!(target_os = "windows") { "X" } else { "0" }
                ),
                Some(repo_file_raw("tests/backup/game1")),
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
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                },
                Retention {
                    full: 1,
                    differential: 1,
                    ..Default::default()
                },
            );
            let backups = vec![Backup::Full(FullBackup {
                name: ".".to_string(),
                when: now(),
                files: btree_map! {
                    mapping_file_key("/file1.txt"): IndividualMappingFile {
                        hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(),
                        size: 1,
                    },
                    mapping_file_key("/file2.txt"): IndividualMappingFile {
                        hash: "9d891e731f75deae56884d79e9816736b7488080".into(),
                        size: 2,
                    },
                },
                ..Default::default()
            })];

            assert_eq!(
                ScanInfo {
                    game_name: s("game1"),
                    found_files: hash_set! {
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
                layout.scan_for_restoration(
                    "game1",
                    &BackupId::Latest,
                    &[],
                    &Default::default(),
                    &Default::default()
                ),
            );
        }

        #[test]
        fn can_scan_game_for_restoration_with_registry() {
            let mut layout = BackupLayout::new(
                StrictPath::new(format!("{}/tests/backup", repo())),
                Retention::default(),
            )
            .game_layout("game3");
            if cfg!(target_os = "windows") {
                assert_eq!(
                    ScanInfo {
                        game_name: s("game3"),
                        found_registry_keys: hash_set! {
                            ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change_as(ScanChange::Same)
                                .with_value_same("binary")
                                .with_value_same("dword")
                                .with_value_same("expandSz")
                                .with_value_same("multiSz")
                                .with_value_same("qword")
                                .with_value_same("sz")
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
                    layout.scan_for_restoration(
                        "game3",
                        &BackupId::Latest,
                        &[],
                        &Default::default(),
                        &Default::default()
                    ),
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
                            registry: IndividualMappingRegistry {
                                hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into())
                            },
                            ..Default::default()
                        })),
                        ..Default::default()
                    },
                    layout.scan_for_restoration(
                        "game3",
                        &BackupId::Latest,
                        &[],
                        &Default::default(),
                        &Default::default()
                    ),
                );
            }
        }

        #[test]
        fn can_validate_a_simple_full_backup_when_valid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                ..Default::default()
            };
            assert!(layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_simple_full_backup_when_invalid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".into(),
                        files: btree_map! {
                            mapping_file_key("/fake.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                ..Default::default()
            };
            assert!(!layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_simple_diff_backup_when_valid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        children: VecDeque::from(vec![DifferentialBackup {
                            name: ".".into(),
                            files: btree_map! {
                                mapping_file_key("/file1.txt"): None,
                                mapping_file_key("/file2.txt"): Some(IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 }),
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                ..Default::default()
            };
            assert!(layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_simple_diff_backup_when_invalid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        children: VecDeque::from(vec![DifferentialBackup {
                            name: ".".into(),
                            files: btree_map! {
                                mapping_file_key("/fake.txt"): Some(IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 }),
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
                ..Default::default()
            };
            assert!(!layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_zip_full_backup_when_valid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "test.zip".into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1-zipped", repo_raw())),
                ..Default::default()
            };
            assert!(layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_zip_full_backup_when_invalid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "test.zip".into(),
                        files: btree_map! {
                            mapping_file_key("/fake.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1-zipped", repo_raw())),
                ..Default::default()
            };
            assert!(!layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_zip_diff_backup_when_valid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "test.zip".into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        children: VecDeque::from(vec![DifferentialBackup {
                            name: "test.zip".into(),
                            files: btree_map! {
                                mapping_file_key("/file1.txt"): None,
                                mapping_file_key("/file2.txt"): Some(IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 }),
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1-zipped", repo_raw())),
                ..Default::default()
            };
            assert!(layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_zip_diff_backup_when_invalid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "test.zip".into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        children: VecDeque::from(vec![DifferentialBackup {
                            name: "test.zip".into(),
                            files: btree_map! {
                                mapping_file_key("/fake.txt"): Some(IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 }),
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1-zipped", repo_raw())),
                ..Default::default()
            };
            assert!(!layout.validate(BackupId::Latest));
        }
    }
}
