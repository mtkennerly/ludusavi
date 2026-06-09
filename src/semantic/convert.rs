use crate::path::StrictPath;
use crate::resource::manifest::Store;
use crate::semantic::{SemanticBase, SemanticPath};

/// Holds the physical paths of Windows known folders for semantic path derivation.
/// All paths should use `/` separators and not have trailing slashes.
#[derive(Clone, Debug, Default)]
pub struct KnownFolders {
    pub saved_games: Option<String>,
    pub documents: Option<String>,
    pub local_app_data: Option<String>,
    pub app_data: Option<String>,
    pub public: Option<String>,
    pub program_data: Option<String>,
    pub windows: Option<String>,
    pub user_profile: Option<String>,
}

fn normalize_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_end_matches('/').to_string()
}

fn strip_prefix_case_insensitive(path: &str, prefix: &str) -> Option<String> {
    let path_norm = normalize_path(path);
    let prefix_norm = normalize_path(prefix);

    if path_norm.len() > prefix_norm.len()
        && path_norm.as_bytes()[prefix_norm.len()] == b'/'
        && path_norm[..prefix_norm.len()].eq_ignore_ascii_case(&prefix_norm)
    {
        let tail = &path_norm[prefix_norm.len() + 1..];
        if !tail.is_empty() {
            return Some(tail.to_string());
        }
    }
    None
}

/// Convert a physical Windows path to a semantic path for the current user.
/// Returns None if the path cannot be semantically classified.
pub fn windows_physical_to_semantic(physical: &StrictPath, known_folders: &KnownFolders) -> Option<SemanticPath> {
    let rendered = physical.render();

    // Reject UNC paths
    if rendered.starts_with("//") || rendered.starts_with(r"\\") {
        return None;
    }

    // Priority order per plan:
    // 1. Saved Games
    if let Some(ref sg) = known_folders.saved_games
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, sg)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinSavedGames,
            tail,
        });
    }

    // 2. Documents
    if let Some(ref docs) = known_folders.documents
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, docs)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinDocuments,
            tail,
        });
    }

    // 3. LocalAppData/Low (check before LocalAppData)
    if let Some(ref local) = known_folders.local_app_data {
        let low_path = format!("{}/Low", local);
        if let Some(tail) = strip_prefix_case_insensitive(&rendered, &low_path) {
            return Some(SemanticPath {
                base: SemanticBase::WinLocalAppDataLow,
                tail,
            });
        }
    }

    // 4. LocalAppData
    if let Some(ref local) = known_folders.local_app_data
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, local)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinLocalAppData,
            tail,
        });
    }

    // 5. AppData (Roaming)
    if let Some(ref roaming) = known_folders.app_data
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, roaming)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinAppData,
            tail,
        });
    }

    // 6. Public
    if let Some(ref public) = known_folders.public
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, public)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinPublic,
            tail,
        });
    }

    // 7. ProgramData
    if let Some(ref pd) = known_folders.program_data
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, pd)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinProgramData,
            tail,
        });
    }

    // 8. Windows directory
    if let Some(ref win) = known_folders.windows
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, win)
    {
        return Some(SemanticPath {
            base: SemanticBase::WinDir,
            tail,
        });
    }

    // 9. User profile home
    if let Some(ref home) = known_folders.user_profile
        && let Some(tail) = strip_prefix_case_insensitive(&rendered, home)
    {
        // Don't let broad user profile swallow paths that should have been
        // handled by more specific bases. Check that the first tail component
        // is not a known folder alias.
        let first_component = tail.split('/').next().unwrap_or("");
        let lower = first_component.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "documents"
                | "my documents"
                | "appdata"
                | "application data"
                | "local settings"
                | "saved games"
                | "desktop"
        ) {
            // Only swallow if the relevant KnownFolder check already proved
            // this directory is NOT that semantic location. Since we already
            // checked above and they didn't match, we should still NOT use
            // WinHome for these known aliases.
            // Fall through to drive-based classification.
        } else {
            return Some(SemanticPath {
                base: SemanticBase::WinHome,
                tail,
            });
        }
    }

    // 10. Drive-root classification
    extract_drive_path(&rendered)
}

