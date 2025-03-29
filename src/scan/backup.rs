use std::collections::HashMap;

use crate::{
    lang::TRANSLATOR,
    path::StrictPath,
    scan::{registry::RegistryItem, ScanChange, ScanChangeCount, ScanInfo},
};

#[derive(Clone, Debug)]
pub enum BackupError {
    Raw(String),
    App(crate::prelude::Error),
    #[cfg(test)]
    Test,
}

impl BackupError {
    pub fn message(&self) -> String {
        match self {
            BackupError::Raw(error) => error.clone(),
            BackupError::App(error) => TRANSLATOR.handle_error(error),
            #[cfg(test)]
            BackupError::Test => "test".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BackupInfo {
    pub failed_files: HashMap<StrictPath, BackupError>,
    pub failed_registry: HashMap<RegistryItem, BackupError>,
}

impl BackupInfo {
    pub fn successful(&self) -> bool {
        self.failed_files.is_empty() && self.failed_registry.is_empty()
    }

    pub fn total_failure(scan: &ScanInfo, error: BackupError) -> Self {
        let mut backup_info = Self::default();

        for (scan_key, file) in &scan.found_files {
            if file.ignored {
                continue;
            }
            backup_info.failed_files.insert(scan_key.clone(), error.clone());
        }
        for (scan_key, reg_path) in &scan.found_registry_keys {
            if reg_path.ignored {
                continue;
            }
            backup_info.failed_registry.insert(scan_key.clone(), error.clone());
        }

        backup_info
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationStatus {
    /// How many games were found.
    pub total_games: usize,
    /// How many bytes are used by files associated with found games.
    pub total_bytes: u64,
    /// How many games were processed.
    /// This excludes ignored, failed, and cancelled games.
    pub processed_games: usize,
    /// How many bytes were processed.
    /// This excludes ignored, failed, and cancelled games.
    pub processed_bytes: u64,
    /// Total count of `new`, `same`, and `different` games.
    pub changed_games: ScanChangeCount,
}

impl OperationStatus {
    pub fn add_game(&mut self, scan_info: &ScanInfo, backup_info: Option<&BackupInfo>, processed: bool) {
        self.total_games += 1;
        self.total_bytes += scan_info.total_possible_bytes();
        if processed {
            self.processed_games += 1;
            self.processed_bytes += scan_info.sum_bytes(backup_info);

            match scan_info.overall_change() {
                ScanChange::New => {
                    self.changed_games.new += 1;
                }
                ScanChange::Different => {
                    self.changed_games.different += 1;
                }
                ScanChange::Removed => {
                    self.changed_games.removed += 1;
                }
                ScanChange::Same => {
                    self.changed_games.same += 1;
                }
                ScanChange::Unknown => {}
            }
        }
    }

    pub fn processed_all_games(&self) -> bool {
        self.total_games == self.processed_games
    }

    pub fn processed_all_bytes(&self) -> bool {
        self.total_bytes == self.processed_bytes
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
pub enum OperationStepDecision {
    #[default]
    Processed,
    #[allow(unused)]
    Cancelled,
    Ignored,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackupId {
    #[default]
    Latest,
    Named(String),
}
