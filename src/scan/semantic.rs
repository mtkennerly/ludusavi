use std::collections::HashMap;

use crate::{
    api::{Config, StrictPath},
    resource::{
        config::Root,
        manifest::{Manifest, Store},
    },
    scan::{
        Launchers, SteamShortcuts, TitleFinder,
        layout::{BackupSemantics, SemanticDirKind},
    },
};

pub use self::{convert::KnownFolders, prefix::Prefix};

mod convert;
mod prefix;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Base {
    WinHome,
    WinDocuments,
    WinAppData,
    WinLocalAppData,
    WinLocalAppDataLow,
    WinSavedGames,
    WinPublic,
    WinProgramData,
    WinDir,
    WinDrive(char),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
    pub base: Base,
    pub tail: String,
}

/// Context for generating Wine ↔ Windows redirects at restore time.
pub struct Wine {
    /// First valid `wine_prefix` from the matching custom game,
    /// or else the first valid prefix detected from a launcher or Steam.
    pub preferred_prefix: Option<Prefix>,
    /// Whether `preferred_prefix` was detected, rather than configured by the user.
    /// A detected prefix does not override the original one while that still exists,
    /// but an explicitly configured prefix always does.
    pub detected: bool,
    /// Current Windows known folders, only populated on Windows.
    pub known_folders: Option<KnownFolders>,
}

impl Wine {
    /// Build a context from the current game's config and system state.
    /// Returns None if redirect_wine is disabled or no usable context exists.
    fn for_game(game_name: &str, config: &Config, detected_prefixes: &[StrictPath]) -> Option<Self> {
        if !config.scan.redirect_wine {
            return None;
        }

        // Find the first valid wine_prefix from a matching custom game,
        // or else from the locally detected prefixes.
        let configured_prefix = config
            .custom_games
            .iter()
            .find(|cg| cg.name == game_name)
            .and_then(|cg| {
                cg.wine_prefix
                    .iter()
                    .filter(|wp| !wp.trim().is_empty())
                    .find_map(|wp| Prefix::validated(&StrictPath::new(wp)))
            });
        let detected = configured_prefix.is_none();
        let preferred_prefix = configured_prefix.or_else(|| detected_prefixes.iter().find_map(Prefix::validated));

        // On Windows, populate known_folders so that Wine→Windows restore can
        // convert semantic paths to physical paths.
        let known_folders = KnownFolders::windows();

        // Return context if we have either a usable prefix or known folders.
        if preferred_prefix.is_some() || known_folders.is_some() {
            Some(Self {
                preferred_prefix,
                detected,
                known_folders,
            })
        } else {
            None
        }
    }
}

/// Paths that may be a game's local Wine prefix, found via launchers and Steam.
/// They are not checked here: `Wine::for_game` validates them and uses the first valid one
/// when a game has no custom Wine prefix.
#[derive(Clone, Debug, Default)]
pub struct PrefixCandidates {
    prefixes: HashMap<String, Vec<StrictPath>>,
}

impl PrefixCandidates {
    pub fn new(
        config: &Config,
        manifest: &Manifest,
        roots: &[Root],
        launchers: &Launchers,
        steam_shortcuts: &SteamShortcuts,
        games: &[String],
    ) -> Self {
        let mut prefixes = HashMap::<String, Vec<StrictPath>>::new();

        if !config.scan.redirect_wine {
            return Self { prefixes };
        }

        for game in games {
            for root in roots {
                for prefix in launchers.get_game(root, game).filter_map(|x| x.prefix.as_ref()) {
                    prefixes
                        .entry(game.clone())
                        .or_default()
                        .extend(launcher_prefix_candidates(prefix));
                }
            }

            let shortcut = steam_shortcuts.get(game).map(|x| x.id);
            let ids: Vec<_> = match manifest.0.get(game) {
                Some(x) => x.all_ids().steam(shortcut).collect(),
                None => shortcut.into_iter().collect(),
            };
            for root in roots.iter().filter(|x| x.store() == Store::Steam) {
                for id in &ids {
                    prefixes
                        .entry(game.clone())
                        .or_default()
                        .push(root.path().joined(format!("steamapps/compatdata/{id}/pfx")));
                }
            }
        }

        Self { prefixes }
    }

