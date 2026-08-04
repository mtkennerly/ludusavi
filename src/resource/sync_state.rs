use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::prelude::StrictPath;

/// Identifier for this machine, used to key per-device data such as [`GameSyncEntry::prefixes`].
pub fn current_device() -> String {
    whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string())
}

/// Per-game sync metadata stored in settings.config at the backup root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameSyncEntry {
    /// ISO 8601 timestamp of last push from this device.
    pub last_push: DateTime<Utc>,
    /// Hostname of the device that performed the last push.
    pub device: String,
    /// Relative path to the game's mapping.yaml (e.g., "Game Name/mapping.yaml").
    pub mapping_path: String,
    /// Wine/Proton prefix for this game, keyed by device.
    ///
    /// A game's saves live at a different absolute path on each machine, because both
    /// the username and the Steam compatdata app ID differ. Each device records only
    /// its own key here; the maps are unioned in [`SyncStateFile::merge_from`], so a
    /// restoring device can look up where *it* should write.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prefixes: BTreeMap<String, String>,
}

/// The settings.config file at the backup root.
/// Updated on each push to track sync state across devices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncStateFile {
    /// Schema version for future compatibility.
    pub version: u32,
    /// ISO 8601 timestamp of the last update to this file.
    pub last_updated: DateTime<Utc>,
    /// Hostname of the device that performed the last update.
    pub device: String,
    /// Per-game sync entries, keyed by game name.
    #[serde(default)]
    pub games: BTreeMap<String, GameSyncEntry>,
}

impl Default for SyncStateFile {
    fn default() -> Self {
        Self {
            version: 1,
            last_updated: Utc::now(),
            device: current_device(),
            games: BTreeMap::new(),
        }
    }
}

impl SyncStateFile {
    /// The filename for the sync state file.
    pub const FILE_NAME: &'static str = "settings.config";

