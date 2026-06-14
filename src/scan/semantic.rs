use crate::{
    api::{Config, StrictPath},
    scan::layout::{BackupSemantics, SemanticDirKind},
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
    /// First valid `wine_prefix` from the matching custom game.
    pub preferred_prefix: Option<Prefix>,
    /// Current Windows known folders, only populated on Windows.
    pub known_folders: Option<KnownFolders>,
}

impl Wine {
    /// Build a context from the current game's config and system state.
    /// Returns None if redirect_wine is disabled or no usable context exists.
    pub fn for_game(game_name: &str, config: &Config) -> Option<Self> {
        if !config.scan.redirect_wine {
            return None;
        }

        // Find the first valid wine_prefix from a matching custom game.
        let preferred_prefix = config
            .custom_games
            .iter()
            .find(|cg| cg.name == game_name)
            .and_then(|cg| {
                cg.wine_prefix
                    .iter()
                    .filter(|wp| !wp.trim().is_empty())
                    .find_map(|wp| Prefix::validated(&StrictPath::new(wp)))
            });

        // On Windows, populate known_folders so that Wine→Windows restore can
        // convert semantic paths to physical paths.
        let known_folders = KnownFolders::windows();

        // Return context if we have either a usable prefix or known folders.
        if preferred_prefix.is_some() || known_folders.is_some() {
            Some(Self {
                preferred_prefix,
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

    let wine_match = semantics
        .directories
        .iter()
        .find(|(dir, semantics)| stored_raw.starts_with(dir.as_str()) && semantics.kind == SemanticDirKind::Wine);

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

        // Wine backup → Wine restore (same or different prefix):
        // Use semantic conversion to handle username changes correctly.
        if let Some(prefix) = &context.preferred_prefix {
            let prefix_sp = StrictPath::new(prefix_path.clone());
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
