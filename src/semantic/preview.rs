use std::collections::BTreeSet;

use crate::path::StrictPath;
use crate::resource::config::Config;
use crate::scan::ScanInfo;
use crate::scan::layout::{BackupLayout, PathFormat};

/// Analysis result for semantic backup preview/dry-run.
#[derive(Clone, Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemanticPreviewAnalysis {
    /// Legacy keys that would become semantic keys.
    pub migrations: Vec<SemanticMigration>,
    /// Games that would start a new full backup chain.
    pub new_full_chains: Vec<String>,
    /// Configured prefixes that failed validation.
    pub invalid_prefixes: Vec<InvalidPrefix>,
    /// Semantic key conflicts.
    pub conflicts: Vec<PreviewConflict>,
}

/// A pending migration from a legacy physical key to a semantic key.
#[derive(Clone, Debug, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemanticMigration {
    /// Game whose backup key would change.
    pub game_name: String,
    /// Current physical key shown by the preview.
    pub legacy_key: String,
    /// Portable key that would be used by the new full chain.
    pub semantic_key: String,
}

/// A configured prefix that failed validation.
#[derive(Clone, Debug, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvalidPrefix {
    /// Game that configured the invalid prefix.
    pub game_name: String,
    /// Configured prefix path.
    pub path: String,
    /// Why the prefix cannot be used.
    pub reason: String,
}

/// A duplicate semantic key produced by multiple physical files.
#[derive(Clone, Debug, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewConflict {
    /// Game with duplicate portable keys.
    pub game_name: String,
    /// Portable key shared by multiple files.
    pub semantic_key: String,
    /// Physical files that produced the same portable key.
    pub physical_paths: Vec<String>,
}

impl SemanticPreviewAnalysis {
    /// Whether this preview has no portable-backup warnings or notices.
    pub fn is_empty(&self) -> bool {
        self.migrations.is_empty()
            && self.new_full_chains.is_empty()
            && self.invalid_prefixes.is_empty()
            && self.conflicts.is_empty()
    }

    /// Analyze a backup preview for portable backup changes and conflicts.
    pub fn from_backup_preview(config: &Config, layout: &BackupLayout, scans: &[(&str, &ScanInfo)]) -> Self {
        let mut analysis = Self {
            migrations: vec![],
            new_full_chains: vec![],
            invalid_prefixes: vec![],
            conflicts: vec![],
        };

        for (display_name, scan) in scans {
            let starts_new_full_chain = will_start_new_semantic_full_backup(layout, scan);
            if starts_new_full_chain {
                analysis.new_full_chains.push((*display_name).to_string());
                for (scan_key, file) in &scan.found_files {
                    if let Some(semantic_key) = &file.semantic_key {
                        analysis.migrations.push(SemanticMigration {
                            game_name: (*display_name).to_string(),
                            legacy_key: file.effective(scan_key).render(),
                            semantic_key: semantic_key.serialize(),
                        });
                    }
                }
            }

            analysis
                .invalid_prefixes
                .extend(invalid_configured_prefixes_for_scan(config, display_name, scan));

            analysis
                .conflicts
                .extend(scan.semantic_conflicts().iter().map(|conflict| PreviewConflict {
                    game_name: (*display_name).to_string(),
                    semantic_key: conflict.semantic_key.serialize(),
                    physical_paths: conflict.physical_paths.iter().map(|path| path.render()).collect(),
                }));
        }

        analysis
    }
}

/// Validate configured Wine prefixes for the scanned game and its preferred display alias.
pub fn invalid_configured_prefixes_for_scan(
    config: &Config,
    display_name: &str,
    scan: &ScanInfo,
) -> Vec<InvalidPrefix> {
    let game_name = scan.game_name.as_str();
    let alias_name = config.display_name(game_name);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut prefixes = Vec::new();

    for game in &config.custom_games {
        if game.name == game_name || game.name == alias_name {
            for prefix in &game.wine_prefix {
                push_unique_prefix(&mut seen, &mut prefixes, StrictPath::new(prefix));
            }
        }
    }
    if let Some(preference) = config
        .restore
        .preferred_wine_prefixes
        .get(game_name)
        .or_else(|| config.restore.preferred_wine_prefixes.get(alias_name))
    {
        push_unique_prefix(&mut seen, &mut prefixes, preference.path.clone());
    }

    validate_configured_prefixes(display_name, &prefixes)
}

fn push_unique_prefix(seen: &mut BTreeSet<String>, prefixes: &mut Vec<StrictPath>, path: StrictPath) {
    if seen.insert(path.render()) {
        prefixes.push(path);
    }
}

/// Check whether this preview will start a new portable full backup chain.
pub fn will_start_new_semantic_full_backup(layout: &BackupLayout, scan: &ScanInfo) -> bool {
    scan.has_semantic_keys()
        && scan.found_anything_processable()
        && layout
            .try_game_layout(&scan.game_name)
            .and_then(|game| game.latest_full_path_format())
            .is_some_and(|path_format| path_format == PathFormat::Legacy)
}

