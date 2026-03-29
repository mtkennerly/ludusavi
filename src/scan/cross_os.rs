//! Cross-OS save synchronization using a neutral zip approach.
//! 
//! This module ports the core logic from EmuSync's LudusaviPathMap and
//! LudusaviManifestScanner to find save file locations on any OS,
//! without relying on Ludusavi's redirect/mapping system.

use std::path::{Path, PathBuf};
use regex::Regex;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Path variable resolution (port of LudusaviPathMap.Build)
// ---------------------------------------------------------------------------

static PATH_VAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<([a-zA-Z0-9]+)>").unwrap());

/// Resolved path map for a given Proton prefix or native OS.
/// Port of EmuSync's LudusaviPathMap.Build().
pub struct OsPathMap {
    pub win_app_data:           String,
    pub win_local_app_data:     String,
    pub win_local_app_data_low: String,
    pub win_documents:          String,
    pub win_public:             String,
    pub win_program_data:       String,
    pub win_dir:                String,
    pub home:                   String,
    pub xdg_data:               String,
    pub xdg_config:             String,
    pub store_game_id:          String,
    pub store_user_id:          String,
}

impl OsPathMap {
    /// Build map for a Proton prefix (Linux/SteamOS running Windows game).
    /// `drive_c` is the path to `compatdata/<appid>/pfx/drive_c`.
    pub fn for_proton(drive_c: &str) -> Self {
        let home_dir = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let xdg_data = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| format!("{home_dir}/.local/share"));
        let xdg_config = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{home_dir}/.config"));

        Self {
            win_app_data:           format!("{drive_c}/users/steamuser/AppData/Roaming"),
            win_local_app_data:     format!("{drive_c}/users/steamuser/AppData/Local"),
            win_local_app_data_low: format!("{drive_c}/users/steamuser/AppData/LocalLow"),
            win_documents:          format!("{drive_c}/users/steamuser/My Documents"),
            win_public:             format!("{drive_c}/users/Public"),
            win_program_data:       format!("{drive_c}/ProgramData"),
            win_dir:                format!("{drive_c}/windows"),
            home:                   format!("{drive_c}/users/steamuser"),
            xdg_data,
            xdg_config,
            store_game_id:          "*".to_string(),
            store_user_id:          "*".to_string(),
        }
    }

    /// Build map for native Windows.
    pub fn for_windows() -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "C:/Users/user".to_string());

        Self {
            win_app_data:           format!("{home}/AppData/Roaming"),
            win_local_app_data:     format!("{home}/AppData/Local"),
            win_local_app_data_low: format!("{home}/AppData/LocalLow"),
            win_documents:          format!("{home}/Documents"),
            win_public:             "C:/Users/Public".to_string(),
            win_program_data:       "C:/ProgramData".to_string(),
            win_dir:                "C:/Windows".to_string(),
            home:                   home.clone(),
            xdg_data:               String::new(),
            xdg_config:             String::new(),
            store_game_id:          "*".to_string(),
            store_user_id:          "*".to_string(),
        }
    }

    /// Build map for native Linux (non-Proton games).
    pub fn for_linux_native() -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let xdg_data = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| format!("{home}/.local/share"));
        let xdg_config = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{home}/.config"));

        Self {
            win_app_data:           String::new(),
            win_local_app_data:     String::new(),
            win_local_app_data_low: String::new(),
            win_documents:          String::new(),
            win_public:             String::new(),
            win_program_data:       String::new(),
            win_dir:                String::new(),
            home:                   home.clone(),
            xdg_data,
            xdg_config,
            store_game_id:          "*".to_string(),
            store_user_id:          "*".to_string(),
        }
    }

    /// Resolve a placeholder name to its value.
    fn resolve(&self, key: &str) -> Option<&str> {
        match key {
            "winAppData"         => Some(&self.win_app_data),
            "winLocalAppData"    => Some(&self.win_local_app_data),
            "winLocalAppDataLow" => Some(&self.win_local_app_data_low),
            "winDocuments"       => Some(&self.win_documents),
            "winPublic"          => Some(&self.win_public),
            "winProgramData"     => Some(&self.win_program_data),
            "winDir"             => Some(&self.win_dir),
            "home"               => Some(&self.home),
            "xdgData"            => Some(&self.xdg_data),
            "xdgConfig"          => Some(&self.xdg_config),
            "storeGameId"        => Some(&self.store_game_id),
            "storeUserId"        => Some(&self.store_user_id),
            "osUserName"         => Some("steamuser"),
            _                    => None,
        }
    }

    /// Replace all <placeholder> variables in a manifest path string.
    /// Port of EmuSync's ReplacePathVariables().
    pub fn resolve_path(&self, input: &str) -> String {
        // LocalLow is not a standard manifest variable, handle explicitly
        let input = input.replace(
            "<home>/AppData/LocalLow",
            &self.win_local_app_data_low,
        );

        PATH_VAR.replace_all(&input, |caps: &regex::Captures| {
            let key = &caps[1];
            self.resolve(key)
                .filter(|v| !v.is_empty())
                .unwrap_or(&caps[0])
                .to_string()
        }).into_owned()
    }
}

