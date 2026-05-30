use crate::{
    path::StrictPath,
    resource::config::Config,
    scan::{Launchers, layout::PathContext, preview::ScanInfo},
    semantic::prefix::{ValidatedPrefix, validate_prefix},
};

/// The outcome of evaluating Wine prefix resolution for a single game.
///
/// This is the pure, testable decision seam required by Requirement 1.
/// It classifies the resolution result without performing GUI rendering,
/// file writes, or restore operations.
#[derive(Clone, Debug)]
pub enum ResolutionOutcome {
    /// A single usable Wine prefix was found.
    Resolved { prefix: ValidatedPrefix },
    /// No usable Wine prefix was found.
    NoCandidate,
    /// Multiple usable Wine prefix candidates were found and none could be disambiguated.
    Ambiguous { candidates: Vec<StrictPath> },
    /// Multiple Wine users in the chosen prefix with no preference to disambiguate.
    AmbiguousUser { candidates: Vec<String> },
    /// An explicit CLI `--wine-prefix` disagrees with a saved per-game preference.
    Conflict {
        game: String,
        cli: StrictPath,
        configured: StrictPath,
    },
    /// A previously saved preferred prefix path no longer validates on this machine.
    StalePreference { game: String, saved: StrictPath },
}

/// What kind of choice the user must make for a game during semantic restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrefixSelectionKind {
    /// Multiple valid prefixes found; user picks one (or browses).
    AmbiguousPrefix { candidates: Vec<StrictPath> },
    /// No valid prefix found; user must browse for one.
    NoPrefix,
    /// Multiple Wine users in the chosen prefix; user picks one.
    AmbiguousUser { candidates: Vec<String> },
    /// A previously saved preferred prefix path no longer validates.
    SavedPrefixMissing { saved: StrictPath },
}

/// A request to prompt the user for a game's Wine prefix during restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefixSelectionRequest {
    pub game: String,
    pub kind: PrefixSelectionKind,
}

/// Pure, platform-agnostic prefix resolution decision function.
///
/// Wraps the existing `resolve_wine_prefix_for_game` logic and classifies the
/// result into an explicit `ResolutionOutcome`. This is the testable seam
/// required by Requirements 1, 4.5, and 14.6.
///
/// Returns `None` on Windows (prefix selection is never needed there).
pub fn decide_prefix_resolution(
    config: &Config,
    game: &str,
    game_wine_prefixes: &[StrictPath],
    cli_wine_prefix: Option<&StrictPath>,
    launchers: &Launchers,
    roots: &[crate::resource::config::Root],
    source_context: Option<&PathContext>,
    is_windows: bool,
) -> Option<ResolutionOutcome> {
    if is_windows {
        return None;
    }

    // Check for stale preference before calling the resolver.
    // If a saved preference exists but doesn't validate, flag it as StalePreference.
    if cli_wine_prefix.is_none()
        && let Some(preference) = crate::semantic::materialize::preferred_wine_prefix_for_game(config, game)
        && validate_prefix(&preference.path).is_none()
    {
        return Some(ResolutionOutcome::StalePreference {
            game: config.display_name(game).to_string(),
            saved: preference.path.clone(),
        });
    }

    match crate::semantic::materialize::resolve_wine_prefix_for_game(
        config,
        game,
        game_wine_prefixes,
        cli_wine_prefix,
        launchers,
        roots,
        source_context,
    ) {
        Ok(Some(prefix)) => Some(ResolutionOutcome::Resolved { prefix }),
        Ok(None) => Some(ResolutionOutcome::NoCandidate),
        Err(crate::prelude::Error::WinePrefixAmbiguity { game: _, candidates }) => {
            Some(ResolutionOutcome::Ambiguous { candidates })
        }
        Err(crate::prelude::Error::WineUserAmbiguity { game: _, candidates }) => {
            Some(ResolutionOutcome::AmbiguousUser { candidates })
        }
        Err(crate::prelude::Error::WinePrefixConflict { game, cli, configured }) => Some(ResolutionOutcome::Conflict {
            game,
            cli: *cli,
            configured: *configured,
        }),
        Err(_) => Some(ResolutionOutcome::NoCandidate),
    }
}