    /// Load from the given directory path (e.g., the backup root).
    pub fn load_from(dir: &StrictPath) -> Self {
        let path = dir.joined(Self::FILE_NAME);
        match path.try_read() {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                log::warn!("Failed to parse {}: {}, using defaults", Self::FILE_NAME, e);
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Save to the given directory path using atomic write (write-to-temp + rename).
    pub fn save_to(&self, dir: &StrictPath) -> Result<(), crate::prelude::AnyError> {
        let path = dir.joined(Self::FILE_NAME);
        let tmp_path = dir.joined(format!("{}.tmp", Self::FILE_NAME));
        if path.create_parent_dir().is_err() {
            log::error!("Failed to create parent dir for sync state file");
        }
        let json = serde_json::to_string_pretty(self)?;
        tmp_path.write_with_content(&json)?;
        tmp_path.move_to(&path)?;
        Ok(())
    }

    /// Merge a game entry into the state file.
    /// Updates the last_updated timestamp and device.
    ///
    /// Preserves any per-device prefixes already recorded for the game, since `entry`
    /// only ever carries this device's view.
    pub fn merge_game(&mut self, game_name: &str, entry: GameSyncEntry) {
        let mut entry = entry;
        if let Some(existing) = self.games.get(game_name) {
            for (device, prefix) in &existing.prefixes {
                entry.prefixes.entry(device.clone()).or_insert_with(|| prefix.clone());
            }
        }
        self.games.insert(game_name.to_string(), entry);
        self.last_updated = Utc::now();
        self.device = current_device();
    }

    /// Merge another device's state into this one.
    ///
    /// Games unique to either side are kept. For a game both know about, the entry with
    /// the newer `last_push` wins, but the `prefixes` maps are always unioned: each
    /// device is authoritative only for its own key, so dropping the other side's keys
    /// would lose the very mapping that makes cross-device restore work.
    pub fn merge_from(&mut self, other: &Self) {
        for (game, incoming) in &other.games {
            match self.games.get_mut(game) {
                None => {
                    self.games.insert(game.clone(), incoming.clone());
                }
                Some(existing) => {
                    let incoming_is_newer = incoming.last_push > existing.last_push;
                    if incoming_is_newer {
                        let mut merged = incoming.clone();
                        for (device, prefix) in &existing.prefixes {
                            merged.prefixes.entry(device.clone()).or_insert_with(|| prefix.clone());
                        }
                        *existing = merged;
                    } else {
                        for (device, prefix) in &incoming.prefixes {
                            existing
                                .prefixes
                                .entry(device.clone())
                                .or_insert_with(|| prefix.clone());
                        }
                    }
                }
            }
        }
    }

    /// The Wine/Proton prefix recorded for a game on a given device, if any.
    pub fn prefix_for(&self, game_name: &str, device: &str) -> Option<&str> {
        self.games.get(game_name)?.prefixes.get(device).map(|x| x.as_str())
    }

    /// Record this device's Wine/Proton prefix for a game.
    /// No-op if the game has no entry yet; entries are created on push.
    pub fn set_prefix(&mut self, game_name: &str, device: &str, prefix: &str) {
        if let Some(entry) = self.games.get_mut(game_name) {
            entry.prefixes.insert(device.to_string(), prefix.to_string());
        }
    }

    /// Get the sync info for a specific game.
    pub fn game_info(&self, game_name: &str) -> Option<&GameSyncEntry> {
        self.games.get(game_name)
    }

    /// Check if a game exists in the sync state.
    pub fn has_game(&self, game_name: &str) -> bool {
        self.games.contains_key(game_name)
    }

    /// Remove a game from the sync state.
    pub fn remove_game(&mut self, game_name: &str) {
        self.games.remove(game_name);
        self.last_updated = Utc::now();
        self.device = current_device();
    }

    /// Create a GameSyncEntry for a game push.
    pub fn push_entry(game_name: &str) -> GameSyncEntry {
        let escaped = crate::scan::layout::escape_folder_name(game_name);
        GameSyncEntry {
            last_push: Utc::now(),
            device: current_device(),
            mapping_path: format!("{}/mapping.yaml", escaped),
            prefixes: BTreeMap::new(),
        }
    }

    /// Create a GameSyncEntry for a game pull (records receipt, not push).
    pub fn pull_entry(game_name: &str) -> GameSyncEntry {
        let escaped = crate::scan::layout::escape_folder_name(game_name);
        GameSyncEntry {
            last_push: Utc::now(),
            device: format!("{} (pulled)", current_device()),
            mapping_path: format!("{}/mapping.yaml", escaped),
            prefixes: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(device: &str, last_push: DateTime<Utc>) -> GameSyncEntry {
        GameSyncEntry {
            last_push,
            device: device.to_string(),
            mapping_path: "Test Game/mapping.yaml".to_string(),
            prefixes: BTreeMap::new(),
        }
    }

    fn entry_with_prefix(device: &str, last_push: DateTime<Utc>, prefix: &str) -> GameSyncEntry {
        let mut x = entry(device, last_push);
        x.prefixes.insert(device.to_string(), prefix.to_string());
        x
    }

    #[test]
    fn sync_state_file_default() {
        let state = SyncStateFile::default();
        assert_eq!(state.version, 1);
        assert!(state.games.is_empty());
    }

    #[test]
    fn sync_state_file_merge_game() {
        let mut state = SyncStateFile::default();
        state.merge_game("Test Game", entry("test-pc", Utc::now()));
        assert_eq!(state.games.len(), 1);
        assert_eq!(state.games.get("Test Game").unwrap().device, "test-pc");
    }

    #[test]
    fn merge_game_preserves_other_devices_prefixes() {
        let mut state = SyncStateFile::default();
        state.merge_game("Test Game", entry_with_prefix("deck", Utc::now(), "/home/deck/pfx"));
        // A later push from this device must not drop the Deck's recorded prefix.
        state.merge_game("Test Game", entry_with_prefix("pc", Utc::now(), "/home/kookoo/pfx"));

        let prefixes = &state.games.get("Test Game").unwrap().prefixes;
        assert_eq!(prefixes.get("deck").map(|x| x.as_str()), Some("/home/deck/pfx"));
        assert_eq!(prefixes.get("pc").map(|x| x.as_str()), Some("/home/kookoo/pfx"));
    }

    #[test]
    fn merge_from_unions_prefixes_across_devices() {
        let now = Utc::now();
        let mut local = SyncStateFile::default();
        local.merge_game("Test Game", entry_with_prefix("pc", now, "/home/kookoo/pfx"));

        let mut remote = SyncStateFile::default();
        remote.merge_game(
            "Test Game",
            entry_with_prefix("deck", now + chrono::Duration::seconds(10), "/home/deck/pfx"),
        );

        local.merge_from(&remote);

        let prefixes = &local.games.get("Test Game").unwrap().prefixes;
        assert_eq!(prefixes.get("pc").map(|x| x.as_str()), Some("/home/kookoo/pfx"));
        assert_eq!(prefixes.get("deck").map(|x| x.as_str()), Some("/home/deck/pfx"));
    }

    #[test]
    fn merge_from_keeps_newer_last_push() {
        let older = Utc::now();
        let newer = older + chrono::Duration::seconds(60);

        let mut local = SyncStateFile::default();
        local.merge_game("Test Game", entry("pc", older));
        let mut remote = SyncStateFile::default();
        remote.merge_game("Test Game", entry("deck", newer));
        local.merge_from(&remote);
        assert_eq!(local.games.get("Test Game").unwrap().device, "deck");

        // ...and the reverse: a stale remote must not clobber a newer local entry.
        let mut local = SyncStateFile::default();
        local.merge_game("Test Game", entry("pc", newer));
        let mut remote = SyncStateFile::default();
        remote.merge_game("Test Game", entry("deck", older));
        local.merge_from(&remote);
        assert_eq!(local.games.get("Test Game").unwrap().device, "pc");
    }

    #[test]
    fn merge_from_preserves_local_only_games() {
        let mut local = SyncStateFile::default();
        local.merge_game("Local Only", entry("pc", Utc::now()));

        let mut remote = SyncStateFile::default();
        remote.merge_game("Remote Only", entry("deck", Utc::now()));

        local.merge_from(&remote);

        assert!(local.has_game("Local Only"), "pulling must not drop local-only games");
        assert!(local.has_game("Remote Only"));
    }

    #[test]
    fn prefixes_omitted_from_json_when_empty() {
        let mut state = SyncStateFile::default();
        state.merge_game("Test Game", entry("pc", Utc::now()));
        let json = serde_json::to_string(&state).unwrap();
        assert!(
            !json.contains("prefixes"),
            "non-Wine games should not gain a prefixes key"
        );
    }

    #[test]
    fn old_file_without_prefixes_deserializes() {
        let json = r#"{
            "version": 1,
            "last_updated": "2026-07-19T11:14:29.292488980Z",
            "device": "kookooPC",
            "games": {
                "Baldur's Gate 3": {
                    "last_push": "2026-07-19T11:14:29.292486820Z",
                    "device": "kookooPC",
                    "mapping_path": "Baldur's Gate 3/mapping.yaml"
                }
            }
        }"#;
        let loaded: SyncStateFile = serde_json::from_str(json).unwrap();
        assert!(loaded.games.get("Baldur's Gate 3").unwrap().prefixes.is_empty());
    }

    #[test]
    fn prefix_accessors() {
        let mut state = SyncStateFile::default();
        state.merge_game("Test Game", entry("pc", Utc::now()));
        assert_eq!(state.prefix_for("Test Game", "pc"), None);

        state.set_prefix("Test Game", "pc", "/home/kookoo/pfx");
        assert_eq!(state.prefix_for("Test Game", "pc"), Some("/home/kookoo/pfx"));
        assert_eq!(state.prefix_for("Test Game", "deck"), None);
        assert_eq!(state.prefix_for("Missing Game", "pc"), None);
    }

    #[test]
    fn sync_state_file_roundtrip() {
        let mut state = SyncStateFile::default();
        state.merge_game("Test Game", entry_with_prefix("test-pc", Utc::now(), "/home/test/pfx"));

        let json = serde_json::to_string(&state).unwrap();
        let loaded: SyncStateFile = serde_json::from_str(&json).unwrap();
        assert_eq!(state, loaded);
    }
}
