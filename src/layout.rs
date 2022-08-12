use std::collections::{HashSet, VecDeque};

use chrono::{Datelike, Timelike};

use crate::{
    config::Retention,
    path::StrictPath,
    prelude::{BackupId, BackupInfo, ScanInfo, ScannedFile, ScannedRegistry},
};

const SAFE: &str = "_";

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
}

impl std::fmt::Display for Backup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FullBackup {
    pub name: String,
    pub when: Option<chrono::DateTime<chrono::Utc>>,
    pub children: Vec<DifferentialBackup>,
}

impl FullBackup {
    pub fn latest_diff_mut(&mut self) -> Option<&mut DifferentialBackup> {
        self.children.last_mut()
    }

    pub fn label(&self) -> String {
        match self.when {
            None => self.name.clone(),
            Some(when) => chrono::DateTime::<chrono::Local>::from(when)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackupOmission {
    /// Strings are StrictPath in rendered form.
    #[serde(
        default,
        serialize_with = "crate::serialization::ordered_set",
        skip_serializing_if = "crate::serialization::is_empty_set"
    )]
    pub files: HashSet<String>,
    #[serde(default, skip_serializing_if = "crate::serialization::is_false")]
    pub registry: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DifferentialBackup {
    pub name: String,
    pub when: Option<chrono::DateTime<chrono::Utc>>,
    pub omit: BackupOmission,
}

impl DifferentialBackup {
    pub fn omits_file(&self, file: &StrictPath) -> bool {
        self.omit.files.iter().any(|x| StrictPath::from(x).same_path(file))
    }

    pub fn omits_registry(&self) -> bool {
        self.omit.registry
    }

    pub fn label(&self) -> String {
        match self.when {
            None => self.name.clone(),
            Some(when) => chrono::DateTime::<chrono::Local>::from(when)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        }
    }
}

fn default_backup_list() -> VecDeque<FullBackup> {
    VecDeque::from(vec![FullBackup {
        name: ".".to_string(),
        ..Default::default()
    }])
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

    pub fn game_file(&mut self, base: &StrictPath, original_file: &StrictPath, backup: &str) -> StrictPath {
        let (drive, plain_path) = original_file.split_drive();
        let drive_folder = self.drive_folder_name(&drive);
        StrictPath::relative(
            format!("{}/{}/{}", backup, drive_folder, plain_path),
            Some(base.interpret()),
        )
    }

    fn latest_backup(&self) -> Option<(&FullBackup, Option<&DifferentialBackup>)> {
        let full = self.backups.back();
        full.map(|x| (x, x.children.last()))
    }

    fn latest_full_backup_mut(&mut self) -> Option<&mut FullBackup> {
        self.backups.back_mut()
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
            if full.name == "." && full.when.is_none() {
                full.when = file
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok().map(chrono::DateTime::from));
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
            .filter_map(|e| e.ok())
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BackupPlan {
    kind: BackupKind,
    mapping: IndividualMapping,
    name: String,
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
                files.extend(self.restorable_files_in(&full.name));
            }
            Some((full, Some(diff))) => {
                files.extend(self.restorable_files_in(&diff.name));

                for full_file in self.restorable_files_in(&full.name) {
                    let already_in_diff = files.iter().any(|x| {
                        x.original_path
                            .as_ref()
                            .unwrap()
                            .same_path(full_file.original_path.as_ref().unwrap())
                    });
                    let omitted_in_diff = diff.omits_file(full_file.original_path.as_ref().unwrap());
                    if !already_in_diff && !omitted_in_diff {
                        files.insert(full_file);
                    }
                }
            }
        }

        files
    }

    fn restorable_files_in(&self, backup: &str) -> std::collections::HashSet<ScannedFile> {
        let mut files = std::collections::HashSet::new();
        for drive_dir in walkdir::WalkDir::new(self.path.joined(backup).interpret())
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let raw_drive_dir = drive_dir.path().display().to_string();
            let drive_mapping = match self.mapping.drives.get::<str>(&drive_dir.file_name().to_string_lossy()) {
                Some(x) => x,
                None => continue,
            };

            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|x| x.file_type().is_file())
            {
                let raw_file = file.path().display().to_string();
                let original_path = Some(StrictPath::new(raw_file.replace(&raw_drive_dir, drive_mapping)));
                files.insert(ScannedFile {
                    path: StrictPath::new(raw_file),
                    size: match file.metadata() {
                        Ok(m) => m.len(),
                        _ => 0,
                    },
                    original_path,
                    ignored: false,
                });
            }
        }
        files
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

