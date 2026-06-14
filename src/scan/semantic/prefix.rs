use crate::path::StrictPath;

const SYSTEM_USERS: &[&str] = &["public", "default", "default user", "all users"];

#[derive(Clone, Debug)]
pub struct ValidatedPrefix {
    pub path: StrictPath,
    pub wine_user: String,
}

/// Validate a candidate Wine prefix path.
///
/// `candidate/drive_c` must exist as a directory.
/// At least one of `candidate/system.reg`, `candidate/user.reg`, or
/// `candidate/dosdevices` must exist.
/// `candidate/drive_c/users` must exist as a directory.
pub fn validate_prefix(candidate: &StrictPath) -> Option<ValidatedPrefix> {
    let drive_c = candidate.joined("drive_c");
    if !drive_c.is_dir() {
        return None;
    }

    let has_marker = candidate.joined("system.reg").exists()
        || candidate.joined("user.reg").exists()
        || candidate.joined("dosdevices").is_dir();
    if !has_marker {
        return None;
    }

    let users = drive_c.joined("users");
    if !users.is_dir() {
        return None;
    }

    let wine_user = detect_wine_user(&users)?;
    Some(ValidatedPrefix {
        path: candidate.clone(),
        wine_user,
    })
}

fn detect_wine_user(users_path: &StrictPath) -> Option<String> {
    let mut candidates = vec![];
    for entry in users_path.read_dir().ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if !SYSTEM_USERS.contains(&lower.as_str()) && entry.path().is_dir() {
            candidates.push(name);
        }
    }

    if candidates.len() == 1 {
        return candidates.into_iter().next();
    }

    if let Some(user) = candidates.iter().find(|c| c.eq_ignore_ascii_case("steamuser")) {
        return Some(user.clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_prefix(base: &str) -> String {
        let prefix = format!("{}/test_prefix", base);
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
        let result = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        assert_eq!(result.wine_user, "steamuser");
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
    fn preserves_steamuser_capitalization() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/multi_user", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/SteamUser", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/deck", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));

        let result = validate_prefix(&StrictPath::new(&prefix)).unwrap();
        assert_eq!(result.wine_user, "SteamUser");
    }

    #[test]
    fn rejects_ambiguous_wine_users_without_steamuser() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = format!("{}/multi_user", tmp.path().to_str().unwrap());
        let _ = fs::create_dir_all(format!("{}/drive_c/users/alice", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/bob", prefix));
        let _ = fs::create_dir_all(format!("{}/drive_c/users/Public", prefix));
        let _ = fs::create_dir_all(format!("{}/dosdevices", prefix));
        let _ = fs::File::create(format!("{}/system.reg", prefix));

        assert!(validate_prefix(&StrictPath::new(&prefix)).is_none());
    }
}
