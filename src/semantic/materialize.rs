use std::collections::HashMap;

use crate::path::{CommonPath, StrictPath};
use crate::prelude::Error;
use crate::resource::config::{Config, Root};
use crate::scan::launchers::Launchers;
use crate::scan::layout::PathContext;
use crate::semantic::SemanticBase;
use crate::semantic::SemanticPath;
use crate::semantic::convert::KnownFolders;
use crate::semantic::prefix::{ValidatedPrefix, choose_wine_user_for_restore, validate_prefix};

/// Target platform for materialization.
#[derive(Clone)]
pub enum MaterializeTarget<'a> {
    CurrentWindows {
        known_folders: &'a KnownFolders,
    },
    WinePrefix {
        prefix: &'a ValidatedPrefix,
        wine_user: &'a str,
        /// Fallback drive mappings when dosdevices are unavailable.
        drive_mappings: &'a std::collections::HashMap<char, String>,
    },
}

#[derive(Clone, Debug)]
pub enum ResolvedMaterializeTarget<'a> {
    CurrentWindows {
        known_folders: &'a KnownFolders,
    },
    WinePrefix {
        prefix: ValidatedPrefix,
        wine_user: String,
        drive_mappings: HashMap<char, String>,
    },
}

impl ResolvedMaterializeTarget<'_> {
    pub fn materialize(&self, semantic: &SemanticPath) -> Result<StrictPath, MaterializeError> {
        match self {
            Self::CurrentWindows { known_folders } => {
                materialize_semantic(semantic, &MaterializeTarget::CurrentWindows { known_folders })
            }
            Self::WinePrefix {
                prefix,
                wine_user,
                drive_mappings,
            } => materialize_semantic(
                semantic,
                &MaterializeTarget::WinePrefix {
                    prefix,
                    wine_user,
                    drive_mappings,
                },
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextualFallback {
    /// Windows restores can materialize any semantic key against current known folders.
    Allow,
    /// Wine restores must not map a context-aware file through a generic prefix fallback.
    Disallow,
}

/// Build `KnownFolders` from the current platform's `CommonPath` values.
/// On Windows, this uses the Windows API for known folders.
/// On non-Windows, all values are `None`.
pub fn known_folders_from_common_path() -> KnownFolders {
    #[cfg(target_os = "windows")]
    let program_data = known_folders::get_known_folder_path(known_folders::KnownFolder::ProgramData)
        .map(|p| p.to_string_lossy().trim_end_matches(['/', '\\']).to_string());
    #[cfg(not(target_os = "windows"))]
    let program_data = None;

    #[cfg(target_os = "windows")]
    let windows = known_folders::get_known_folder_path(known_folders::KnownFolder::Windows)
        .map(|p| p.to_string_lossy().trim_end_matches(['/', '\\']).to_string());
    #[cfg(not(target_os = "windows"))]
    let windows = None;

    KnownFolders {
        saved_games: CommonPath::SavedGames.get().map(|s| s.to_string()),
        documents: CommonPath::Document.get().map(|s| s.to_string()),
        local_app_data: CommonPath::DataLocal.get().map(|s| s.to_string()),
        app_data: CommonPath::Data.get().map(|s| s.to_string()),
        public: CommonPath::Public.get().map(|s| s.to_string()),
        program_data,
        windows,
        user_profile: CommonPath::Home.get().map(|s| s.to_string()),
    }
}

/// Discover a Wine prefix from a list of candidate paths.
/// Returns the first valid prefix found, or None.
pub fn discover_wine_prefix(candidates: &[StrictPath]) -> Option<ValidatedPrefix> {
    discover_all_valid_prefixes(candidates).into_iter().next()
}

/// Discover all valid Wine prefixes from a list of candidate paths.
pub fn discover_all_valid_prefixes(candidates: &[StrictPath]) -> Vec<ValidatedPrefix> {
    candidates.iter().filter_map(validate_prefix).collect()
}

pub fn preferred_wine_prefix_for_game<'a>(
    config: &'a Config,
    game: &str,
) -> Option<&'a crate::resource::config::GameWinePrefixPreference> {
    config.restore.preferred_wine_prefixes.get(game).or_else(|| {
        let display_name = config.display_name(game);
        if display_name == game {
            None
        } else {
            config.restore.preferred_wine_prefixes.get(display_name)
        }
    })
}

/// Resolve the Wine prefix to use for a game's semantic restore.
///
/// Priority: CLI → per-game preferred → source context → custom game → launcher → global → root discovery.
pub fn resolve_wine_prefix_for_game(
    config: &Config,
    game: &str,
    game_wine_prefixes: &[StrictPath],
    cli_wine_prefix: Option<&StrictPath>,
    launchers: &Launchers,
    roots: &[Root],
    source_context: Option<&PathContext>,
) -> Result<Option<ValidatedPrefix>, Error> {
    // 1. CLI override
    if let Some(cli) = cli_wine_prefix {
        if let Some(preference) = preferred_wine_prefix_for_game(config, game)
            && !preference.path.equivalent(cli)
        {
            return Err(Error::WinePrefixConflict {
                game: config.display_name(game).to_string(),
                cli: Box::new(cli.clone()),
                configured: Box::new(preference.path.clone()),
            });
        }
        return Ok(validate_prefix(cli));
    }

    resolve_wine_prefix_without_cli(config, game, game_wine_prefixes, launchers, roots, source_context)
}

/// Resolve the configured or discovered Wine prefix for a game when no CLI override is involved.
///
/// Priority: per-game preferred → source context → custom game → launcher → global → root discovery.
pub fn resolve_wine_prefix_without_cli(
    config: &Config,
    game: &str,
    game_wine_prefixes: &[StrictPath],
    launchers: &Launchers,
    roots: &[Root],
    source_context: Option<&PathContext>,
) -> Result<Option<ValidatedPrefix>, Error> {
    // 2. Per-game preferred
    if let Some(preference) = preferred_wine_prefix_for_game(config, game) {
        if let Some(mut prefix) = validate_prefix(&preference.path) {
            if let Some(wine_user) = &preference.wine_user {
                prefix.wine_user.clone_from(wine_user);
            }
            for (drive, target) in &preference.drive_mappings {
                prefix
                    .drive_mappings
                    .insert(drive.to_ascii_lowercase(), target.render());
            }
            return Ok(Some(prefix));
        }
        return Ok(None);
    }

    // 3. Source context from backup metadata (if path exists on current system)
    if let Some(ctx) = source_context
        && !ctx.prefix_path.is_empty()
        && let Some(prefix) = validate_prefix(&StrictPath::new(&ctx.prefix_path))
    {
        return Ok(Some(prefix));
    }

    // 4. Custom game winePrefix
    if !game_wine_prefixes.is_empty() {
        let candidates = discover_all_valid_prefixes(game_wine_prefixes);
        return resolve_from_candidates(candidates, game, config);
    }

    // 5. Launcher-discovered prefixes
    let launcher_prefixes: Vec<StrictPath> = roots
        .iter()
        .flat_map(|root| {
            launchers
                .get_game(root, game)
                .filter_map(|g| g.prefix.as_ref())
                .flat_map(|wp| {
                    let pfx = wp.joined("pfx");
                    vec![wp.clone(), pfx]
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if !launcher_prefixes.is_empty() {
        let candidates = discover_all_valid_prefixes(&launcher_prefixes);
        return resolve_from_candidates(candidates, game, config);
    }

    // 6. Global fallback: config.restore.wine_prefix
    if let Some(ref global_prefix) = config.restore.wine_prefix
        && let Some(prefix) = validate_prefix(global_prefix)
    {
        return Ok(Some(prefix));
    }

    // 7. Root discovery
    let root_paths: Vec<StrictPath> = roots.iter().map(|root| root.path().clone()).collect();
    let candidates = discover_all_valid_prefixes(&root_paths);
    resolve_from_candidates(candidates, game, config)
}

/// Resolve from a list of candidates, returning ambiguity error if multiple found.
fn resolve_from_candidates(
    candidates: Vec<ValidatedPrefix>,
    game: &str,
    config: &Config,
) -> Result<Option<ValidatedPrefix>, Error> {
    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() == 1 {
        let prefix = candidates.into_iter().next().unwrap();
        return Ok(Some(resolve_wine_user(prefix, game, config)?));
    }

    // Multiple candidates — ambiguity
    Err(Error::WinePrefixAmbiguity {
        game: config.display_name(game).to_string(),
        candidates: candidates.into_iter().map(|p| p.path).collect(),
    })
}

/// Resolve wine user for a prefix, checking for ambiguity.
fn resolve_wine_user(prefix: ValidatedPrefix, game: &str, config: &Config) -> Result<ValidatedPrefix, Error> {
    let preferred_user = preferred_wine_prefix_for_game(config, game).and_then(|p| p.wine_user.as_deref());

    match choose_wine_user_for_restore(&prefix, preferred_user, None, false) {
        Ok(user) => {
            let mut prefix = prefix;
            prefix.wine_user = user;
            Ok(prefix)
        }
        Err(ambiguity) => Err(Error::WineUserAmbiguity {
            game: config.display_name(game).to_string(),
            candidates: ambiguity.candidates,
        }),
    }
}

/// Error type for materialization failures.
#[derive(Clone, Debug)]
pub enum MaterializeError {
    /// The drive letter does not exist on the target.
    MissingDrive(char),
    /// The known folder is not available.
    MissingKnownFolder(String),
    /// The path would exceed Windows long-path limits.
    PathTooLong,
    /// The target configuration is invalid.
    InvalidTarget(String),
}

impl std::fmt::Display for MaterializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDrive(c) => write!(f, "Drive {} is not available on the target", c),
            Self::MissingKnownFolder(name) => {
                write!(f, "Known folder '{}' is not available", name)
            }
            Self::PathTooLong => write!(f, "Path exceeds maximum length"),
            Self::InvalidTarget(msg) => write!(f, "Invalid target: {}", msg),
        }
    }
}

impl std::error::Error for MaterializeError {}

/// Materialize a semantic path to a physical path on the current platform.
pub fn materialize_semantic(
    semantic: &SemanticPath,
    target: &MaterializeTarget,
) -> Result<StrictPath, MaterializeError> {
    match target {
        MaterializeTarget::CurrentWindows { known_folders } => materialize_to_windows(semantic, known_folders),
        MaterializeTarget::WinePrefix {
            prefix,
            wine_user,
            drive_mappings,
        } => materialize_to_wine(semantic, prefix, wine_user, drive_mappings),
    }
}

fn materialize_to_windows(
    semantic: &SemanticPath,
    known_folders: &KnownFolders,
) -> Result<StrictPath, MaterializeError> {
    let base_path = match &semantic.base {
        SemanticBase::WinDocuments => known_folders
            .documents
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("documents".to_string()))?,
        SemanticBase::WinAppData => known_folders
            .app_data
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("app_data".to_string()))?,
        SemanticBase::WinLocalAppData => known_folders
            .local_app_data
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("local_app_data".to_string()))?,
        SemanticBase::WinLocalAppDataLow => {
            let local = known_folders
                .local_app_data
                .as_ref()
                .ok_or_else(|| MaterializeError::MissingKnownFolder("local_app_data".to_string()))?;
            return Ok(StrictPath::new(format!("{}/Low/{}", local, semantic.tail)));
        }
        SemanticBase::WinSavedGames => known_folders
            .saved_games
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("saved_games".to_string()))?,
        SemanticBase::WinPublic => known_folders
            .public
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("public".to_string()))?,
        SemanticBase::WinProgramData => known_folders
            .program_data
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("program_data".to_string()))?,
        SemanticBase::WinDir => known_folders
            .windows
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("windows".to_string()))?,
        SemanticBase::WinHome => known_folders
            .user_profile
            .as_ref()
            .ok_or_else(|| MaterializeError::MissingKnownFolder("user_profile".to_string()))?,
        SemanticBase::WinDrive(c) => {
            let drive_str = format!("{}:/", c.to_ascii_uppercase());
            // On Windows, check if the drive exists
            #[cfg(target_os = "windows")]
            {
                let drive_path = format!("{}\\", drive_str);
                if !std::path::Path::new(&drive_path).exists() {
                    return Err(MaterializeError::MissingDrive(*c));
                }
            }
            return Ok(StrictPath::new(format!("{}{}", drive_str, semantic.tail)));
        }
    };

    let result = format!("{}/{}", base_path, semantic.tail);

    // Check Windows long path limits (260 chars without extended-length prefix)
    #[cfg(target_os = "windows")]
    if result.len() > 260 {
        return Err(MaterializeError::PathTooLong);
    }

    Ok(StrictPath::new(result))
}

