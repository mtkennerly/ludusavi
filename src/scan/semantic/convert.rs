use std::sync::LazyLock;

use regex::Regex;

use crate::{
    path::{CommonPath, StrictPath},
    scan::semantic,
};

/// Holds the physical paths of Windows known folders for semantic path derivation.
/// All paths should use `/` separators and not have trailing slashes.
#[derive(Clone, Debug, Default)]
pub struct KnownFolders {
    pub saved_games: Option<String>,
    pub documents: Option<String>,
    pub local_app_data: Option<String>,
    pub local_low_app_data: Option<String>,
    pub app_data: Option<String>,
    pub public: Option<String>,
    pub program_data: Option<String>,
    pub windows: Option<String>,
    pub user_profile: Option<String>,
}

impl KnownFolders {
    /// Populate Windows known folder paths.
    /// Returns None if the home directory cannot be determined.
    pub fn new() -> Option<Self> {
        fn common_path(path: CommonPath) -> Option<String> {
            path.get().map(|p| p.replace('\\', "/"))
        }

        let user_profile = common_path(CommonPath::Home)?;

        let program_data = std::env::var("ProgramData").ok().map(|p| p.replace('\\', "/"));
        let windows = std::env::var("SystemRoot").ok().map(|p| p.replace('\\', "/"));

        Some(Self {
            saved_games: common_path(CommonPath::SavedGames),
            documents: common_path(CommonPath::Document),
            local_app_data: common_path(CommonPath::DataLocal),
            local_low_app_data: common_path(CommonPath::DataLocalLow),
            app_data: common_path(CommonPath::Data),
            public: common_path(CommonPath::Public),
            program_data,
            windows,
            user_profile: Some(user_profile),
        })
    }
}

fn normalize_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_end_matches('/').to_string()
}

fn tail_for_known_folder(path: &StrictPath, prefix: &str) -> Option<String> {
    path.case_insensitive_tail_for(&StrictPath::new(prefix))
        .map(|tail| tail.join("/"))
}

/// Convert a physical Windows path to a semantic path for the current user.
/// Returns None if the path cannot be semantically classified.
pub fn windows_physical_to_semantic(physical: &StrictPath, known_folders: &KnownFolders) -> Option<semantic::Path> {
    let rendered = physical.render();

    if rendered.starts_with("//") || rendered.starts_with(r"\\") {
        return None;
    }

    if let Some(sg) = &known_folders.saved_games
        && let Some(tail) = tail_for_known_folder(physical, sg)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinSavedGames,
            tail,
        });
    }

    if let Some(docs) = &known_folders.documents
        && let Some(tail) = tail_for_known_folder(physical, docs)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinDocuments,
            tail,
        });
    }

    if let Some(local_low) = &known_folders.local_low_app_data
        && let Some(tail) = tail_for_known_folder(physical, local_low)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppDataLow,
            tail,
        });
    }

    if let Some(local) = &known_folders.local_app_data
        && let Some(tail) = tail_for_known_folder(physical, local)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppData,
            tail,
        });
    }

    if let Some(roaming) = &known_folders.app_data
        && let Some(tail) = tail_for_known_folder(physical, roaming)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinAppData,
            tail,
        });
    }

    if let Some(public) = &known_folders.public
        && let Some(tail) = tail_for_known_folder(physical, public)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinPublic,
            tail,
        });
    }

    if let Some(pd) = &known_folders.program_data
        && let Some(tail) = tail_for_known_folder(physical, pd)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinProgramData,
            tail,
        });
    }

    if let Some(win) = &known_folders.windows
        && let Some(tail) = tail_for_known_folder(physical, win)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinDir,
            tail,
        });
    }

    if let Some(home) = &known_folders.user_profile
        && let Some(tail) = tail_for_known_folder(physical, home)
    {
        return Some(semantic::Path {
            base: semantic::Base::WinHome,
            tail,
        });
    }

    static USER_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^C:/Users/[^/]+/(.+)$").unwrap());

    let normalized = normalize_path(&rendered);
    if let Some(captures) = USER_PATH.captures(&normalized)
        && let Some(tail) = captures.get(1)
    {
        return classify_windows_user_subpath(tail.as_str());
    }

    extract_drive_path(&rendered)
}

fn extract_drive_path(rendered: &str) -> Option<semantic::Path> {
    let norm = normalize_path(rendered);

    let bytes = norm.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        let drive_letter = (bytes[0] as char).to_ascii_lowercase();
        let tail = &norm[3..];
        if !tail.is_empty() {
            return Some(semantic::Path {
                base: semantic::Base::WinDrive(drive_letter),
                tail: tail.to_string(),
            });
        }
    }
    None
}

