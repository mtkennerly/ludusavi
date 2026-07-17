use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::prelude::StrictPath;
use whoami;

/// Per-game sync metadata stored in settings.config at the backup root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameSyncEntry {
    /// ISO 8601 timestamp of last push from this device.
    pub last_push: DateTime<Utc>,
    /// Hostname of the device that performed the last push.
    pub device: String,
    /// Relative path to the game's mapping.yaml (e.g., "Game Name/mapping.yaml").
    pub mapping_path: String,
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
            device: whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string()),
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
        if !path.exists() {
            return Self::default();
        }
        match path.try_read() {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to the given directory path.
    pub fn save_to(&self, dir: &StrictPath) -> Result<(), crate::prelude::AnyError> {
        let path = dir.joined(Self::FILE_NAME);
        if path.create_parent_dir().is_err() {
            log::error!("Failed to create parent dir for sync state file");
        }
        let json = serde_json::to_string_pretty(self)?;
        path.write_with_content(&json)?;
        Ok(())
    }

    /// Merge a game entry into the state file.
    /// Updates the last_updated timestamp and device.
    pub fn merge_game(&mut self, game_name: &str, entry: GameSyncEntry) {
        self.games.insert(game_name.to_string(), entry);
        self.last_updated = Utc::now();
        self.device = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());
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
        self.device = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());
    }

    /// Create a GameSyncEntry for a game push.
    pub fn create_entry(game_name: &str) -> GameSyncEntry {
        let escaped = crate::scan::layout::escape_folder_name(game_name);
        GameSyncEntry {
            last_push: Utc::now(),
            device: whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string()),
            mapping_path: format!("{}/mapping.yaml", escaped),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_state_file_default() {
        let state = SyncStateFile::default();
        assert_eq!(state.version, 1);
        assert!(state.games.is_empty());
    }

    #[test]
    fn sync_state_file_merge_game() {
        let mut state = SyncStateFile::default();
        let entry = GameSyncEntry {
            last_push: Utc::now(),
            device: "test-pc".to_string(),
            mapping_path: "Test Game/mapping.yaml".to_string(),
        };
        state.merge_game("Test Game", entry.clone());
        assert_eq!(state.games.len(), 1);
        assert_eq!(state.games.get("Test Game").unwrap().device, "test-pc");
    }

    #[test]
    fn sync_state_file_roundtrip() {
        let mut state = SyncStateFile::default();
        let entry = GameSyncEntry {
            last_push: Utc::now(),
            device: "test-pc".to_string(),
            mapping_path: "Test Game/mapping.yaml".to_string(),
        };
        state.merge_game("Test Game", entry);

        let json = serde_json::to_string(&state).unwrap();
        let loaded: SyncStateFile = serde_json::from_str(&json).unwrap();
        assert_eq!(state, loaded);
    }
}
