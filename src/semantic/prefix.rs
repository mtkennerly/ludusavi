use std::collections::HashMap;

use crate::path::StrictPath;

/// System user directories that should be excluded from Wine user detection.
const SYSTEM_USERS: &[&str] = &["public", "default", "default user", "all users"];

/// A validated Wine prefix with detected metadata.
#[derive(Clone, Debug)]
pub struct ValidatedPrefix {
    pub path: StrictPath,
    pub wine_user: String,
    pub has_drive_c: bool,
    /// Drive letter mappings from dosdevices (lowercase letter → target path).
    pub drive_mappings: HashMap<char, String>,
}

/// Validate a candidate prefix path.
/// Returns None if validation fails.
///
/// Validation rules:
/// 1. `candidate/drive_c` must exist as a directory.
/// 2. At least one of `candidate/system.reg`, `candidate/user.reg`, or
///    `candidate/dosdevices` must exist.
/// 3. `candidate/drive_c/users` must exist as a directory.
pub fn validate_prefix(candidate: &StrictPath) -> Option<ValidatedPrefix> {
    let candidate_rendered = candidate.render();

    // 1. Check drive_c exists
    let drive_c = format!("{}/drive_c", candidate_rendered);
    let drive_c_path = StrictPath::new(&drive_c);
    if !drive_c_path.is_dir() {
        return None;
    }

    // 2. Check for Wine state markers
    let system_reg = format!("{}/system.reg", candidate_rendered);
    let user_reg = format!("{}/user.reg", candidate_rendered);
    let dosdevices = format!("{}/dosdevices", candidate_rendered);

    let has_marker = StrictPath::new(&system_reg).exists()
        || StrictPath::new(&user_reg).exists()
        || StrictPath::new(&dosdevices).is_dir();

    if !has_marker {
        return None;
    }

    // 3. Check drive_c/users exists
    let users_dir = format!("{}/drive_c/users", candidate_rendered);
    let users_path = StrictPath::new(&users_dir);
    if !users_path.is_dir() {
        return None;
    }

    // 4. Detect Wine user
    let wine_user = detect_wine_user(&users_dir)?;

    // 5. Scan dosdevices for drive mappings
    let drive_mappings = scan_dosdevices(&dosdevices);

    Some(ValidatedPrefix {
        path: candidate.clone(),
        wine_user,
        has_drive_c: true,
        drive_mappings,
    })
}

/// Detect the Wine user from the users directory.
/// Returns None if no valid user is found.
fn detect_wine_user(users_dir: &str) -> Option<String> {
    let users_path = StrictPath::new(users_dir);
    let entries = match std::fs::read_dir(users_path.interpret().ok()?) {
        Ok(e) => e,
        Err(_) => return None,
    };

    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if !SYSTEM_USERS.contains(&lower.as_str()) && entry.path().is_dir() {
            candidates.push(name);
        }
    }

    if candidates.len() == 1 {
        return Some(candidates.into_iter().next().unwrap());
    }

    // If multiple candidates, prefer "steamuser" for Proton
    if candidates.iter().any(|c| c.eq_ignore_ascii_case("steamuser")) {
        return Some("steamuser".to_string());
    }

    // Return first candidate if any (caller should handle ambiguity)
    candidates.into_iter().next()
}

/// Scan dosdevices directory for drive letter symlinks.
fn scan_dosdevices(dosdevices_dir: &str) -> HashMap<char, String> {
    let mut mappings = HashMap::new();

    let path = match StrictPath::new(dosdevices_dir).interpret() {
        Ok(p) => p,
        Err(_) => return mappings,
    };

    let entries = match std::fs::read_dir(&path) {
        Ok(e) => e,
        Err(_) => return mappings,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Look for patterns like "d:" or "D:"
        if name.len() == 2 && name.ends_with(':') {
            let letter = name.as_bytes()[0];
            if letter.is_ascii_alphabetic() {
                // Check if it's a symlink
                if let Ok(target) = std::fs::read_link(entry.path()) {
                    mappings.insert(
                        (letter as char).to_ascii_lowercase(),
                        target.to_string_lossy().to_string(),
                    );
                }
            }
        }
    }

    mappings
}