/// Convert a physical path inside a validated Wine prefix to a semantic path.
/// `prefix_path` is the validated prefix root (parent of drive_c).
/// `wine_user` is the detected Wine username for this prefix.
/// Uses the **lexical** prefix-relative path, NOT realpath.
pub fn wine_physical_to_semantic(
    physical: &StrictPath,
    prefix_path: &StrictPath,
    wine_user: &str,
) -> Option<semantic::Path> {
    let relative = physical.case_insensitive_tail_for(prefix_path)?.join("/");

    let user_prefix = format!("drive_c/users/{}", wine_user.to_ascii_lowercase());
    let relative_lower = relative.to_ascii_lowercase();

    if relative_lower.starts_with(&user_prefix) {
        let after_user = &relative[user_prefix.len()..];
        if after_user.is_empty() || !after_user.starts_with('/') {
            return None;
        }
        let sub_path = &after_user[1..];

        return classify_windows_user_subpath(sub_path);
    }

    let public_prefix = "drive_c/users/public";
    if relative_lower.starts_with(public_prefix) {
        let after = &relative[public_prefix.len()..];
        if after.is_empty() || !after.starts_with('/') {
            return None;
        }
        let tail = &after[1..];
        if !tail.is_empty() {
            return Some(semantic::Path {
                base: semantic::Base::WinPublic,
                tail: tail.to_string(),
            });
        }
        return None;
    }

    let pd_prefix = "drive_c/programdata";
    if relative_lower.starts_with(pd_prefix) {
        let after = &relative[pd_prefix.len()..];
        if after.is_empty() || !after.starts_with('/') {
            return None;
        }
        let tail = &after[1..];
        if !tail.is_empty() {
            return Some(semantic::Path {
                base: semantic::Base::WinProgramData,
                tail: tail.to_string(),
            });
        }
        return None;
    }

    let win_prefix = "drive_c/windows";
    if relative_lower.starts_with(win_prefix) {
        let after = &relative[win_prefix.len()..];
        if after.is_empty() || !after.starts_with('/') {
            return None;
        }
        let tail = &after[1..];
        if !tail.is_empty() {
            return Some(semantic::Path {
                base: semantic::Base::WinDir,
                tail: tail.to_string(),
            });
        }
        return None;
    }

    if let Some(after_drive) = relative_lower.strip_prefix("drive_")
        && after_drive.len() >= 2
        && after_drive.as_bytes()[0].is_ascii_alphabetic()
        && after_drive.as_bytes()[1] == b'/'
    {
        let letter = (after_drive.as_bytes()[0] as char).to_ascii_lowercase();
        if letter != 'c' {
            let tail = &relative[8..];
            if !tail.is_empty() {
                return Some(semantic::Path {
                    base: semantic::Base::WinDrive(letter),
                    tail: tail.to_string(),
                });
            }
        }
    }

    let dc_prefix = "drive_c";
    if relative_lower.starts_with(dc_prefix) {
        let after = &relative[dc_prefix.len()..];
        if let Some(tail) = after.strip_prefix('/')
            && !tail.is_empty()
        {
            return Some(semantic::Path {
                base: semantic::Base::WinDrive('c'),
                tail: tail.to_string(),
            });
        }
    }

    None
}

/// Classify a sub-path under the Wine user's profile directory.
fn classify_windows_user_subpath(sub_path: &str) -> Option<semantic::Path> {
    let lower = sub_path.to_ascii_lowercase();

    // Saved Games
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "saved games") {
        return Some(semantic::Path {
            base: semantic::Base::WinSavedGames,
            tail,
        });
    }

    // Documents / My Documents
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "my documents") {
        return Some(semantic::Path {
            base: semantic::Base::WinDocuments,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "documents") {
        return Some(semantic::Path {
            base: semantic::Base::WinDocuments,
            tail,
        });
    }

    // AppData/LocalLow, AppData/Local/Low, or Local Settings/Application Data/Low
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/locallow") {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppDataLow,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/local/low") {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppDataLow,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "local settings/application data/low") {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppDataLow,
            tail,
        });
    }

    // AppData/Local or Local Settings/Application Data
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/local") {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppData,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "local settings/application data") {
        return Some(semantic::Path {
            base: semantic::Base::WinLocalAppData,
            tail,
        });
    }

    // AppData/Roaming or Application Data
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/roaming") {
        return Some(semantic::Path {
            base: semantic::Base::WinAppData,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "application data") {
        return Some(semantic::Path {
            base: semantic::Base::WinAppData,
            tail,
        });
    }

    // Default: WinHome
    if !sub_path.is_empty() {
        return Some(semantic::Path {
            base: semantic::Base::WinHome,
            tail: sub_path.to_string(),
        });
    }

    None
}