    pub fn scan(config: &Config, manifest: &Manifest, title_finder: &TitleFinder, games: &[String]) -> Self {
        if !config.scan.redirect_wine {
            return Self::default();
        }

        let roots = config.expanded_roots();
        let launchers = Launchers::scan(&roots, manifest, games, title_finder, None);
        let steam_shortcuts = SteamShortcuts::scan(title_finder);
        Self::new(config, manifest, &roots, &launchers, &steam_shortcuts, games)
    }

    pub fn for_game(&self, game_name: &str, config: &Config) -> Option<Wine> {
        let detected = self.prefixes.get(game_name).map(Vec::as_slice).unwrap_or_default();
        Wine::for_game(game_name, config, detected)
    }
}

/// A launcher's prefix may hold the real prefix in `pfx`, so that comes first.
/// But if `pfx` links back to the prefix (e.g., with umu), then found files are recorded by
/// their real path, so only the prefix itself is a candidate.
fn launcher_prefix_candidates(prefix: &StrictPath) -> Vec<StrictPath> {
    let mut candidates = vec![];
    if !std::path::Path::new(prefix.raw()).join("pfx").is_symlink() {
        candidates.push(prefix.joined("pfx"));
    }
    candidates.push(prefix.clone());
    candidates
}

/// Generate a redirect for restoring a file from a backup with Wine semantics.
///
/// Linux/Wine backup → Windows restore: convert Wine path to Windows known-folder path.
/// Windows backup → Linux/Wine restore: convert Windows path to Wine prefix path.
pub fn generate_restore_redirect(
    stored_path: &StrictPath,
    semantics: &BackupSemantics,
    context: &Wine,
) -> Option<StrictPath> {
    let stored_raw = stored_path.raw();

    // Prefer the most specific prefix, since one may be nested in another (e.g., `<prefix>/pfx`).
    let wine_match = semantics
        .directories
        .iter()
        .filter(|(dir, semantics)| {
            semantics.kind == SemanticDirKind::Wine
                && stored_path
                    .case_insensitive_tail_for(&StrictPath::new((*dir).clone()))
                    .is_some()
        })
        .max_by_key(|(dir, _)| dir.len());

    if let Some((prefix_path, _)) = wine_match {
        // Linux/Wine backup → Windows restore: preferred_prefix is None, known_folders is Some.
        if let Some(kf) = &context.known_folders
            && context.preferred_prefix.is_none()
        {
            let prefix_sp = StrictPath::new(prefix_path.clone());
            let wine_user = prefix::detect_wine_user_from_raw_path(stored_raw, prefix_path)?;
            let semantic = convert::wine_physical_to_semantic(stored_path, &prefix_sp, &wine_user)?;
            return materialize_to_windows(&semantic, kf);
        }

        // Wine backup → Wine restore (different prefix):
        // Use semantic conversion to handle username changes correctly.
        if let Some(prefix) = &context.preferred_prefix {
            let prefix_sp = StrictPath::new(prefix_path.clone());
            // The original prefix still exists, so keep the original path,
            // unless the user explicitly configured a different prefix for this game.
            if context.detected && Prefix::validated(&prefix_sp).is_some() {
                return None;
            }
            let wine_user = prefix::detect_wine_user_from_raw_path(stored_raw, prefix_path)?;
            if let Some(semantic) = convert::wine_physical_to_semantic(stored_path, &prefix_sp, &wine_user)
                .and_then(|s| materialize_to_wine(&s, prefix))
            {
                return Some(semantic);
            }
        }
    }

    // Windows backup → Linux/Wine restore: detect Windows special folders heuristically.
    // This handles the case where the stored path is a Windows path (e.g., C:/Users/...)
    // and we're restoring into a Wine prefix.
    if let Some(prefix) = &context.preferred_prefix
        && let Some(semantic) = convert::windows_physical_to_semantic(stored_path, &KnownFolders::default())
        && let Some(target) = materialize_to_wine(&semantic, prefix)
    {
        return Some(target);
    }

    None
}

