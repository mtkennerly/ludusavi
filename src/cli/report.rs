use crate::{
    lang::TRANSLATOR,
    prelude::StrictPath,
    scan::{BackupInfo, DuplicateDetector, OperationStatus, OperationStepDecision, ScanChange, ScanInfo},
};

#[derive(Debug, Default, serde::Serialize)]
struct ApiErrors {
    #[serde(rename = "someGamesFailed", skip_serializing_if = "Option::is_none")]
    some_games_failed: Option<bool>,
    #[serde(rename = "unknownGames", skip_serializing_if = "Option::is_none")]
    unknown_games: Option<Vec<String>>,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiFile {
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    failed: bool,
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    ignored: bool,
    change: ScanChange,
    bytes: u64,
    #[serde(rename = "originalPath", skip_serializing_if = "Option::is_none")]
    original_path: Option<String>,
    #[serde(rename = "redirectedPath", skip_serializing_if = "Option::is_none")]
    redirected_path: Option<String>,
    #[serde(
        rename = "duplicatedBy",
        serialize_with = "crate::serialization::ordered_set",
        skip_serializing_if = "crate::serialization::is_empty_set"
    )]
    duplicated_by: std::collections::HashSet<String>,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiRegistry {
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    failed: bool,
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    ignored: bool,
    change: ScanChange,
    #[serde(
        rename = "duplicatedBy",
        serialize_with = "crate::serialization::ordered_set",
        skip_serializing_if = "crate::serialization::is_empty_set"
    )]
    duplicated_by: std::collections::HashSet<String>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    values: std::collections::BTreeMap<String, ApiRegistryValue>,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiRegistryValue {
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    ignored: bool,
    change: ScanChange,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
enum ApiGame {
    Operative {
        decision: OperationStepDecision,
        change: ScanChange,
        #[serde(serialize_with = "crate::serialization::ordered_map")]
        files: std::collections::HashMap<String, ApiFile>,
        #[serde(serialize_with = "crate::serialization::ordered_map")]
        registry: std::collections::HashMap<String, ApiRegistry>,
    },
    Stored {
        backups: Vec<ApiBackup>,
    },
    Found {},
}

#[derive(Debug, serde::Serialize)]
struct ApiBackup {
    name: String,
    when: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct JsonOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<ApiErrors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overall: Option<OperationStatus>,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    games: std::collections::HashMap<String, ApiGame>,
}

#[derive(Debug)]
pub enum Reporter {
    Standard {
        parts: Vec<String>,
        status: Option<OperationStatus>,
    },
    Json {
        output: JsonOutput,
    },
}

impl Reporter {
    pub fn standard() -> Self {
        Self::Standard {
            parts: vec![],
            status: Some(Default::default()),
        }
    }

    pub fn json() -> Self {
        Self::Json {
            output: JsonOutput {
                errors: Default::default(),
                overall: Some(Default::default()),
                games: Default::default(),
            },
        }
    }

    fn trip_some_games_failed(&mut self) {
        if let Reporter::Json { output, .. } = self {
            if let Some(errors) = &mut output.errors {
                errors.some_games_failed = Some(true);
            } else {
                output.errors = Some(ApiErrors {
                    some_games_failed: Some(true),
                    ..Default::default()
                });
            }
        }
    }

    pub fn trip_unknown_games(&mut self, games: Vec<String>) {
        if let Reporter::Json { output, .. } = self {
            if let Some(errors) = &mut output.errors {
                errors.unknown_games = Some(games);
            } else {
                output.errors = Some(ApiErrors {
                    unknown_games: Some(games),
                    ..Default::default()
                });
            }
        }
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
        backup_info: &BackupInfo,
        decision: &OperationStepDecision,
        duplicate_detector: &DuplicateDetector,
    ) -> bool {
        let mut successful = true;
        let restoring = scan_info.restoring();

        match self {
            Self::Standard { parts, status } => {
                if !scan_info.found_anything() {
                    return true;
                }

                parts.push(TRANSLATOR.cli_game_header(
                    name,
                    scan_info.sum_bytes(&Some(backup_info.to_owned())),
                    decision,
                    duplicate_detector.is_game_duplicated(scan_info),
                    &scan_info.count_changes(),
                ));
                for entry in itertools::sorted(&scan_info.found_files) {
                    let entry_successful = !backup_info.failed_files.contains(entry);
                    if !entry_successful {
                        successful = false;
                    }
                    parts.push(TRANSLATOR.cli_game_line_item(
                        &entry.readable(restoring),
                        entry_successful,
                        entry.ignored,
                        duplicate_detector.is_file_duplicated(entry),
                        entry.change,
                        false,
                    ));

                    if let Some(alt) = entry.alt_readable(restoring) {
                        if restoring {
                            parts.push(TRANSLATOR.cli_game_line_item_redirected(&alt));
                        } else {
                            parts.push(TRANSLATOR.cli_game_line_item_redirecting(&alt));
                        }
                    }
                }
                for entry in itertools::sorted(&scan_info.found_registry_keys) {
                    let entry_successful = !backup_info.failed_registry.contains(&entry.path);
                    if !entry_successful {
                        successful = false;
                    }
                    parts.push(TRANSLATOR.cli_game_line_item(
                        &entry.path.render(),
                        entry_successful,
                        entry.ignored,
                        duplicate_detector.is_registry_duplicated(&entry.path),
                        entry.change,
                        false,
                    ));
                    for (value_name, value) in itertools::sorted(&entry.values) {
                        parts.push(TRANSLATOR.cli_game_line_item(
                            value_name,
                            true,
                            value.ignored,
                            duplicate_detector.is_registry_value_duplicated(&entry.path, value_name),
                            value.change,
                            true,
                        ));
                    }
                }

                // Blank line between games.
                parts.push("".to_string());

                if let Some(status) = status.as_mut() {
                    status.add_game(
                        scan_info,
                        &Some(backup_info.clone()),
                        decision == &OperationStepDecision::Processed,
                    );
                }
            }
            Self::Json { output } => {
                if !scan_info.found_anything() {
                    return true;
                }

                let decision = decision.clone();
                let mut files = std::collections::HashMap::new();
                let mut registry = std::collections::HashMap::new();

                for entry in itertools::sorted(&scan_info.found_files) {
                    let mut api_file = ApiFile {
                        bytes: entry.size,
                        failed: backup_info.failed_files.contains(entry),
                        ignored: entry.ignored,
                        change: entry.change,
                        ..Default::default()
                    };
                    if duplicate_detector.is_file_duplicated(entry) {
                        let mut duplicated_by = duplicate_detector.file(entry);
                        duplicated_by.remove(&scan_info.game_name);
                        api_file.duplicated_by = duplicated_by;
                    }

                    if let Some(alt) = entry.alt_readable(restoring) {
                        if restoring {
                            api_file.original_path = Some(alt);
                        } else {
                            api_file.redirected_path = Some(alt);
                        }
                    }
                    if api_file.failed {
                        successful = false;
                    }

                    files.insert(entry.readable(restoring), api_file);
                }
                for entry in itertools::sorted(&scan_info.found_registry_keys) {
                    let mut api_registry = ApiRegistry {
                        failed: backup_info.failed_registry.contains(&entry.path),
                        ignored: entry.ignored,
                        change: entry.change,
                        values: entry
                            .values
                            .iter()
                            .map(|(k, v)| {
                                (
                                    k.clone(),
                                    ApiRegistryValue {
                                        change: v.change,
                                        ignored: v.ignored,
                                    },
                                )
                            })
                            .collect(),
                        ..Default::default()
                    };
                    if duplicate_detector.is_registry_duplicated(&entry.path) {
                        let mut duplicated_by = duplicate_detector.registry(&entry.path);
                        duplicated_by.remove(&scan_info.game_name);
                        api_registry.duplicated_by = duplicated_by;
                    }

                    if api_registry.failed {
                        successful = false;
                    }

                    registry.insert(entry.path.render(), api_registry);
                }

                if let Some(overall) = output.overall.as_mut() {
                    overall.add_game(
                        scan_info,
                        &Some(backup_info.clone()),
                        decision == OperationStepDecision::Processed,
                    );
                }
                output.games.insert(
                    name.to_string(),
                    ApiGame::Operative {
                        decision,
                        change: scan_info.count_changes().overall(),
                        files,
                        registry,
                    },
                );
            }
        }

        if !successful {
            self.trip_some_games_failed();
        }
        successful
    }

    pub fn add_backup(&mut self, name: &str, scan_info: &ScanInfo) {
        match self {
            Self::Standard { parts, .. } => {
                if scan_info.available_backups.is_empty() {
                    return;
                }

                parts.push(format!("{}:", name));
                for backup in &scan_info.available_backups {
                    if let Some(comment) = backup.comment() {
                        parts.push(format!(
                            "  - \"{}\" ({}) - {}",
                            backup.name(),
                            backup.when_local().format("%Y-%m-%dT%H:%M:%S"),
                            comment,
                        ));
                    } else {
                        parts.push(format!(
                            "  - \"{}\" ({})",
                            backup.name(),
                            backup.when_local().format("%Y-%m-%dT%H:%M:%S"),
                        ));
                    }
                }

                // Blank line between games.
                parts.push("".to_string());
            }
            Self::Json { output } => {
                if scan_info.available_backups.is_empty() {
                    return;
                }

                let mut backups = vec![];
                for backup in &scan_info.available_backups {
                    backups.push(ApiBackup {
                        name: backup.name().to_string(),
                        when: *backup.when(),
                        comment: backup.comment().to_owned(),
                    });
                }

                output.games.insert(name.to_string(), ApiGame::Stored { backups });
            }
        }
    }

    pub fn add_found_titles(&mut self, names: &std::collections::BTreeSet<String>) {
        match self {
            Self::Standard { parts, .. } => {
                for name in names {
                    parts.push(name.to_owned());
                }
            }
            Self::Json { output } => {
                for name in names {
                    output.games.insert(name.to_owned(), ApiGame::Found {});
                }
            }
        }
    }

    fn render(&self, path: &StrictPath) -> String {
        match self {
            Self::Standard { parts, status } => match status {
                Some(status) => parts.join("\n") + "\n" + &TRANSLATOR.cli_summary(status, path),
                None => parts.join("\n"),
            },
            Self::Json { output } => serde_json::to_string_pretty(&output).unwrap(),
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

#[cfg(test)]
mod tests {
    use maplit::hashset;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        scan::{registry_compat::RegistryItem, ScannedFile, ScannedRegistry},
        testing::s,
    };

    fn drive() -> String {
        if cfg!(target_os = "windows") {
            StrictPath::new(s("foo")).render()[..2].to_string()
        } else {
            s("")
        }
    }

    #[test]
    fn can_render_in_standard_mode_with_minimal_input() {
        let mut reporter = Reporter::standard();
        reporter.add_game(
            "foo",
            &ScanInfo::default(),
            &BackupInfo::default(),
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        assert_eq!(
            format!(
                r#"
Overall:
  Games: 0
  Size: 0 B
  Location: {}/dev/null
            "#,
                &drive()
            )
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
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(s("/file1")),
                        size: 102_400,
                        hash: "1".to_string(),
                        original_path: None,
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: StrictPath::new(s("/file2")),
                        size: 51_200,
                        hash: "2".to_string(),
                        original_path: None,
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key2"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key3").with_value_same("Value1"),
                },
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {
                    ScannedFile::new("/file2", 51_200, "2"),
                },
                failed_registry: hashset! {
                    RegistryItem::new(s("HKEY_CURRENT_USER/Key1"))
                },
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        assert_eq!(
            r#"
foo [100.00 KiB]:
  - <drive>/file1
  - [FAILED] <drive>/file2
  - [FAILED] HKEY_CURRENT_USER/Key1
  - HKEY_CURRENT_USER/Key2
  - HKEY_CURRENT_USER/Key3
    - Value1

Overall:
  Games: 1
  Size: 100.00 KiB / 150.00 KiB
  Location: <drive>/dev/null
            "#
            .trim()
            .replace("<drive>", &drive()),
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
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(s("/file1")),
                        size: 1,
                        hash: "1".to_string(),
                        original_path: None,
                        ignored: false,
                        change: ScanChange::Same,
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {},
                failed_registry: hashset! {},
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        reporter.add_game(
            "bar",
            &ScanInfo {
                game_name: s("bar"),
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(s("/file2")),
                        size: 3,
                        hash: "2".to_string(),
                        original_path: None,
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {},
                failed_registry: hashset! {},
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        assert_eq!(
            r#"
foo [1 B]:
  - <drive>/file1

bar [3 B]:
  - <drive>/file2

Overall:
  Games: 2
  Size: 4 B
  Location: <drive>/dev/null
            "#
            .trim()
            .replace("<drive>", &drive()),
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
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(format!("{}/backup/file1", drive())),
                        size: 102_400,
                        hash: "1".to_string(),
                        original_path: Some(StrictPath::new(format!("{}/original/file1", drive()))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: StrictPath::new(format!("{}/backup/file2", drive())),
                        size: 51_200,
                        hash: "2".to_string(),
                        original_path: Some(StrictPath::new(format!("{}/original/file2", drive()))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo::default(),
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        assert_eq!(
            r#"
foo [150.00 KiB]:
  - <drive>/original/file1
  - <drive>/original/file2

Overall:
  Games: 1
  Size: 150.00 KiB
  Location: <drive>/dev/null
            "#
            .trim()
            .replace("<drive>", &drive()),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_standard_mode_with_duplicated_entries() {
        let mut reporter = Reporter::standard();

        let mut duplicate_detector = DuplicateDetector::default();
        for name in &["foo", "bar"] {
            duplicate_detector.add_game(&ScanInfo {
                game_name: s(name),
                found_files: hashset! {
                    ScannedFile::new("/file1", 102_400, "1"),
                },
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                },
                ..Default::default()
            });
        }

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hashset! {
                    ScannedFile::new("/file1", 102_400, "1"),
                },
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                },
                ..Default::default()
            },
            &BackupInfo::default(),
            &OperationStepDecision::Processed,
            &duplicate_detector,
        );
        assert_eq!(
            r#"
foo [100.00 KiB] [DUPLICATES]:
  - [DUPLICATED] <drive>/file1
  - [DUPLICATED] HKEY_CURRENT_USER/Key1

Overall:
  Games: 1
  Size: 100.00 KiB
  Location: <drive>/dev/null
            "#
            .trim()
            .replace("<drive>", &drive()),
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
                found_files: hashset! {
                    ScannedFile::new(s("/new"), 1, "1".to_string()).change(ScanChange::New),
                    ScannedFile::new(s("/different"), 1, "1".to_string()).change(ScanChange::Different),
                    ScannedFile::new(s("/same"), 1, "1".to_string()).change(ScanChange::Same),
                    ScannedFile::new(s("/unknown"), 1, "1".to_string()).change(ScanChange::Unknown),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {},
                failed_registry: hashset! {},
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        reporter.add_game(
            "bar",
            &ScanInfo {
                game_name: s("bar"),
                found_files: hashset! {
                    ScannedFile::new(s("/brand-new"), 1, "1".to_string()).change(ScanChange::New),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {},
                failed_registry: hashset! {},
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
        );
        assert_eq!(
            r#"
foo [4 B] [Δ]:
  - [Δ] <drive>/different
  - [+] <drive>/new
  - <drive>/same
  - <drive>/unknown

bar [1 B] [+]:
  - [+] <drive>/brand-new

Overall:
  Games: 2 [+1] [Δ1]
  Size: 5 B
  Location: <drive>/dev/null
            "#
            .trim()
            .replace("<drive>", &drive()),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_minimal_input() {
        let mut reporter = Reporter::json();

        reporter.add_game(
            "foo",
            &ScanInfo::default(),
            &BackupInfo::default(),
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
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
                found_files: hashset! {
                    ScannedFile::new("/file1", 100, "1"),
                    ScannedFile::new("/file2", 50, "2"),
                },
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key2"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key3").with_value_same("Value1")
                },
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {
                    ScannedFile::new("/file2", 50, "2"),
                },
                failed_registry: hashset! {
                    RegistryItem::new(s("HKEY_CURRENT_USER/Key1"))
                },
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
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
        "<drive>/file1": {
          "change": "Unknown",
          "bytes": 100
        },
        "<drive>/file2": {
          "failed": true,
          "change": "Unknown",
          "bytes": 50
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Key1": {
          "failed": true,
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
            .trim()
            .replace("<drive>", &drive()),
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
                found_files: hashset! {
                    ScannedFile {
                        path: StrictPath::new(format!("{}/backup/file1", drive())),
                        size: 100,
                        hash: "1".to_string(),
                        original_path: Some(StrictPath::new(format!("{}/original/file1", drive()))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                    ScannedFile {
                        path: StrictPath::new(format!("{}/backup/file2", drive())),
                        size: 50,
                        hash: "2".to_string(),
                        original_path: Some(StrictPath::new(format!("{}/original/file2", drive()))),
                        ignored: false,
                        change: Default::default(),
                        container: None,
                        redirected: None,
                    },
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo::default(),
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
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
        "<drive>/original/file1": {
          "change": "Unknown",
          "bytes": 100
        },
        "<drive>/original/file2": {
          "change": "Unknown",
          "bytes": 50
        }
      },
      "registry": {}
    }
  }
}
            "#
            .trim()
            .replace("<drive>", &drive()),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }

    #[test]
    fn can_render_in_json_mode_with_duplicated_entries() {
        let mut reporter = Reporter::json();

        let mut duplicate_detector = DuplicateDetector::default();
        for name in &["foo", "bar"] {
            duplicate_detector.add_game(&ScanInfo {
                game_name: s(name),
                found_files: hashset! {
                    ScannedFile::new("/file1", 102_400, "1"),
                },
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                },
                ..Default::default()
            });
        }

        reporter.add_game(
            "foo",
            &ScanInfo {
                game_name: s("foo"),
                found_files: hashset! {
                    ScannedFile::new("/file1", 100, "2"),
                },
                found_registry_keys: hashset! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                },
                ..Default::default()
            },
            &BackupInfo::default(),
            &OperationStepDecision::Processed,
            &duplicate_detector,
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
        "<drive>/file1": {
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
            .trim()
            .replace("<drive>", &drive()),
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
                found_files: hashset! {
                    ScannedFile::new("/new", 1, "1").change(ScanChange::New),
                    ScannedFile::new("/different", 1, "2").change(ScanChange::Different),
                    ScannedFile::new("/same", 1, "2").change(ScanChange::Same),
                    ScannedFile::new("/unknown", 1, "2").change(ScanChange::Unknown),
                },
                found_registry_keys: hashset! {},
                ..Default::default()
            },
            &BackupInfo {
                failed_files: hashset! {},
                failed_registry: hashset! {},
            },
            &OperationStepDecision::Processed,
            &DuplicateDetector::default(),
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
        "<drive>/different": {
          "change": "Different",
          "bytes": 1
        },
        "<drive>/new": {
          "change": "New",
          "bytes": 1
        },
        "<drive>/same": {
          "change": "Same",
          "bytes": 1
        },
        "<drive>/unknown": {
          "change": "Unknown",
          "bytes": 1
        }
      },
      "registry": {}
    }
  }
}
            "#
            .trim()
            .replace("<drive>", &drive()),
            reporter.render(&StrictPath::new(s("/dev/null")))
        );
    }
}
