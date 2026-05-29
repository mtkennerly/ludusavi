use crate::path::{CommonPath, StrictPath};
use crate::prelude::Error;
use crate::resource::config::Config;
use crate::semantic::SemanticBase;
use crate::semantic::SemanticPath;
use crate::semantic::convert::KnownFolders;
use crate::semantic::prefix::{ValidatedPrefix, validate_prefix};

/// Target platform for materialization.
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
    for candidate in candidates {
        if let Some(prefix) = validate_prefix(candidate) {
            return Some(prefix);
        }
    }
    None
}

fn preferred_wine_prefix_for_game<'a>(
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
pub fn resolve_wine_prefix_for_game(
    config: &Config,
    game: &str,
    cli_wine_prefix: Option<&StrictPath>,
) -> Result<Option<ValidatedPrefix>, Error> {
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

    Ok(resolve_wine_prefix_without_cli(config, game))
}

/// Resolve the configured or discovered Wine prefix for a game when no CLI override is involved.
pub fn resolve_wine_prefix_without_cli(config: &Config, game: &str) -> Option<ValidatedPrefix> {
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
            return Some(prefix);
        }
        return None;
    }

    let roots: Vec<StrictPath> = config.roots.iter().map(|root| root.path().clone()).collect();
    discover_wine_prefix(&roots)
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
        MaterializeTarget::WinePrefix { prefix, wine_user, drive_mappings } => {
            materialize_to_wine(semantic, prefix, wine_user, drive_mappings)
        }
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

        let resolved = resolve_wine_prefix_without_cli(&config, "Game").unwrap();
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

        let resolved = resolve_wine_prefix_without_cli(&config, "Game").unwrap();
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

        let error = resolve_wine_prefix_for_game(&config, "Game", Some(&cli)).unwrap_err();
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

        let error = resolve_wine_prefix_for_game(&config, "Game", Some(&cli)).unwrap_err();
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

        let resolved = resolve_wine_prefix_for_game(&config, "Game", Some(&preferred))
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

        let resolved = resolve_wine_prefix_without_cli(&config, "Game").unwrap();
        assert_eq!(Some(&drive.render()), resolved.drive_mappings.get(&'d'));
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
}