/// Materialize a semantic path to a Windows physical path using known folders.
fn materialize_to_windows(semantic: &Path, known_folders: &KnownFolders) -> Option<StrictPath> {
    let base_path = match &semantic.base {
        Base::WinHome => known_folders.user_profile.as_deref()?,
        Base::WinDocuments => known_folders.documents.as_deref()?,
        Base::WinAppData => known_folders.app_data.as_deref()?,
        Base::WinLocalAppData => known_folders.local_app_data.as_deref()?,
        Base::WinLocalAppDataLow => known_folders.local_low_app_data.as_deref()?,
        Base::WinSavedGames => known_folders.saved_games.as_deref()?,
        Base::WinPublic => known_folders.public.as_deref()?,
        Base::WinProgramData => known_folders.program_data.as_deref()?,
        Base::WinDir => known_folders.windows.as_deref()?,
        Base::WinDrive(_) => return None,
    };

    let path = format!("{}/{}", base_path.trim_end_matches('/'), semantic.tail);
    Some(StrictPath::new(path))
}

/// Materialize a semantic path into a Wine prefix path.
/// Maps semantic bases to their Wine directory equivalents under `drive_c/`.
fn materialize_to_wine(semantic: &Path, prefix: &Prefix) -> Option<StrictPath> {
    let base_path = match &semantic.base {
        Base::WinDocuments => format!("drive_c/users/{}/Documents", prefix.wine_user),
        Base::WinAppData => format!("drive_c/users/{}/AppData/Roaming", prefix.wine_user),
        Base::WinLocalAppData => format!("drive_c/users/{}/AppData/Local", prefix.wine_user),
        Base::WinLocalAppDataLow => format!("drive_c/users/{}/AppData/LocalLow", prefix.wine_user),
        Base::WinSavedGames => format!("drive_c/users/{}/Saved Games", prefix.wine_user),
        Base::WinPublic => "drive_c/users/Public".to_string(),
        Base::WinProgramData => "drive_c/ProgramData".to_string(),
        Base::WinDir => "drive_c/Windows".to_string(),
        Base::WinHome => format!("drive_c/users/{}", prefix.wine_user),
        Base::WinDrive(c) => {
            let drive = prefix.path.joined(format!("drive_{c}"));
            if *c != 'c' && !drive.is_dir() {
                return None;
            }
            format!("drive_{}", c)
        }
    };

    let path = format!(
        "{}/{}/{}",
        prefix.path.raw().trim_end_matches('/'),
        base_path,
        semantic.tail
    );
    Some(StrictPath::new(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prefix(path: &std::path::Path, user: &str) {
        std::fs::create_dir_all(path.join("drive_c/users").join(user)).unwrap();
        std::fs::File::create(path.join("system.reg")).unwrap();
    }

    fn config() -> Config {
        let mut config = Config::default();
        config.scan.redirect_wine = true;
        config
    }

    fn env(game: &str, prefix: &StrictPath) -> PrefixCandidates {
        PrefixCandidates {
            prefixes: HashMap::from([(game.to_string(), vec![prefix.joined("pfx"), prefix.clone()])]),
        }
    }

    #[test]
    fn detected_prefix_prefers_pfx() {
        let tmp = tempfile::tempdir().unwrap();
        make_prefix(tmp.path(), "deck");
        make_prefix(&tmp.path().join("pfx"), "steamuser");
        let prefix = StrictPath::from(tmp.path());

        let wine = env("game", &prefix).for_game("game", &config()).unwrap();
        let preferred = wine.preferred_prefix.unwrap();
        assert_eq!(prefix.joined("pfx"), preferred.path);
        assert_eq!("steamuser", preferred.wine_user);

        assert!(env("game", &prefix).for_game("game", &Config::default()).is_none());
        // On Windows, the context still exists for its known folders, but without a prefix.
        let other = env("game", &prefix).for_game("other", &config());
        assert!(other.is_none_or(|x| x.preferred_prefix.is_none()));
    }

    #[test]
    fn restores_windows_path_into_detected_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        make_prefix(&tmp.path().join("pfx"), "steamuser");
        let prefix = StrictPath::from(tmp.path());
        let wine = env("game", &prefix).for_game("game", &config()).unwrap();

        assert_eq!(
            Some(StrictPath::new(format!(
                "{}/drive_c/users/steamuser/AppData/Roaming/game/save.dat",
                prefix.joined("pfx").raw()
            ))),
            generate_restore_redirect(
                &StrictPath::new("C:/Users/Alice/AppData/Roaming/game/save.dat"),
                &BackupSemantics::default(),
                &wine,
            )
        );
    }

    #[test]
    fn keeps_original_path_when_original_prefix_exists() {
        let tmp = tempfile::tempdir().unwrap();
        make_prefix(tmp.path(), "deck");
        make_prefix(&tmp.path().join("pfx"), "steamuser");
        let prefix = StrictPath::from(tmp.path());
        let wine = env("game", &prefix).for_game("game", &config()).unwrap();

        let semantics = BackupSemantics {
            directories: [(
                prefix.render(),
                crate::scan::layout::DirectorySemantics {
                    kind: SemanticDirKind::Wine,
                },
            )]
            .into(),
        };
        assert_eq!(
            None,
            generate_restore_redirect(
                &StrictPath::new(format!(
                    "{}/drive_c/users/deck/AppData/Roaming/game/save.dat",
                    prefix.render()
                )),
                &semantics,
                &wine,
            )
        );
    }

    #[test]
    fn configured_prefix_overrides_original_prefix() {
        let original = tempfile::tempdir().unwrap();
        make_prefix(original.path(), "deck");
        let original = StrictPath::from(original.path());
        let configured = tempfile::tempdir().unwrap();
        make_prefix(configured.path(), "steamuser");
        let configured = StrictPath::from(configured.path());

        let mut config = config();
        config.custom_games.push(crate::resource::config::CustomGame {
            name: "game".to_string(),
            wine_prefix: vec![configured.render()],
            ..Default::default()
        });
        let wine = PrefixCandidates::default().for_game("game", &config).unwrap();
        assert!(!wine.detected);

        let semantics = BackupSemantics {
            directories: [(
                original.render(),
                crate::scan::layout::DirectorySemantics {
                    kind: SemanticDirKind::Wine,
                },
            )]
            .into(),
        };
        assert_eq!(
            Some(StrictPath::new(format!(
                "{}/drive_c/users/steamuser/AppData/Roaming/game/save.dat",
                configured.render()
            ))),
            generate_restore_redirect(
                &StrictPath::new(format!(
                    "{}/drive_c/users/deck/AppData/Roaming/game/save.dat",
                    original.render()
                )),
                &semantics,
                &wine,
            )
        );
    }

    #[test]
    fn launcher_prefix_tries_pfx_first() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = StrictPath::from(tmp.path());

        assert_eq!(
            vec![prefix.joined("pfx"), prefix.clone()],
            launcher_prefix_candidates(&prefix)
        );
    }

    #[cfg(unix)]
    #[test]
    fn launcher_prefix_skips_pfx_that_links_back() {
        let tmp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join("pfx")).unwrap();
        let prefix = StrictPath::from(tmp.path());

        assert_eq!(vec![prefix.clone()], launcher_prefix_candidates(&prefix));
    }

    #[test]
    fn restores_into_local_steam_shortcut_prefix() {
        use crate::scan::steam::SteamShortcut;

        let tmp = tempfile::tempdir().unwrap();
        make_prefix(&tmp.path().join("steamapps/compatdata/4000000001/pfx"), "steamuser");
        let steam = StrictPath::from(tmp.path());
        let prefix_candidates = PrefixCandidates::new(
            &config(),
            &Manifest::default(),
            &[Root::new(steam.clone(), Store::Steam)],
            &Launchers::default(),
            &SteamShortcuts::with(
                "game",
                SteamShortcut {
                    id: 4000000001,
                    start_dir: None,
                },
            ),
            &["game".to_string()],
        );
        let wine = prefix_candidates.for_game("game", &config()).unwrap();

        // Backed up on another machine, where the shortcut had a different ID.
        let source = "/home/deck/.local/share/Steam/steamapps/compatdata/4000000002/pfx";
        let semantics = BackupSemantics {
            directories: [(
                source.to_string(),
                crate::scan::layout::DirectorySemantics {
                    kind: SemanticDirKind::Wine,
                },
            )]
            .into(),
        };
        assert_eq!(
            Some(
                steam.joined(
                    "steamapps/compatdata/4000000001/pfx/drive_c/users/steamuser/AppData/Roaming/game/save.dat"
                )
            ),
            generate_restore_redirect(
                &StrictPath::new(format!(
                    "{source}/drive_c/users/steamuser/AppData/Roaming/game/save.dat"
                )),
                &semantics,
                &wine,
            )
        );
    }
}