    fn need_backup(&self, scan: &ScanInfo) -> bool {
        let mut mapping = self.mapping.clone();

        let (full, diff) = match mapping.latest_backup() {
            Some((full, diff)) => (full.clone(), diff.cloned()),
            None => return true,
        };

        // If scan contains new or changed files:
        for scanned in scan.found_files.iter().filter(|x| !x.ignored) {
            if let Some(diff) = &diff {
                let stored_diff = mapping.game_file(&self.path, &scanned.path, &diff.name);

                if diff.omits_file(&scanned.path) {
                    return true;
                } else if stored_diff.is_file() {
                    if stored_diff.same_content(&scanned.path) {
                        continue;
                    } else {
                        return true;
                    }
                }
            }

            let stored_full = mapping.game_file(&self.path, &scanned.path, &full.name);
            if !stored_full.is_file() || !stored_full.same_content(&scanned.path) {
                return true;
            }
        }

        // If scan is missing files:
        let mut stored_files: HashSet<_> = self
            .restorable_files_in(&full.name)
            .iter()
            .filter_map(|x| x.original_path.as_ref().map(|y| y.interpret()))
            .collect();
        if let Some(diff) = &diff {
            stored_files.extend(
                self.restorable_files_in(&diff.name)
                    .iter()
                    .filter_map(|x| x.original_path.as_ref().map(|y| y.interpret())),
            );
            for omit in &diff.omit.files {
                stored_files.remove(&StrictPath::from(omit).interpret());
            }
        }
        let scanned_files: HashSet<_> = scan
            .found_files
            .iter()
            .filter(|x| !x.ignored)
            .map(|x| x.path.interpret())
            .collect();
        if stored_files != scanned_files {
            return true;
        }

        // If scan has new/changed registry or is missing some:
        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;
            let scanned_hives = Hives::from(scan);

            let full_reg_file = self.path.joined(&full.name).joined("registry.yaml");

            match &diff {
                None => match Hives::load(&full_reg_file) {
                    None => {
                        if !scan.found_registry_keys.is_empty() {
                            return true;
                        }
                    }
                    Some(stored) => {
                        if !stored.same_content(&scanned_hives) {
                            return true;
                        }
                    }
                },
                Some(diff) => {
                    let diff_reg_file = self.path.joined(&diff.name).joined("registry.yaml");

                    match (Hives::load(&full_reg_file), Hives::load(&diff_reg_file)) {
                        (None, None) => {
                            if !scan.found_registry_keys.is_empty() {
                                return true;
                            }
                        }
                        (Some(stored_full), None) => {
                            if diff.omits_registry() {
                                if !scan.found_registry_keys.is_empty() {
                                    return true;
                                }
                            } else if !stored_full.same_content(&scanned_hives) {
                                return true;
                            }
                        }
                        (_, Some(stored_diff)) => {
                            if !stored_diff.same_content(&scanned_hives) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
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

    fn generate_full_backup_name(&self, now: &chrono::DateTime<chrono::Utc>) -> String {
        if self.retention.full == 1 {
            ".".to_string()
        } else {
            format!("backup-{}", Self::generate_file_friendly_timestamp(now))
        }
    }

    fn generate_differential_backup_name(&self, now: &chrono::DateTime<chrono::Utc>) -> String {
        format!("backup-{}", Self::generate_file_friendly_timestamp(now))
    }

    fn plan_backup(&self, scan: &ScanInfo, now: &chrono::DateTime<chrono::Utc>) -> Option<BackupPlan> {
        if !scan.found_anything() {
            return None;
        }

        if !self.need_backup(scan) {
            return None;
        }

        let mut plan = BackupPlan {
            mapping: self.mapping.clone(),
            ..Default::default()
        };

        let (fulls, diffs) = self.count_backups();
        plan.kind = if fulls > 0 && diffs < self.retention.differential {
            BackupKind::Differential
        } else {
            BackupKind::Full
        };

        plan.name = match plan.kind {
            BackupKind::Full => self.generate_full_backup_name(now),
            BackupKind::Differential => self.generate_differential_backup_name(now),
        };

        match plan.kind {
            BackupKind::Full => {
                plan.mapping.backups.push_back(FullBackup {
                    name: plan.name.clone(),
                    when: Some(*now),
                    children: Default::default(),
                });
                while plan.mapping.backups.len() as u8 > self.retention.full {
                    plan.mapping.backups.pop_front();
                }
            }
            BackupKind::Differential => {
                let new = DifferentialBackup {
                    name: plan.name.clone(),
                    when: Some(*now),
                    omit: Default::default(),
                };
                if let Some(latest_full) = plan.mapping.latest_full_backup_mut() {
                    latest_full.children.push(new);
                }
            }
        }

        for file in &scan.found_files {
            if file.ignored {
                continue;
            }

            if plan.kind == BackupKind::Differential {
                if let Some(latest_full) = plan.mapping.backups.back().cloned() {
                    let stored = plan.mapping.game_file(&self.path, &file.path, &latest_full.name);
                    if stored.same_content(&file.path) {
                        continue;
                    }
                }
            }

            plan.files.insert(file.clone());
        }

        if plan.kind == BackupKind::Differential {
            if let Some((latest_full, _)) = plan.mapping.latest_backup() {
                let mut full_file_list: HashSet<_> = self
                    .restorable_files_in(&latest_full.name)
                    .iter()
                    .map(|x| x.original_path.as_ref().unwrap().render())
                    .collect();

                if let Some(latest_full) = plan.mapping.latest_full_backup_mut() {
                    if let Some(latest_diff) = latest_full.latest_diff_mut() {
                        let new_file_list: HashSet<_> = scan
                            .found_files
                            .iter()
                            .filter(|x| !x.ignored)
                            .map(|x| x.path.render())
                            .collect();
                        full_file_list.retain(|x| !new_file_list.contains(x));
                        latest_diff.omit.files = full_file_list;
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;

            let mut hives = Hives::default();
            let (found, _) = hives.incorporate(&scan.found_registry_keys);

            match plan.kind {
                BackupKind::Full => {
                    if found {
                        plan.registry = scan.found_registry_keys.clone();
                    }
                }
                BackupKind::Differential => {
                    if let Some((latest_full, _)) = plan.mapping.latest_backup() {
                        let stored = Hives::load(&self.registry_file_in(&latest_full.name));
                        match (found, stored) {
                            (false, None) => {}
                            (false, Some(_)) => {
                                if let Some(latest_full) = plan.mapping.backups.back_mut() {
                                    if let Some(new_diff) = latest_full.children.last_mut() {
                                        new_diff.omit.registry = true;
                                    }
                                }
                            }
                            (true, None) => {
                                plan.registry = scan.found_registry_keys.clone();
                            }
                            (true, Some(stored)) => {
                                if !hives.same_content(&stored) {
                                    plan.registry = scan.found_registry_keys.clone();
                                }
                            }
                        }
                    }
                }
            }
        }

        Some(plan)
    }

    fn execute_backup(&mut self, plan: BackupPlan) -> BackupInfo {
        let mut backup_info = BackupInfo::default();
        self.mapping = plan.mapping;

        let mut relevant_files = vec![];
        for file in &plan.files {
            let target_file = self.mapping.game_file(&self.path, &file.path, &plan.name);
            if file.path.same_content(&target_file) {
                relevant_files.push(target_file);
                continue;
            }
            if target_file.create_parent_dir().is_err() {
                backup_info.failed_files.insert(file.clone());
                continue;
            }
            if std::fs::copy(&file.path.interpret(), &target_file.interpret()).is_err() {
                backup_info.failed_files.insert(file.clone());
                continue;
            }
            relevant_files.push(target_file);
        }

        #[cfg(target_os = "windows")]
        {
            use crate::registry::Hives;
            let target_registry_file = self.registry_file_in(&plan.name);

            if !plan.registry.is_empty() {
                let hives = Hives::from(&plan.registry);
                hives.save(&target_registry_file);
            } else {
                let _ = target_registry_file.remove();
            }
        }

        if plan.kind == BackupKind::Full {
            self.remove_irrelevant_backup_files(&plan.name, &relevant_files);
        }

        for irrelevant_parent in self.mapping.irrelevant_parents(&self.path) {
            let _ = irrelevant_parent.remove();
        }

        self.save();
        backup_info
    }

    pub fn back_up(&mut self, scan: &ScanInfo, now: &chrono::DateTime<chrono::Utc>) -> BackupInfo {
        match self.plan_backup(scan, now) {
            None => BackupInfo::default(),
            Some(plan) => self.execute_backup(plan),
        }
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
            .filter_map(|e| e.ok())
            .filter(|x| x.file_name().to_string_lossy().starts_with("drive-"))
        {
            for file in walkdir::WalkDir::new(drive_dir.path())
                .max_depth(100)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
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
        for file in self.find_irrelevant_backup_files(backup, relevant_files) {
            let _ = file.remove();
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
enum BackupKind {
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
            .filter_map(|e| e.ok())
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
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/nonexistent", repo()))
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
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/foo_bar", repo()))
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
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/ludusavi-renamed-Kioq", repo()))
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
                    StrictPath::new(format!("\\\\?\\{}\\tests\\backup/_._", repo()))
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
            assert_eq!(None, layout.plan_backup(&scan, &now()));
        }

        #[test]
        fn can_plan_backup_when_initial_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
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
                Some(BackupPlan {
                    kind: BackupKind::Full,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        // Drive mapping will be populated on first backup execution:
                        drives: Default::default(),
                        backups: VecDeque::from(vec![FullBackup {
                            name: ".".to_string(),
                            when: Some(now()),
                            children: vec![],
                        }]),
                    },
                    name: ".".to_string(),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }

        #[test]
        fn can_plan_backup_when_merged_single_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
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
                        when: Some(past()),
                        children: vec![],
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 0,
                },
            };
            assert_eq!(
                Some(BackupPlan {
                    kind: BackupKind::Full,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        drives: drives(),
                        backups: VecDeque::from(vec![FullBackup {
                            name: ".".to_string(),
                            when: Some(now()),
                            children: vec![],
                        }]),
                    },
                    name: ".".to_string(),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }

        #[test]
        fn can_plan_backup_when_multiple_full_retained() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
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
                        when: Some(past()),
                        children: vec![],
                    }]),
                },
                retention: Retention {
                    full: 2,
                    differential: 0,
                },
            };
            assert_eq!(
                Some(BackupPlan {
                    kind: BackupKind::Full,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        drives: drives(),
                        backups: VecDeque::from(vec![
                            FullBackup {
                                name: ".".to_string(),
                                when: Some(past()),
                                children: vec![],
                            },
                            FullBackup {
                                name: format!("backup-{}", now_str()),
                                when: Some(now()),
                                children: vec![],
                            },
                        ]),
                    },
                    name: format!("backup-{}", now_str()),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }

        #[test]
        fn can_plan_backup_when_full_rollover() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            };
            let layout = GameLayout {
                path: StrictPath::new(format!("{}/tests/backup/game1", repo())),
                mapping: IndividualMapping {
                    name: "game1".to_string(),
                    drives: drives(),
                    backups: VecDeque::from_iter(vec![
                        FullBackup {
                            name: ".".to_string(),
                            when: Some(past()),
                            children: vec![],
                        },
                        FullBackup {
                            name: format!("backup-{}", past2_str()),
                            when: Some(past2()),
                            children: vec![],
                        },
                    ]),
                },
                retention: Retention {
                    full: 2,
                    differential: 0,
                },
            };
            assert_eq!(
                Some(BackupPlan {
                    kind: BackupKind::Full,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        drives: drives(),
                        backups: VecDeque::from(vec![
                            FullBackup {
                                name: format!("backup-{}", past2_str()),
                                when: Some(past2()),
                                children: vec![],
                            },
                            FullBackup {
                                name: format!("backup-{}", now_str()),
                                when: Some(now()),
                                children: vec![],
                            },
                        ]),
                    },
                    name: format!("backup-{}", now_str()),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }

        #[test]
        fn can_plan_backup_when_initial_differential() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
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
                        when: Some(past()),
                        children: vec![],
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                Some(BackupPlan {
                    kind: BackupKind::Differential,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        drives: drives(),
                        backups: VecDeque::from(vec![FullBackup {
                            name: ".".to_string(),
                            when: Some(past()),
                            children: vec![DifferentialBackup {
                                name: format!("backup-{}", now_str()),
                                when: Some(now()),
                                omit: Default::default(),
                            },],
                        },]),
                    },
                    name: format!("backup-{}", now_str()),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }

        #[test]
        fn can_plan_backup_when_differential_rollover_to_new_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
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
                        when: Some(past()),
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", past2_str()),
                            when: Some(past2()),
                            omit: Default::default(),
                        }],
                    }]),
                },
                retention: Retention {
                    full: 2,
                    differential: 1,
                },
            };
            assert_eq!(
                Some(BackupPlan {
                    kind: BackupKind::Full,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        drives: drives(),
                        backups: VecDeque::from(vec![
                            FullBackup {
                                name: ".".to_string(),
                                when: Some(past()),
                                children: vec![DifferentialBackup {
                                    name: format!("backup-{}", past2_str()),
                                    when: Some(past2()),
                                    omit: Default::default(),
                                },],
                            },
                            FullBackup {
                                name: format!("backup-{}", now_str()),
                                when: Some(now()),
                                children: vec![],
                            },
                        ]),
                    },
                    name: format!("backup-{}", now_str()),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }

        #[test]
        fn can_plan_backup_when_differential_rollover_to_merged_single_full() {
            let scan = ScanInfo {
                game_name: "game1".to_string(),
                found_files: hashset! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1),
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
                        when: Some(past()),
                        children: vec![DifferentialBackup {
                            name: format!("backup-{}", past2_str()),
                            when: Some(past2()),
                            omit: Default::default(),
                        }],
                    }]),
                },
                retention: Retention {
                    full: 1,
                    differential: 1,
                },
            };
            assert_eq!(
                Some(BackupPlan {
                    kind: BackupKind::Full,
                    mapping: IndividualMapping {
                        name: "game1".to_string(),
                        drives: drives(),
                        backups: VecDeque::from(vec![FullBackup {
                            name: ".".to_string(),
                            when: Some(now()),
                            children: vec![],
                        },]),
                    },
                    name: ".".to_string(),
                    files: scan.found_files.clone(),
                    registry: hashset! {},
                }),
                layout.plan_backup(&scan, &now()),
            );
        }
    }
}
