use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    io::Write,
};

use filetime::FileTime;

use chrono::{Datelike, Timelike};

use crate::{
    config::{BackupFormat, BackupFormats, RedirectConfig, Retention, ZipCompression},
    path::StrictPath,
    prelude::{game_file_restoration_target, BackupId, BackupInfo, ScanInfo, ScannedFile, ScannedRegistry},
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
    base64::encode(&name).replace('/', SAFE)
}

fn escape_folder_name(name: &str) -> String {
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

    escaped
        .replace('\\', SAFE)
        .replace('/', SAFE)
        .replace(':', SAFE)
        .replace('*', SAFE)
        .replace('?', SAFE)
        .replace('"', SAFE)
        .replace('<', SAFE)
        .replace('>', SAFE)
        .replace('|', SAFE)
        .replace('\0', SAFE)
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
}

impl std::fmt::Display for Backup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FullBackup {
    pub name: String,
    pub when: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub files: BTreeMap<String, IndividualMappingFile>,
    #[serde(default)]
    pub registry: IndividualMappingRegistry,
    pub children: Vec<DifferentialBackup>,
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
#[serde(default)] // #132: if mtime is missing in mapping file, use Default
pub struct IndividualMappingFile {
    pub hash: String,
    pub size: u64,
    pub mtime: chrono::DateTime<chrono::Utc>,
}


#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndividualMappingRegistry {
    pub hash: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndividualMapping {
    pub name: String,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    pub drives: std::collections::HashMap<String, String>,
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

    fn reversed_drives(&self) -> std::collections::HashMap<String, String> {
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
            Some(base.interpret()),
        )
    }