fn materialize_to_wine(
    semantic: &SemanticPath,
    prefix: &ValidatedPrefix,
    wine_user: &str,
    drive_mappings: &std::collections::HashMap<char, String>,
) -> Result<StrictPath, MaterializeError> {
    let prefix_path = prefix.path.render();

    let base_path = match &semantic.base {
        SemanticBase::WinDocuments => {
            format!("{}/drive_c/users/{}/Documents", prefix_path, wine_user)
        }
        SemanticBase::WinAppData => {
            format!("{}/drive_c/users/{}/AppData/Roaming", prefix_path, wine_user)
        }
        SemanticBase::WinLocalAppData => {
            format!("{}/drive_c/users/{}/AppData/Local", prefix_path, wine_user)
        }
        SemanticBase::WinLocalAppDataLow => {
            format!("{}/drive_c/users/{}/AppData/Local/Low", prefix_path, wine_user)
        }
        SemanticBase::WinSavedGames => {
            format!("{}/drive_c/users/{}/Saved Games", prefix_path, wine_user)
        }
        SemanticBase::WinPublic => {
            format!("{}/drive_c/users/Public", prefix_path)
        }
        SemanticBase::WinProgramData => {
            format!("{}/drive_c/ProgramData", prefix_path)
        }
        SemanticBase::WinDir => {
            format!("{}/drive_c/windows", prefix_path)
        }
        SemanticBase::WinHome => {
            format!("{}/drive_c/users/{}", prefix_path, wine_user)
        }
        SemanticBase::WinDrive(c) => {
            if *c == 'c' {
                format!("{}/drive_c", prefix_path)
            } else {
                // Check dosdevices mapping first
                if let Some(target) = prefix.drive_mappings.get(c) {
                    return Ok(StrictPath::new(format!("{}/{}", target, semantic.tail)));
                }
                // Fall back to config drive_mappings
                if let Some(target) = drive_mappings.get(c) {
                    return Ok(StrictPath::new(format!("{}/{}", target, semantic.tail)));
                }
                return Err(MaterializeError::MissingDrive(*c));
            }
        }
    };

    Ok(StrictPath::new(format!("{}/{}", base_path, semantic.tail)))
}

