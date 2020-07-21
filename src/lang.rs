use crate::{
    manifest::Store,
    prelude::{Error, OperationStatus},
};

#[derive(Clone, Copy, Debug)]
pub enum Language {
    English,
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Translator {
    language: Language,
}

impl Translator {
    pub fn window_title(&self) -> String {
        let version = option_env!("LUDUSAVI_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
        match option_env!("LUDUSAVI_VARIANT") {
            Some(variant) => format!("Ludusavi v{} ({})", version, variant),
            None => format!("Ludusavi v{}", version),
        }
    }

    pub fn handle_error(&self, error: &Error) -> String {
        match error {
            Error::ConfigInvalid { why } => self.config_is_invalid(why),
            Error::ManifestInvalid { why } => self.manifest_is_invalid(why),
            Error::ManifestCannotBeUpdated => self.manifest_cannot_be_updated(),
            Error::CliBackupTargetExists { path } => self.cli_backup_target_exists(path),
            Error::CliUnrecognizedGames { games } => self.cli_unrecognized_games(games),
            Error::CliUnableToRequestConfirmation => self.cli_unable_to_request_confirmation(),
            Error::SomeEntriesFailed => self.some_entries_failed(),
            Error::CannotPrepareBackupTarget { path } => self.cannot_prepare_backup_target(path),
            Error::RestorationSourceInvalid { path } => self.restoration_source_is_invalid(path),
            Error::RegistryIssue => self.registry_issue(),
            Error::UnableToBrowseFileSystem => self.unable_to_browse_file_system(),
        }
    }

    pub fn cli_backup_target_exists(&self, path: &str) -> String {
        match self.language {
            Language::English => format!("The backup target already exists ( {} ). Either choose a different --target or delete it with --force.", path),
        }
    }

    pub fn cli_unrecognized_games(&self, games: &[String]) -> String {
        let prefix = match self.language {
            Language::English => "No info for these games:",
        };
        let lines: Vec<_> = games.iter().map(|x| format!("  - {}", x)).collect();
        format!("{}\n{}", prefix, lines.join("\n"))
    }

    pub fn cli_confirm_restoration(&self, path: &str) -> String {
        match self.language {
            Language::English => format!("Do you want to restore from {}?", path),
        }
    }

    pub fn cli_unable_to_request_confirmation(&self) -> String {
        #[cfg(target_os = "windows")]
        let extra_note: String = match self.language {
            Language::English => "If you are using a Bash emulator (like Git Bash), try running winpty.",
        }
        .into();

        #[cfg(not(target_os = "windows"))]
        let extra_note = "";

        match self.language {
            Language::English => format!("Unable to request confirmation. {}", extra_note),
        }
    }

    pub fn some_entries_failed(&self) -> String {
        match self.language {
            Language::English => format!("Some entries failed to process; look for {} in the output for details. Double check whether you can access those files or whether their paths are very long.", self.label_failed()),
        }
    }

    pub fn label_failed(&self) -> String {
        match self.language {
            Language::English => "[FAILED]",
        }
        .into()
    }

    pub fn cli_game_header(&self, name: &str, bytes: u64) -> String {
        match self.language {
            Language::English => format!("{} [{}]:", name, self.mib(bytes, false)),
        }
    }

    pub fn cli_game_line_item_successful(&self, item: &str) -> String {
        match self.language {
            Language::English => format!("  - {}", item),
        }
    }

    pub fn cli_game_line_item_failed(&self, item: &str) -> String {
        match self.language {
            Language::English => format!("  - {} {}", self.label_failed(), item),
        }
    }

    pub fn cli_summary(&self, status: &OperationStatus, location: &str) -> String {
        if status.completed() {
            match self.language {
                Language::English => format!(
                    "\nOverall:\n  Games: {}\n  Size: {}\n  Location: {}",
                    status.total_games,
                    self.mib(status.total_bytes, true),
                    location
                ),
            }
        } else {
            match self.language {
                Language::English => format!(
                    "\nOverall:\n  Games: {} of {}\n  Size: {} of {}\n  Location: {}",
                    status.processed_games,
                    status.total_games,
                    self.mib_unlabelled(status.processed_bytes),
                    self.mib(status.total_bytes, true),
                    location
                ),
            }
        }
    }

    pub fn game_list_entry_title_failed(&self, name: &str) -> String {
        match self.language {
            Language::English => format!("{} {}", name, self.label_failed()),
        }
    }

    pub fn failed_file_entry_line(&self, path: &str) -> String {
        match self.language {
            Language::English => format!("{} {}", self.label_failed(), path),
        }
    }

    pub fn backup_button(&self) -> String {
        match self.language {
            Language::English => "Back up",
        }
        .into()
    }

    pub fn preview_button(&self) -> String {
        match self.language {
            Language::English => "Preview",
        }
        .into()
    }

    pub fn restore_button(&self) -> String {
        match self.language {
            Language::English => "Restore",
        }
        .into()
    }

    pub fn nav_backup_button(&self) -> String {
        match self.language {
            Language::English => "BACKUP MODE",
        }
        .into()
    }

    pub fn nav_restore_button(&self) -> String {
        match self.language {
            Language::English => "RESTORE MODE",
        }
        .into()
    }

    pub fn add_root_button(&self) -> String {
        match self.language {
            Language::English => "Add root",
        }
        .into()
    }

    pub fn remove_root_button(&self) -> String {
        match self.language {
            Language::English => "Remove",
        }
        .into()
    }

    pub fn continue_button(&self) -> String {
        match self.language {
            Language::English => "Continue",
        }
        .into()
    }

    pub fn cancel_button(&self) -> String {
        match self.language {
            Language::English => "Cancel",
        }
        .into()
    }

    pub fn cancelling_button(&self) -> String {
        match self.language {
            Language::English => "Cancelling...",
        }
        .into()
    }

    pub fn browse_button(&self) -> String {
        match self.language {
            Language::English => "Browse",
        }
        .into()
    }

    pub fn okay_button(&self) -> String {
        match self.language {
            Language::English => "Okay",
        }
        .into()
    }

    pub fn select_all_button(&self) -> String {
        match self.language {
            Language::English => "Select all",
        }
        .into()
    }

    pub fn deselect_all_button(&self) -> String {
        match self.language {
            Language::English => "Deselect all",
        }
        .into()
    }

    pub fn no_roots_are_configured(&self) -> String {
        match self.language {
            Language::English => "Add some roots to back up even more data.",
        }
        .into()
    }

    pub fn config_is_invalid(&self, why: &str) -> String {
        match self.language {
            Language::English => format!("Error: The config file is invalid.\n{}", why),
        }
    }

    pub fn manifest_is_invalid(&self, why: &str) -> String {
        match self.language {
            Language::English => format!("Error: The manifest file is invalid.\n{}", why),
        }
    }

    pub fn manifest_cannot_be_updated(&self) -> String {
        match self.language {
            Language::English => "Error: Unable to download an update to the manifest file.",
        }
        .into()
    }

    pub fn cannot_prepare_backup_target(&self, target: &str) -> String {
        match self.language {
            Language::English => format!("Error: Unable to prepare backup target (either creating or emptying the folder). If you have the folder open in your file browser, try closing it: {}", target),
        }
    }

    pub fn restoration_source_is_invalid(&self, source: &str) -> String {
        match self.language {
            Language::English => {
                format!("Error: The restoration source is invalid (either doesn't exist or isn't a directory). Please double check the location: {}", source)
            }
        }
    }

    pub fn registry_issue(&self) -> String {
        match self.language {
            Language::English => "Error: Some registry entries were skipped.",
        }
        .into()
    }

    pub fn unable_to_browse_file_system(&self) -> String {
        match self.language {
            Language::English => "Error: Unable to browse on your system.",
        }
        .into()
    }

    pub fn mib(&self, bytes: u64, show_zero: bool) -> String {
        let mib = self.mib_unlabelled(bytes);
        if !show_zero && mib == "0.00" {
            match self.language {
                Language::English => "~ 0",
            }
            .into()
        } else {
            match self.language {
                Language::English => format!("{} MiB", mib),
            }
        }
    }

    pub fn mib_unlabelled(&self, bytes: u64) -> String {
        format!("{:.2}", bytes as f64 / 1024.0 / 1024.0)
    }

    pub fn processed_games(&self, status: &OperationStatus) -> String {
        if status.completed() {
            match self.language {
                Language::English => format!("{} games | {}", status.total_games, self.mib(status.total_bytes, true)),
            }
        } else {
            match self.language {
                Language::English => format!(
                    "{} of {} games | {} of {}",
                    status.processed_games,
                    status.total_games,
                    self.mib_unlabelled(status.processed_bytes),
                    self.mib(status.total_bytes, true)
                ),
            }
        }
    }

    pub fn backup_target_label(&self) -> String {
        match self.language {
            Language::English => "Back up to:",
        }
        .into()
    }

    pub fn restore_source_label(&self) -> String {
        match self.language {
            Language::English => "Restore from:",
        }
        .into()
    }

    pub fn store(&self, store: &Store) -> String {
        match self.language {
            Language::English => match store {
                Store::Steam => "Steam",
                Store::Other => "Other",
            },
        }
        .into()
    }

    pub fn modal_confirm_backup(&self, target: &str, target_exists: bool) -> String {
        match (self.language, target_exists) {
            (Language::English, false) => format!("Are you sure you want to proceed with the backup? The target folder does not already exist, so it will be created: {}", target),
            (Language::English, true) => format!("Are you sure you want to proceed with the backup? The target folder already exists, so it will be deleted and recreated from scratch: {}", target),
        }
    }

    pub fn modal_confirm_restore(&self, source: &str) -> String {
        match self.language {
            Language::English => format!("Are you sure you want to proceed with the restoration? This will overwrite any current files with the backups from here: {}", source),
        }
    }
}
