use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use crate::{
    cloud::CloudChange,
    lang::TRANSLATOR,
    prelude::StrictPath,
    resource::manifest::Os,
    scan::{
        compare_ranked_titles_ref, layout::Backup, registry, BackupError, BackupInfo, DuplicateDetector,
        OperationStatus, OperationStepDecision, ScanChange, ScanInfo, TitleMatch,
    },
};

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrors {
    /// Whether any games failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub some_games_failed: Option<bool>,
    /// Names of unknown games, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown_games: Option<Vec<String>>,
    /// When this field is present,
    /// Ludusavi could not automatically synchronize with the cloud because of conflicting data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_conflict: Option<concern::CloudConflict>,
    /// When this field is present,
    /// Ludusavi tried and failed to automatically synchronize with the cloud.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_sync_failed: Option<concern::CloudSyncFailed>,
}

impl ApiErrors {
    /// This is used by the standard reporter.
    pub fn messages(&self) -> Vec<String> {
        let mut out = vec![];

        if self.cloud_conflict.is_some() {
            out.push(TRANSLATOR.prefix_warning(&TRANSLATOR.cloud_synchronize_conflict()));
        }

        if self.cloud_sync_failed.is_some() {
            out.push(TRANSLATOR.prefix_warning(&TRANSLATOR.unable_to_synchronize_with_cloud()));
        }

        out
    }
}

pub mod concern {
    #[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
    pub struct CloudConflict {}