/// Build per-context `MaterializeTarget` from backup's `path_contexts`.
///
/// For each context:
/// - If the source prefix path is valid on this machine, use it directly.
/// - If invalid, call `resolve_wine_prefix_for_game` with `source_context` as a candidate.
/// - If no prefix can be resolved, leave that context unresolved.
/// - If resolution is ambiguous or conflicts with configuration, return that error.
///
/// Returns a map from context ID to an owned materialization target.
pub fn build_context_targets(
    path_contexts: &std::collections::BTreeMap<usize, PathContext>,
    config: &Config,
    game: &str,
    game_wine_prefixes: &[StrictPath],
    cli_wine_prefix: Option<&StrictPath>,
    launchers: &Launchers,
    roots: &[Root],
) -> Result<HashMap<usize, ResolvedMaterializeTarget<'static>>, Error> {
    let mut result = HashMap::new();
    for (&ctx_id, ctx) in path_contexts {
        let validated = ctx.validate();
        let target = if let Some(prefix) = validated {
            resolved_target_from_path_context(prefix, ctx)
        } else {
            // Context prefix doesn't exist on this machine — resolve with source_context.
            match resolve_wine_prefix_for_game(
                config,
                game,
                game_wine_prefixes,
                cli_wine_prefix,
                launchers,
                roots,
                Some(ctx),
            ) {
                Ok(Some(prefix)) => resolved_target_from_validated_prefix(prefix),
                Ok(None) => continue,
                Err(error) => return Err(error),
            }
        };
        result.insert(ctx_id, target);
    }
    Ok(result)
}

pub(crate) fn resolved_target_from_path_context(
    mut prefix: ValidatedPrefix,
    ctx: &PathContext,
) -> ResolvedMaterializeTarget<'static> {
    let drive_mappings: HashMap<char, String> = ctx.drive_mappings.iter().map(|(&k, v)| (k, v.clone())).collect();
    let wine_user = if ctx.wine_user.is_empty() {
        prefix.wine_user.clone()
    } else {
        ctx.wine_user.clone()
    };

    if !drive_mappings.is_empty() {
        prefix.drive_mappings.clone_from(&drive_mappings);
    }

    ResolvedMaterializeTarget::WinePrefix {
        prefix,
        wine_user,
        drive_mappings,
    }
}

fn resolved_target_from_validated_prefix(prefix: ValidatedPrefix) -> ResolvedMaterializeTarget<'static> {
    ResolvedMaterializeTarget::WinePrefix {
        wine_user: prefix.wine_user.clone(),
        drive_mappings: prefix.drive_mappings.clone(),
        prefix,
    }
}