/// Does this restore need a Wine-prefix prompt on the current machine?
///
/// Pure and platform-parameterized so it can be unit-tested on any OS.
/// Returns false on Windows and for legacy (non-semantic) backups.
pub fn restore_needs_wine_prefix(scan_info: &ScanInfo, is_windows: bool) -> bool {
    if is_windows {
        return false;
    }
    // Semantic-v1 restore is indicated by populated path_contexts and/or
    // found files carrying a semantic_key. Legacy backups have neither.
    !scan_info.path_contexts.is_empty() || scan_info.found_files.values().any(|f| f.semantic_key.is_some())
}

/// Is a saved preferred prefix still usable?
pub fn saved_prefix_is_valid(path: &StrictPath) -> bool {
    validate_prefix(path).is_some()
}

/// Convert a `ResolutionOutcome` into a GUI `PrefixSelectionRequest`, if interactive.
pub fn outcome_to_selection_request(game: &str, outcome: &ResolutionOutcome) -> Option<PrefixSelectionRequest> {
    match outcome {
        ResolutionOutcome::Resolved { .. } => None,
        ResolutionOutcome::NoCandidate => Some(PrefixSelectionRequest {
            game: game.to_string(),
            kind: PrefixSelectionKind::NoPrefix,
        }),
        ResolutionOutcome::Ambiguous { candidates } => Some(PrefixSelectionRequest {
            game: game.to_string(),
            kind: PrefixSelectionKind::AmbiguousPrefix {
                candidates: candidates.clone(),
            },
        }),
        ResolutionOutcome::AmbiguousUser { candidates } => Some(PrefixSelectionRequest {
            game: game.to_string(),
            kind: PrefixSelectionKind::AmbiguousUser {
                candidates: candidates.clone(),
            },
        }),
        ResolutionOutcome::StalePreference { saved, .. } => Some(PrefixSelectionRequest {
            game: game.to_string(),
            kind: PrefixSelectionKind::SavedPrefixMissing { saved: saved.clone() },
        }),
        ResolutionOutcome::Conflict { .. } => None, // Conflict is an error, not interactive
    }
}

/// Format an actionable CLI error message for a `ResolutionOutcome`.
pub fn format_cli_prefix_message(game: &str, outcome: &ResolutionOutcome) -> Option<String> {
    match outcome {
        ResolutionOutcome::Resolved { .. } => None,
        ResolutionOutcome::NoCandidate => Some(format!(
            "No Wine prefix found for '{}'. Use --wine-prefix <path> to specify one.",
            game
        )),
        ResolutionOutcome::Ambiguous { candidates } => {
            let mut msg = format!(
                "Multiple Wine prefixes found for '{}'. Use --wine-prefix <path> to select one.\nCandidates:",
                game
            );
            for c in candidates {
                msg.push_str(&format!("\n  - {}", c.render()));
            }
            Some(msg)
        }
        ResolutionOutcome::AmbiguousUser { candidates } => {
            let mut msg = format!(
                "Multiple Wine users found for '{}'. Set restore.preferredWinePrefixes.{}.wineUser in config.\nCandidates:",
                game, game
            );
            for c in candidates {
                msg.push_str(&format!("\n  - {}", c));
            }
            Some(msg)
        }
        ResolutionOutcome::StalePreference { game, saved } => Some(format!(
            "Saved Wine prefix for '{}' is no longer available: {}\nUse --wine-prefix <path> to specify a replacement.",
            game,
            saved.render()
        )),
        ResolutionOutcome::Conflict { game, cli, configured } => Some(format!(
            "Cannot use --wine-prefix for '{}' because it conflicts with the game's preferred Wine prefix.\n  Command prefix: {}\n  Preferred prefix: {}",
            game,
            cli.render(),
            configured.render()
        )),
    }
}