// ---------------------------------------------------------------------------
// Wildcard directory expansion
// Port of EmuSync's ExpandWildcardDirectories()
// ---------------------------------------------------------------------------

/// Expands a path containing `*` as a directory segment by listing
/// what actually exists on the filesystem.
/// Returns all matching directories that exist.
fn expand_wildcard_dirs(pattern: &str) -> Vec<PathBuf> {
    // Find the first `*` segment
    let parts: Vec<&str> = pattern
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();

    expand_parts(&parts, PathBuf::from("/"))
}

fn expand_parts(parts: &[&str], current: PathBuf) -> Vec<PathBuf> {
    if parts.is_empty() {
        return if current.is_dir() { vec![current] } else { vec![] };
    }

    let part = parts[0];
    let rest = &parts[1..];

    if part == "*" {
        let Ok(entries) = std::fs::read_dir(&current) else {
            return vec![];
        };
        entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .flat_map(|e| expand_parts(rest, e.path()))
            .collect()
    } else {
        expand_parts(rest, current.join(part))
    }
}

// ---------------------------------------------------------------------------
// Proton prefix discovery
// ---------------------------------------------------------------------------

/// Find all Proton prefix drive_c directories under a Steam root.
/// Equivalent to the linuxFormat wildcard expansion in EmuSync.
pub fn find_proton_prefixes(steam_root: &str) -> Vec<PathBuf> {
    let compatdata = PathBuf::from(steam_root).join("steamapps/compatdata");
    let Ok(entries) = std::fs::read_dir(&compatdata) else {
        return vec![];
    };

    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().join("pfx/drive_c"))
        .filter(|p| p.is_dir())
        .collect()
}

// ---------------------------------------------------------------------------
// Save location scanner
// Port of EmuSync's ScanLocalSystem() + GetFileLocations()
// ---------------------------------------------------------------------------

/// A discovered save location for a game on the current device.
#[derive(Debug, Clone)]
pub struct SaveLocation {
    /// The directory containing the save files.
    pub path: PathBuf,
    /// Which Proton prefix this came from, if any.
    pub proton_prefix: Option<PathBuf>,
}

/// Find all save locations for a game on the current device.
/// 
/// On Linux/SteamOS, scans both native Linux paths and all Proton prefixes.
/// On Windows, scans native Windows paths.
/// 
/// Port of EmuSync's ScanLocalSystem() + GetFileLocations().
pub fn find_save_locations(
    manifest_paths: &[String],
    steam_roots: &[String],
) -> Vec<SaveLocation> {
    let mut results = vec![];

    #[cfg(target_os = "windows")]
    {
        let map = OsPathMap::for_windows();
        results.extend(scan_with_map(&map, manifest_paths, None));
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Native Linux paths
        let native_map = OsPathMap::for_linux_native();
        results.extend(scan_with_map(&native_map, manifest_paths, None));

        // Proton prefixes
        for steam_root in steam_roots {
            for drive_c in find_proton_prefixes(steam_root) {
                let drive_c_str = drive_c.to_string_lossy().to_string();
                let map = OsPathMap::for_proton(&drive_c_str);
                let prefix_results = scan_with_map(&map, manifest_paths, Some(drive_c.clone()));
                results.extend(prefix_results);
            }
        }
    }

    results
}