    pub fn game_file_immutable(&self, base: &StrictPath, original_file: &StrictPath, backup: &str) -> StrictPath {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name_immutable(&drive);
        StrictPath::relative(
            format!("{}/{}/{}", backup, drive_folder, plain_path),
            Some(base.interpret()),
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
        full.map(|x| (x, x.children.last()))
    }

    pub fn save(&self, file: &StrictPath) {
        let new_content = serde_yaml::to_string(&self).unwrap();

        if let Ok(old) = Self::load(file) {
            let old_content = serde_yaml::to_string(&old).unwrap();
            if old_content == new_content {
                return;
            }
        }

        if file.create_parent_dir().is_ok() {
            std::fs::write(file.interpret(), self.serialize().as_bytes()).unwrap();
        }
    }

    pub fn serialize(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }

    pub fn load(file: &StrictPath) -> Result<Self, ()> {
        if !file.is_file() {
            return Err(());
        }
        let content = std::fs::read_to_string(&file.interpret()).unwrap();
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

    pub fn load_from_string(content: &str) -> Result<Self, ()> {
        match serde_yaml::from_str(content) {
            Ok(x) => Ok(x),
            Err(_) => Err(()),
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

        for child in walkdir::WalkDir::new(base.interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(crate::prelude::filter_map_walkdir)
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

    pub fn load(path: StrictPath, retention: Retention) -> Result<Self, ()> {
        let mapping = Self::mapping_file(&path);
        Ok(Self {
            path,
            mapping: IndividualMapping::load(&mapping)?,
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

    pub fn restorable_files(&self, id: &BackupId) -> std::collections::HashSet<ScannedFile> {
        let mut files = std::collections::HashSet::new();

        match self.find_by_id(id) {
            None => {}
            Some((full, None)) => {
                files.extend(self.restorable_files_from_full_backup(full));
            }
            Some((full, Some(diff))) => {
                files.extend(self.restorable_files_from_diff_backup(diff));

                for full_file in self.restorable_files_from_full_backup(full) {
                    let original_path = full_file.original_path.as_ref().unwrap().render();
                    if diff.file(original_path) == BackupInclusion::Inherited {
                        files.insert(full_file);
                    }
                }
            }
        }

        files
    }

    fn restorable_files_from_full_backup(&self, backup: &FullBackup) -> std::collections::HashSet<ScannedFile> {
        let mut restorables = std::collections::HashSet::new();

        for (k, v) in &backup.files {
            let original_path = StrictPath::new(k.to_string());
            match backup.format() {
                BackupFormat::Simple => {
                    restorables.insert(ScannedFile {
                        path: self
                            .mapping
                            .game_file_immutable(&self.path, &original_path, &backup.name),
                        size: v.size,
                        mtime: v.mtime,
                        hash: v.hash.clone(),
                        original_path: Some(original_path),
                        ignored: false,
                        container: None,
                    });
                }
                BackupFormat::Zip => {
                    restorables.insert(ScannedFile {
                        path: StrictPath::new(self.mapping.game_file_for_zip_immutable(&original_path)),
                        size: v.size,
                        mtime: v.mtime,
                        hash: v.hash.clone(),
                        original_path: Some(original_path),
                        ignored: false,
                        container: Some(self.path.joined(&backup.name)),
                    });
                }
            }
        }

        restorables
    }

    fn restorable_files_from_diff_backup(&self, backup: &DifferentialBackup) -> std::collections::HashSet<ScannedFile> {
        let mut restorables = std::collections::HashSet::new();

        for (k, v) in &backup.files {
            let v = some_or_continue!(v);
            let original_path = StrictPath::new(k.to_string());
            match backup.format() {
                BackupFormat::Simple => {
                    restorables.insert(ScannedFile {
                        path: self
                            .mapping
                            .game_file_immutable(&self.path, &original_path, &backup.name),
                        size: v.size,
                        mtime: v.mtime,
                        hash: v.hash.clone(),
                        original_path: Some(original_path),
                        ignored: false,
                        container: None,
                    });
                }
                BackupFormat::Zip => {
                    restorables.insert(ScannedFile {
                        path: StrictPath::new(self.mapping.game_file_for_zip_immutable(&original_path)),
                        size: v.size,
                        mtime: v.mtime,
                        hash: v.hash.clone(),
                        original_path: Some(original_path),
                        ignored: false,
                        container: Some(self.path.joined(&backup.name)),
                    });
                }
            }
        }

        restorables
    }

    fn restorable_files_in_simple(&self, backup: &str) -> std::collections::HashSet<ScannedFile> {
        let mut files = std::collections::HashSet::new();
        for drive_dir in walkdir::WalkDir::new(self.path.joined(backup).interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(crate::prelude::filter_map_walkdir)
        {
            let raw_drive_dir = drive_dir.path().display().to_string();
            let drive_mapping =
                some_or_continue!(self.mapping.drives.get::<str>(&drive_dir.file_name().to_string_lossy()));

            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(crate::prelude::filter_map_walkdir)
                .filter(|x| x.file_type().is_file())
            {
                let raw_file = file.path().display().to_string();
                let original_path = Some(StrictPath::new(raw_file.replace(&raw_drive_dir, drive_mapping)));
                let path = StrictPath::new(raw_file);
                files.insert(ScannedFile {
                    size: path.size(),
                    mtime: file.metadata().unwrap().modified().unwrap().into(),
                    hash: path.sha1(),
                    path,
                    original_path,
                    ignored: false,
                    container: None,
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
                let handle = std::fs::File::open(&self.path.joined(backup).interpret()).ok()?;
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

    fn count_backups(&self) -> (u8, u8) {
        let full = self.mapping.backups.len();
        let differential = self.mapping.backups.back().map(|x| x.children.len()).unwrap_or(0);
        (full as u8, differential as u8)
    }

    fn need_backup(&self, backup: &Backup) -> bool {
        let (prior_full, prior_diff) = match self.mapping.latest_backup() {
            None => return true,
            Some((full, diff)) => (full, diff),
        };

        if let Backup::Differential(current_diff) = backup {
            if let Some(prior_diff) = prior_diff {
                if prior_diff.files == current_diff.files && prior_diff.registry == current_diff.registry {
                    return false;
                }
            }
        }

        let mut prior_files = prior_full.files.clone();
        let mut prior_registry = prior_full.registry.clone();
        if let Some(diff) = prior_diff {
            for (k, v) in &diff.files {
                match v {
                    None => {
                        prior_files.remove(k);
                    }
                    Some(v) => {
                        prior_files.insert(k.clone(), v.clone());
                    }
                }
            }
            if let Some(registry) = &diff.registry {
                prior_registry = registry.clone();
            }
        }

        let (current_files, current_registry) = match backup {
            Backup::Full(current_full) => (current_full.files.clone(), current_full.registry.clone()),
            Backup::Differential(current_diff) => {
                let mut current_files = prior_full.files.clone();
                let mut current_registry = prior_full.registry.clone();

                for (k, v) in &current_diff.files {
                    match v {
                        None => {
                            current_files.remove(k);
                        }
                        Some(v) => {
                            current_files.insert(k.clone(), v.clone());
                        }
                    }
                }
                if let Some(registry) = &current_diff.registry {
                    current_registry = registry.clone();
                }

                (current_files, current_registry)
            }
        };

        prior_files != current_files || prior_registry != current_registry
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
        if *kind == BackupKind::Full && self.retention.full == 1 && format.chosen == BackupFormat::Simple {
            ".".to_string()
        } else {
            let name = format!("backup-{}", Self::generate_file_friendly_timestamp(now));
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
        if !scan.found_anything() {
            return None;
        }

        let (fulls, diffs) = self.count_backups();
        let kind = if fulls > 0 && diffs < self.retention.differential {
            BackupKind::Differential
        } else {
            BackupKind::Full
        };

        let backup = match kind {
            BackupKind::Full => Backup::Full(self.plan_full_backup(scan, now, format)),
            BackupKind::Differential => Backup::Differential(self.plan_differential_backup(scan, now, format)),
        };

        self.need_backup(&backup).then_some(backup)
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
            files.insert(
                file.path.render(),
                IndividualMappingFile {
                    hash: file.hash.clone(),
                    size: file.size,
                    mtime: file.mtime,
                },
            );
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;
            let hives = Hives::from(&scan.found_registry_keys);
            if !hives.is_empty() {
                registry.hash = Some(crate::prelude::sha1(hives.serialize()));
            }
        }

        FullBackup {
            name: self.generate_backup_name(&BackupKind::Full, now, format),
            when: *now,
            files,
            registry,
            children: vec![],
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

        for file in scan.found_files.iter().filter(|x| !x.ignored) {
            files.insert(
                file.path.render(),
                Some(IndividualMappingFile {
                    hash: file.hash.clone(),
                    size: file.size,
                    mtime: file.mtime,
                }),
            );
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;
            let hives = Hives::from(&scan.found_registry_keys);
            if !hives.is_empty() {
                registry = Some(IndividualMappingRegistry {
                    hash: Some(crate::prelude::sha1(hives.serialize())),
                });
            }
        }

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
            files,
            registry,
        }
    }

    fn execute_backup_as_simple(&mut self, backup: &Backup, scan: &ScanInfo) -> BackupInfo {
        let mut backup_info = BackupInfo::default();

        let mut relevant_files = vec![];
        for file in &scan.found_files {
            if !backup.includes_file(file.path.render()) {
                log::debug!("[{}] skipped: {}", self.mapping.name, file.path.raw());
                continue;
            }

            let target_file = self.mapping.game_file(&self.path, &file.path, backup.name());
            if file.path.same_content(&target_file) {
                log::info!("[{}] already matches: {}", self.mapping.name, file.path.raw());
                relevant_files.push(target_file);
                continue;
            }
            // TODO #132: SLX honor timestamps - once file is written (or collect them and do it once?)
            if let Err(e) = target_file.create_parent_dir() {
                log::error!(
                    "[{}] unable to create parent directories: {} -> {} | {e}",
                    self.mapping.name,
                    file.path.raw(),
                    target_file.raw()
                );
                backup_info.failed_files.insert(file.clone());
                continue;
            }
            if let Err(e) = std::fs::copy(&file.path.interpret(), &target_file.interpret()) {
                log::error!(
                    "[{}] unable to copy: {} -> {} | {e}",
                    self.mapping.name,
                    file.path.raw(),
                    target_file.raw()
                );
                backup_info.failed_files.insert(file.clone());
                continue;
            }
            // DONE #132: SLX honor timestamps - set timestamp of file based on ScanInfo
            if let Err(e) = filetime::set_file_mtime(target_file.interpret(), FileTime::from_system_time(file.mtime.into())) {
                log::error!(
                    "[{}] unable to set modification time: {} -> {} to {:#?} | {e}",
                    self.mapping.name,
                    file.path.raw(),
                    target_file.raw(),
                    file.mtime
                );
            }
            log::info!(
                "[{}] backed up: {} -> {}",
                self.mapping.name,
                file.path.raw(),
                target_file.raw()
            );
            relevant_files.push(target_file);
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;
            let target_registry_file = self.registry_file_in(backup.name());

            if backup.includes_registry() {
                let hives = Hives::from(&scan.found_registry_keys);
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

        let fail_file =
            |file: &ScannedFile, backup_info: &mut BackupInfo| backup_info.failed_files.insert(file.clone());
        let fail_all = |backup_info: &mut BackupInfo| {
            for file in &scan.found_files {
                backup_info.failed_files.insert(file.clone());
            }
        };

        let archive_path = self.path.joined(backup.name());
        let archive_file = match std::fs::File::create(archive_path.interpret()) {
            Ok(x) => x,
            Err(e) => {
                log::error!(
                    "[{}] unable to create zip file: {} | {e}",
                    self.mapping.name,
                    archive_path.raw()
                );
                fail_all(&mut backup_info);
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
            .large_file(true);

        'item: for file in &scan.found_files {
            if !backup.includes_file(file.path.render()) {
                log::debug!("[{}] skipped: {}", self.mapping.name, file.path.raw());
                continue;
            }

            let target_file_id = self.mapping.game_file_for_zip(&file.path);
            // DONE #132: SLX honor timestamps - in options last_modified_time
            let mtime: chrono::DateTime<chrono::Utc> = file.mtime.into();
            let local_options = options.last_modified_time(zip::DateTime::from_date_and_time(
                mtime.year() as u16,
                mtime.month() as u8,
                mtime.day() as u8,
                mtime.hour() as u8,
                mtime.minute() as u8,
                mtime.second() as u8,
            ).unwrap());
            if let Err(e) = zip.start_file(&target_file_id, local_options) {
                log::error!(
                    "[{}] unable to start zip file record: {} -> {} | {e}",
                    self.mapping.name,
                    file.path.raw(),
                    &target_file_id
                );
                fail_file(file, &mut backup_info);
                continue;
            }

            use std::io::Read;
            let handle = match std::fs::File::open(file.path.interpret()) {
                Ok(x) => x,
                Err(e) => {
                    log::error!(
                        "[{}] unable to open source: {} | {e}",
                        self.mapping.name,
                        file.path.raw()
                    );
                    fail_file(file, &mut backup_info);
                    continue;
                }
            };
            let mut reader = std::io::BufReader::new(handle);
            let mut buffer = [0; 1024];

            loop {
                let read = match reader.read(&mut buffer[..]) {
                    Ok(x) => x,
                    Err(e) => {
                        log::error!(
                            "[{}] unable to read source: {} | {e}",
                            self.mapping.name,
                            file.path.raw()
                        );
                        fail_file(file, &mut backup_info);
                        continue 'item;
                    }
                };
                if read == 0 {
                    log::info!(
                        "[{}] backed up: {} -> {}",
                        self.mapping.name,
                        file.path.raw(),
                        &target_file_id
                    );
                    break;
                }
                if let Err(e) = zip.write_all(&buffer[0..read]) {
                    log::error!(
                        "[{}] unable to write target: {} -> {} | {e}",
                        self.mapping.name,
                        file.path.raw(),
                        &target_file_id
                    );
                    fail_file(file, &mut backup_info);
                    continue 'item;
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;

            if backup.includes_registry() {
                let hives = Hives::from(&scan.found_registry_keys);
                if zip.start_file("registry.yaml", options).is_ok() {
                    let _ = zip.write_all(hives.serialize().as_bytes());
                }
            }
        }

        if zip.finish().is_err() {
            fail_all(&mut backup_info);
        }

        backup_info
    }

    fn insert_backup(&mut self, backup: Backup) {
        match backup {
            Backup::Full(backup) => {
                self.mapping.backups.push_back(backup);
                while self.mapping.backups.len() as u8 > self.retention.full {
                    self.mapping.backups.pop_front();
                }
            }
            Backup::Differential(backup) => {
                if let Some(parent) = self.mapping.backups.back_mut() {
                    parent.children.push(backup);
                }
            }
        }
    }

    fn execute_backup(&mut self, backup: &Backup, scan: &ScanInfo, format: &BackupFormats) -> BackupInfo {
        let backup_info = if backup.only_inherits_and_overrides() {
            BackupInfo::default()
        } else {
            match format.chosen {
                BackupFormat::Simple => self.execute_backup_as_simple(backup, scan),
                BackupFormat::Zip => self.execute_backup_as_zip(backup, scan, format),
            }
        };

        for irrelevant_parent in self.mapping.irrelevant_parents(&self.path) {
            let _ = irrelevant_parent.remove();
        }

        self.save();
        backup_info
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

        for file in self.restorable_files_in_simple(&backup.name) {
            files.insert(
                file.original_path.unwrap().render(),
                IndividualMappingFile {
                    hash: file.path.sha1(),
                    size: file.path.size(),
                    mtime: file.mtime,
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
            let mut backup = self.mapping.backups.back_mut().unwrap();
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
        self.migrate_legacy_backup();
        match self.plan_backup(scan, now, format) {
            None => {
                log::info!("[{}] no need for new backup", &scan.game_name);
                BackupInfo::default()
            }
            Some(backup) => {
                log::info!(
                    "[{}] creating a {:?} backup: {}",
                    &scan.game_name,
                    backup.kind(),
                    backup.name()
                );
                self.insert_backup(backup.clone());
                self.execute_backup(&backup, scan, format)
            }
        }
    }

    pub fn restore(&self, scan: &ScanInfo, redirects: &[RedirectConfig]) -> BackupInfo {
        log::trace!("[{}] beginning restore", &scan.game_name);

        let mut failed_files = std::collections::HashSet::new();
        let failed_registry = std::collections::HashSet::new();

        for file in &scan.found_files {
            let original_path = some_or_continue!(&file.original_path);
            let (target, _) = game_file_restoration_target(original_path, redirects);

            if let Err(e) = match &file.container {
                None => self.restore_file_from_simple(&target, file),
                Some(container) => self.restore_file_from_zip(&target, file, container),
            } {
                log::error!(
                    "[{}] failed to restore: {} -> {} | {e}",
                    self.mapping.name,
                    original_path.raw(),
                    target.raw()
                );
                failed_files.insert(file.clone());
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;

            let mut hives = Hives::default();
            let (found, _) = hives.incorporate(&scan.found_registry_keys);

            if found {
                // TODO: Track failed keys.
                let _ = hives.restore();
            }
        }

        log::trace!("[{}] completed restore", &scan.game_name);

        BackupInfo {
            failed_files,
            failed_registry,
        }
    }

    fn restore_file_from_simple(
        &self,
        target: &StrictPath,
        file: &ScannedFile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::trace!(
            "[{}] about to restore (simple): {} -> {}",
            self.mapping.name,
            file.path.raw(),
            target.raw()
        );

        if target.exists() && target.try_same_content(&file.path)? {
            log::info!(
                "[{}] already matches: {} -> {}",
                self.mapping.name,
                file.path.raw(),
                target.raw()
            );
            return Ok(());
        }

        // TODO #132: SLX honor timestamps - once file is written (or collect them and do it once?)
        target.create_parent_dir()?;
        for i in 0..99 {
            if let Err(e) = target.unset_readonly() {
                log::warn!(
                    "[{}] try {i}, failed to unset read-only on target: {} | {e}",
                    self.mapping.name,
                    target.raw()
                );
            } else if let Err(e) = std::fs::copy(&file.path.interpret(), &target.interpret()) {
                log::warn!(
                    "[{}] try {i}, failed to copy to target: {} -> {} | {e}",
                    self.mapping.name,
                    file.path.raw(),
                    target.raw()
                );
                // DONE #132: SLX honor timestamps - set timestamp of file based on ScanInfo
                if let Err(e) = filetime::set_file_mtime(target.interpret(), FileTime::from_system_time(file.mtime.into())) {
                    log::error!(
                        "[{}] unable to set modification time: {} -> {} to {:#?} | {e}",
                        self.mapping.name,
                        file.path.raw(),
                        target.raw(),
                        file.mtime
                    );
                }
            } else {
                log::info!(
                    "[{}] restored: {} -> {}",
                    &self.mapping.name,
                    file.path.raw(),
                    target.raw()
                );
                return Ok(());
            }
            // File might be busy, especially if multiple games share a file,
            // like in a collection, so retry after a delay:
            std::thread::sleep(std::time::Duration::from_millis(i * self.mapping.name.len() as u64));
        }

        Err("Unable to restore file".into())
    }

    fn restore_file_from_zip(
        &self,
        target: &StrictPath,
        file: &ScannedFile,
        container: &StrictPath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "[{}] about to restore (zip): {} -> {}",
            self.mapping.name,
            file.path.raw(),
            target.raw()
        );

        let handle = std::fs::File::open(&container.interpret())?;
        let mut archive = zip::ZipArchive::new(handle)?;

        if target.exists() && target.try_same_content_as_zip(&mut archive.by_name(&file.path.raw())?)? {
            log::info!(
                "[{}] already matches: {} -> {}",
                self.mapping.name,
                file.path.raw(),
                target.raw()
            );
            return Ok(());
        }

        // TODO #132: SLX honor timestamps - after child file is written
        target.create_parent_dir()?;
        for i in 0..99 {
            if i > 0 {
                // File might be busy, especially if multiple games share a file,
                // like in a collection, so retry after a delay:
                std::thread::sleep(std::time::Duration::from_millis(i * self.mapping.name.len() as u64));
            }
            if let Err(e) = target.unset_readonly() {
                log::warn!(
                    "[{}] try {i}, failed to unset read-only on target: {} | {e}",
                    self.mapping.name,
                    target.raw()
                );
                continue;
            }
            let mut target_handle = match std::fs::File::create(&target.interpret()) {
                Ok(x) => x,
                Err(e) => {
                    log::warn!(
                        "[{}] try {i}, failed to get handle: {} -> {} | {e}",
                        self.mapping.name,
                        file.path.raw(),
                        target.raw()
                    );
                    continue;
                }
            };
            if let Err(e) = std::io::copy(&mut archive.by_name(&file.path.raw())?, &mut target_handle) {
                log::warn!(
                    "[{}] try {i}, failed to copy to target: {} -> {} | {e}",
                    self.mapping.name,
                    file.path.raw(),
                    target.raw()
                );
            } else {
                // DONE #132: SLX honor timestamps - set timestamp of file based on ScanInfo
                if let Err(e) = filetime::set_file_mtime(target.interpret(), FileTime::from_system_time(file.mtime.into())) {
                    log::error!(
                        "[{}] unable to set modification time: {} -> {} to {:#?} | {e}",
                        self.mapping.name,
                        file.path.raw(),
                        target.raw(),
                        file.mtime
                    );
                }
                log::info!(
                    "[{}] restored: {} -> {}",
                    &self.mapping.name,
                    file.path.raw(),
                    target.raw()
                );
                return Ok(());
            }
        }

        Err("Unable to restore file".into())
    }

    fn mapping_file(path: &StrictPath) -> StrictPath {
        path.joined("mapping.yaml")
    }

    fn find_irrelevant_backup_files(&self, backup: &str, relevant_files: &[StrictPath]) -> Vec<StrictPath> {
        #[allow(clippy::needless_collect)]
        let relevant_files: Vec<_> = relevant_files.iter().map(|x| x.interpret()).collect();
        let mut irrelevant_files = vec![];

        for drive_dir in walkdir::WalkDir::new(self.path.joined(backup).interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(crate::prelude::filter_map_walkdir)
            .filter(|x| x.file_name().to_string_lossy().starts_with("drive-"))
        {
            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(crate::prelude::filter_map_walkdir)
                .filter(|x| x.file_type().is_file())
            {
                let backup_file = StrictPath::new(file.path().display().to_string());
                if !relevant_files.contains(&backup_file.interpret()) {
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
            log::debug!(
                "[{}] removing irrelevant backup file: {}",
                self.mapping.name,
                file.raw()
            );
            let _ = file.remove();
        }
        log::trace!("[{}] done removing irrelevant backup files", self.mapping.name);
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
    games: std::collections::HashMap<String, StrictPath>,
    retention: Retention,
}

impl BackupLayout {
    pub fn new(base: StrictPath, retention: Retention) -> Self {
        let games = Self::load(&base);
        Self { base, games, retention }
    }

    pub fn load(base: &StrictPath) -> std::collections::HashMap<String, StrictPath> {
        let mut overall = std::collections::HashMap::new();

        for game_dir in walkdir::WalkDir::new(base.interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .skip(1) // the base path itself
            .filter_map(crate::prelude::filter_map_walkdir)
            .filter(|x| x.file_type().is_dir())
        {
            let game_dir = StrictPath::from(&game_dir);
            let mapping_file = game_dir.joined("mapping.yaml");
            if mapping_file.is_file() {
                if let Ok(mapping) = IndividualMapping::load(&mapping_file) {
                    overall.insert(mapping.name.clone(), game_dir);
                }
            }
        }

        overall
    }

    pub fn game_layout(&self, name: &str) -> GameLayout {
        let path = self.game_folder(name);

        match GameLayout::load(path.clone(), self.retention.clone()) {
            Ok(x) => x,
            Err(_) => GameLayout {
                path,
                mapping: IndividualMapping::new(name.to_string()),
                retention: self.retention.clone(),
            },
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use maplit::*;

    fn repo() -> String {
        env!("CARGO_MANIFEST_DIR").to_string()
    }

    mod individual_mapping {
        use super::*;
        use pretty_assertions::assert_eq;

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
        use std::collections::HashMap;

        use super::*;
        use pretty_assertions::assert_eq;

        fn layout() -> BackupLayout {
            BackupLayout::new(
                StrictPath::new(format!("{}/tests/backup", repo())),
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

        #[test]
        fn can_find_existing_game_folder_with_matching_name() {
            assert_eq!(
                StrictPath::new(if cfg!(target_os = "windows") {
                    format!("\\\\?\\{}\\tests\\backup\\game1", repo())
                } else {
                    format!("{}/tests/backup/game1", repo())
                }),
                layout().game_folder("game1")
            );
        }

        #[test]
        fn can_find_existing_game_folder_with_rename() {
            assert_eq!(
                StrictPath::new(if cfg!(target_os = "windows") {
                    format!("\\\\?\\{}\\tests\\backup\\game3-renamed", repo())
                } else {
                    format!("{}/tests/backup/game3-renamed", repo())
                }),
                layout().game_folder("game3")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_without_rename() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup\\nonexistent", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/nonexistent", repo()))
                },
                layout().game_folder("nonexistent")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_with_partial_rename() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup\\foo_bar", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/foo_bar", repo()))
                },
                layout().game_folder("foo:bar")
            );
        }

        #[test]
        fn can_determine_game_folder_that_does_not_exist_with_total_rename() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup\\ludusavi-renamed-Kioq", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/ludusavi-renamed-Kioq", repo()))
                },
                layout().game_folder("***")
            );
        }

        #[test]
        fn can_determine_game_folder_by_escaping_dots_at_start_and_end() {
            assert_eq!(
                if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup\\_._", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/_._", repo()))
                },
                layout().game_folder("...")
            );
        }

        #[test]
        fn can_find_irrelevant_backup_files() {
            assert_eq!(
                vec![if cfg!(target_os = "windows") {
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup\\game1\\drive-X\\file2.txt", repo()))
                } else {
                    StrictPath::new(format!("{}/tests/backup/game1/drive-X/file2.txt", repo()))
                }],
                game_layout("game1", &format!("{}/tests/backup/game1", repo())).find_irrelevant_backup_files(
                    ".",
                    &[StrictPath::new(format!(
                        "{}/tests/backup/game1/drive-X/file1.txt",
                        repo()
                    ))]
                )
            );
            assert_eq!(
                Vec::<StrictPath>::new(),
                game_layout("game1", &format!("{}/tests/backup/game1", repo())).find_irrelevant_backup_files(
                    ".",
                    &[
                        StrictPath::new(format!("{}/tests/backup/game1/drive-X/file1.txt", repo())),
                        StrictPath::new(format!("{}/tests/backup/game1/drive-X/file2.txt", repo())),
                    ]
                )
            );
        }

        fn drives() -> HashMap<String, String> {
            let (drive, _) = StrictPath::new("foo".to_string()).split_drive();
            let folder = IndividualMapping::new_drive_folder_name(&drive);
            hashmap! { folder => drive }
        }

        fn past() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd(2000, 1, 2)
                .and_hms(3, 4, 1)
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn past_str() -> String {
            "20000102T030401Z".to_string()
        }

        fn past2() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd(2000, 1, 2)
                .and_hms(3, 4, 2)
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn past2_str() -> String {
            "20000102T030402Z".to_string()
        }

        fn now() -> chrono::DateTime<chrono::Utc> {
            chrono::NaiveDate::from_ymd(2000, 1, 2)
                .and_hms(3, 4, 5)
                .and_local_timezone(chrono::Utc)
                .unwrap()
        }

        fn now_str() -> String {
            "20000102T030405Z".to_string()
        }

        #[test]
        fn can_plan_backup_when_empty() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {},
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping::new("game1".to_string()),
                retention: Retention::default(),
            };
            assert_eq!(None, layout.plan_backup(&scan, &now(), &BackupFormats::default()));
        }

        #[test]
        fn can_plan_backup_when_initial_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "new", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping::new("game1".to_string()),
                retention: Retention::default(),
            };
            assert_eq!(
                Some(Backup::Full(FullBackup {
                    name: ".".to_string(),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_unchanged_since_last_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "old", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 0,
                },
            };
            assert_eq!(None, layout.plan_backup(&scan, &now(), &BackupFormats::default()));
        }

        #[test]
        fn can_plan_backup_when_merged_single_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "new", SystemTime::UNIX_EPOCH),
                    ScannedFile::new(format!("{}/tests/root/game1/file2.txt", repo()), 2, "old", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 0,
                },
            };
            assert_eq!(
                Some(Backup::Full(FullBackup {
                    name: ".".to_string(),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                        StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_multiple_full_retained() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "new", SystemTime::UNIX_EPOCH),
                    ScannedFile::new(format!("{}/tests/root/game1/file2.txt", repo()), 2, "old", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 2,
                    differential: 0,
                },
            };
            assert_eq!(
                Some(Backup::Full(FullBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                        StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_initial_differential() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/unchanged.txt", repo()), 1, "old", SystemTime::UNIX_EPOCH),
                    ScannedFile::new(format!("{}/tests/root/game1/changed.txt", repo()), 2, "new", SystemTime::UNIX_EPOCH),
                    ScannedFile {
                        path: StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())),
                        size: 4,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: None,
                        ignored: true,
                        container: None,
                    },
                    ScannedFile::new(format!("{}/tests/root/game1/added.txt", repo()), 5, "new", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/unchanged.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/changed.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/delete.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 3, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 4, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                Some(Backup::Differential(DifferentialBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/changed.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                        StrictPath::new(format!("{}/tests/root/game1/delete.txt", repo())).render() => None,
                        StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => None,
                        StrictPath::new(format!("{}/tests/root/game1/added.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 5, mtime: SystemTime::UNIX_EPOCH }),
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_second_differential() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/changed.txt", repo()), 2, "newer", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/unchanged.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/changed.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/delete.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 3, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 4, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", now_str()),
                            when: now(),
                            files: btreemap! {
                                StrictPath::new(format!("{}/tests/root/game1/changed.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                                StrictPath::new(format!("{}/tests/root/game1/delete.txt", repo())).render() => None,
                                StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => None,
                                StrictPath::new(format!("{}/tests/root/game1/added.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 5, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 2,
                },
            };
            assert_eq!(
                Some(Backup::Differential(DifferentialBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/unchanged.txt", repo())).render() => None,
                        StrictPath::new(format!("{}/tests/root/game1/changed.txt", repo())).render() => Some(IndividualMappingFile { hash: "newer".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                        StrictPath::new(format!("{}/tests/root/game1/delete.txt", repo())).render() => None,
                        StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => None,
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_unchanged_since_last_differential() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "new", SystemTime::UNIX_EPOCH),
                    ScannedFile::new(format!("{}/tests/root/game1/file2.txt", repo()), 2, "old", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", past2_str()),
                            when: past2(),
                            files: btreemap! {
                                StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH }),
                                StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => Some(IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 2,
                    differential: 1,
                },
            };
            assert_eq!(None, layout.plan_backup(&scan, &now(), &BackupFormats::default()));
        }

        #[test]
        fn can_plan_backup_when_changed_since_last_differential_but_matches_last_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "old", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", past2_str()),
                            when: past2(),
                            files: btreemap! {
                                StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 2,
                },
            };
            assert_eq!(
                Some(Backup::Differential(DifferentialBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_differential_ignores_something_from_last_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/ignore.txt", repo()), 2, "new", SystemTime::UNIX_EPOCH).ignored(),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 4, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 2,
                },
            };
            assert_eq!(
                Some(Backup::Differential(DifferentialBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => None,
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_differential_unignores_something_from_last_differential() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/ignore.txt", repo()), 2, "new", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 4, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", now_str()),
                            when: now(),
                            files: btreemap! {
                                StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => None,
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 2,
                },
            };
            assert_eq!(
                Some(Backup::Differential(DifferentialBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/ignore.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_differential_rollover_to_new_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "newer", SystemTime::UNIX_EPOCH),
                    ScannedFile::new(format!("{}/tests/root/game1/file2.txt", repo()), 2, "old", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: ".".to_string(),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", past2_str()),
                            when: past2(),
                            files: btreemap! {
                                StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH }),
                                StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => Some(IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 2,
                    differential: 1,
                },
            };
            assert_eq!(
                Some(Backup::Full(FullBackup {
                    name: format!("backup-{}", now_str()),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "newer".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                        StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        #[test]
        fn can_plan_backup_when_differential_rollover_to_merged_single_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root/game1/file1.txt", repo()), 1, "old", SystemTime::UNIX_EPOCH),
                    ScannedFile::new(format!("{}/tests/root/game1/file2.txt", repo()), 2, "new", SystemTime::UNIX_EPOCH),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: format!("backup-{}", past_str()),
                        when: past(),
                        files: btreemap! {
                            StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", past2_str()),
                            when: past2(),
                            files: btreemap! {
                                StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => Some(IndividualMappingFile { hash: "new".into(), size: 1, mtime: SystemTime::UNIX_EPOCH }),
                                StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => Some(IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                Some(Backup::Full(FullBackup {
                    name: ".".to_string(),
                    when: now(),
                    files: btreemap! {
                        StrictPath::new(format!("{}/tests/root/game1/file1.txt", repo())).render() => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                        StrictPath::new(format!("{}/tests/root/game1/file2.txt", repo())).render() => IndividualMappingFile { hash: "new".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                    },
                    ..Default::default()
                })),
                layout.plan_backup(&scan, &now(), &BackupFormats::default()),
            );
        }

        fn make_path(file: &str) -> StrictPath {
            StrictPath::new(if cfg!(target_os = "windows") {
                format!(
                    "\\\\?\\{}\\tests\\backup\\game1\\{}",
                    repo().replace('/', "\\"),
                    file.replace('/', "\\")
                )
            } else {
                format!("{}/tests/backup/game1/{}", repo(), file)
            })
        }

        fn make_restorable_path(backup: &str, file: &str) -> StrictPath {
            StrictPath::relative(
                format!(
                    "{backup}/drive-{}/{file}",
                    if cfg!(target_os = "windows") { "X" } else { "0" }
                ),
                Some(if cfg!(target_os = "windows") {
                    format!("\\\\?\\{}\\tests\\backup\\game1", repo().replace('/', "\\"))
                } else {
                    format!("{}/tests/backup/game1", repo())
                }),
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
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1".into(),
                        when: past(),
                        files: btreemap! {
                            mapping_file_key("/file1.txt") => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            mapping_file_key("/file2.txt") => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                hashset! {
                    ScannedFile {
                        path: make_restorable_path("backup-1", "file1.txt"),
                        size: 1,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        container: None,
                    },
                    ScannedFile {
                        path: make_restorable_path("backup-1", "file2.txt"),
                        size: 2,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        container: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest),
            );
        }

        #[test]
        fn can_report_restorable_files_for_full_backup_in_zip_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1.zip".into(),
                        when: past(),
                        files: btreemap! {
                            mapping_file_key("/file1.txt") => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            mapping_file_key("/file2.txt") => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                        },
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                hashset! {
                    ScannedFile {
                        path: make_restorable_path_zip("file1.txt"),
                        size: 1,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file1.txt")),
                        ignored: false,
                        container: Some(make_path("backup-1.zip")),
                    },
                    ScannedFile {
                        path: make_restorable_path_zip("file2.txt"),
                        size: 2,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/file2.txt")),
                        ignored: false,
                        container: Some(make_path("backup-1.zip")),
                    },
                },
                layout.restorable_files(&BackupId::Latest),
            );
        }

        #[test]
        fn can_report_restorable_files_for_differential_backup_in_simple_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1".into(),
                        when: past(),
                        files: btreemap! {
                            mapping_file_key("/unchanged.txt") => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            mapping_file_key("/changed.txt") => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                            mapping_file_key("/delete.txt") => IndividualMappingFile { hash: "old".into(), size: 3, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: "backup-2".into(),
                            when: past2(),
                            files: btreemap! {
                                mapping_file_key("/changed.txt") => Some(IndividualMappingFile { hash: "new".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                                mapping_file_key("/delete.txt") => None,
                                mapping_file_key("/added.txt") => Some(IndividualMappingFile { hash: "new".into(), size: 5, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                hashset! {
                    ScannedFile {
                        path: make_restorable_path("backup-1", "unchanged.txt"),
                        size: 1,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/unchanged.txt")),
                        ignored: false,
                        container: None,
                    },
                    ScannedFile {
                        path: make_restorable_path("backup-2", "changed.txt"),
                        size: 2,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/changed.txt")),
                        ignored: false,
                        container: None,
                    },
                    ScannedFile {
                        path: make_restorable_path("backup-2", "added.txt"),
                        size: 5,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/added.txt")),
                        ignored: false,
                        container: None,
                    },
                },
                layout.restorable_files(&BackupId::Latest),
            );
        }

        #[test]
        fn can_report_restorable_files_for_differential_backup_in_zip_format() {
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives_x(),
                    backups: VecDeque::from(vec![FullBackup {
                        name: "backup-1.zip".into(),
                        when: past(),
                        files: btreemap! {
                            mapping_file_key("/unchanged.txt") => IndividualMappingFile { hash: "old".into(), size: 1, mtime: SystemTime::UNIX_EPOCH },
                            mapping_file_key("/changed.txt") => IndividualMappingFile { hash: "old".into(), size: 2, mtime: SystemTime::UNIX_EPOCH },
                            mapping_file_key("/delete.txt") => IndividualMappingFile { hash: "old".into(), size: 3, mtime: SystemTime::UNIX_EPOCH },
                        },
                        children: vec![DifferentialBackup {
                            name: "backup-2.zip".into(),
                            when: past2(),
                            files: btreemap! {
                                mapping_file_key("/changed.txt") => Some(IndividualMappingFile { hash: "new".into(), size: 2, mtime: SystemTime::UNIX_EPOCH }),
                                mapping_file_key("/delete.txt") => None,
                                mapping_file_key("/added.txt") => Some(IndividualMappingFile { hash: "new".into(), size: 5, mtime: SystemTime::UNIX_EPOCH }),
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                hashset! {
                    ScannedFile {
                        path: make_restorable_path_zip("unchanged.txt"),
                        size: 1,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "old".into(),
                        original_path: Some(make_original_path("/unchanged.txt")),
                        ignored: false,
                        container: Some(make_path("backup-1.zip")),
                    },
                    ScannedFile {
                        path: make_restorable_path_zip("changed.txt"),
                        size: 2,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/changed.txt")),
                        ignored: false,
                        container: Some(make_path("backup-2.zip")),
                    },
                    ScannedFile {
                        path: make_restorable_path_zip("added.txt"),
                        size: 5,
                        mtime: SystemTime::UNIX_EPOCH,
                        hash: "new".into(),
                        original_path: Some(make_original_path("/added.txt")),
                        ignored: false,
                        container: Some(make_path("backup-2.zip")),
                    },
                },
                layout.restorable_files(&BackupId::Latest),
            );
        }
    }
}
