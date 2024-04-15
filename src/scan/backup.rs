use std::collections::HashMap;

use crate::{
    lang::TRANSLATOR,
    scan::{registry_compat::RegistryItem, ScanChangeCount, ScanInfo, ScannedFile},
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
    pub failed_files: HashMap<ScannedFile, BackupError>,
    pub failed_registry: HashMap<RegistryItem, BackupError>,
}

impl BackupInfo {
    pub fn successful(&self) -> bool {
        self.failed_files.is_empty() && self.failed_registry.is_empty()
    }

    pub fn total_failure(scan: &ScanInfo, error: BackupError) -> Self {
        let mut backup_info = Self::default();

        for file in &scan.found_files {
            if file.ignored {
                continue;
            }
            backup_info.failed_files.insert(file.clone(), error.clone());
        }
        for reg_path in &scan.found_registry_keys {
            if reg_path.ignored {
                continue;
            }
            backup_info.failed_registry.insert(reg_path.path.clone(), error.clone());
        }

        backup_info
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct OperationStatus {
    #[serde(rename = "totalGames")]
    pub total_games: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "processedGames")]
    pub processed_games: usize,
    #[serde(rename = "processedBytes")]
    pub processed_bytes: u64,
    #[serde(rename = "changedGames")]
    pub changed_games: ScanChangeCount,
}

impl OperationStatus {
    pub fn add_game(&mut self, scan_info: &ScanInfo, backup_info: &Option<BackupInfo>, processed: bool) {
        self.total_games += 1;
        self.total_bytes += scan_info.total_possible_bytes();
        if processed {
            self.processed_games += 1;
            self.processed_bytes += scan_info.sum_bytes(backup_info.as_ref());
        }

        let changes = scan_info.count_changes();
        if changes.brand_new() {
            self.changed_games.new += 1;
        } else if changes.updated() {
            self.changed_games.different += 1;
        } else {
            self.changed_games.same += 1;
        }
    }

    pub fn processed_all_games(&self) -> bool {
        self.total_games == self.processed_games
    }

    pub fn processed_all_bytes(&self) -> bool {
        self.total_bytes == self.processed_bytes
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub enum OperationStepDecision {
    #[default]
    Processed,
    Cancelled,
    Ignored,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackupId {
    #[default]
    Latest,
    Named(String),
}