    #[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
    pub struct CloudSyncFailed {}
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
pub struct SaveError {
    /// If the entry failed, then this explains why.
    pub message: String,
}

impl From<&BackupError> for SaveError {
    fn from(value: &BackupError) -> Self {
        Self {
            message: value.message(),
        }
    }
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiFile {
    /// Whether this entry failed to process.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SaveError>,
    /// Whether this entry was ignored.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ignored: bool,
    /// How this item compares to its previous backup (if doing a new backup)
    /// or how its previous backup compares to the current system state (if doing a restore).
    pub change: ScanChange,
    /// Size of the file.
    pub bytes: u64,
    /// If the file was restored to a
    /// redirected location, then this is its original path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    /// If the file was backed up to a redirected location,
    /// then this is its location within the backup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirected_path: Option<String>,
    /// Any other games that also have the same file path.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub duplicated_by: BTreeSet<String>,
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiRegistry {
    /// Whether this entry failed to process.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SaveError>,
    /// Whether this entry was ignored.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ignored: bool,
    /// How this item compares to its previous backup (if doing a new backup)
    /// or how its previous backup compares to the current system state (if doing a restore).
    pub change: ScanChange,
    /// Any other games that also have the same registry path.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub duplicated_by: BTreeSet<String>,
    /// Any registry values inside of the registry key.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, ApiRegistryValue>,
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiRegistryValue {
    /// Whether this entry was ignored.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ignored: bool,
    /// How this item compares to its previous backup (if doing a new backup)
    /// or how its previous backup compares to the current system state (if doing a restore).
    pub change: ScanChange,
    /// Any other games that also have the same registry key+value.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub duplicated_by: BTreeSet<String>,
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiDump {
    /// Serialized registry content, if any, when enabled by `--dump-registry`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl ApiDump {
    pub fn is_empty(&self) -> bool {
        let Self { registry } = self;

        registry.is_none()
    }
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ApiGame {
    /// Used by the `backup` and `restore` commands.
    Operative {
        /// How Ludusavi decided to handle this game.
        decision: OperationStepDecision,
        /// How this game compares to its previous backup (if doing a new backup)
        /// or how its previous backup compares to the current system state (if doing a restore).
        change: ScanChange,
        /// Each key is a file path.
        files: BTreeMap<String, ApiFile>,
        /// Each key is a registry path.
        registry: BTreeMap<String, ApiRegistry>,
        /// Raw data.
        #[serde(skip_serializing_if = "ApiDump::is_empty")]
        dump: ApiDump,
    },
    /// Used by the `backups` command.
    Stored {
        #[serde(rename = "backupPath")]
        backup_path: String,
        backups: Vec<ApiBackup>,
    },
    /// Used by the `find` command.
    Found {
        /// How well the title matches the query.
        /// Range: 0.0 to 1.0 (higher is better).
        score: Option<f64>,
    },
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiBackup {
    pub name: String,
    pub when: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Os>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub locked: bool,
}

/// General output used by commands in `--api` mode
#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutput {
    /// Any errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<ApiErrors>,
    /// Overall stats, populated by the `backup` and `restore` commands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall: Option<OperationStatus>,
    /// Each key is the name of a game.
    pub games: BTreeMap<String, ApiGame>,
    /// Each key is the path of a file relative to the cloud folder.
    /// Populated by the `cloud` commands.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cloud: BTreeMap<String, CloudEntry>,
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudEntry {
    /// How this file compares to the cloud version (if doing an upload)
    /// or the local version (if doing a download).
    pub change: ScanChange,
}

#[derive(Debug)]
pub enum Reporter {
    Standard {
        parts: Vec<String>,
        status: Option<OperationStatus>,
        errors: ApiErrors,
    },
    Json {
        output: ApiOutput,
    },
}

impl Reporter {
    pub fn standard() -> Self {
        Self::Standard {
            parts: vec![],
            status: Some(Default::default()),
            errors: Default::default(),
        }
    }

    pub fn json() -> Self {
        Self::Json {
            output: ApiOutput {
                errors: Default::default(),
                overall: Some(Default::default()),
                games: Default::default(),
                cloud: Default::default(),
            },
        }
    }

    fn set_errors(&mut self, f: impl FnOnce(&mut ApiErrors)) {
        match self {
            Reporter::Standard { errors, .. } => f(errors),
            Reporter::Json { output } => {
                if let Some(errors) = &mut output.errors.as_mut() {
                    f(errors)
                } else {
                    let mut errors = ApiErrors::default();
                    f(&mut errors);
                    output.errors = Some(errors);
                }
            }
        }
    }

    fn trip_some_games_failed(&mut self) {
        self.set_errors(|e| {
            e.some_games_failed = Some(true);
        });
    }

    pub fn trip_unknown_games(&mut self, games: Vec<String>) {
        self.set_errors(|e| {
            e.unknown_games = Some(games);
        });
    }

    pub fn trip_cloud_conflict(&mut self) {
        self.set_errors(|e| {
            e.cloud_conflict = Some(concern::CloudConflict {});
        });
    }

    pub fn trip_cloud_sync_failed(&mut self) {
        self.set_errors(|e| {
            e.cloud_sync_failed = Some(concern::CloudSyncFailed {});
        });
    }

    pub fn suppress_overall(&mut self) {
        match self {
            Self::Standard { status, .. } => {
                *status = None;
            }
            Self::Json { output, .. } => {
                output.overall = None;
            }
        }
    }

    pub fn add_game(
        &mut self,
        name: &str,
        scan_info: &ScanInfo,
        backup_info: Option<&BackupInfo>,
        decision: &OperationStepDecision,
        duplicate_detector: &DuplicateDetector,
        dump_registry: bool,
    ) -> bool {
        if !scan_info.can_report_game() {
            return true;
        }

        let mut successful = true;
        let scan_kind = scan_info.scan_kind();

        match self {
            Self::Standard { parts, status, .. } => {
                parts.push(TRANSLATOR.cli_game_header(
                    name,
                    scan_info.sum_bytes(backup_info),
                    decision,
                    !duplicate_detector.is_game_duplicated(&scan_info.game_name).resolved(),
                    scan_info.overall_change(),
                ));
                for (scan_key, entry) in itertools::sorted(&scan_info.found_files) {
                    let entry_successful = backup_info
                        .as_ref()
                        .map(|x| !x.failed_files.contains_key(scan_key))
                        .unwrap_or(true);
                    if !entry_successful {
                        successful = false;
                    }
                    parts.push(TRANSLATOR.cli_game_line_item(
                        &entry.readable(scan_key, scan_kind),
                        entry_successful,
                        entry.ignored,
                        !duplicate_detector.is_file_duplicated(scan_key, entry).resolved(),
                        entry.change(),
                        false,
                    ));

                    if let Some(alt) = entry.alt_readable(scan_key, scan_kind) {
                        match scan_kind {
                            crate::scan::ScanKind::Backup => {
                                parts.push(TRANSLATOR.cli_game_line_item_redirecting(&alt))
                            }
                            crate::scan::ScanKind::Restore => {
                                parts.push(TRANSLATOR.cli_game_line_item_redirected(&alt))
                            }
                        }
                    }

                    if let Some(error) = backup_info.as_ref().and_then(|x| x.failed_files.get(scan_key)) {
                        parts.push(TRANSLATOR.cli_game_line_item_error(error));
                    }
                }
                for (scan_key, entry) in itertools::sorted(&scan_info.found_registry_keys) {
                    let entry_successful = backup_info
                        .as_ref()
                        .map(|x| !x.failed_registry.contains_key(scan_key))
                        .unwrap_or(true);
                    if !entry_successful {
                        successful = false;
                    }
                    parts.push(TRANSLATOR.cli_game_line_item(
                        &scan_key.render(),
                        entry_successful,
                        entry.ignored,
                        !duplicate_detector.is_registry_duplicated(scan_key).resolved(),
                        entry.change(scan_info.scan_kind()),
                        false,
                    ));

                    if let Some(error) = backup_info.as_ref().and_then(|x| x.failed_registry.get(scan_key)) {
                        parts.push(TRANSLATOR.cli_game_line_item_error(error));
                    }

                    for (value_name, value) in itertools::sorted(&entry.values) {
                        parts.push(
                            TRANSLATOR.cli_game_line_item(
                                value_name,
                                true,
                                value.ignored,
                                !duplicate_detector
                                    .is_registry_value_duplicated(scan_key, value_name)
                                    .resolved(),
                                value.change(scan_info.scan_kind()),
                                true,
                            ),
                        );
                    }
                }

                if let Some(dumped_registry) = scan_info.dumped_registry.as_ref().filter(|_| dump_registry) {
                    let label = TRANSLATOR.custom_registry_label();
                    parts.push(format!("---------- {} ----------", &label));
                    parts.push(dumped_registry.serialize(registry::Format::Reg));
                    parts.push("-".repeat(22 + label.len()));
                }

                // Blank line between games.
                parts.push("".to_string());

                if let Some(status) = status.as_mut() {
                    status.add_game(scan_info, backup_info, decision == &OperationStepDecision::Processed);
                }
            }
            Self::Json { output } => {
                let decision = decision.clone();
                let mut files = BTreeMap::new();
                let mut registry = BTreeMap::new();

                for (scan_key, entry) in itertools::sorted(&scan_info.found_files) {
                    let mut api_file = ApiFile {
                        bytes: entry.size,
                        failed: backup_info
                            .as_ref()
                            .map(|x| x.failed_files.contains_key(scan_key))
                            .unwrap_or(false),
                        error: backup_info
                            .as_ref()
                            .and_then(|x| x.failed_files.get(scan_key).map(SaveError::from)),
                        ignored: entry.ignored,
                        change: entry.change(),
                        ..Default::default()
                    };
                    if !duplicate_detector.is_file_duplicated(scan_key, entry).resolved() {
                        let mut duplicated_by: BTreeSet<_> =
                            duplicate_detector.file(scan_key, entry).into_keys().collect();
                        duplicated_by.remove(&scan_info.game_name);
                        api_file.duplicated_by = duplicated_by;
                    }

                    if let Some(alt) = entry.alt_readable(scan_key, scan_kind) {
                        match scan_kind {
                            crate::scan::ScanKind::Backup => {
                                api_file.redirected_path = Some(alt);
                            }
                            crate::scan::ScanKind::Restore => {
                                api_file.original_path = Some(alt);
                            }
                        }
                    }
                    if api_file.failed {
                        successful = false;
                    }

                    files.insert(entry.readable(scan_key, scan_kind), api_file);
                }
                for (scan_key, entry) in itertools::sorted(&scan_info.found_registry_keys) {
                    let mut api_registry = ApiRegistry {
                        failed: backup_info
                            .as_ref()
                            .map(|x| x.failed_registry.contains_key(scan_key))
                            .unwrap_or(false),
                        error: backup_info
                            .as_ref()
                            .and_then(|x| x.failed_registry.get(scan_key).map(SaveError::from)),
                        ignored: entry.ignored,
                        change: entry.change(scan_info.scan_kind()),
                        values: entry
                            .values
                            .iter()
                            .map(|(k, v)| {
                                (
                                    k.clone(),
                                    ApiRegistryValue {
                                        change: v.change(scan_info.scan_kind()),
                                        ignored: v.ignored,
                                        duplicated_by: {
                                            if !duplicate_detector.is_registry_value_duplicated(scan_key, k).resolved()
                                            {
                                                let mut duplicated_by: BTreeSet<_> = duplicate_detector
                                                    .registry_value(scan_key, k)
                                                    .into_keys()
                                                    .collect();
                                                duplicated_by.remove(&scan_info.game_name);
                                                duplicated_by
                                            } else {
                                                BTreeSet::new()
                                            }
                                        },
                                    },
                                )
                            })
                            .collect(),
                        ..Default::default()
                    };
                    if !duplicate_detector.is_registry_duplicated(scan_key).resolved() {
                        let mut duplicated_by: BTreeSet<_> =
                            duplicate_detector.registry(scan_key).into_keys().collect();
                        duplicated_by.remove(&scan_info.game_name);
                        api_registry.duplicated_by = duplicated_by;
                    }

                    if api_registry.failed {
                        successful = false;
                    }

                    registry.insert(scan_key.render(), api_registry);
                }

                let dump = ApiDump {
                    registry: dump_registry
                        .then(|| {
                            scan_info
                                .dumped_registry
                                .as_ref()
                                .map(|hives| hives.serialize(registry::Format::Reg))
                        })
                        .flatten(),
                };

                if let Some(overall) = output.overall.as_mut() {
                    overall.add_game(scan_info, backup_info, decision == OperationStepDecision::Processed);
                }
                output.games.insert(
                    scan_info.game_name.clone(),
                    ApiGame::Operative {
                        decision,
                        change: scan_info.overall_change(),
                        files,
                        registry,
                        dump,
                    },
                );
            }
        }

        if !successful {
            self.trip_some_games_failed();
        }
        successful
    }

    pub fn add_backups(
        &mut self,
        name: &str,
        display_title: &str,
        backup_dir: StrictPath,
        available_backups: &[Backup],
    ) {
        match self {
            Self::Standard { parts, .. } => {
                if available_backups.is_empty() {
                    return;
                }

                parts.push(format!("{display_title}:"));
                parts.push(format!("  {} {}", TRANSLATOR.folder_label(), backup_dir.render()));
                for backup in available_backups {
                    let mut line = format!(
                        "  - \"{}\" ({})",
                        backup.name(),
                        backup.when_local().format("%Y-%m-%dT%H:%M:%S"),
                    );
                    if let Some(os) = backup.os() {
                        line += &format!(" [{os:?}]");
                    }
                    if backup.locked() {
                        line += " [🔒]";
                    }
                    if let Some(comment) = backup.comment() {
                        line += &format!(" - {comment}");
                    }
                    parts.push(line);
                }

                // Blank line between games.
                parts.push("".to_string());
            }
            Self::Json { output } => {
                if available_backups.is_empty() {
                    return;
                }

                let mut backups = vec![];
                for backup in available_backups {
                    backups.push(ApiBackup {
                        name: backup.name().to_string(),
                        when: *backup.when(),
                        os: backup.os(),
                        comment: backup.comment().cloned(),
                        locked: backup.locked(),
                    });
                }

                output.games.insert(
                    name.to_string(),
                    ApiGame::Stored {
                        backup_path: backup_dir.render(),
                        backups,
                    },
                );
            }
        }
    }

    pub fn add_found_titles(&mut self, games: &BTreeMap<String, TitleMatch>) {
        match self {
            Self::Standard { parts, .. } => {
                let games: Vec<_> = games.iter().sorted_by(compare_ranked_titles_ref).collect();

                for (name, _info) in games {
                    parts.push(name.to_owned());
                }
            }
            Self::Json { output } => {
                for (name, info) in games {
                    output
                        .games
                        .insert(name.to_owned(), ApiGame::Found { score: info.score });
                }
            }
        }
    }

    fn render(&self, path: &StrictPath) -> String {
        match self {
            Self::Standard { parts, status, errors } => match status {
                Some(status) => {
                    let mut out = parts.join("\n") + "\n" + &TRANSLATOR.cli_summary(status, path);
                    for message in errors.messages() {
                        out += &format!("\n\n{message}");
                    }
                    out
                }
                None => parts.join("\n"),
            },
            Self::Json { output } => serde_json::to_string_pretty(&output).unwrap(),
        }
    }

    #[allow(unused)]
    pub fn json_output(self) -> Option<ApiOutput> {
        match self {
            Self::Standard { .. } => None,
            Self::Json { output } => Some(output),
        }
    }

    pub fn print_failure(&self) {
        // The standard reporter doesn't need to print on failure because
        // that's handled generically in main.
        if let Self::Json { .. } = self {
            self.print(&StrictPath::new("".to_string()));
        }
    }

    pub fn print(&self, path: &StrictPath) {
        println!("{}", self.render(path));
    }
}

pub fn report_cloud_changes(changes: &[CloudChange], api: bool) {
    if api {
        let mut output = ApiOutput {
            errors: None,
            overall: None,
            games: Default::default(),
            cloud: Default::default(),
        };

        output.cloud = changes
            .iter()
            .map(|x| (x.path.clone(), CloudEntry { change: x.change }))
            .collect();
        eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
        return;
    }

    if changes.is_empty() {
        eprintln!("{}", TRANSLATOR.no_cloud_changes());
    } else {
        for CloudChange { path, change } in changes.iter().sorted() {
            println!("[{}] {}", change.symbol(), path);
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, hash_map};

    use super::*;
    use crate::{
        scan::{BackupError, ScannedFile, ScannedRegistry},
        testing::s,
    };

    #[test]
    fn can_render_in_standard_mode_with_minimal_input() {
        let mut reporter = Reporter::standard();
        reporter.add_game(
            "foo",
            &ScanInfo::default(),
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
Overall:
  Games: 0
  Size: 0 B
  Location: /dev/null
            "#
            .trim_end(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        )
    }

    #[test]
    fn can_render_in_standard_mode_with_one_game_in_backup_mode() {
        let mut reporter = Reporter::standard();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/file1".into(): ScannedFile {
                        size: 102_400,
                        hash: "1".to_string(),
                        original_path: None,
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    "/file2".into(): ScannedFile {
                        size: 51_200,
                        hash: "2".to_string(),
                        original_path: None,
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Key1".into(): ScannedRegistry::new(),
                    "HKEY_CURRENT_USER/Key2".into(): ScannedRegistry::new(),
                    "HKEY_CURRENT_USER/Key3".into(): ScannedRegistry::new().with_value_same("Value1"),
                },
                ..Default::default()
            },
            Some(&BackupInfo {
                failed_files: hash_map! {
                    "/file2".into(): BackupError::Test,
                },
                failed_registry: hash_map! {
                    "HKEY_CURRENT_USER/Key1".into(): BackupError::Test
                },
            }),
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
foo [100.00 KiB]:
  - /file1
  - [FAILED] /file2
    - test
  - [FAILED] HKEY_CURRENT_USER/Key1
    - test
  - HKEY_CURRENT_USER/Key2
  - HKEY_CURRENT_USER/Key3
    - Value1

Overall:
  Games: 1
  Size: 100.00 KiB / 150.00 KiB
  Location: /dev/null
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_standard_mode_with_multiple_games_in_backup_mode() {
        let mut reporter = Reporter::standard();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/file1".into(): ScannedFile {
                        size: 1,
                        hash: "1".to_string(),
                        original_path: None,
                        ignored: false,
                        change: ScanChange::Same,
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        reporter.add_game(
            "bar",
            &ScanInfo {
                game_name: s("bar"),
                found_files: hash_map! {
                    "/file2".into(): ScannedFile {
                        size: 3,
                        hash: "2".to_string(),
                        original_path: None,
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
foo [1 B]:
  - /file1

bar [3 B]:
  - /file2

Overall:
  Games: 2
  Size: 4 B
  Location: /dev/null
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_standard_mode_with_one_game_in_restore_mode() {
        let mut reporter = Reporter::standard();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/backup/file1".into(): ScannedFile {
                        size: 102_400,
                        hash: "1".to_string(),
                        original_path: Some(StrictPath::new(s("/original/file1"))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    "/backup/file2".into(): ScannedFile {
                        size: 51_200,
                        hash: "2".to_string(),
                        original_path: Some(StrictPath::new(s("/original/file2"))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
foo [150.00 KiB]:
  - /original/file1
  - /original/file2

Overall:
  Games: 1
  Size: 150.00 KiB
  Location: /dev/null
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_standard_mode_with_duplicated_entries() {
        let mut reporter = Reporter::standard();

        let mut duplicate_detector = DuplicateDetector::default();
        for name in &["foo", "bar"] {
            duplicate_detector.add_game(
                &ScanInfo {
                    game_name: s(name),
                    found_files: hash_map! {
                        "/file1".into(): ScannedFile::new(102_400, "1").change_as(ScanChange::New),
                    },
                    found_registry_keys: hash_map! {
                        "HKEY_CURRENT_USER/Key1".into(): ScannedRegistry::new().change_as(ScanChange::New),
                    },
                    ..Default::default()
                },
                true,
            );
        }

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/file1".into(): ScannedFile::new(102_400, "1"),
                },
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Key1".into(): ScannedRegistry::new(),
                },
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &duplicate_detector,
            false,
        );
        assert_eq!(
            r#"
foo [100.00 KiB] [DUPLICATES]:
  - [DUPLICATED] /file1
  - [DUPLICATED] HKEY_CURRENT_USER/Key1

Overall:
  Games: 1
  Size: 100.00 KiB
  Location: /dev/null
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_standard_mode_with_different_file_changes() {
        let mut reporter = Reporter::standard();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/new".into(): ScannedFile::new(1, "1".to_string()).change_as(ScanChange::New),
                    "/different".into(): ScannedFile::new(1, "1".to_string()).change_as(ScanChange::Different),
                    "/same".into(): ScannedFile::new(1, "1".to_string()).change_as(ScanChange::Same),
                    "/unknown".into(): ScannedFile::new(1, "1".to_string()).change_as(ScanChange::Unknown),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        reporter.add_game(
            "bar",
            &ScanInfo {
                game_name: s("bar"),
                found_files: hash_map! {
                    "/brand-new".into(): ScannedFile::new(1, "1".to_string()).change_as(ScanChange::New),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
foo [4 B] [Δ]:
  - [Δ] /different
  - [+] /new
  - /same
  - /unknown

bar [1 B] [+]:
  - [+] /brand-new

Overall:
  Games: 2 [+1] [Δ1]
  Size: 5 B
  Location: /dev/null
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_standard_mode_with_dumps() {
        let mut reporter = Reporter::standard();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Key".into(): ScannedRegistry::new().with_value_same("value"),
                },
                dumped_registry: Some(registry::Hives(btree_map! {
                    r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                        r"Key".into(): registry::Entries(btree_map! {
                            "value".into(): registry::Entry::Sz("data".to_string()),
                        }),
                    })
                })),
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            true,
        );
        assert_eq!(
            r#"
foo [0 B]:
  - HKEY_CURRENT_USER/Key
    - value
---------- Registry: ----------
Windows Registry Editor Version 5.00

[HKEY_CURRENT_USER\Key]
"value"="data"

-------------------------------

Overall:
  Games: 1
  Size: 0 B
  Location: /dev/null
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_minimal_input() {
        let mut reporter = Reporter::json();

        reporter.add_game(
            "foo",
            &ScanInfo::default(),
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            true,
        );
        assert_eq!(
            r#"
{
  "overall": {
    "totalGames": 0,
    "totalBytes": 0,
    "processedGames": 0,
    "processedBytes": 0,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 0
    }
  },
  "games": {}
}
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_one_game_in_backup_mode() {
        let mut reporter = Reporter::json();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/file1".into(): ScannedFile::new(100, "1"),
                    "/file2".into(): ScannedFile::new(50, "2"),
                },
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Key1".into(): ScannedRegistry::new(),
                    "HKEY_CURRENT_USER/Key2".into(): ScannedRegistry::new(),
                    "HKEY_CURRENT_USER/Key3".into(): ScannedRegistry::new().with_value_same("Value1")
                },
                ..Default::default()
            },
            Some(&BackupInfo {
                failed_files: hash_map! {
                    "/file2".into(): BackupError::Test,
                },
                failed_registry: hash_map! {
                    "HKEY_CURRENT_USER/Key1".into(): BackupError::Test
                },
            }),
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
{
  "errors": {
    "someGamesFailed": true
  },
  "overall": {
    "totalGames": 1,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 100,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {
        "/file1": {
          "change": "Unknown",
          "bytes": 100
        },
        "/file2": {
          "failed": true,
          "error": {
            "message": "test"
          },
          "change": "Unknown",
          "bytes": 50
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Key1": {
          "failed": true,
          "error": {
            "message": "test"
          },
          "change": "Unknown"
        },
        "HKEY_CURRENT_USER/Key2": {
          "change": "Unknown"
        },
        "HKEY_CURRENT_USER/Key3": {
          "change": "Unknown",
          "values": {
            "Value1": {
              "change": "Same"
            }
          }
        }
      }
    }
  }
}
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_one_game_in_restore_mode() {
        let mut reporter = Reporter::json();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/backup/file1".into(): ScannedFile {
                        size: 100,
                        hash: "1".to_string(),
                        original_path: Some(StrictPath::new(s("/original/file1"))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    "/backup/file2".into(): ScannedFile {
                        size: 50,
                        hash: "2".to_string(),
                        original_path: Some(StrictPath::new(s("/original/file2"))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
  {
  "overall": {
    "totalGames": 1,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 150,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {
        "/original/file1": {
          "change": "Unknown",
          "bytes": 100
        },
        "/original/file2": {
          "change": "Unknown",
          "bytes": 50
        }
      },
      "registry": {}
    }
  }
}
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_duplicated_entries() {
        let mut reporter = Reporter::json();

        let mut duplicate_detector = DuplicateDetector::default();
        for name in &["foo", "bar"] {
            duplicate_detector.add_game(
                &ScanInfo {
                    game_name: s(name),
                    found_files: hash_map! {
                        "/file1".into(): ScannedFile::new(102_400, "1"),
                    },
                    found_registry_keys: hash_map! {
                        "HKEY_CURRENT_USER/Key1".into(): ScannedRegistry::new(),
                    },
                    ..Default::default()
                },
                true,
            );
        }

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/file1".into(): ScannedFile::new(100, "2"),
                },
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Key1".into(): ScannedRegistry::new(),
                },
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &duplicate_detector,
            false,
        );
        assert_eq!(
            r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 100,
    "processedGames": 1,
    "processedBytes": 100,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {
        "/file1": {
          "change": "Unknown",
          "bytes": 100,
          "duplicatedBy": [
            "bar"
          ]
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Key1": {
          "change": "Unknown",
          "duplicatedBy": [
            "bar"
          ]
        }
      }
    }
  }
}
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_different_file_changes() {
        let mut reporter = Reporter::json();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hash_map! {
                    "/new".into(): ScannedFile::new(1, "1").change_as(ScanChange::New),
                    "/different".into(): ScannedFile::new(1, "2").change_as(ScanChange::Different),
                    "/same".into(): ScannedFile::new(1, "2").change_as(ScanChange::Same),
                    "/unknown".into(): ScannedFile::new(1, "2").change_as(ScanChange::Unknown),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            false,
        );
        assert_eq!(
            r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 4,
    "processedGames": 1,
    "processedBytes": 4,
    "changedGames": {
      "new": 0,
      "different": 1,
      "same": 0
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Different",
      "files": {
        "/different": {
          "change": "Different",
          "bytes": 1
        },
        "/new": {
          "change": "New",
          "bytes": 1
        },
        "/same": {
          "change": "Same",
          "bytes": 1
        },
        "/unknown": {
          "change": "Unknown",
          "bytes": 1
        }
      },
      "registry": {}
    }
  }
}
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_dumps() {
        let mut reporter = Reporter::json();

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Key".into(): ScannedRegistry::new().change_as(ScanChange::Same).with_value_same("value"),
                },
                dumped_registry: Some(registry::Hives(btree_map! {
                    r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                        r"Key".into(): registry::Entries(btree_map! {
                            "value".into(): registry::Entry::Sz("data".to_string()),
                        }),
                    })
                })),
                ..Default::default()
            },
            None,
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
            true,
        );
        assert_eq!(
            r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 0,
    "processedGames": 1,
    "processedBytes": 0,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {},
      "registry": {
        "HKEY_CURRENT_USER/Key": {
          "change": "Same",
          "values": {
            "value": {
              "change": "Same"
            }
          }
        }
      },
      "dump": {
        "registry": "Windows Registry Editor Version 5.00\n\n[HKEY_CURRENT_USER\\Key]\n\"value\"=\"data\"\n"
      }
    }
  }
}
            "#
            .trim(),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }
}