/// Validate configured prefixes and return those that fail.
pub fn validate_configured_prefixes(game_name: &str, prefixes: &[StrictPath]) -> Vec<InvalidPrefix> {
    let mut invalid = Vec::new();

    for prefix_path in prefixes {
        let rendered = prefix_path.render();
        if crate::semantic::prefix::validate_prefix(prefix_path).is_none() {
            invalid.push(InvalidPrefix {
                game_name: game_name.to_string(),
                path: rendered,
                reason: "Not a valid Wine prefix (missing drive_c, system.reg, or dosdevices)".to_string(),
            });
        }
    }

    invalid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{SemanticBase, SemanticPath};
    use crate::{
        resource::config::Config,
        scan::{
            ScanChange, ScannedFile,
            layout::{BackupLayout, FullBackup, GameLayout, IndividualMapping, IndividualMappingFile, PathFormat},
        },
        testing::s,
    };
    use std::collections::{BTreeMap, VecDeque};

    #[test]
    fn empty_analysis_is_empty() {
        let analysis = SemanticPreviewAnalysis {
            migrations: vec![],
            new_full_chains: vec![],
            invalid_prefixes: vec![],
            conflicts: vec![],
        };
        assert!(analysis.is_empty());
    }

    #[test]
    fn analysis_with_content_is_not_empty() {
        let analysis = SemanticPreviewAnalysis {
            migrations: vec![SemanticMigration {
                game_name: "Test".to_string(),
                legacy_key: "key".to_string(),
                semantic_key: SemanticPath {
                    base: SemanticBase::WinDocuments,
                    tail: "file.dat".to_string(),
                }
                .serialize(),
            }],
            new_full_chains: vec![],
            invalid_prefixes: vec![],
            conflicts: vec![],
        };
        assert!(!analysis.is_empty());
    }

    #[test]
    fn backup_preview_analysis_reports_migrations_and_full_chain_switches() {
        let temp = tempfile::tempdir().unwrap();
        let backup_root = StrictPath::new(temp.path().join("backup").to_string_lossy().to_string());
        let game_path = backup_root.joined("Game");
        GameLayout::new(
            game_path.clone(),
            IndividualMapping {
                name: "Game".to_string(),
                backups: VecDeque::from([FullBackup {
                    path_format: PathFormat::Legacy,
                    files: BTreeMap::from([(
                        "/legacy/save.dat".to_string(),
                        IndividualMappingFile {
                            hash: "old".to_string(),
                            size: 3,
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        )
        .save();
        let layout = BackupLayout::new(backup_root);
        let scan = ScanInfo {
            game_name: s("Game"),
            has_backups: true,
            found_files: [(
                StrictPath::new("/prefix/drive_c/users/steamuser/Documents/Game/save.dat"),
                ScannedFile {
                    size: 3,
                    hash: "new".to_string(),
                    change: ScanChange::Different,
                    semantic_key: Some(SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let analysis = SemanticPreviewAnalysis::from_backup_preview(&Config::default(), &layout, &[("Game", &scan)]);

        assert_eq!(1, analysis.migrations.len());
        assert_eq!(
            "/prefix/drive_c/users/steamuser/Documents/Game/save.dat",
            analysis.migrations[0].legacy_key
        );
        assert_eq!("<winDocuments>/Game/save.dat", analysis.migrations[0].semantic_key);
        assert_eq!(vec!["Game".to_string()], analysis.new_full_chains);
    }

    #[test]
    fn backup_preview_analysis_reports_invalid_alias_prefix() {
        let scan = ScanInfo {
            game_name: s("Game"),
            ..Default::default()
        };
        let mut config = Config::default();
        config.custom_games.push(crate::resource::config::CustomGame {
            name: s("Display Game"),
            alias: Some(s("Game")),
            prefer_alias: true,
            wine_prefix: vec![s("/not/a/prefix")],
            ..Default::default()
        });

        let analysis = SemanticPreviewAnalysis::from_backup_preview(
            &config,
            &BackupLayout::new(StrictPath::new("/tmp/backup")),
            &[("Display Game", &scan)],
        );

        assert_eq!(1, analysis.invalid_prefixes.len());
        assert_eq!("Display Game", analysis.invalid_prefixes[0].game_name);
        assert_eq!("/not/a/prefix", analysis.invalid_prefixes[0].path);
    }

    #[test]
    fn backup_preview_analysis_reports_invalid_preferred_prefix() {
        let scan = ScanInfo {
            game_name: s("Game"),
            ..Default::default()
        };
        let mut config = Config::default();
        config.restore.preferred_wine_prefixes.insert(
            s("Game"),
            crate::resource::config::GameWinePrefixPreference {
                path: StrictPath::new("/not/a/preferred-prefix"),
                ..Default::default()
            },
        );

        let analysis = SemanticPreviewAnalysis::from_backup_preview(
            &config,
            &BackupLayout::new(StrictPath::new("/tmp/backup")),
            &[("Game", &scan)],
        );

        assert_eq!(1, analysis.invalid_prefixes.len());
        assert_eq!("Game", analysis.invalid_prefixes[0].game_name);
        assert_eq!("/not/a/preferred-prefix", analysis.invalid_prefixes[0].path);
    }
}