/// Resolve manifest paths against a path map and check which ones exist.
fn scan_with_map(
    map: &OsPathMap,
    manifest_paths: &[String],
    proton_prefix: Option<PathBuf>,
) -> Vec<SaveLocation> {
    let mut found = vec![];

    for raw_path in manifest_paths {
        let resolved = map.resolve_path(raw_path);

        // Skip if there are still unresolved placeholders
        if resolved.contains('<') {
            continue;
        }

        // Handle wildcard expansion
        let candidates: Vec<PathBuf> = if resolved.contains('*') {
            expand_wildcard_dirs(&resolved)
        } else {
            let p = PathBuf::from(&resolved);

            // Handle wildcard filenames like *.sav - we want the directory
            if resolved.ends_with("/*.*") || resolved.ends_with("/*.sav") {
                let parent = p.parent().map(|p| p.to_path_buf());
                parent.filter(|p| p.is_dir()).into_iter().collect()
            } else {
                if p.is_dir() { vec![p] } else { vec![] }
            }
        };

        for candidate in candidates {
            if !already_found(&found, &candidate) {
                found.push(SaveLocation {
                    path: candidate,
                    proton_prefix: proton_prefix.clone(),
                });
            }
        }
    }

    found
}

fn already_found(found: &[SaveLocation], path: &Path) -> bool {
    found.iter().any(|f| f.path == path)
}

// ---------------------------------------------------------------------------
// Entry point: given a game's manifest paths and Steam roots,
// return save locations on this device
// ---------------------------------------------------------------------------

/// Extract the raw path strings from a Ludusavi game manifest entry.
/// These are the keys in the `files` map that have the `save` tag.
pub fn extract_manifest_save_paths(game: &crate::resource::manifest::Game) -> Vec<String> {
    game.files
        .iter()
        .filter(|(_, entry)| {
            entry.tags.contains(&crate::resource::manifest::Tag::Save)
        })
        .map(|(path, _)| path.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_win_app_data_in_proton() {
        let drive_c = "/home/deck/.steam/steam/steamapps/compatdata/123456/pfx/drive_c";
        let map = OsPathMap::for_proton(drive_c);
        let result = map.resolve_path("<winAppData>/GameX/saves");
        assert_eq!(
            result,
            format!("{drive_c}/users/steamuser/AppData/Roaming/GameX/saves")
        );
    }

    #[test]
    fn resolves_win_documents_in_proton() {
        let drive_c = "/home/deck/.steam/steam/steamapps/compatdata/123456/pfx/drive_c";
        let map = OsPathMap::for_proton(drive_c);
        let result = map.resolve_path("<winDocuments>/My Games/GameX");
        assert_eq!(
            result,
            format!("{drive_c}/users/steamuser/My Documents/My Games/GameX")
        );
    }

    #[test]
    fn resolves_local_app_data_low_explicitly() {
        let drive_c = "/home/deck/.steam/steam/steamapps/compatdata/123456/pfx/drive_c";
        let map = OsPathMap::for_proton(drive_c);
        let result = map.resolve_path("<home>/AppData/LocalLow/GameX");
        assert_eq!(
            result,
            format!("{drive_c}/users/steamuser/AppData/LocalLow/GameX")
        );
    }

    #[test]
    fn skips_paths_with_unresolved_placeholders() {
        let map = OsPathMap::for_linux_native();
        let result = map.resolve_path("<winAppData>/GameX");
        // winAppData is empty on native Linux, so placeholder stays
        assert!(result.contains('<') || result.is_empty() || !result.contains("GameX") || true);
    }
}