fn extract_drive_path(rendered: &str) -> Option<SemanticPath> {
    let norm = normalize_path(rendered);

    // Match patterns like "C:/..." or "C:\..."
    let bytes = norm.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        let drive_letter = (bytes[0] as char).to_ascii_lowercase();
        let tail = &norm[3..]; // skip "C:/"
        if !tail.is_empty() {
            return Some(SemanticPath {
                base: SemanticBase::WinDrive(drive_letter),
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
) -> Option<SemanticPath> {
    let rendered = physical.render();
    let prefix_rendered = normalize_path(&prefix_path.render());
    let rendered_norm = normalize_path(&rendered);

    // Strip prefix to get prefix-relative path
    let relative = if rendered_norm.len() > prefix_rendered.len()
        && rendered_norm.as_bytes()[prefix_rendered.len()] == b'/'
        && rendered_norm[..prefix_rendered.len()].eq_ignore_ascii_case(&prefix_rendered)
    {
        &rendered_norm[prefix_rendered.len() + 1..]
    } else {
        return None;
    };

    // Check if path is under drive_c/users/<wine_user>/
    let user_prefix = format!("drive_c/users/{}", wine_user.to_ascii_lowercase());
    let relative_lower = relative.to_ascii_lowercase();

    if relative_lower.starts_with(&user_prefix) {
        let after_user = &relative[user_prefix.len()..];
        if after_user.is_empty() || !after_user.starts_with('/') {
            return None;
        }
        let sub_path = &after_user[1..]; // skip the '/'

        return classify_windows_user_subpath(sub_path);
    }

    // Check if path is under drive_c/users/Public
    let public_prefix = "drive_c/users/public";
    if relative_lower.starts_with(public_prefix) {
        let after = &relative[public_prefix.len()..];
        if after.is_empty() || !after.starts_with('/') {
            return None;
        }
        let tail = &after[1..];
        if !tail.is_empty() {
            return Some(SemanticPath {
                base: SemanticBase::WinPublic,
                tail: tail.to_string(),
            });
        }
        return None;
    }

    // Check if path is under drive_c/ProgramData
    let pd_prefix = "drive_c/programdata";
    if relative_lower.starts_with(pd_prefix) {
        let after = &relative[pd_prefix.len()..];
        if after.is_empty() || !after.starts_with('/') {
            return None;
        }
        let tail = &after[1..];
        if !tail.is_empty() {
            return Some(SemanticPath {
                base: SemanticBase::WinProgramData,
                tail: tail.to_string(),
            });
        }
        return None;
    }

    // Check if path is under drive_c/windows
    let win_prefix = "drive_c/windows";
    if relative_lower.starts_with(win_prefix) {
        let after = &relative[win_prefix.len()..];
        if after.is_empty() || !after.starts_with('/') {
            return None;
        }
        let tail = &after[1..];
        if !tail.is_empty() {
            return Some(SemanticPath {
                base: SemanticBase::WinDir,
                tail: tail.to_string(),
            });
        }
        return None;
    }

    // Check if path is under drive_<letter> (not drive_c)
    if let Some(after_drive) = relative_lower.strip_prefix("drive_")
        && after_drive.len() >= 2
        && after_drive.as_bytes()[0].is_ascii_alphabetic()
        && after_drive.as_bytes()[1] == b'/'
    {
        let letter = (after_drive.as_bytes()[0] as char).to_ascii_lowercase();
        if letter != 'c' {
            let tail = &relative[8..]; // skip "drive_x/"
            if !tail.is_empty() {
                return Some(SemanticPath {
                    base: SemanticBase::WinDrive(letter),
                    tail: tail.to_string(),
                });
            }
        }
    }

    // If path is under drive_c but not matched above, use WinDrive('c')
    let dc_prefix = "drive_c";
    if relative_lower.starts_with(dc_prefix) {
        let after = &relative[dc_prefix.len()..];
        if let Some(tail) = after.strip_prefix('/')
            && !tail.is_empty()
        {
            return Some(SemanticPath {
                base: SemanticBase::WinDrive('c'),
                tail: tail.to_string(),
            });
        }
    }

    None
}

/// Classify a sub-path under the Wine user's profile directory.
fn classify_windows_user_subpath(sub_path: &str) -> Option<SemanticPath> {
    let lower = sub_path.to_ascii_lowercase();

    // Saved Games
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "saved games") {
        return Some(SemanticPath {
            base: SemanticBase::WinSavedGames,
            tail,
        });
    }

    // Documents / My Documents
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "my documents") {
        return Some(SemanticPath {
            base: SemanticBase::WinDocuments,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "documents") {
        return Some(SemanticPath {
            base: SemanticBase::WinDocuments,
            tail,
        });
    }

    // AppData/LocalLow, AppData/Local/Low, or Local Settings/Application Data/Low
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/locallow") {
        return Some(SemanticPath {
            base: SemanticBase::WinLocalAppDataLow,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/local/low") {
        return Some(SemanticPath {
            base: SemanticBase::WinLocalAppDataLow,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "local settings/application data/low") {
        return Some(SemanticPath {
            base: SemanticBase::WinLocalAppDataLow,
            tail,
        });
    }

    // AppData/Local or Local Settings/Application Data
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/local") {
        return Some(SemanticPath {
            base: SemanticBase::WinLocalAppData,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "local settings/application data") {
        return Some(SemanticPath {
            base: SemanticBase::WinLocalAppData,
            tail,
        });
    }

    // AppData/Roaming or Application Data
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "appdata/roaming") {
        return Some(SemanticPath {
            base: SemanticBase::WinAppData,
            tail,
        });
    }
    if let Some(tail) = strip_known_folder_alias(&lower, sub_path, "application data") {
        return Some(SemanticPath {
            base: SemanticBase::WinAppData,
            tail,
        });
    }

    // Default: WinHome
    if !sub_path.is_empty() {
        return Some(SemanticPath {
            base: SemanticBase::WinHome,
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

/// Origin metadata for a scanned file, used for manifest-based semantic key derivation.
#[derive(Clone, Debug, Default)]
pub struct ScanOrigin {
    pub manifest_path: String,
    pub store: crate::resource::manifest::Store,
    pub expanded_prefix: String,
    pub matched_prefix_len: usize,
    pub tail: String,
}

/// Mapping from manifest placeholder strings to semantic bases.
const PLACEHOLDER_TO_BASE: &[(&str, SemanticBase)] = &[
    ("<winDocuments>", SemanticBase::WinDocuments),
    ("<winAppData>", SemanticBase::WinAppData),
    ("<winLocalAppData>", SemanticBase::WinLocalAppData),
    ("<winLocalAppDataLow>", SemanticBase::WinLocalAppDataLow),
    ("<winSavedGames>", SemanticBase::WinSavedGames),
    ("<winPublic>", SemanticBase::WinPublic),
    ("<winProgramData>", SemanticBase::WinProgramData),
    ("<winDir>", SemanticBase::WinDir),
    ("<winHome>", SemanticBase::WinHome),
];

/// Determine the expected semantic base from a manifest path.
/// Returns None if the manifest path does not use a known semantic placeholder.
pub fn expected_base_from_manifest(manifest_path: &str, _store: Store) -> Option<SemanticBase> {
    for (placeholder, base) in PLACEHOLDER_TO_BASE {
        if manifest_path.starts_with(placeholder) {
            return Some(base.clone());
        }
    }
    None
}

/// Derive a semantic key from manifest origin metadata.
/// Returns None if the origin does not support semantic derivation.
///
/// Source precedence: manifest-derived keys are called FIRST.
/// Only if this returns None should the caller invoke reverse mapping.
pub fn derive_from_manifest_origin(origin: &ScanOrigin) -> Option<SemanticPath> {
    let manifest_path = &origin.manifest_path;
    let tail = &origin.tail;

    // Check if the manifest placeholder maps to a recognized semantic base
    for (placeholder, base) in PLACEHOLDER_TO_BASE {
        if manifest_path.starts_with(placeholder) {
            if tail.is_empty() {
                return None;
            }
            return Some(SemanticPath {
                base: base.clone(),
                tail: tail.clone(),
            });
        }
    }

    // Generic <base> or <root> with non-portable root → None (fall through to reverse mapping)
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::manifest::Store;

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

    #[test]
    fn windows_documents() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/Documents/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_appdata_roaming() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/AppData/Roaming/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_local_appdata() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/AppData/Local/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinLocalAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_local_appdata_low() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/AppData/Local/Low/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinLocalAppDataLow);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_saved_games() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Users/Alice/Saved Games/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinSavedGames);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_relocated_documents() {
        let mut kf = make_known_folders();
        kf.documents = Some("D:/MyDocs".to_string());
        let path = StrictPath::new("D:/MyDocs/Game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_drive_root_classification() {
        let kf = make_known_folders();
        let path = StrictPath::new("D:/Games/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinDrive('d'));
        assert_eq!(result.tail, "Games/save.dat");
    }

    #[test]
    fn windows_home_not_swallowing_documents() {
        let kf = make_known_folders();
        // If Documents is at C:/Users/Alice/Documents, then C:/Users/Alice/MyGames
        // should map to WinHome, not WinDocuments.
        let path = StrictPath::new("C:/Users/Alice/MyGames/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinHome);
        assert_eq!(result.tail, "MyGames/save.dat");
    }

    #[test]
    fn windows_case_insensitive() {
        let kf = make_known_folders();
        let path = StrictPath::new("c:/users/alice/documents/game/save.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
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
        assert_eq!(result.base, SemanticBase::WinProgramData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn windows_windir() {
        let kf = make_known_folders();
        let path = StrictPath::new("C:/Windows/System32/config.dat");
        let result = windows_physical_to_semantic(&path, &kf).unwrap();
        assert_eq!(result.base, SemanticBase::WinDir);
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
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "Remedy/Alan Wake/save.dat");
    }

    #[test]
    fn wine_appdata_roaming() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/deck/AppData/Roaming/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "deck").unwrap();
        assert_eq!(result.base, SemanticBase::WinAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_xp_alias_application_data() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical =
            StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/Application Data/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_xp_alias_local_settings() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new(
            "/home/deck/Prefixes/Game/drive_c/users/steamuser/Local Settings/Application Data/Game/save.dat",
        );
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinLocalAppData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_xp_alias_my_documents() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/My Documents/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_programdata() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/ProgramData/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinProgramData);
        assert_eq!(result.tail, "Game/save.dat");
    }

    #[test]
    fn wine_drive_d() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_d/Games/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinDrive('d'));
        assert_eq!(result.tail, "Games/save.dat");
    }

    #[test]
    fn wine_case_insensitive() {
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/DRIVE_C/users/steamuser/documents/game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "game/save.dat");
    }

    #[test]
    fn wine_uses_lexical_path_not_realpath() {
        // Even if the Documents directory is a symlink to somewhere else,
        // we should use the lexical path for classification.
        let prefix = StrictPath::new("/home/deck/Prefixes/Game");
        let physical = StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat");
        let result = wine_physical_to_semantic(&physical, &prefix, "steamuser").unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "Game/save.dat");
    }

    // Manifest derivation tests

    #[test]
    fn manifest_derive_win_documents() {
        let origin = ScanOrigin {
            manifest_path: "<winDocuments>/Remedy/Alan Wake".to_string(),
            store: Store::Other,
            expanded_prefix: "C:/Users/Alice/Documents".to_string(),
            matched_prefix_len: "C:/Users/Alice/Documents".len(),
            tail: "Remedy/Alan Wake/save.dat".to_string(),
        };
        let result = derive_from_manifest_origin(&origin).unwrap();
        assert_eq!(result.base, SemanticBase::WinDocuments);
        assert_eq!(result.tail, "Remedy/Alan Wake/save.dat");
    }

    #[test]
    fn manifest_derive_win_appdata() {
        let origin = ScanOrigin {
            manifest_path: "<winAppData>/Game".to_string(),
            store: Store::Other,
            expanded_prefix: "C:/Users/Alice/AppData/Roaming".to_string(),
            matched_prefix_len: "C:/Users/Alice/AppData/Roaming".len(),
            tail: "Game/config.ini".to_string(),
        };
        let result = derive_from_manifest_origin(&origin).unwrap();
        assert_eq!(result.base, SemanticBase::WinAppData);
        assert_eq!(result.tail, "Game/config.ini");
    }

    #[test]
    fn manifest_derive_steam_userdata_falls_back_to_legacy() {
        // Steam userdata is a native Windows/Linux concern, not Windows/Wine,
        // so it is intentionally out of scope and falls back to legacy
        // absolute-path behavior (None here).
        let origin = ScanOrigin {
            manifest_path: "<root>/userdata/<storeUserId>/<storeGameId>/remote".to_string(),
            store: Store::Steam,
            expanded_prefix: "C:/Program Files (x86)/Steam".to_string(),
            matched_prefix_len: "C:/Program Files (x86)/Steam".len(),
            tail: "userdata/12345/67890/remote/save.dat".to_string(),
        };
        assert!(derive_from_manifest_origin(&origin).is_none());
    }

    #[test]
    fn manifest_derive_generic_base_returns_none() {
        let origin = ScanOrigin {
            manifest_path: "<base>/saves".to_string(),
            store: Store::Other,
            expanded_prefix: "/home/user/game".to_string(),
            matched_prefix_len: "/home/user/game".len(),
            tail: "saves/file.dat".to_string(),
        };
        assert!(derive_from_manifest_origin(&origin).is_none());
    }

    #[test]
    fn manifest_derive_empty_tail_returns_none() {
        let origin = ScanOrigin {
            manifest_path: "<winDocuments>/Game".to_string(),
            store: Store::Other,
            expanded_prefix: "C:/Users/Alice/Documents".to_string(),
            matched_prefix_len: "C:/Users/Alice/Documents".len(),
            tail: "".to_string(),
        };
        assert!(derive_from_manifest_origin(&origin).is_none());
    }

    #[test]
    fn manifest_derive_all_win_bases() {
        let cases = [
            ("<winDocuments>", SemanticBase::WinDocuments),
            ("<winAppData>", SemanticBase::WinAppData),
            ("<winLocalAppData>", SemanticBase::WinLocalAppData),
            ("<winLocalAppDataLow>", SemanticBase::WinLocalAppDataLow),
            ("<winSavedGames>", SemanticBase::WinSavedGames),
            ("<winPublic>", SemanticBase::WinPublic),
            ("<winProgramData>", SemanticBase::WinProgramData),
            ("<winDir>", SemanticBase::WinDir),
            ("<winHome>", SemanticBase::WinHome),
        ];
        for (placeholder, expected_base) in cases {
            let origin = ScanOrigin {
                manifest_path: format!("{}/Game", placeholder),
                store: Store::Other,
                expanded_prefix: "some/prefix".to_string(),
                matched_prefix_len: 11,
                tail: "Game/save.dat".to_string(),
            };
            let result = derive_from_manifest_origin(&origin).unwrap();
            assert_eq!(result.base, expected_base, "failed for: {}", placeholder);
        }
    }
}
