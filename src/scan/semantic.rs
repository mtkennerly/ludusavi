use crate::{
    api::StrictPath,
    scan::layout::{BackupSemantics, SemanticDirKind},
};

pub use self::{convert::KnownFolders, discovery::WineEnvironment, prefix::Prefix};

mod convert;
mod discovery;
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
    /// Local prefixes this game could be restored into, most-specific first.
    /// Empty on Windows.
    pub prefixes: Vec<Prefix>,
    /// Current Windows known folders, only populated on Windows.
    pub known_folders: Option<KnownFolders>,
}

impl Wine {
    /// Build a context from the current game's config and system state.
    /// Returns None if redirect_wine is disabled or no usable context exists.
    pub fn for_game(game_name: &str, env: &WineEnvironment) -> Option<Self> {
        if !env.config.scan.redirect_wine {
            return None;
        }

        let prefixes = env.prefixes_for_game(game_name);

        // On Windows, populate known_folders so that Wine→Windows restore can
        // convert semantic paths to physical paths.
        let known_folders = KnownFolders::windows();

        // Return context if we have either a usable prefix or known folders.
        if !prefixes.is_empty() || known_folders.is_some() {
            Some(Self {
                prefixes,
                known_folders,
            })
        } else {
            None
        }
    }
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

    // Longest match wins: a merged full+diff semantics map can hold several prefixes, and
    // one may be nested inside another (e.g. a prefix and its own `pfx` subdirectory).
    let wine_match = semantics
        .directories
        .iter()
        .filter(|(dir, semantics)| stored_raw.starts_with(dir.as_str()) && semantics.kind == SemanticDirKind::Wine)
        .max_by_key(|(dir, _)| dir.len());

    if let Some((prefix_path, _)) = wine_match {
        // Linux/Wine backup → Windows restore: no local prefixes, known_folders is Some.
        if let Some(kf) = &context.known_folders
            && context.prefixes.is_empty()
        {
            let prefix_sp = StrictPath::new(prefix_path.clone());
            let wine_user = prefix::detect_wine_user_from_raw_path(stored_raw, prefix_path)?;
            let semantic = convert::wine_physical_to_semantic(stored_path, &prefix_sp, &wine_user)?;
            return materialize_to_windows(&semantic, kf);
        }

        // Wine backup → Wine restore (same or different prefix):
        // Use semantic conversion to handle username changes correctly.
        if !context.prefixes.is_empty() {
            let prefix_sp = StrictPath::new(prefix_path.clone());
            let wine_user = prefix::detect_wine_user_from_raw_path(stored_raw, prefix_path)?;
            // Decomposing depends only on the stored path and the backup's own prefix,
            // so it is done once and then materialized against each local candidate.
            if let Some(semantic) = convert::wine_physical_to_semantic(stored_path, &prefix_sp, &wine_user)
                && let Some(target) = pick_prefix(&semantic, &context.prefixes)
            {
                return Some(target);
            }
        }
    }

    // Windows backup → Linux/Wine restore: detect Windows special folders heuristically.
    // This handles the case where the stored path is a Windows path (e.g., C:/Users/...)
    // and we're restoring into a Wine prefix.
    if !context.prefixes.is_empty()
        && let Some(semantic) = convert::windows_physical_to_semantic(stored_path, &KnownFolders::default())
        && let Some(target) = pick_prefix(&semantic, &context.prefixes)
    {
        return Some(target);
    }

    None
}

/// Materialize a semantic path against the best of several candidate prefixes.
///
/// With one candidate this is just `materialize_to_wine`. With several — e.g. a game
/// owned on Steam that also has a non-Steam shortcut — prefer the prefix that already
/// holds this game's save directory, since that is where the user actually plays. Falls
/// back to the highest-ranked prefix that materializes at all, for a first-ever restore
/// onto a machine where the directory doesn't exist yet.
fn pick_prefix(semantic: &Path, prefixes: &[Prefix]) -> Option<StrictPath> {
    if let [only] = prefixes {
        return materialize_to_wine(semantic, only);
    }

    let mut fallback = None;
    for prefix in prefixes {
        let Some(target) = materialize_to_wine(semantic, prefix) else {
            continue;
        };
        if target.parent().is_some_and(|parent| parent.is_dir()) {
            return Some(target);
        }
        if fallback.is_none() {
            fallback = Some(target);
        }
    }
    fallback
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
