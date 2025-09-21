use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    io::Write,
};

use chrono::{Datelike, Timelike};

use crate::{
    path::StrictPath,
    prelude::{AnyError, Error, INVALID_FILE_CHARS},
    resource::{
        config::{
            BackupFormat, BackupFormats, RedirectConfig, Retention, ToggledPaths, ToggledRegistry, ZipCompression,
        },
        manifest::Os,
    },
    scan::{
        game_file_target, prepare_backup_target, registry, BackupError, BackupId, BackupInfo, ScanChange, ScanInfo,
        ScanKind, ScannedFile,
    },
};

#[cfg_attr(not(target_os = "windows"), allow(unused))]
use crate::scan::ScannedRegistry;

const SAFE: &str = "_";
const SOLO: &str = ".";

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
    pub when: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub registry_content: Option<registry::Hives>,
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

    #[cfg_attr(not(target_os = "windows"), allow(unused))]
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
                    if backup_info.failed_files.keys().any(|x| x.raw() == file) {
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
                    if backup_info.failed_files.keys().any(|x| x.raw() == file) {
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
#[serde(default, rename_all = "camelCase")]
pub struct FullBackup {
    pub name: String,
    pub when: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Os>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Locked backups do not count toward retention limits and are never deleted.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    pub files: BTreeMap<String, IndividualMappingFile>,
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
#[serde(default, rename_all = "camelCase")]
pub struct DifferentialBackup {
    pub name: String,
    pub when: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Os>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Locked backups do not count toward retention limits and are never deleted.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    pub files: BTreeMap<String, Option<IndividualMappingFile>>,
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

#[derive(Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IndividualMappingFile {
    pub hash: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IndividualMappingRegistry {
    pub hash: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IndividualMapping {
    pub name: String,
    pub drives: BTreeMap<String, String>,
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
        StrictPath::relative(format!("{backup}/{drive_folder}/{plain_path}"), base.interpret().ok())
    }

    pub fn game_file_immutable(&self, base: &StrictPath, original_file: &StrictPath, backup: &str) -> StrictPath {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name_immutable(&drive);
        StrictPath::relative(format!("{backup}/{drive_folder}/{plain_path}"), base.interpret().ok())
    }

    fn game_file_for_zip(&mut self, original_file: &StrictPath) -> String {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name(&drive);
        format!("{drive_folder}/{plain_path}").replace('\\', "/")
    }

    fn game_file_for_zip_immutable(&self, original_file: &StrictPath) -> String {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name_immutable(&drive);
        format!("{drive_folder}/{plain_path}").replace('\\', "/")
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
        let content = Self::load_raw(file).map_err(|e| {
            log::error!("Unable to read mapping: {:?} | {:?}", &file, e);
            e
        })?;
        let mut parsed = Self::load_from_string(&content).map_err(|e| {
            log::error!("Unable to parse mapping: {:?} | {:?}", &file, e);
            e
        })?;

        // Handle legacy files without backup timestamps.
        for full in parsed.backups.iter_mut() {
            if full.name == SOLO && full.when == chrono::DateTime::<chrono::Utc>::default() {
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

        if !self.has_backup(SOLO) {
            for format in registry::Format::ALL {
                irrelevant.push(base.joined(format.filename()));
            }
        }

        let Ok(base) = base.interpret() else {
            return vec![];
        };

        for child in walkdir::WalkDir::new(base)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|x| crate::scan::filter_map_walkdir(&self.name, x))
        {
            let name = child.file_name().to_string_lossy();

            if name.starts_with("drive-") && !self.has_backup(SOLO) {
                irrelevant.push(StrictPath::from(&child));
            }
            if name.starts_with("backup-") && !relevant.clone().any(|x| x == name) {
                irrelevant.push(StrictPath::from(&child));
            }
        }

        irrelevant
    }
}

#[derive(Clone, Debug, Default)]
pub struct GameLayout {
    pub path: StrictPath,
    mapping: IndividualMapping,
}

impl GameLayout {
    #[cfg(test)]
    pub fn new(path: StrictPath, mapping: IndividualMapping) -> Self {
        Self { path, mapping }
    }

    pub fn load(path: StrictPath) -> Result<Self, AnyError> {
        let mapping = Self::mapping_file(&path);
        Ok(Self {
            path,
            mapping: IndividualMapping::load(&mapping)?,
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

    pub fn validate_id(&self, id: &BackupId) -> Result<(), Error> {
        match self.find_by_id(id) {
            Some(_) => Ok(()),
            None => match id {
                BackupId::Latest => Err(Error::NoSaveDataFound),
                BackupId::Named(_) => Err(Error::CliInvalidBackupId),
            },
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
        scan_kind: ScanKind,
        redirects: &[RedirectConfig],
        reverse_redirects_on_restore: bool,
        toggled_paths: &ToggledPaths,
        only_constructive_backups: bool,
    ) -> Option<ScanInfo> {
        if self.mapping.backups.is_empty() {
            None
        } else {
            Some(ScanInfo {
                game_name: self.mapping.name.clone(),
                found_files: self.restorable_files(
                    &BackupId::Latest,
                    scan_kind,
                    redirects,
                    reverse_redirects_on_restore,
                    toggled_paths,
                ),
                // Registry is handled separately.
                found_registry_keys: Default::default(),
                available_backups: vec![],
                backup: None,
                has_backups: true,
                // Registry is handled separately.
                dumped_registry: None,
                only_constructive_backups,
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
        scan_kind: ScanKind,
        redirects: &[RedirectConfig],
        reverse_redirects_on_restore: bool,
        toggled_paths: &ToggledPaths,
    ) -> HashMap<StrictPath, ScannedFile> {
        let mut files = HashMap::new();

        match self.find_by_id(id) {
            None => {}
            Some((full, None)) => {
                files.extend(self.restorable_files_from_full_backup(
                    full,
                    scan_kind,
                    redirects,
                    reverse_redirects_on_restore,
                    toggled_paths,
                ));
            }
            Some((full, Some(diff))) => {
                files.extend(self.restorable_files_from_diff_backup(
                    diff,
                    scan_kind,
                    redirects,
                    reverse_redirects_on_restore,
                    toggled_paths,
                ));

                for (scan_key, full_file) in self.restorable_files_from_full_backup(
                    full,
                    scan_kind,
                    redirects,
                    reverse_redirects_on_restore,
                    toggled_paths,
                ) {
                    let original_path = full_file.original_path.as_ref().unwrap().render();
                    if diff.file(original_path) == BackupInclusion::Inherited {
                        files.insert(scan_key, full_file);
                    }
                }
            }
        }

        files
    }

    fn restorable_files_from_full_backup(
        &self,
        backup: &FullBackup,
        scan_kind: ScanKind,
        redirects: &[RedirectConfig],
        reverse_redirects_on_restore: bool,
        toggled_paths: &ToggledPaths,
    ) -> HashMap<StrictPath, ScannedFile> {
        let mut restorables = HashMap::new();

        for (mapping_key, v) in &backup.files {
            let original_path = StrictPath::new(mapping_key.to_string());
            let redirected = game_file_target(
                &original_path,
                redirects,
                reverse_redirects_on_restore,
                ScanKind::Restore,
            );
            let ignorable_path = redirected.as_ref().unwrap_or(&original_path);
            match backup.format() {
                BackupFormat::Simple => {
                    let scan_key = self
                        .mapping
                        .game_file_immutable(&self.path, &original_path, &backup.name);

                    restorables.insert(
                        scan_key,
                        ScannedFile {
                            change: match scan_kind {
                                ScanKind::Backup => ScanChange::Unknown,
                                ScanKind::Restore => {
                                    ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                                }
                            },
                            size: v.size,
                            hash: v.hash.clone(),
                            ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                            redirected,
                            original_path: Some(original_path),
                            container: None,
                        },
                    );
                }
                BackupFormat::Zip => {
                    let scan_key = StrictPath::new(self.mapping.game_file_for_zip_immutable(&original_path));

                    restorables.insert(
                        scan_key,
                        ScannedFile {
                            change: match scan_kind {
                                ScanKind::Backup => ScanChange::Unknown,
                                ScanKind::Restore => {
                                    ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                                }
                            },
                            size: v.size,
                            hash: v.hash.clone(),
                            ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                            redirected,
                            original_path: Some(original_path),
                            container: Some(self.path.joined(&backup.name)),
                        },
                    );
                }
            }
        }

        restorables
    }

    fn restorable_files_from_diff_backup(
        &self,
        backup: &DifferentialBackup,
        scan_kind: ScanKind,
        redirects: &[RedirectConfig],
        reverse_redirects_on_restore: bool,
        toggled_paths: &ToggledPaths,
    ) -> HashMap<StrictPath, ScannedFile> {
        let mut restorables = HashMap::new();

        for (mapping_key, v) in &backup.files {
            let v = some_or_continue!(v);
            let original_path = StrictPath::new(mapping_key.to_string());
            let redirected = game_file_target(
                &original_path,
                redirects,
                reverse_redirects_on_restore,
                ScanKind::Restore,
            );
            let ignorable_path = redirected.as_ref().unwrap_or(&original_path);
            match backup.format() {
                BackupFormat::Simple => {
                    let scan_key = self
                        .mapping
                        .game_file_immutable(&self.path, &original_path, &backup.name);

                    restorables.insert(
                        scan_key,
                        ScannedFile {
                            change: match scan_kind {
                                ScanKind::Backup => ScanChange::Unknown,
                                ScanKind::Restore => {
                                    ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                                }
                            },
                            size: v.size,
                            hash: v.hash.clone(),
                            ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                            redirected,
                            original_path: Some(original_path),
                            container: None,
                        },
                    );
                }
                BackupFormat::Zip => {
                    let scan_key = StrictPath::new(self.mapping.game_file_for_zip_immutable(&original_path));

                    restorables.insert(
                        scan_key,
                        ScannedFile {
                            change: match scan_kind {
                                ScanKind::Backup => ScanChange::Unknown,
                                ScanKind::Restore => {
                                    ScanChange::evaluate_restore(redirected.as_ref().unwrap_or(&original_path), &v.hash)
                                }
                            },
                            size: v.size,
                            hash: v.hash.clone(),
                            ignored: toggled_paths.is_ignored(&self.mapping.name, ignorable_path),
                            redirected,
                            original_path: Some(original_path),
                            container: Some(self.path.joined(&backup.name)),
                        },
                    );
                }
            }
        }

        restorables
    }

    // Since this is only used for a specific migration use case,
    // we don't need to fill out all of the `ScannedFile` info.
    fn restorable_files_in_simple(&self, backup: &str) -> HashMap<StrictPath, ScannedFile> {
        let Ok(path) = self.path.joined(backup).interpret() else {
            return HashMap::new();
        };

        let mut files = HashMap::new();
        for drive_dir in walkdir::WalkDir::new(path)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|x| crate::scan::filter_map_walkdir(&self.mapping.name, x))
        {
            let raw_drive_dir = drive_dir.path().display().to_string();
            let drive_mapping =
                some_or_continue!(self.mapping.drives.get::<str>(&drive_dir.file_name().to_string_lossy()));

            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(|x| crate::scan::filter_map_walkdir(&self.mapping.name, x))
                .filter(|x| x.file_type().is_file())
            {
                let raw_file = file.path().display().to_string();
                let original_path = Some(StrictPath::new(raw_file.replace(&raw_drive_dir, drive_mapping)));
                let scan_key = StrictPath::new(raw_file);
                let size = scan_key.size();
                let hash = scan_key.sha1();
                files.insert(
                    scan_key,
                    ScannedFile {
                        change: crate::scan::ScanChange::Unknown,
                        size,
                        hash,
                        original_path,
                        ignored: false,
                        container: None,
                        redirected: None,
                    },
                );
            }
        }
        files
    }

    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    pub fn registry_content(&self, id: &BackupId) -> Option<registry::Hives> {
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

    fn registry_content_in(&self, backup: &str, format: &BackupFormat) -> Option<registry::Hives> {
        match format {
            BackupFormat::Simple => {
                for format in registry::Format::ALL {
                    let candidate = self.path.joined(backup).joined(format.filename());
                    let hives = registry::Hives::load(&candidate);
                    if hives.is_some() {
                        return hives;
                    }
                }

                None
            }
            BackupFormat::Zip => {
                let handle = self.path.joined(backup).open().ok()?;
                let mut archive = zip::ZipArchive::new(handle).ok()?;

                for format in registry::Format::ALL {
                    if let Ok(mut file) = archive.by_name(format.filename()) {
                        let mut buffer = vec![];
                        std::io::copy(&mut file, &mut buffer).ok()?;
                        let content = String::from_utf8(buffer).ok()?;

                        return registry::Hives::deserialize(&content, *format);
                    }
                }

                None
            }
        }
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
        retention: Retention,
    ) -> String {
        if *kind == BackupKind::Full
            && retention.full == 1
            && format.chosen == BackupFormat::Simple
            && self.mapping.backups.iter().all(|x| !x.locked)
        {
            SOLO.to_string()
        } else {
            let timestamp = Self::generate_file_friendly_timestamp(now);
            let name = match *kind {
                BackupKind::Full => format!("backup-{timestamp}"),
                BackupKind::Differential => format!("backup-{timestamp}-diff"),
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
        retention: Retention,
    ) -> Option<Backup> {
        if !scan.found_anything_processable() && !retention.force_new_full {
            return None;
        }

        let kind = self.plan_backup_kind(retention);

        let backup = match kind {
            BackupKind::Full => Backup::Full(self.plan_full_backup(scan, now, format, retention)),
            BackupKind::Differential => {
                Backup::Differential(self.plan_differential_backup(scan, now, format, retention))
            }
        };

        backup.needed().then_some(backup)
    }

    fn plan_backup_kind(&self, retention: Retention) -> BackupKind {
        if retention.force_new_full {
            return BackupKind::Full;
        }

        let fulls = self.mapping.backups.iter().filter(|full| !full.locked).count() as u8;
        let diffs = self
            .mapping
            .backups
            .back()
            .map(|x| x.children.iter().filter(|diff| !diff.locked).count())
            .unwrap_or(0) as u8;

        if fulls > 0 && (diffs < retention.differential || (retention.full == 1 && retention.differential > 0)) {
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
        retention: Retention,
    ) -> FullBackup {
        let mut files = BTreeMap::new();
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut registry = IndividualMappingRegistry::default();

        for (scan_key, file) in scan.found_files.iter().filter(|(_, x)| !x.ignored) {
            match file.change() {
                ScanChange::New | ScanChange::Different | ScanChange::Same => {
                    files.insert(
                        file.mapping_key(scan_key),
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
            registry.hash = hives.sha1(registry::Format::Reg);
        }

        FullBackup {
            name: self.generate_backup_name(&BackupKind::Full, now, format, retention),
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
        retention: Retention,
    ) -> DifferentialBackup {
        let mut files = BTreeMap::new();
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut registry = Some(IndividualMappingRegistry::default());

        for (scan_key, file) in &scan.found_files {
            match file.change() {
                ScanChange::New | ScanChange::Different | ScanChange::Same => {
                    files.insert(
                        file.mapping_key(scan_key),
                        (!file.ignored).then(|| IndividualMappingFile {
                            hash: file.hash.clone(),
                            size: file.size,
                        }),
                    );
                }
                ScanChange::Removed => {
                    files.insert(file.mapping_key(scan_key), None);
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
                registry = Some(IndividualMappingRegistry {
                    hash: hives.sha1(registry::Format::Reg),
                });
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
            name: self.generate_backup_name(&BackupKind::Differential, now, format, retention),
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
        for (scan_key, file) in &scan.found_files {
            if !backup.includes_file(file.mapping_key(scan_key)) {
                log::debug!("[{}] skipped: {}", self.mapping.name, scan_key.raw());
                continue;
            }

            let target_file = self
                .mapping
                .game_file(&self.path, file.effective(scan_key), backup.name());
            if scan_key.same_content(&target_file) {
                log::info!(
                    "[{}] already matches: {:?} -> {:?}",
                    self.mapping.name,
                    &scan_key,
                    &target_file
                );
                relevant_files.push(target_file);
                continue;
            }
            if let Err(e) = scan_key.copy_to_path(&self.mapping.name, &target_file) {
                backup_info
                    .failed_files
                    .insert(scan_key.clone(), BackupError::Raw(e.to_string()));
                continue;
            }
            log::info!("[{}] backed up: {:?} -> {:?}", self.mapping.name, scan_key, target_file);
            relevant_files.push(target_file);
        }

        #[cfg(target_os = "windows")]
        {
            if backup.includes_registry() {
                let target_registry_file = self.path.joined(backup.name()).joined(registry::Format::Reg.filename());
                let mut hives = registry::Hives::default();
                if let Err(failed) = hives.back_up(&scan.game_name, &scan.found_registry_keys) {
                    backup_info.failed_registry.extend(failed);
                }
                hives.save(&target_registry_file);
                relevant_files.push(target_registry_file);
            }
        }

        if backup.full() && backup.name() == SOLO {
            self.remove_irrelevant_backup_files(backup.name(), &relevant_files);
            self.remove_empty_backup_subdirs(backup.name());
        }

        backup_info
    }

    fn execute_backup_as_zip(&mut self, backup: &Backup, scan: &ScanInfo, format: &BackupFormats) -> BackupInfo {
        let mut backup_info = BackupInfo::default();

        let fail_file = |file: &StrictPath, backup_info: &mut BackupInfo, error: String| {
            backup_info.failed_files.insert(file.clone(), BackupError::Raw(error))
        };
        let fail_all = |backup_info: &mut BackupInfo, error: String| {
            for file in scan.found_files.keys() {
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

        'item: for (scan_key, file) in &scan.found_files {
            if !backup.includes_file(file.mapping_key(scan_key)) {
                log::debug!("[{}] skipped: {:?}", self.mapping.name, &scan_key);
                continue;
            }

            let target_file_id = self.mapping.game_file_for_zip(file.effective(scan_key));

            let mtime = match scan_key.get_mtime_zip() {
                Ok(x) => x,
                Err(e) => {
                    log::error!(
                        "[{}] unable to get mtime: {:?} -> {} | {e}",
                        self.mapping.name,
                        &scan_key,
                        &target_file_id
                    );
                    fail_file(scan_key, &mut backup_info, e.to_string());
                    continue;
                }
            };

            #[cfg(target_os = "windows")]
            let mode: Option<u32> = None;
            #[cfg(not(target_os = "windows"))]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                scan_key.metadata().map(|metadata| metadata.permissions().mode()).ok()
            };

            let local_options = match mode {
                Some(mode) => options.last_modified_time(mtime).unix_permissions(mode),
                None => options.last_modified_time(mtime),
            };

            if let Err(e) = zip.start_file(&target_file_id, local_options) {
                log::error!(
                    "[{}] unable to start zip file record: {:?} -> {} | {e}",
                    self.mapping.name,
                    &scan_key,
                    &target_file_id
                );
                fail_file(scan_key, &mut backup_info, e.to_string());
                continue;
            }

            use std::io::Read;
            let handle = match scan_key.open() {
                Ok(x) => x,
                Err(e) => {
                    log::error!("[{}] unable to open source: {:?} | {e}", self.mapping.name, &scan_key);
                    fail_file(scan_key, &mut backup_info, e.to_string());
                    continue;
                }
            };
            let mut reader = std::io::BufReader::new(handle);
            let mut buffer = [0; 1024];

            loop {
                let read = match reader.read(&mut buffer[..]) {
                    Ok(x) => x,
                    Err(e) => {
                        log::error!("[{}] unable to read source: {:?} | {e}", self.mapping.name, &scan_key);
                        fail_file(scan_key, &mut backup_info, e.to_string());
                        continue 'item;
                    }
                };
                if read == 0 {
                    log::info!(
                        "[{}] backed up: {:?} -> {}",
                        self.mapping.name,
                        &scan_key,
                        &target_file_id
                    );
                    break;
                }
                if let Err(e) = zip.write_all(&buffer[0..read]) {
                    log::error!(
                        "[{}] unable to write target: {:?} -> {} | {e}",
                        self.mapping.name,
                        &scan_key,
                        &target_file_id
                    );
                    fail_file(scan_key, &mut backup_info, e.to_string());
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
                let format = registry::Format::Reg;
                if zip.start_file(format.filename(), options).is_ok() {
                    let _ = zip.write_all(hives.serialize(format).as_bytes());
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

    fn forget_excess_backups(&mut self, retention: Retention) {
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
        let mut excess_fulls = unlocked_fulls.saturating_sub(retention.full as usize);

        for (i, full) in self.mapping.backups.iter_mut().enumerate() {
            let locked = full.locked || full.children.iter().any(|diff| diff.locked);
            if !locked && excess_fulls > 0 {
                excess.push((i, None));
                excess_fulls -= 1;
            }

            let unlocked_diffs = full.children.iter().filter(|diff| !diff.locked).count();
            let mut excess_diffs = unlocked_diffs.saturating_sub(retention.differential as usize);

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

    /// Handle legacy/irregular backups.
    pub fn migrate_backups(&mut self, save: bool) {
        self.migrate_legacy_backup(save);
        self.migrate_initial_empty_backup(save);
    }

    /// Handle legacy backups from before multi-backup support.
    pub fn migrate_legacy_backup(&mut self, save: bool) {
        if !self.mapping.backups.is_empty() || self.mapping.drives.is_empty() {
            // If `backups` are not empty, then we've already migrated and have backups.
            // If `drives` is empty, then this is a brand new mapping and there are no backups yet.
            return;
        }

        let mut backup = FullBackup {
            name: SOLO.to_string(),
            ..Default::default()
        };

        log::info!("[{}] migrating legacy backup", &self.mapping.name);

        for (scan_key, file) in self.restorable_files_in_simple(&backup.name) {
            backup.files.insert(
                file.mapping_key(&scan_key),
                IndividualMappingFile {
                    hash: scan_key.sha1(),
                    size: scan_key.size(),
                },
            );
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hives) = self.registry_content_in(&backup.name, &BackupFormat::Simple) {
                backup.registry = IndividualMappingRegistry {
                    hash: hives.sha1(registry::Format::Yaml),
                };
            }
        }

        if !backup.files.is_empty() || backup.registry.hash.is_some() {
            self.mapping.backups.push_back(backup);
            if save {
                self.save();
            }
        }
    }

    /// See: https://github.com/mtkennerly/ludusavi/issues/360
    fn migrate_initial_empty_backup(&mut self, save: bool) -> Option<()> {
        let initial = self.mapping.backups.front_mut()?;
        if !initial.files.is_empty() || initial.registry.hash.is_some() {
            // Initial backup is not empty.
            return None;
        }

        if initial.children.is_empty() {
            self.mapping.backups.pop_front();
            if save {
                self.save();
            }
            return Some(());
        }

        let DifferentialBackup {
            name,
            when,
            os,
            comment,
            locked,
            files,
            registry,
        } = initial.children.pop_front()?;

        initial.name = name;
        initial.when = when;
        initial.os = os;
        initial.comment = comment;
        initial.locked = initial.locked || locked;
        initial.files = files.into_iter().filter_map(|(k, v)| Some((k, v?))).collect();
        if let Some(registry) = registry {
            initial.registry = registry;
        }

        if save {
            self.save();
        }

        Some(())
    }

    pub fn back_up(
        &mut self,
        scan: &ScanInfo,
        now: &chrono::DateTime<chrono::Utc>,
        format: &BackupFormats,
        retention: Retention,
        only_constructive: bool,
    ) -> Option<BackupInfo> {
        if !scan.found_anything() {
            log::trace!("[{}] nothing to back up", &scan.game_name);
            return None;
        }

        if only_constructive && !scan.found_constructive() {
            log::info!("[{}] nothing constructive to back up", &scan.game_name);
            return None;
        }

        log::trace!("[{}] preparing for backup", &scan.game_name);
        if let Err(e) = prepare_backup_target(&self.path) {
            log::error!(
                "[{}] failed to prepare backup target: {:?} | {e:?}",
                scan.game_name,
                &self.path
            );
            return Some(BackupInfo::total_failure(scan, BackupError::App(e)));
        }

        self.migrate_backups(true);
        match self.plan_backup(scan, now, format, retention) {
            None => {
                log::info!("[{}] no need for new backup", &scan.game_name);
                None
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
                    self.forget_excess_backups(retention);
                    self.save();
                }
                self.prune_irrelevant_parents();
                Some(backup_info)
            }
        }
    }

    pub fn get_backups(&mut self) -> Vec<Backup> {
        let mut available_backups = vec![];

        if self.path.is_dir() {
            self.migrate_backups(true);
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
        reverse_redirects_on_restore: bool,
        toggled_paths: &ToggledPaths,
        #[cfg_attr(not(target_os = "windows"), allow(unused))] toggled_registry: &ToggledRegistry,
    ) -> ScanInfo {
        log::trace!("[{name}] beginning scan for restore");

        let mut found_files = HashMap::new();
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut found_registry_keys = HashMap::new();
        let mut available_backups = vec![];
        let mut backup = None;
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut dumped_registry = None;

        let id = self.verify_id(id);

        if self.path.is_dir() {
            self.migrate_backups(true);
            found_files = self.restorable_files(
                &id,
                ScanKind::Restore,
                redirects,
                reverse_redirects_on_restore,
                toggled_paths,
            );
            available_backups = self.restorable_backups_flattened();
            backup = self.find_by_id_flattened(&id);
        }

        #[cfg(target_os = "windows")]
        {
            use crate::scan::{registry, RegistryItem, ScannedRegistryValue, ScannedRegistryValues};

            if let Some(hives) = self.registry_content(&id) {
                for (hive_name, keys) in hives.0.iter() {
                    for (key_name, entries) in keys.0.iter() {
                        let live_entries = registry::win::try_read_registry_key(hive_name, key_name);
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

                        let ignored = toggled_registry.is_ignored(name, &path, None)
                            && entries
                                .0
                                .keys()
                                .all(|x| toggled_registry.is_ignored(name, &path, Some(x)));

                        found_registry_keys.insert(
                            path,
                            ScannedRegistry {
                                ignored,
                                change: match &live_entries {
                                    None => ScanChange::New,
                                    Some(_) => ScanChange::Same,
                                },
                                values: live_values,
                            },
                        );
                    }
                }

                dumped_registry = Some(hives);
            }
        }

        let has_backups = !available_backups.is_empty();

        log::trace!("[{name}] completed scan for restore");

        ScanInfo {
            game_name: name.to_string(),
            found_files,
            found_registry_keys,
            available_backups,
            backup,
            has_backups,
            dumped_registry,
            only_constructive_backups: false,
        }
    }

    pub fn restore(
        &self,
        scan: &ScanInfo,
        #[cfg_attr(not(target_os = "windows"), allow(unused))] toggled: &ToggledRegistry,
    ) -> BackupInfo {
        log::trace!("[{}] beginning restore", &scan.game_name);

        let mut failed_files = HashMap::new();
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut failed_registry = HashMap::new();

        let mut containers: HashMap<StrictPath, zip::ZipArchive<std::fs::File>> = HashMap::new();
        let mut failed_containers: HashMap<StrictPath, BackupError> = HashMap::new();

        for (scan_key, file) in &scan.found_files {
            let target = file.effective(scan_key);

            if !file.change().is_changed() || file.ignored {
                log::info!(
                    "[{}] skipping file; change={:?}, ignored={}: {:?} -> {:?}",
                    self.mapping.name,
                    file.change,
                    file.ignored,
                    scan_key,
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
                        scan_key,
                        &target,
                    );
                    failed_files.insert(scan_key.clone(), e.clone());
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
                            failed_files.insert(scan_key.clone(), BackupError::Raw(e.to_string()));
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
                            failed_files.insert(scan_key.clone(), BackupError::Raw(e.to_string()));
                            continue;
                        }
                    };
                    log::debug!("[{}] loaded zip archive: {:?}", &self.mapping.name, &container);
                    containers.insert(container.clone(), archive);
                }
            }

            let outcome = match &file.container {
                None => self.restore_file_from_simple(target, scan_key),
                Some(container) => {
                    let Some(archive) = containers.get_mut(container) else {
                        continue;
                    };
                    self.restore_file_from_zip(target, scan_key, archive)
                }
            };

            match outcome {
                Ok(_) => {
                    log::info!("[{}] restored: {:?} -> {:?}", &self.mapping.name, scan_key, &target);
                }
                Err(e) => {
                    log::error!(
                        "[{}] failed to restore: {:?} -> {:?} | {e}",
                        self.mapping.name,
                        scan_key,
                        &target
                    );
                    failed_files.insert(scan_key.clone(), BackupError::Raw(e.to_string()));
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(backup) = scan.backup.as_ref() {
                if let Some(hives) = self.registry_content(&backup.id()) {
                    if let Err(failed) = hives.restore(&scan.game_name, toggled) {
                        failed_registry.extend(failed);
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

    fn restore_file_from_simple(&self, target: &StrictPath, scan_key: &StrictPath) -> Result<(), AnyError> {
        log::trace!(
            "[{}] about to restore (simple): {:?} -> {:?}",
            self.mapping.name,
            scan_key,
            &target
        );

        Ok(scan_key.copy_to_path(&self.mapping.name, target)?)
    }

    fn restore_file_from_zip(
        &self,
        target: &StrictPath,
        scan_key: &StrictPath,
        archive: &mut zip::ZipArchive<std::fs::File>,
    ) -> Result<(), AnyError> {
        log::debug!(
            "[{}] about to restore (zip): {:?} -> {:?}",
            self.mapping.name,
            scan_key,
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
                    scan_key,
                    &target
                );
                return Err(Box::new(e));
            }
        };
        let mut source_file = archive.by_name(scan_key.raw())?;
        if let Err(e) = std::io::copy(&mut source_file, &mut target_handle) {
            log::warn!(
                "[{}] failed to copy to target: {:?} -> {:?} | {e}",
                self.mapping.name,
                &scan_key,
                &target,
            );
            return Err(Box::new(e));
        }

        let mtime = source_file.last_modified();
        if let Err(e) = target.set_mtime_zip(mtime) {
            log::error!(
                "[{}] unable to set modification time: {:?} -> {:?} to {:#?} | {e:?}",
                self.mapping.name,
                scan_key,
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
        let relevant_files: Vec<_> = relevant_files.iter().filter_map(|x| x.interpret().ok()).collect();
        let mut irrelevant_files = vec![];

        let Ok(walk_path) = self.path.joined(backup).interpret() else {
            return vec![];
        };

        for format in registry::Format::ALL {
            let Ok(path) = self.path.joined(format.filename()).interpreted() else {
                continue;
            };
            if !relevant_files.contains(&path.raw().into()) && path.is_file() {
                irrelevant_files.push(path);
            }
        }

        for drive_dir in walkdir::WalkDir::new(walk_path)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|x| crate::scan::filter_map_walkdir(&self.mapping.name, x))
            .filter(|x| x.file_name().to_string_lossy().starts_with("drive-"))
        {
            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(|x| crate::scan::filter_map_walkdir(&self.mapping.name, x))
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

    fn remove_irrelevant_backup_files(&self, backup: &str, relevant_files: &[StrictPath]) {
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

    fn remove_empty_backup_subdirs(&self, backup: &str) {
        log::trace!("[{}] looking for empty backup subdirs in {}", self.mapping.name, backup);

        let Ok(walk_path) = self.path.joined(backup).interpret() else {
            return;
        };

        for drive_dir in walkdir::WalkDir::new(walk_path)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|x| crate::scan::filter_map_walkdir(&self.mapping.name, x))
            .filter(|x| x.file_name().to_string_lossy().starts_with("drive-"))
        {
            for entry in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .contents_first(true)
                .into_iter()
                .filter_map(|x| crate::scan::filter_map_walkdir(&self.mapping.name, x))
                .filter(|x| x.file_type().is_dir())
            {
                let empty = std::fs::read_dir(entry.path()).is_ok_and(|mut xs| xs.next().is_none());
                if empty {
                    let folder = StrictPath::new(entry.path().display().to_string());
                    log::debug!("[{}] removing empty backup subdir: {:?}", self.mapping.name, &folder);
                    let _ = folder.remove();
                }
            }
        }

        log::trace!("[{}] done removing empty backup subdirs", self.mapping.name);
    }

    pub fn modify_backup(
        &mut self,
        id: &BackupId,
        on_full: impl FnOnce(&mut FullBackup),
        on_diff: impl FnOnce(&mut DifferentialBackup),
    ) {
        match id {
            BackupId::Latest => {
                if let Some(full) = self.mapping.backups.back_mut() {
                    if let Some(diff) = full.children.back_mut() {
                        on_diff(diff);
                    } else {
                        on_full(full);
                    }
                }
            }
            BackupId::Named(id) => {
                for full in &mut self.mapping.backups {
                    if full.name == *id {
                        on_full(full);
                        return;
                    }
                    for diff in &mut full.children {
                        if diff.name == *id {
                            on_diff(diff);
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn set_backup_comment(&mut self, id: &BackupId, comment: &str) {
        let value = || {
            if comment.is_empty() {
                None
            } else {
                Some(comment.to_string())
            }
        };

        self.modify_backup(id, |x| x.comment = value(), |x| x.comment = value());
    }

    pub fn set_backup_locked(&mut self, id: &BackupId, locked: bool) {
        self.modify_backup(id, |x| x.locked = locked, |x| x.locked = locked);
    }

    /// Returns whether the backup is valid.
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
                            eprintln!("can't find {stored}");
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
                                eprintln!("can't find {stored}");
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
}

impl BackupLayout {
    pub fn new(base: StrictPath) -> Self {
        let games = Self::load(&base);
        let games_lowercase = games.iter().map(|(k, v)| (k.to_lowercase(), v.clone())).collect();
        Self {
            base,
            games,
            games_lowercase,
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
            .filter_map(|x| crate::scan::filter_map_walkdir("ludusavi::BackupLayout", x))
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

        match GameLayout::load(path.clone()) {
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
            },
        }
    }

    pub fn try_game_layout(&self, name: &str) -> Option<GameLayout> {
        let path = self.game_folder(name);

        GameLayout::load(path).ok().map(|mut x| {
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
        scan_kind: ScanKind,
        redirects: &[RedirectConfig],
        reverse_redirects_on_restore: bool,
        toggled_paths: &ToggledPaths,
        only_constructive: bool,
    ) -> Option<LatestBackup> {
        if self.contains_game(name) {
            let game_layout = self.game_layout(name);
            let latest_timestamp = *game_layout.find_by_id_flattened(&BackupId::Latest)?.when();
            let scan = game_layout.latest_backup(
                scan_kind,
                redirects,
                reverse_redirects_on_restore,
                toggled_paths,
                only_constructive,
            );
            scan.map(|scan| LatestBackup {
                scan,
                when: latest_timestamp,
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
    use velcro::{btree_map, hash_map};

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
            BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo_raw())))
        }

        fn game_layout(name: &str, path: &str) -> GameLayout {
            GameLayout {
                path: StrictPath::new(path.to_string()),
                mapping: IndividualMapping::new(name.to_string()),
            }
        }

        fn drives() -> BTreeMap<String, String> {
            let (drive, _) = StrictPath::cwd().split_drive();
            let folder = IndividualMapping::new_drive_folder_name(&drive);
            btree_map! { folder: drive }
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
                    .find_irrelevant_backup_files(SOLO, &[repo_path("tests/backup/game1/drive-X/file1.txt")])
            );
            assert_eq!(
                Vec::<StrictPath>::new(),
                game_layout("game1", &repo_file("tests/backup/game1")).find_irrelevant_backup_files(
                    SOLO,
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
            };
            assert_eq!(
                None,
                layout.plan_backup(&scan, &now(), &BackupFormats::default(), Retention::default())
            );
        }

        #[test]
        fn can_plan_backup_kind_when_first_time() {
            let layout = GameLayout::default();
            assert_eq!(BackupKind::Full, layout.plan_backup_kind(Retention::default()));
        }

        #[test]
        fn can_plan_backup_kind_when_merged_single_full() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup::default()]),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind(Retention::new(1, 0)));
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
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind(Retention::new(1, 0)));
        }

        #[test]
        fn can_plan_backup_kind_when_multiple_full() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup::default()]),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind(Retention::new(2, 0)));
        }

        #[test]
        fn can_plan_backup_kind_when_single_full_with_differential() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup::default()]),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind(Retention::new(1, 1)));
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
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind(Retention::new(1, 1)));
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
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind(Retention::new(2, 2)));
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
                ..Default::default()
            };
            assert_eq!(BackupKind::Full, layout.plan_backup_kind(Retention::new(2, 2)));
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
                ..Default::default()
            };
            assert_eq!(BackupKind::Differential, layout.plan_backup_kind(Retention::new(1, 2)));
        }

        #[test]
        fn can_plan_full_backup_with_files() {
            let scan = ScanInfo {
                found_files: hash_map! {
                    repo_file("new").into(): ScannedFile::with_change(1, "n", ScanChange::New),
                    repo_file("different").into(): ScannedFile::with_change(2, "d", ScanChange::Different),
                    repo_file("removed").into(): ScannedFile::with_change(3, "r", ScanChange::Removed),
                    repo_file("same").into(): ScannedFile::with_change(5, "s", ScanChange::Same),
                    repo_file("unknown").into(): ScannedFile::with_change(6, "u", ScanChange::Unknown),
                },
                ..Default::default()
            };
            let layout = GameLayout::default();
            assert_eq!(
                FullBackup {
                    name: SOLO.to_string(),
                    when: now(),
                    os: Some(Os::HOST),
                    files: btree_map! {
                        StrictPath::new(repo_file("new")).render(): IndividualMappingFile { hash: "n".into(), size: 1 },
                        StrictPath::new(repo_file("different")).render(): IndividualMappingFile { hash: "d".into(), size: 2 },
                        StrictPath::new(repo_file("same")).render(): IndividualMappingFile { hash: "s".into(), size: 5 },
                    },
                    ..Default::default()
                },
                layout.plan_full_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
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
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi".into(): ScannedRegistry::new().change_as(ScanChange::New).ignored(),
                    "HKEY_CURRENT_USER/Software/Ludusavi/game3".into(): ScannedRegistry::new().change_as(ScanChange::Different)
                        .with_value("binary", ScanChange::New, false)
                        .with_value("dword", ScanChange::Different, false)
                        .with_value("expandSz", ScanChange::Removed, false)
                        .with_value("multiSz", ScanChange::Same, false)
                        .with_value("qword", ScanChange::Same, true)
                        .with_value("sz", ScanChange::Unknown, false),
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().change_as(ScanChange::Removed)
                },
                ..Default::default()
            };
            let layout = GameLayout::default();
            let hives = Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\game3"): Entries(btree_map! {
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
                    name: SOLO.to_string(),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: IndividualMappingRegistry {
                        hash: hives.sha1(registry::Format::Reg),
                    },
                    ..Default::default()
                },
                layout.plan_full_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
            );
        }

        #[test]
        fn can_plan_differential_backup_with_files() {
            let scan = ScanInfo {
                found_files: hash_map! {
                    repo_file("new").into(): ScannedFile::with_change(1, "n", ScanChange::New),
                    repo_file("different").into(): ScannedFile::with_change(2, "d+", ScanChange::Different),
                    repo_file("removed").into(): ScannedFile::with_change(0, "", ScanChange::Removed),
                    repo_file("same").into(): ScannedFile::with_change(5, "s", ScanChange::Same),
                    repo_file("unknown").into(): ScannedFile::with_change(6, "u", ScanChange::Unknown),
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: SOLO.to_string(),
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
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
            );
        }

        #[test]
        fn can_plan_second_differential_backup_with_different_ignored_files() {
            let scan = ScanInfo {
                found_files: hash_map! {
                    // Ignored in first differential backup:
                    repo_file("file1").into(): ScannedFile::with_change(1, "1", ScanChange::New).ignored(),
                    // Newly ignored:
                    repo_file("file2").into(): ScannedFile::with_change(2, "2", ScanChange::Same).ignored(),
                    // Just here to keep the backup from being inert (all ignores):
                    repo_file("file3").into(): ScannedFile::with_change(3, "3", ScanChange::Same),
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: SOLO.to_string(),
                        when: past(),
                        files: btree_map! {
                            StrictPath::new(repo_file("file1")).render(): IndividualMappingFile { hash: "1".into(), size: 1 },
                            StrictPath::new(repo_file("file2")).render(): IndividualMappingFile { hash: "2".into(), size: 2 },
                            StrictPath::new(repo_file("file3")).render(): IndividualMappingFile { hash: "3".into(), size: 3 },
                        },
                        children: VecDeque::from([DifferentialBackup {
                            name: format!("backup-{}-diff", now_str()),
                            when: now(),
                            os: Some(Os::HOST),
                            files: btree_map! {
                                StrictPath::new(repo_file("file1")).render(): None,
                            },
                            ..Default::default()
                        }]),
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
                        // This matches the latest composite,
                        // but we have to reiterate this in the new differential:
                        StrictPath::new(repo_file("file1")).render(): None,
                        // New ignore:
                        StrictPath::new(repo_file("file2")).render(): None,
                    },
                    registry: None,
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_new() {
            use crate::scan::registry::{Entries, Hives, Keys};

            let scan = ScanInfo {
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().change_as(ScanChange::New)
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: SOLO.to_string(),
                        when: past(),
                        registry: IndividualMappingRegistry { hash: None },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            };
            let hives = Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            });
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: Some(IndividualMappingRegistry {
                        hash: hives.sha1(registry::Format::Reg),
                    }),
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_changed() {
            use crate::scan::registry::{Entries, Hives, Keys};

            let scan = ScanInfo {
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().change_as(ScanChange::Different)
                        .with_value("removed", ScanChange::Removed, false)
                        // Fake registry values are ignored because `Hives` re-reads the actual registry.
                        .with_value("fake", ScanChange::New, false)
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: SOLO.to_string(),
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
            let hives = Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            });
            assert_eq!(
                DifferentialBackup {
                    name: format!("backup-{}-diff", now_str()),
                    when: now(),
                    os: Some(Os::HOST),
                    registry: Some(IndividualMappingRegistry {
                        hash: hives.sha1(registry::Format::Reg),
                    }),
                    ..Default::default()
                },
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_unchanged() {
            use crate::scan::registry::{Entries, Hives, Keys};

            let scan = ScanInfo {
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().change_as(ScanChange::Same)
                },
                ..Default::default()
            };
            let hives = Hives(btree_map! {
                s("HKEY_CURRENT_USER"): Keys(btree_map! {
                    s("Software\\Ludusavi\\other"): Entries::default()
                })
            });
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: SOLO.to_string(),
                        when: past(),
                        registry: IndividualMappingRegistry {
                            hash: hives.sha1(registry::Format::Reg),
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
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
            );
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn can_plan_differential_backup_with_registry_removed() {
            let scan = ScanInfo {
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().change_as(ScanChange::Removed)
                },
                ..Default::default()
            };
            let layout = GameLayout {
                mapping: IndividualMapping {
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: SOLO.to_string(),
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
                layout.plan_differential_backup(&scan, &now(), &BackupFormats::default(), Retention::default()),
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
                ..Default::default()
            };

            layout.forget_excess_backups(Retention::new(1, 1));
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
                            name: SOLO.to_string(),
                            comment: Some("old".to_string()),
                            ..Default::default()
                        },
                        FullBackup {
                            name: SOLO.to_string(),
                            comment: Some("new".to_string()),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                ..Default::default()
            };

            layout.forget_excess_backups(Retention::new(1, 0));
            assert_eq!(
                VecDeque::from_iter(vec![FullBackup {
                    name: SOLO.to_string(),
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
                ..Default::default()
            };

            layout.forget_excess_backups(Retention::new(1, 1));
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
            repo_path(&format!("tests/backup/game1/{file}"))
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
            StrictPath::new(format!(
                "drive-{}/{file}",
                if cfg!(target_os = "windows") { "X" } else { "0" }
            ))
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
            };
            assert_eq!(
                hash_map! {
                    make_restorable_path("backup-1", "file1.txt"): ScannedFile {
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    make_restorable_path("backup-1", "file2.txt"): ScannedFile {
                        size: 2,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, ScanKind::Backup, &[], false, &Default::default()),
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
            };
            assert_eq!(
                hash_map! {
                    make_restorable_path_zip("file1.txt"): ScannedFile {
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-1.zip")),
                        redirected: None,
                    },
                    make_restorable_path_zip("file2.txt"): ScannedFile {
                        size: 2,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-1.zip")),
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, ScanKind::Backup, &[], false, &Default::default()),
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
            };
            assert_eq!(
                hash_map! {
                    make_restorable_path("backup-1", "unchanged.txt"): ScannedFile {
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/unchanged.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    make_restorable_path("backup-2", "changed.txt"): ScannedFile {
                        size: 2,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/changed.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    make_restorable_path("backup-2", "added.txt"): ScannedFile {
                        size: 5,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/added.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, ScanKind::Backup, &[], false, &Default::default()),
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
            };
            assert_eq!(
                hash_map! {
                    make_restorable_path_zip("unchanged.txt"): ScannedFile {
                        size: 1,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/unchanged.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-1.zip")),
                        redirected: None,
                    },
                    make_restorable_path_zip("changed.txt"): ScannedFile {
                        size: 2,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/changed.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-2.zip")),
                        redirected: None,
                    },
                    make_restorable_path_zip("added.txt"): ScannedFile {
                        size: 5,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/added.txt")),
                        ignored: false,
                        change: Default::default(),
                        container: Some(make_path("backup-2.zip")),
                        redirected: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest, ScanKind::Backup, &[], false, &Default::default()),
            );
        }
    }

    mod game_layout {
        use pretty_assertions::assert_eq;

        use crate::testing::{drives_x_always, drives_x_static, repo_file_raw};

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
                        name: SOLO.into(),
                        when: now(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                },
            );
            let backups = vec![Backup::Full(FullBackup {
                name: SOLO.to_string(),
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
                    found_files: hash_map! {
                        restorable_file_simple(SOLO, "file1.txt"): ScannedFile {
                            size: 1,
                            hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(),
                            original_path: Some(make_original_path("/file1.txt")),
                            ignored: false,
                            change: ScanChange::New,
                            container: None,
                            redirected: None,
                        },
                        restorable_file_simple(SOLO, "file2.txt"): ScannedFile {
                            size: 2,
                            hash: "9d891e731f75deae56884d79e9816736b7488080".into(),
                            original_path: Some(make_original_path("/file2.txt")),
                            ignored: false,
                            change: ScanChange::New,
                            container: None,
                            redirected: None,
                        },
                    },
                    found_registry_keys: Default::default(),
                    available_backups: backups.clone(),
                    backup: Some(backups[0].clone()),
                    has_backups: true,
                    dumped_registry: None,
                    only_constructive_backups: false,
                },
                layout.scan_for_restoration(
                    "game1",
                    &BackupId::Latest,
                    &[],
                    false,
                    &Default::default(),
                    &Default::default()
                ),
            );
        }

        #[test]
        fn can_scan_game_for_restoration_with_registry() {
            let mut layout =
                BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo()))).game_layout("game3");
            if cfg!(target_os = "windows") {
                assert_eq!(
                    ScanInfo {
                        game_name: s("game3"),
                        found_files: Default::default(),
                        found_registry_keys: hash_map! {
                            "HKEY_CURRENT_USER/Software/Ludusavi/game3".into(): ScannedRegistry::new().change_as(ScanChange::Same)
                                .with_value_same("binary")
                                .with_value_same("dword")
                                .with_value_same("expandSz")
                                .with_value_same("multiSz")
                                .with_value_same("qword")
                                .with_value_same("sz")
                        },
                        available_backups: vec![Backup::Full(FullBackup {
                            name: SOLO.to_string(),
                            when: now(),
                            registry: IndividualMappingRegistry {
                                hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into()),
                            },
                            ..Default::default()
                        })],
                        backup: Some(Backup::Full(FullBackup {
                            name: SOLO.to_string(),
                            when: now(),
                            registry: IndividualMappingRegistry {
                                hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into()),
                            },
                            ..Default::default()
                        })),
                        has_backups: true,
                        dumped_registry: Some(registry::Hives(btree_map! {
                            r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                                r"Software\Ludusavi\game3".into(): registry::Entries(btree_map! {
                                    "binary".into(): registry::Entry::Binary(vec![65]),
                                    "dword".into(): registry::Entry::Dword(1),
                                    "expandSz".into(): registry::Entry::ExpandSz("baz".to_string()),
                                    "multiSz".into(): registry::Entry::MultiSz("bar".to_string()),
                                    "qword".into(): registry::Entry::Qword(2),
                                    "sz".into(): registry::Entry::Sz("foo".to_string()),
                                }),
                            })
                        })),
                        only_constructive_backups: false,
                    },
                    layout.scan_for_restoration(
                        "game3",
                        &BackupId::Latest,
                        &[],
                        false,
                        &Default::default(),
                        &Default::default()
                    ),
                );
            } else {
                assert_eq!(
                    ScanInfo {
                        game_name: s("game3"),
                        found_files: Default::default(),
                        found_registry_keys: Default::default(),
                        available_backups: vec![Backup::Full(FullBackup {
                            name: SOLO.to_string(),
                            when: now(),
                            registry: IndividualMappingRegistry {
                                hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into()),
                            },
                            ..Default::default()
                        })],
                        backup: Some(Backup::Full(FullBackup {
                            name: SOLO.to_string(),
                            when: now(),
                            registry: IndividualMappingRegistry {
                                hash: Some("4e2cab4b4e3ab853e5767fae35f317c26c655c52".into())
                            },
                            ..Default::default()
                        })),
                        has_backups: true,
                        dumped_registry: None,
                        only_constructive_backups: false,
                    },
                    layout.scan_for_restoration(
                        "game3",
                        &BackupId::Latest,
                        &[],
                        false,
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
                        name: SOLO.into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
            };
            assert!(layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_simple_full_backup_when_invalid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: SOLO.into(),
                        files: btree_map! {
                            mapping_file_key("/fake.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                path: StrictPath::new(format!("{}/tests/backup/game1", repo_raw())),
            };
            assert!(!layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_simple_diff_backup_when_valid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: SOLO.into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        children: VecDeque::from(vec![DifferentialBackup {
                            name: SOLO.into(),
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
            };
            assert!(layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_validate_a_simple_diff_backup_when_invalid() {
            let layout = GameLayout {
                mapping: IndividualMapping {
                    drives: drives_x_always(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: SOLO.into(),
                        files: btree_map! {
                            mapping_file_key("/file1.txt"): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                            mapping_file_key("/file2.txt"): IndividualMappingFile { hash: "9d891e731f75deae56884d79e9816736b7488080".into(), size: 2 },
                        },
                        children: VecDeque::from(vec![DifferentialBackup {
                            name: SOLO.into(),
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
            };
            assert!(!layout.validate(BackupId::Latest));
        }

        #[test]
        fn can_migrate_legacy_backup() {
            let layout = BackupLayout::new(StrictPath::new(format!("{}/tests/backup", repo_raw())));

            let before = IndividualMapping {
                name: "migrate-legacy-backup".to_string(),
                drives: drives_x_static(),
                ..Default::default()
            };
            let after = IndividualMapping {
                name: "migrate-legacy-backup".to_string(),
                drives: drives_x_static(),
                backups: VecDeque::from(vec![FullBackup {
                    name: SOLO.into(),
                    files: btree_map! {
                        "X:/file1.txt".into(): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                    },
                    ..Default::default()
                }]),
            };

            let mut game_layout = layout.game_layout("migrate-legacy-backup");
            assert_eq!(before, game_layout.mapping);

            game_layout.migrate_legacy_backup(false);
            assert_eq!(after, game_layout.mapping);

            // Idempotent:
            game_layout.migrate_legacy_backup(false);
            assert_eq!(after, game_layout.mapping);

            // No-op with default data:
            let mut game_layout = GameLayout::default();
            game_layout.migrate_legacy_backup(false);
            assert_eq!(GameLayout::default().mapping, game_layout.mapping);
        }

        #[test]
        fn can_migrate_initial_empty_backup_without_children() {
            let before = IndividualMapping {
                name: "migrate-initial-empty-backup".to_string(),
                drives: drives_x_static(),
                backups: VecDeque::from(vec![
                    FullBackup {
                        name: SOLO.into(),
                        ..Default::default()
                    },
                    FullBackup {
                        name: "backup-20240626T100614Z-diff".into(),
                        when: chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            "2024-06-26T10:06:14.120957700Z",
                        )
                        .unwrap()
                        .to_utc(),
                        os: Some(Os::Windows),
                        files: btree_map! {
                            "X:/file1.txt".into(): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                        },
                        ..Default::default()
                    },
                ]),
            };
            let after = IndividualMapping {
                name: "migrate-initial-empty-backup".to_string(),
                drives: drives_x_static(),
                backups: VecDeque::from(vec![FullBackup {
                    name: "backup-20240626T100614Z-diff".into(),
                    when: chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339("2024-06-26T10:06:14.120957700Z")
                        .unwrap()
                        .to_utc(),
                    os: Some(Os::Windows),
                    files: btree_map! {
                        "X:/file1.txt".into(): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                    },
                    ..Default::default()
                }]),
            };

            let mut game_layout = GameLayout {
                path: format!("{}/tests/backup/migrate-initial-empty-backup/mapping.yaml", repo_raw()).into(),
                mapping: before.clone(),
            };
            assert_eq!(before, game_layout.mapping);

            game_layout.migrate_initial_empty_backup(false);
            assert_eq!(after, game_layout.mapping);

            // Idempotent:
            game_layout.migrate_initial_empty_backup(false);
            assert_eq!(after, game_layout.mapping);

            // No-op with default data:
            let mut game_layout = GameLayout::default();
            game_layout.migrate_initial_empty_backup(false);
            assert_eq!(GameLayout::default().mapping, game_layout.mapping);
        }

        #[test]
        fn can_migrate_initial_empty_backup_with_children() {
            let before = IndividualMapping {
                name: "migrate-initial-empty-backup".to_string(),
                drives: drives_x_static(),
                backups: VecDeque::from(vec![FullBackup {
                    name: SOLO.into(),
                    children: VecDeque::from(vec![DifferentialBackup {
                        name: "backup-20240626T100614Z-diff".to_string(),
                        when: chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            "2024-06-26T10:06:14.120957700Z",
                        )
                        .unwrap()
                        .to_utc(),
                        os: Some(Os::Windows),
                        files: btree_map! {
                            "X:/file1.txt".into(): Some(IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 }),
                        },
                        ..Default::default()
                    }]),
                    ..Default::default()
                }]),
            };
            let after = IndividualMapping {
                name: "migrate-initial-empty-backup".to_string(),
                drives: drives_x_static(),
                backups: VecDeque::from(vec![FullBackup {
                    name: "backup-20240626T100614Z-diff".into(),
                    when: chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339("2024-06-26T10:06:14.120957700Z")
                        .unwrap()
                        .to_utc(),
                    os: Some(Os::Windows),
                    files: btree_map! {
                        "X:/file1.txt".into(): IndividualMappingFile { hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".into(), size: 1 },
                    },
                    ..Default::default()
                }]),
            };

            let mut game_layout = GameLayout {
                path: format!("{}/tests/backup/migrate-initial-empty-backup/mapping.yaml", repo_raw()).into(),
                mapping: before.clone(),
            };
            assert_eq!(before, game_layout.mapping);

            game_layout.migrate_initial_empty_backup(false);
            assert_eq!(after, game_layout.mapping);

            // Idempotent:
            game_layout.migrate_initial_empty_backup(false);
            assert_eq!(after, game_layout.mapping);

            // No-op with default data:
            let mut game_layout = GameLayout::default();
            game_layout.migrate_initial_empty_backup(false);
            assert_eq!(GameLayout::default().mapping, game_layout.mapping);
        }
    }
}