/// Convert a wine-prefix-related `Error` back into a `ResolutionOutcome`
/// so that `outcome_to_selection_request` can classify it uniformly.
/// Returns `None` for non-prefix errors.
pub fn error_to_resolution_outcome(error: &crate::prelude::Error) -> Option<(String, ResolutionOutcome)> {
    match error {
        crate::prelude::Error::WinePrefixAmbiguity { game, candidates } => Some((
            game.clone(),
            ResolutionOutcome::Ambiguous {
                candidates: candidates.clone(),
            },
        )),
        crate::prelude::Error::WineUserAmbiguity { game, candidates } => Some((
            game.clone(),
            ResolutionOutcome::AmbiguousUser {
                candidates: candidates.clone(),
            },
        )),
        crate::prelude::Error::WinePrefixConflict { game, cli, configured } => Some((
            game.clone(),
            ResolutionOutcome::Conflict {
                game: game.clone(),
                cli: *cli.clone(),
                configured: *configured.clone(),
            },
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::saves::ScannedFile;
    use crate::semantic::SemanticPath;
    use std::collections::{BTreeMap, HashMap};

    fn make_legacy_scan_info() -> ScanInfo {
        ScanInfo {
            game_name: "TestGame".to_string(),
            found_files: HashMap::new(),
            path_contexts: BTreeMap::new(),
            ..Default::default()
        }
    }

    fn make_semantic_scan_info() -> ScanInfo {
        let mut found_files = HashMap::new();
        found_files.insert(
            StrictPath::new("/some/path"),
            ScannedFile {
                semantic_key: Some(SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap()),
                ..Default::default()
            },
        );
        ScanInfo {
            game_name: "TestGame".to_string(),
            found_files,
            path_contexts: BTreeMap::new(),
            ..Default::default()
        }
    }

    fn make_semantic_scan_info_with_contexts() -> ScanInfo {
        ScanInfo {
            game_name: "TestGame".to_string(),
            found_files: HashMap::new(),
            path_contexts: {
                let mut m = BTreeMap::new();
                m.insert(
                    0,
                    crate::scan::layout::PathContext {
                        prefix_path: "/home/user/.wine".to_string(),
                        wine_user: "steamuser".to_string(),
                        drive_mappings: BTreeMap::new(),
                    },
                );
                m
            },
            ..Default::default()
        }
    }

    #[test]
    fn legacy_backup_does_not_need_wine_prefix() {
        let scan = make_legacy_scan_info();
        assert!(!restore_needs_wine_prefix(&scan, false));
    }

    #[test]
    fn legacy_backup_does_not_need_wine_prefix_on_windows() {
        let scan = make_legacy_scan_info();
        assert!(!restore_needs_wine_prefix(&scan, true));
    }

    #[test]
    fn semantic_backup_needs_wine_prefix_on_non_windows() {
        let scan = make_semantic_scan_info();
        assert!(restore_needs_wine_prefix(&scan, false));
    }

    #[test]
    fn semantic_backup_does_not_need_wine_prefix_on_windows() {
        let scan = make_semantic_scan_info();
        assert!(!restore_needs_wine_prefix(&scan, true));
    }

    #[test]
    fn semantic_backup_with_contexts_needs_wine_prefix_on_non_windows() {
        let scan = make_semantic_scan_info_with_contexts();
        assert!(restore_needs_wine_prefix(&scan, false));
    }

    #[test]
    fn saved_prefix_is_valid_returns_false_for_missing_path() {
        let path = StrictPath::new("/nonexistent/path/that/does/not/exist");
        assert!(!saved_prefix_is_valid(&path));
    }

    // --- ResolutionOutcome decision function tests ---

    #[test]
    fn decide_prefix_resolution_returns_none_on_windows() {
        let config = Config::default();
        let launchers = Launchers::default();
        let outcome = decide_prefix_resolution(
            &config,
            "TestGame",
            &[],
            None,
            &launchers,
            &[],
            None,
            true, // is_windows
        );
        assert!(outcome.is_none());
    }

    #[test]
    fn decide_prefix_resolution_no_candidate_when_no_sources() {
        let config = Config::default();
        let launchers = Launchers::default();
        let outcome = decide_prefix_resolution(&config, "TestGame", &[], None, &launchers, &[], None, false);
        assert!(matches!(outcome, Some(ResolutionOutcome::NoCandidate)));
    }

    // --- outcome_to_selection_request tests ---

    #[test]
    fn outcome_resolved_produces_no_request() {
        // We can't easily construct a ValidatedPrefix without filesystem,
        // but we can test the other variants.
        let outcome = ResolutionOutcome::NoCandidate;
        let req = outcome_to_selection_request("Game", &outcome);
        assert!(req.is_some());
        assert_eq!(req.unwrap().kind, PrefixSelectionKind::NoPrefix);
    }

    #[test]
    fn outcome_ambiguous_produces_ambiguous_prefix_request() {
        let outcome = ResolutionOutcome::Ambiguous {
            candidates: vec![StrictPath::new("/a"), StrictPath::new("/b")],
        };
        let req = outcome_to_selection_request("Game", &outcome).unwrap();
        match req.kind {
            PrefixSelectionKind::AmbiguousPrefix { candidates } => assert_eq!(candidates.len(), 2),
            _ => panic!("Expected AmbiguousPrefix"),
        }
    }

    #[test]
    fn outcome_ambiguous_user_produces_ambiguous_user_request() {
        let outcome = ResolutionOutcome::AmbiguousUser {
            candidates: vec!["user1".to_string(), "user2".to_string()],
        };
        let req = outcome_to_selection_request("Game", &outcome).unwrap();
        match req.kind {
            PrefixSelectionKind::AmbiguousUser { candidates } => assert_eq!(candidates.len(), 2),
            _ => panic!("Expected AmbiguousUser"),
        }
    }

    #[test]
    fn outcome_stale_preference_produces_saved_prefix_missing_request() {
        let outcome = ResolutionOutcome::StalePreference {
            game: "Game".to_string(),
            saved: StrictPath::new("/old/prefix"),
        };
        let req = outcome_to_selection_request("Game", &outcome).unwrap();
        match req.kind {
            PrefixSelectionKind::SavedPrefixMissing { saved } => {
                assert_eq!(saved.render(), "/old/prefix")
            }
            _ => panic!("Expected SavedPrefixMissing"),
        }
    }

    #[test]
    fn outcome_conflict_produces_no_request() {
        let outcome = ResolutionOutcome::Conflict {
            game: "Game".to_string(),
            cli: StrictPath::new("/cli"),
            configured: StrictPath::new("/config"),
        };
        assert!(outcome_to_selection_request("Game", &outcome).is_none());
    }

    // --- format_cli_prefix_message tests ---

    #[test]
    fn cli_message_resolved_is_none() {
        // Can't construct Resolved without filesystem, test others
        let outcome = ResolutionOutcome::NoCandidate;
        let msg = format_cli_prefix_message("TestGame", &outcome);
        assert!(msg.unwrap().contains("--wine-prefix"));
    }

    #[test]
    fn cli_message_ambiguous_lists_candidates() {
        let outcome = ResolutionOutcome::Ambiguous {
            candidates: vec![StrictPath::new("/a"), StrictPath::new("/b")],
        };
        let msg = format_cli_prefix_message("TestGame", &outcome).unwrap();
        assert!(msg.contains("Multiple Wine prefixes"));
        assert!(msg.contains("/a"));
        assert!(msg.contains("/b"));
    }

    #[test]
    fn cli_message_ambiguous_user_lists_candidates() {
        let outcome = ResolutionOutcome::AmbiguousUser {
            candidates: vec!["alice".to_string(), "bob".to_string()],
        };
        let msg = format_cli_prefix_message("TestGame", &outcome).unwrap();
        assert!(msg.contains("Multiple Wine users"));
        assert!(msg.contains("alice"));
        assert!(msg.contains("bob"));
    }

    #[test]
    fn cli_message_stale_preference_names_game_and_path() {
        let outcome = ResolutionOutcome::StalePreference {
            game: "TestGame".to_string(),
            saved: StrictPath::new("/old/path"),
        };
        let msg = format_cli_prefix_message("TestGame", &outcome).unwrap();
        assert!(msg.contains("no longer available"));
        assert!(msg.contains("/old/path"));
    }

    #[test]
    fn cli_message_conflict_names_both_paths() {
        let outcome = ResolutionOutcome::Conflict {
            game: "TestGame".to_string(),
            cli: StrictPath::new("/cli/path"),
            configured: StrictPath::new("/config/path"),
        };
        let msg = format_cli_prefix_message("TestGame", &outcome).unwrap();
        assert!(msg.contains("conflicts"));
        assert!(msg.contains("/cli/path"));
        assert!(msg.contains("/config/path"));
    }
}