/// Choose the Wine user for restore into a validated prefix.
pub fn choose_wine_user_for_restore(
    prefix: &ValidatedPrefix,
    preferred_wine_user: Option<&str>,
    target_path_hint: Option<&str>,
    is_proton: bool,
) -> Result<String, WineUserAmbiguity> {
    // 1. Configured preferred user
    if let Some(user) = preferred_wine_user {
        return Ok(user.to_string());
    }

    // 2. Target path hint
    if let Some(hint) = target_path_hint {
        let hint_lower = hint.to_ascii_lowercase();
        let users_dir = format!("{}/drive_c/users", prefix.path.render());
        if let Ok(entries) = std::fs::read_dir(StrictPath::new(&users_dir).interpret().unwrap_or_default()) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lower = name.to_ascii_lowercase();
                if !SYSTEM_USERS.contains(&lower.as_str()) && entry.path().is_dir() {
                    let user_path = format!("{}/{}", users_dir, name).to_ascii_lowercase();
                    if hint_lower.starts_with(&user_path) {
                        return Ok(name);
                    }
                }
            }
        }
    }

    // 3. Single non-system user
    let users_dir = format!("{}/drive_c/users", prefix.path.render());
    let mut candidates = Vec::new();
    if let Ok(entries) = std::fs::read_dir(StrictPath::new(&users_dir).interpret().unwrap_or_default()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();
            if !SYSTEM_USERS.contains(&lower.as_str()) && entry.path().is_dir() {
                candidates.push(name);
            }
        }
    }

    if candidates.len() == 1 {
        return Ok(candidates.into_iter().next().unwrap());
    }

    // 4. Proton: prefer steamuser
    if is_proton && candidates.iter().any(|c| c.eq_ignore_ascii_case("steamuser")) {
        return Ok("steamuser".to_string());
    }

    // 5. Ambiguity
    Err(WineUserAmbiguity { candidates })
}

/// Error returned when multiple Wine users are found and none is preferred.
#[derive(Clone, Debug)]
pub struct WineUserAmbiguity {
    pub candidates: Vec<String>,
}

impl std::fmt::Display for WineUserAmbiguity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Multiple Wine users found: [{}]. Please specify a preferred user.",
            self.candidates.join(", ")
        )
    }
}

impl std::error::Error for WineUserAmbiguity {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_prefix(base: &str) -> String {
        let prefix = format!("{}/test_prefix", base);
        let _ = fs::create_dir_all(&prefix);
        let _ = fs::create_dir_all(format!("{}/drive_c/users/steamuser", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));
        prefix
    }

    #[test]
    fn valid_prefix_with_system_reg() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = create_test_prefix(tmp.path().to_str().unwrap());
        let result = validate_prefix(&StrictPath::new(&prefix));
        assert!(result.is_some());
        let vp = result.unwrap();
        assert!(vp.has_drive_c);
        assert_eq!(vp.wine_user, "steamuser");
    }

    #[test]
    fn fails_without_drive_c() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/no_drive_c", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(&prefix);
        let _ = fs::File::create(format!("{}/system.reg", prefix));
        assert!(validate_prefix(&StrictPath::new(&prefix)).is_none());
    }

    #[test]
    fn fails_without_markers() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/no_markers", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/testuser", prefix));
        assert!(validate_prefix(&StrictPath::new(&prefix)).is_none());
    }

    #[test]
    fn fails_without_users_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/no_users", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));
        assert!(validate_prefix(&StrictPath::new(&prefix)).is_none());
    }

    #[test]
    fn detects_single_wine_user() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/single_user", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/myuser", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Default", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));

        let result = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        assert_eq!(result.wine_user, "myuser");
    }

    #[test]
    fn prefers_steamuser_among_multiple() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/multi_user", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/steamuser", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/deck", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));

        let result = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        assert_eq!(result.wine_user, "steamuser");
    }

    #[test]
    fn dosdevices_mappings() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = create_test_prefix(tmp.path().to_str().unwrap());
        // Create a symlink d: -> /mnt/data (skip if not supported)
        let dosdevices = format!("{}/dosdevices", prefix);
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink("/mnt/data", format!("{}/d:", dosdevices));
        }

        let result = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        #[cfg(unix)]
        {
            assert_eq!(result.drive_mappings.get(&'d'), Some(&"/mnt/data".to_string()));
        }
    }

    #[test]
    fn choose_user_prefers_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix_str = create_test_prefix(tmp.path().to_str().unwrap());
        let vp = validate_prefix(&StrictPath::new(&prefix_str)).unwrap();

        let result = choose_wine_user_for_restore(&vp, Some("custom_user"), None, false).unwrap();
        assert_eq!(result, "custom_user");
    }

    #[test]
    fn choose_user_single_user() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/single", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/onlyuser", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));

        let vp = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        let result = choose_wine_user_for_restore(&vp, None, None, false).unwrap();
        assert_eq!(result, "onlyuser");
    }

    #[test]
    fn choose_user_proton_prefers_steamuser() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix_str = create_test_prefix(tmp.path().to_str().unwrap());
        let vp = validate_prefix(&StrictPath::new(&prefix_str)).unwrap();

        let result = choose_wine_user_for_restore(&vp, None, None, true).unwrap();
        assert_eq!(result, "steamuser");
    }

    #[test]
    fn choose_user_multi_user_no_config_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/multi", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/alice", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/bob", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));

        let vp = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        let result = choose_wine_user_for_restore(&vp, None, None, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.candidates.contains(&"alice".to_string()));
        assert!(err.candidates.contains(&"bob".to_string()));
    }
}