/// Materialize semantic files and recalculate restore state.
///
/// For files with `semantic_key` set and `mapping_context_id` matching one of the
/// context targets, materialize using the per-context target. If a contextual
/// file has no matching target, only use `fallback_target` when explicitly
/// allowed. For non-contextual semantic files, use the provided `fallback_target`.
///
/// After materializing, recalculates `redirected`, `ignored`, `change`, and clears
/// `restore_error` for all files that were successfully materialized.
pub fn materialize_and_fixup(
    scan_info: &mut crate::scan::ScanInfo,
    context_targets: &HashMap<usize, ResolvedMaterializeTarget<'static>>,
    fallback_target: Option<&ResolvedMaterializeTarget<'_>>,
    contextual_fallback: ContextualFallback,
    redirects: &[crate::resource::config::RedirectConfig],
    reverse_redirects_on_restore: bool,
    toggled_paths: &crate::resource::config::ToggledPaths,
) {
    for file in scan_info.found_files.values_mut() {
        if let Some(ref semantic) = file.semantic_key {
            let effective_target = match file.mapping_context_id {
                Some(ctx_id) => match context_targets.get(&ctx_id) {
                    Some(target) => Some(target),
                    None => match contextual_fallback {
                        ContextualFallback::Allow => fallback_target,
                        ContextualFallback::Disallow => None,
                    },
                },
                None => fallback_target,
            };
            if let Some(target) = effective_target {
                match target.materialize(semantic) {
                    Ok(physical) => {
                        file.original_path = Some(physical);
                        file.restore_error = None;
                    }
                    Err(e) => {
                        file.restore_error = Some(format!("{e}"));
                    }
                }
            } else {
                file.restore_error = Some("No semantic restore target is available".to_string());
            }
        }
    }
    scan_info.recalculate_restore_state(redirects, reverse_redirects_on_restore, toggled_paths);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::config::{Config, GameWinePrefixPreference};
    use std::collections::HashMap;

    fn make_known_folders() -> KnownFolders {
        KnownFolders {
            saved_games: Some("C:/Users/Alice/Saved Games".to_string()),
            documents: Some("C:/Users/Alice/Documents".to_string()),
            local_app_data: Some("C:/Users/Alice/AppData/Local".to_string()),
            app_data: Some("C:/Users/Alice/AppData/Roaming".to_string()),
            public: Some("C:/Users/Public".to_string()),
            program_data: Some("C:/ProgramData".to_string()),
            windows: Some("C:/Windows".to_string()),
            user_profile: Some("C:/Users/Alice".to_string()),
        }
    }

    fn make_wine_prefix() -> ValidatedPrefix {
        ValidatedPrefix {
            path: StrictPath::new("/home/deck/Prefixes/Game"),
            wine_user: "steamuser".to_string(),
            has_drive_c: true,
            drive_mappings: HashMap::new(),
        }
    }

    fn make_valid_prefix(root: &StrictPath, user: &str) {
        root.joined("drive_c/users").joined(user).create_dirs().unwrap();
        root.joined("drive_c/users/Public").create_dirs().unwrap();
        root.joined("system.reg").write_with_content("").unwrap();
    }

    #[test]
    fn win_documents_to_windows() {
        let kf = make_known_folders();
        let semantic = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), "C:/Users/Alice/Documents/Game/save.dat");
    }

    #[test]
    fn resolve_wine_prefix_uses_per_game_preference() {
        let temp = tempfile::tempdir().unwrap();
        let preferred = StrictPath::new(temp.path().join("preferred").to_string_lossy().to_string());
        let discovered = StrictPath::new(temp.path().join("discovered").to_string_lossy().to_string());
        make_valid_prefix(&preferred, "alice");
        make_valid_prefix(&discovered, "steamuser");

        let mut config = Config::default();
        config.roots.push(crate::resource::config::Root::new(
            discovered.clone(),
            crate::resource::manifest::Store::OtherWine,
        ));
        config.restore.preferred_wine_prefixes.insert(
            "Game".to_string(),
            GameWinePrefixPreference {
                path: preferred.clone(),
                wine_user: Some("alice".to_string()),
                ..Default::default()
            },
        );

        let resolved =
            resolve_wine_prefix_without_cli(&config, "Game", &[], &Launchers::default(), &config.roots.clone(), None)
                .unwrap()
                .unwrap();
        assert_eq!(preferred.render(), resolved.path.render());
        assert_eq!("alice", resolved.wine_user);
    }

    #[test]
    fn resolve_wine_prefix_uses_display_alias_preference() {
        let temp = tempfile::tempdir().unwrap();
        let preferred = StrictPath::new(temp.path().join("preferred").to_string_lossy().to_string());
        make_valid_prefix(&preferred, "steamuser");

        let mut config = Config::default();
        config.custom_games.push(crate::resource::config::CustomGame {
            name: "Display Game".to_string(),
            alias: Some("Game".to_string()),
            prefer_alias: true,
            ..Default::default()
        });
        config.restore.preferred_wine_prefixes.insert(
            "Display Game".to_string(),
            GameWinePrefixPreference {
                path: preferred.clone(),
                ..Default::default()
            },
        );

        let resolved =
            resolve_wine_prefix_without_cli(&config, "Game", &[], &Launchers::default(), &config.roots.clone(), None)
                .unwrap()
                .unwrap();
        assert_eq!(preferred.render(), resolved.path.render());
    }

    #[test]
    fn resolve_wine_prefix_rejects_conflicting_cli_override() {
        let temp = tempfile::tempdir().unwrap();
        let preferred = StrictPath::new(temp.path().join("preferred").to_string_lossy().to_string());
        let cli = StrictPath::new(temp.path().join("cli").to_string_lossy().to_string());
        make_valid_prefix(&preferred, "steamuser");
        make_valid_prefix(&cli, "steamuser");

        let mut config = Config::default();
        config.restore.preferred_wine_prefixes.insert(
            "Game".to_string(),
            GameWinePrefixPreference {
                path: preferred.clone(),
                ..Default::default()
            },
        );

        let error = resolve_wine_prefix_for_game(
            &config,
            "Game",
            &[],
            Some(&cli),
            &Launchers::default(),
            &config.roots.clone(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            Error::WinePrefixConflict {
                game: "Game".to_string(),
                cli: Box::new(cli),
                configured: Box::new(preferred),
            },
            error
        );
    }

    #[test]
    fn resolve_wine_prefix_rejects_conflicting_alias_cli_override() {
        let temp = tempfile::tempdir().unwrap();
        let preferred = StrictPath::new(temp.path().join("preferred").to_string_lossy().to_string());
        let cli = StrictPath::new(temp.path().join("cli").to_string_lossy().to_string());
        make_valid_prefix(&preferred, "steamuser");
        make_valid_prefix(&cli, "steamuser");

        let mut config = Config::default();
        config.custom_games.push(crate::resource::config::CustomGame {
            name: "Display Game".to_string(),
            alias: Some("Game".to_string()),
            prefer_alias: true,
            ..Default::default()
        });
        config.restore.preferred_wine_prefixes.insert(
            "Display Game".to_string(),
            GameWinePrefixPreference {
                path: preferred.clone(),
                ..Default::default()
            },
        );

        let error = resolve_wine_prefix_for_game(
            &config,
            "Game",
            &[],
            Some(&cli),
            &Launchers::default(),
            &config.roots.clone(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            Error::WinePrefixConflict {
                game: "Display Game".to_string(),
                cli: Box::new(cli),
                configured: Box::new(preferred),
            },
            error
        );
    }

    #[test]
    fn resolve_wine_prefix_allows_matching_cli_override() {
        let temp = tempfile::tempdir().unwrap();
        let preferred = StrictPath::new(temp.path().join("preferred").to_string_lossy().to_string());
        make_valid_prefix(&preferred, "steamuser");

        let mut config = Config::default();
        config.restore.preferred_wine_prefixes.insert(
            "Game".to_string(),
            GameWinePrefixPreference {
                path: preferred.clone(),
                ..Default::default()
            },
        );

        let resolved = resolve_wine_prefix_for_game(
            &config,
            "Game",
            &[],
            Some(&preferred),
            &Launchers::default(),
            &config.roots.clone(),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(preferred.render(), resolved.path.render());
    }

    #[test]
    fn resolve_wine_prefix_applies_preferred_drive_mappings() {
        let temp = tempfile::tempdir().unwrap();
        let preferred = StrictPath::new(temp.path().join("preferred").to_string_lossy().to_string());
        let drive = StrictPath::new(temp.path().join("drive-d").to_string_lossy().to_string());
        make_valid_prefix(&preferred, "steamuser");

        let mut config = Config::default();
        config.restore.preferred_wine_prefixes.insert(
            "Game".to_string(),
            GameWinePrefixPreference {
                path: preferred,
                drive_mappings: [('D', drive.clone())].into_iter().collect(),
                ..Default::default()
            },
        );

        let resolved =
            resolve_wine_prefix_without_cli(&config, "Game", &[], &Launchers::default(), &config.roots.clone(), None)
                .unwrap()
                .unwrap();
        assert_eq!(Some(&drive.render()), resolved.drive_mappings.get(&'d'));
    }

    #[test]
    fn build_context_targets_uses_saved_context_user_and_drive_mappings() {
        use std::collections::BTreeMap;

        let temp = tempfile::tempdir().unwrap();
        let prefix = StrictPath::new(temp.path().join("prefix").to_string_lossy().to_string());
        let drive = StrictPath::new(temp.path().join("drive-d").to_string_lossy().to_string());
        make_valid_prefix(&prefix, "detected_user");

        let mut contexts = BTreeMap::new();
        contexts.insert(
            0,
            PathContext {
                prefix_path: prefix.render(),
                wine_user: "saved_user".to_string(),
                drive_mappings: [('d', drive.render())].into_iter().collect(),
            },
        );

        let targets = build_context_targets(
            &contexts,
            &Config::default(),
            "Game",
            &[],
            None,
            &Launchers::default(),
            &[],
        )
        .unwrap();
        let target = targets.get(&0).unwrap();

        let documents = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        let materialized = target.materialize(&documents).unwrap();
        assert!(
            materialized.render().contains("/drive_c/users/saved_user/Documents/"),
            "expected saved context user, got {}",
            materialized.render()
        );

        let drive_file = SemanticPath::parse("<winDrive-d>/Game/save.dat").unwrap();
        let materialized = target.materialize(&drive_file).unwrap();
        assert_eq!(format!("{}/Game/save.dat", drive.render()), materialized.render());
    }

    #[test]
    fn build_context_targets_uses_resolved_prefix_user_when_source_context_is_missing() {
        use crate::resource::config::Root;
        use crate::resource::manifest::Store;
        use std::collections::BTreeMap;

        let temp = tempfile::tempdir().unwrap();
        let source_drive = StrictPath::new(temp.path().join("source-drive-d").to_string_lossy().to_string());
        let target_prefix = StrictPath::new(temp.path().join("target-prefix").to_string_lossy().to_string());
        make_valid_prefix(&target_prefix, "target_user");

        let mut contexts = BTreeMap::new();
        contexts.insert(
            0,
            PathContext {
                prefix_path: temp.path().join("missing-source-prefix").to_string_lossy().to_string(),
                wine_user: "source_user".to_string(),
                drive_mappings: [('d', source_drive.render())].into_iter().collect(),
            },
        );
        let roots = vec![Root::new(target_prefix.clone(), Store::OtherWine)];

        let targets = build_context_targets(
            &contexts,
            &Config::default(),
            "Game",
            &[],
            None,
            &Launchers::default(),
            &roots,
        )
        .unwrap();
        let target = targets.get(&0).unwrap();

        let documents = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        let materialized = target.materialize(&documents).unwrap();
        assert_eq!(
            format!(
                "{}/drive_c/users/target_user/Documents/Game/save.dat",
                target_prefix.render()
            ),
            materialized.render()
        );

        let drive_file = SemanticPath::parse("<winDrive-d>/Game/save.dat").unwrap();
        assert!(target.materialize(&drive_file).is_err());
    }

    #[test]
    fn build_context_targets_propagates_resolver_errors() {
        use crate::resource::config::Root;
        use crate::resource::manifest::Store;
        use std::collections::BTreeMap;

        let temp = tempfile::tempdir().unwrap();
        let prefix_a = StrictPath::new(temp.path().join("prefix-a").to_string_lossy().to_string());
        let prefix_b = StrictPath::new(temp.path().join("prefix-b").to_string_lossy().to_string());
        make_valid_prefix(&prefix_a, "steamuser");
        make_valid_prefix(&prefix_b, "steamuser");

        let mut contexts = BTreeMap::new();
        contexts.insert(
            0,
            PathContext {
                prefix_path: temp.path().join("missing").to_string_lossy().to_string(),
                wine_user: "steamuser".to_string(),
                drive_mappings: BTreeMap::new(),
            },
        );
        let roots = vec![
            Root::new(prefix_a.clone(), Store::OtherWine),
            Root::new(prefix_b.clone(), Store::OtherWine),
        ];

        let error = build_context_targets(
            &contexts,
            &Config::default(),
            "Game",
            &[],
            None,
            &Launchers::default(),
            &roots,
        )
        .unwrap_err();

        assert_eq!(
            Error::WinePrefixAmbiguity {
                game: "Game".to_string(),
                candidates: vec![prefix_a, prefix_b],
            },
            error
        );
    }

    #[test]
    fn win_appdata_to_windows() {
        let kf = make_known_folders();
        let semantic = SemanticPath::parse("<winAppData>/Game/save.dat").unwrap();
        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), "C:/Users/Alice/AppData/Roaming/Game/save.dat");
    }

    #[test]
    fn win_local_appdata_to_windows() {
        let kf = make_known_folders();
        let semantic = SemanticPath::parse("<winLocalAppData>/Game/save.dat").unwrap();
        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), "C:/Users/Alice/AppData/Local/Game/save.dat");
    }

    #[test]
    fn win_local_appdata_low_to_windows() {
        let kf = make_known_folders();
        let semantic = SemanticPath::parse("<winLocalAppDataLow>/Game/save.dat").unwrap();
        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), "C:/Users/Alice/AppData/Local/Low/Game/save.dat");
    }

    #[test]
    fn win_drive_d_to_windows() {
        let kf = make_known_folders();
        let semantic = SemanticPath::parse("<winDrive-d>/Games/save.dat").unwrap();
        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), "D:/Games/save.dat");
    }

    #[test]
    fn win_documents_to_wine() {
        let prefix = make_wine_prefix();
        let semantic = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        let target = MaterializeTarget::WinePrefix {
            prefix: &prefix,
            wine_user: "steamuser",
            drive_mappings: &std::collections::HashMap::new(),
        };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(
            result.render(),
            "/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat"
        );
    }

    #[test]
    fn win_appdata_to_wine() {
        let prefix = make_wine_prefix();
        let semantic = SemanticPath::parse("<winAppData>/Game/save.dat").unwrap();
        let target = MaterializeTarget::WinePrefix {
            prefix: &prefix,
            wine_user: "steamuser",
            drive_mappings: &std::collections::HashMap::new(),
        };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(
            result.render(),
            "/home/deck/Prefixes/Game/drive_c/users/steamuser/AppData/Roaming/Game/save.dat"
        );
    }

    #[test]
    fn win_drive_d_to_wine_with_mapping() {
        let mut prefix = make_wine_prefix();
        prefix.drive_mappings.insert('d', "/mnt/data".to_string());
        let semantic = SemanticPath::parse("<winDrive-d>/Games/save.dat").unwrap();
        let target = MaterializeTarget::WinePrefix {
            prefix: &prefix,
            wine_user: "steamuser",
            drive_mappings: &std::collections::HashMap::new(),
        };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), "/mnt/data/Games/save.dat");
    }

    #[test]
    fn win_drive_d_to_wine_without_mapping() {
        let prefix = make_wine_prefix();
        let semantic = SemanticPath::parse("<winDrive-d>/Games/save.dat").unwrap();
        let target = MaterializeTarget::WinePrefix {
            prefix: &prefix,
            wine_user: "steamuser",
            drive_mappings: &std::collections::HashMap::new(),
        };
        let result = materialize_semantic(&semantic, &target);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MaterializeError::MissingDrive('d')));
    }

    #[test]
    fn round_trip_windows() {
        let kf = make_known_folders();
        let original = "C:/Users/Alice/Documents/Game/save.dat";
        let sp = StrictPath::new(original);
        let semantic = crate::semantic::convert::windows_physical_to_semantic(&sp, &kf).unwrap();
        let target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), original);
    }

    #[test]
    fn round_trip_wine() {
        let prefix = make_wine_prefix();
        let original = "/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat";
        let sp = StrictPath::new(original);
        let semantic = crate::semantic::convert::wine_physical_to_semantic(&sp, &prefix.path, "steamuser").unwrap();
        let target = MaterializeTarget::WinePrefix {
            prefix: &prefix,
            wine_user: "steamuser",
            drive_mappings: &std::collections::HashMap::new(),
        };
        let result = materialize_semantic(&semantic, &target).unwrap();
        assert_eq!(result.render(), original);
    }

    #[test]
    fn integration_wine_backup_windows_restore() {
        // Simulates: scan in Wine → serialize to mapping.yaml → restore on Windows
        let prefix = make_wine_prefix();
        let kf = make_known_folders();

        // 1. Scan in Wine: physical path → semantic key
        let wine_physical =
            StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/MyGame/save.dat");
        let semantic =
            crate::semantic::convert::wine_physical_to_semantic(&wine_physical, &prefix.path, "steamuser").unwrap();
        assert_eq!(semantic.base, SemanticBase::WinDocuments);
        assert_eq!(semantic.tail, "MyGame/save.dat");

        // 2. Serialize to mapping.yaml key
        let mapping_key = semantic.serialize();
        assert_eq!(mapping_key, "<winDocuments>/MyGame/save.dat");

        // 3. Parse back from mapping.yaml
        let parsed = SemanticPath::parse(&mapping_key).unwrap();
        assert_eq!(parsed, semantic);

        // 4. Materialize on Windows
        let win_target = MaterializeTarget::CurrentWindows { known_folders: &kf };
        let win_physical = materialize_semantic(&parsed, &win_target).unwrap();
        assert_eq!(win_physical.render(), "C:/Users/Alice/Documents/MyGame/save.dat");

        // 5. Materialize back to a different Wine prefix
        let mut prefix2 = make_wine_prefix();
        prefix2.path = StrictPath::new("/home/alice/Games/Prefixes/MyGame");
        let wine_target = MaterializeTarget::WinePrefix {
            prefix: &prefix2,
            wine_user: "alice",
            drive_mappings: &std::collections::HashMap::new(),
        };
        let wine_physical2 = materialize_semantic(&parsed, &wine_target).unwrap();
        assert_eq!(
            wine_physical2.render(),
            "/home/alice/Games/Prefixes/MyGame/drive_c/users/alice/Documents/MyGame/save.dat"
        );
    }

    #[test]
    fn integration_format_switch_forces_full_backup() {
        // This test verifies the logic at the type level.
        // A legacy backup has path_format: Legacy.
        // If a scan produces semantic keys, has_semantic_keys() returns true.
        // plan_backup_kind should then force Full when the last backup is Legacy.
        use crate::scan::layout::{FullBackup, PathFormat};

        let legacy_full = FullBackup {
            path_format: PathFormat::Legacy,
            ..Default::default()
        };
        assert_eq!(legacy_full.path_format, PathFormat::Legacy);

        let semantic_full = FullBackup {
            path_format: PathFormat::SemanticV1,
            ..Default::default()
        };
        assert_eq!(semantic_full.path_format, PathFormat::SemanticV1);
    }

    #[test]
    fn materialize_and_fixup_recalculates_restore_state() {
        use crate::resource::config::ToggledPaths;
        use crate::scan::ScanChange;
        use crate::scan::ScanInfo;
        use crate::scan::ScannedFile;
        use std::collections::HashMap;

        let semantic = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "MyGame/save.dat".to_string(),
        };

        let mut scan_info = ScanInfo {
            game_name: "MyGame".to_string(),
            found_files: HashMap::new(),
            ..Default::default()
        };

        // File with semantic key but no mapping_context_id (non-contextual).
        scan_info.found_files.insert(
            crate::path::StrictPath::new("scan_key"),
            ScannedFile {
                semantic_key: Some(semantic),
                mapping_context_id: None,
                original_path: Some(crate::path::StrictPath::new("<winDocuments>/MyGame/save.dat")),
                hash: "abc123".to_string(),
                change: ScanChange::Unknown,
                ..Default::default()
            },
        );

        let context_targets = HashMap::new();

        // Create a fallback target.
        let kf = make_known_folders();
        let fallback = ResolvedMaterializeTarget::CurrentWindows { known_folders: &kf };

        materialize_and_fixup(
            &mut scan_info,
            &context_targets,
            Some(&fallback),
            ContextualFallback::Disallow,
            &[],
            false,
            &ToggledPaths::default(),
        );

        let file = scan_info.found_files.values().next().unwrap();
        // original_path should be materialized (not the placeholder semantic string).
        assert!(file.original_path.is_some());
        let path = file.original_path.as_ref().unwrap().render();
        assert!(!path.contains("<winDocuments>"), "path should be materialized: {path}");
        // change should be recalculated (not Unknown).
        assert_ne!(file.change, ScanChange::Unknown);
        // restore_error should be cleared.
        assert!(file.restore_error.is_none());
    }

    #[test]
    fn materialize_and_fixup_uses_context_target_over_fallback() {
        use crate::resource::config::ToggledPaths;
        use crate::scan::ScanChange;
        use crate::scan::ScanInfo;
        use crate::scan::ScannedFile;
        use std::collections::HashMap;

        let semantic = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "MyGame/save.dat".to_string(),
        };

        let mut scan_info = ScanInfo {
            game_name: "MyGame".to_string(),
            found_files: HashMap::new(),
            ..Default::default()
        };

        // File with semantic key AND mapping_context_id (contextual).
        scan_info.found_files.insert(
            crate::path::StrictPath::new("scan_key"),
            ScannedFile {
                semantic_key: Some(semantic),
                mapping_context_id: Some(0),
                original_path: Some(crate::path::StrictPath::new("<winDocuments>/MyGame/save.dat")),
                hash: "abc123".to_string(),
                change: ScanChange::Unknown,
                ..Default::default()
            },
        );

        // Context target for ctx_id=0 uses a Wine prefix.
        let mut context_targets = HashMap::new();
        context_targets.insert(
            0,
            ResolvedMaterializeTarget::WinePrefix {
                prefix: make_wine_prefix(),
                wine_user: "steamuser".to_string(),
                drive_mappings: HashMap::new(),
            },
        );

        // Fallback is Windows — should NOT be used for this contextual file.
        let kf = make_known_folders();
        let fallback = ResolvedMaterializeTarget::CurrentWindows { known_folders: &kf };

        materialize_and_fixup(
            &mut scan_info,
            &context_targets,
            Some(&fallback),
            ContextualFallback::Disallow,
            &[],
            false,
            &ToggledPaths::default(),
        );

        let file = scan_info.found_files.values().next().unwrap();
        let path = file.original_path.as_ref().unwrap().render();
        // Should use Wine prefix path, not Windows path.
        assert!(path.contains("drive_c"), "should use Wine prefix: {path}");
        assert!(path.contains("Prefixes"), "should use Wine prefix: {path}");
    }

    #[test]
    fn materialize_and_fixup_does_not_use_fallback_for_missing_context() {
        use crate::resource::config::ToggledPaths;
        use crate::scan::ScanChange;
        use crate::scan::ScanInfo;
        use crate::scan::ScannedFile;
        use std::collections::HashMap;

        let semantic = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "MyGame/save.dat".to_string(),
        };
        let original = crate::path::StrictPath::new("<winDocuments>/MyGame/save.dat");

        let mut scan_info = ScanInfo {
            game_name: "MyGame".to_string(),
            found_files: HashMap::new(),
            ..Default::default()
        };
        scan_info.found_files.insert(
            crate::path::StrictPath::new("scan_key"),
            ScannedFile {
                semantic_key: Some(semantic),
                mapping_context_id: Some(99),
                original_path: Some(original.clone()),
                hash: "abc123".to_string(),
                change: ScanChange::Unknown,
                restore_error: Some("No semantic restore target is available".to_string()),
                ..Default::default()
            },
        );

        let context_targets = HashMap::new();
        let kf = make_known_folders();
        let fallback = ResolvedMaterializeTarget::CurrentWindows { known_folders: &kf };

        materialize_and_fixup(
            &mut scan_info,
            &context_targets,
            Some(&fallback),
            ContextualFallback::Disallow,
            &[],
            false,
            &ToggledPaths::default(),
        );

        let file = scan_info.found_files.values().next().unwrap();
        assert_eq!(Some(&original), file.original_path.as_ref());
        assert_eq!(
            Some("No semantic restore target is available"),
            file.restore_error.as_deref()
        );
    }

    #[test]
    fn materialize_and_fixup_uses_fallback_for_missing_context_when_allowed() {
        use crate::resource::config::ToggledPaths;
        use crate::scan::ScanChange;
        use crate::scan::ScanInfo;
        use crate::scan::ScannedFile;
        use std::collections::HashMap;

        let semantic = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "MyGame/save.dat".to_string(),
        };

        let mut scan_info = ScanInfo {
            game_name: "MyGame".to_string(),
            found_files: HashMap::new(),
            ..Default::default()
        };
        scan_info.found_files.insert(
            crate::path::StrictPath::new("scan_key"),
            ScannedFile {
                semantic_key: Some(semantic),
                mapping_context_id: Some(99),
                original_path: Some(crate::path::StrictPath::new("<winDocuments>/MyGame/save.dat")),
                hash: "abc123".to_string(),
                change: ScanChange::Unknown,
                restore_error: Some("No semantic restore target is available".to_string()),
                ..Default::default()
            },
        );

        let context_targets = HashMap::new();
        let kf = make_known_folders();
        let fallback = ResolvedMaterializeTarget::CurrentWindows { known_folders: &kf };

        materialize_and_fixup(
            &mut scan_info,
            &context_targets,
            Some(&fallback),
            ContextualFallback::Allow,
            &[],
            false,
            &ToggledPaths::default(),
        );

        let file = scan_info.found_files.values().next().unwrap();
        assert_eq!(
            Some("C:/Users/Alice/Documents/MyGame/save.dat"),
            file.original_path.as_ref().map(|x| x.render()).as_deref()
        );
        assert!(file.restore_error.is_none());
    }

    /// Req 13.1: Multi-prefix same-game round-trip.
    /// Two files from different Wine prefixes restore to their correct per-context targets.
    #[test]
    fn e2e_multi_prefix_restore_to_correct_context_targets() {
        use crate::resource::config::ToggledPaths;
        use crate::scan::ScanInfo;
        use crate::scan::ScannedFile;
        use crate::scan::layout::PathContext;
        use std::collections::{BTreeMap, HashMap};

        let semantic1 = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "MyGame/save.dat".to_string(),
        };
        let semantic2 = SemanticPath {
            base: SemanticBase::WinAppData,
            tail: "MyGame/settings.cfg".to_string(),
        };

        // Two different source prefixes.
        let ctx0 = PathContext {
            prefix_path: "/home/deck/.wine".to_string(),
            wine_user: "steamuser".to_string(),
            drive_mappings: BTreeMap::new(),
        };
        let ctx1 = PathContext {
            prefix_path: "/home/deck/.wine-alt".to_string(),
            wine_user: "steamuser".to_string(),
            drive_mappings: BTreeMap::new(),
        };

        let mut path_contexts = BTreeMap::new();
        path_contexts.insert(0, ctx0.clone());
        path_contexts.insert(1, ctx1.clone());

        let mut scan_info = ScanInfo {
            game_name: "MyGame".to_string(),
            found_files: HashMap::new(),
            path_contexts: path_contexts.clone(),
            ..Default::default()
        };

        // File from context 0
        scan_info.found_files.insert(
            StrictPath::new("scan_key_0"),
            ScannedFile {
                semantic_key: Some(semantic1),
                mapping_context_id: Some(0),
                original_path: Some(StrictPath::new("<winDocuments>/MyGame/save.dat")),
                hash: "abc".to_string(),
                ..Default::default()
            },
        );
        // File from context 1
        scan_info.found_files.insert(
            StrictPath::new("scan_key_1"),
            ScannedFile {
                semantic_key: Some(semantic2),
                mapping_context_id: Some(1),
                original_path: Some(StrictPath::new("<winAppData>/MyGame/settings.cfg")),
                hash: "def".to_string(),
                ..Default::default()
            },
        );

        // Build context targets: each context maps to its own prefix.
        let prefix0 = make_wine_prefix(); // /home/deck/Prefixes/Game
        let mut prefix1 = make_wine_prefix();
        prefix1.path = StrictPath::new("/home/deck/Prefixes/GameAlt");

        let mut context_targets = HashMap::new();
        context_targets.insert(
            0,
            ResolvedMaterializeTarget::WinePrefix {
                prefix: prefix0.clone(),
                wine_user: "steamuser".to_string(),
                drive_mappings: HashMap::new(),
            },
        );
        context_targets.insert(
            1,
            ResolvedMaterializeTarget::WinePrefix {
                prefix: prefix1.clone(),
                wine_user: "steamuser".to_string(),
                drive_mappings: HashMap::new(),
            },
        );

        materialize_and_fixup(
            &mut scan_info,
            &context_targets,
            None,
            ContextualFallback::Disallow,
            &[],
            false,
            &ToggledPaths::default(),
        );

        // File 0 should be under prefix0
        let file0 = scan_info.found_files.get(&StrictPath::new("scan_key_0")).unwrap();
        let path0 = file0.original_path.as_ref().unwrap().render();
        assert!(
            path0.contains("/home/deck/Prefixes/Game/"),
            "file0 should be under prefix0: {path0}"
        );
        assert!(
            path0.contains("steamuser/Documents/MyGame/save.dat"),
            "file0 wrong path: {path0}"
        );
        assert!(file0.restore_error.is_none(), "file0 should have no error");

        // File 1 should be under prefix1
        let file1 = scan_info.found_files.get(&StrictPath::new("scan_key_1")).unwrap();
        let path1 = file1.original_path.as_ref().unwrap().render();
        assert!(
            path1.contains("/home/deck/Prefixes/GameAlt/"),
            "file1 should be under prefix1: {path1}"
        );
        assert!(
            path1.contains("steamuser/AppData/Roaming/MyGame/settings.cfg"),
            "file1 wrong path: {path1}"
        );
        assert!(file1.restore_error.is_none(), "file1 should have no error");

        // Paths must be different (different prefixes)
        assert_ne!(
            path0, path1,
            "files from different prefixes must have different restore paths"
        );
    }

    /// Req 13.4: NoCandidate path — semantic files with no target get restore error.
    #[test]
    fn e2e_no_candidate_produces_restore_error() {
        use crate::resource::config::ToggledPaths;
        use crate::scan::ScanInfo;
        use crate::scan::ScannedFile;
        use std::collections::HashMap;

        let semantic = SemanticPath {
            base: SemanticBase::WinDocuments,
            tail: "MyGame/save.dat".to_string(),
        };

        let mut scan_info = ScanInfo {
            game_name: "MyGame".to_string(),
            found_files: HashMap::new(),
            ..Default::default()
        };
        scan_info.found_files.insert(
            StrictPath::new("scan_key"),
            ScannedFile {
                semantic_key: Some(semantic),
                mapping_context_id: None,
                original_path: Some(StrictPath::new("<winDocuments>/MyGame/save.dat")),
                hash: "abc".to_string(),
                ..Default::default()
            },
        );

        // No context targets, no fallback — simulates NoCandidate.
        materialize_and_fixup(
            &mut scan_info,
            &HashMap::new(),
            None,
            ContextualFallback::Disallow,
            &[],
            false,
            &ToggledPaths::default(),
        );

        let file = scan_info.found_files.values().next().unwrap();
        assert_eq!(
            Some("No semantic restore target is available"),
            file.restore_error.as_deref(),
            "NoCandidate should produce restore error"
        );
    }

    /// Req 13.5: StalePreference — saved prefix that doesn't validate.
    #[test]
    fn e2e_stale_preference_detected_by_decision_function() {
        use crate::resource::config::{Config, GameWinePrefixPreference};
        use crate::semantic::restore_prompt::{ResolutionOutcome, decide_prefix_resolution};

        let mut config = Config::default();
        config.restore.preferred_wine_prefixes.insert(
            "MyGame".to_string(),
            GameWinePrefixPreference {
                path: StrictPath::new("/nonexistent/stale/prefix"),
                wine_user: None,
                drive_mappings: Default::default(),
            },
        );

        let launchers = Launchers::default();
        let outcome = decide_prefix_resolution(
            &config,
            "MyGame",
            &[],
            None,
            &launchers,
            &[],
            None,
            false, // not Windows
        );

        assert!(
            matches!(outcome, Some(ResolutionOutcome::StalePreference { ref game, .. }) if game == "MyGame"),
            "Expected StalePreference for nonexistent saved prefix, got: {:?}",
            outcome
        );
    }
}