/// Try to strip a known folder alias prefix from the sub-path.
/// `alias` should be lowercase. Returns the tail if the alias matches.
fn strip_known_folder_alias(lower: &str, original: &str, alias: &str) -> Option<String> {
    if lower == alias {
        // Exact match with no tail - not valid
        return None;
    }
    if lower.starts_with(alias) && lower.as_bytes().get(alias.len()) == Some(&b'/') {
        let tail_start = alias.len() + 1;
        if tail_start < original.len() {
            return Some(original[tail_start..].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_known_folders() -> KnownFolders {
        KnownFolders {
            saved_games: Some("C:/Users/Alice/Saved Games".to_string()),
            documents: Some("C:/Users/Alice/Documents".to_string()),
            local_app_data: Some("C:/Users/Alice/AppData/Local".to_string()),
            local_low_app_data: Some("C:/Users/Alice/AppData/LocalLow".to_string()),
            app_data: Some("C:/Users/Alice/AppData/Roaming".to_string()),
            public: Some("C:/Users/Public".to_string()),
            program_data: Some("C:/ProgramData".to_string()),
            windows: Some("C:/Windows".to_string()),
            user_profile: Some("C:/Users/Alice".to_string()),
        }
    }

    #[test]
    fn windows_documents() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/Documents/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_appdata_roaming() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/AppData/Roaming/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_local_appdata() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/AppData/Local/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinLocalAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_local_appdata_low() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/AppData/LocalLow/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinLocalAppDataLow);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_saved_games() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/Saved Games/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinSavedGames);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_relocated_documents() {
        let mut kf = make_known_folders();
        kf.documents = Some("D:/MyDocs".to_string());
        let path = StrictPath::new("D:/MyDocs/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_drive_root_classification() {
        let kf = make_known_folders();
        let path = StrictPath::new("D:/Games/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinDrive('d'));
        assert_eq!(result.tail, "Games/save.dat");
    }

    #[test]
    fn windows_home_not_swallowing_documents() {
        let kf = make_known_folders();
        // If Documents is at C:/Users/Alice/Documents, then C:/Users/Alice/MyGames
        // should map to WinHome, not WinDocuments.
        let path = StrictPath::new("C:/Users/Alice/MyGames/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinHome);
        assert_eq!(result.tail, "MyGames/save.dat");
    }

    #[test]
    fn windows_case_insensitive() {
        let kf = make_known_folders();
        let path = StrictPath::new("c:/users/alice/documents/game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "game/save.dat");
    }

    #[test]
    fn windows_rejects_unc() {
        let kf = make_known_folders();
        let path = StrictPath::new("//server/share/file.dat");
        assert!(windows_physical_to_semantic(&path, &kf).is_none());
    }

    #[test]
    fn windows_programdata() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/ProgramData/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinProgramData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_windir() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Windows/System32/config.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, semantic::Base::WinDir);
        assert_eq!(result.tail, "System32/config.dat");
    }

    // Wine conversion tests

    #[test]
    fn wine_documents() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Alan Wake");
        let physical = StrictPath::new(
            "/home/deck/Prefixes/Alan Wake/drive_c/users/steamuser/Documents/Remedy/Alan Wake/save.dat",
        );
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "Remedy/Alan Wake/save.dat");
    }

    #[test]
    fn wine_appdata_roaming() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/deck/AppData/Roaming/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "deck").unwrap();
        assert_eq!(result.base, semantic::Base::WinAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_xp_alias_application_data() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical =
            StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/Application Data/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_xp_alias_local_settings() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new(
            "/home/deck/Prefixes/Game/drive_c/users/steamuser/Local Settings/Application Data/Game/save.dat",
        );
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinLocalAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_xp_alias_my_documents() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/My Documents/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_programdata() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/ProgramData/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinProgramData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_drive_d() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_d/Games/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinDrive('d'));
        assert_eq!(result.tail, "Games/save.dat");
    }

    #[test]
    fn wine_case_insensitive() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/DRIVE_C/users/steamuser/documents/game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "game/save.dat");
    }

    #[test]
    fn wine_uses_lexical_path_not_realpath() {
        // Even if the Documents directory is a symlink to somewhere else,
        // we should use the lexical path for classification.
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, semantic::Base::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }
}
