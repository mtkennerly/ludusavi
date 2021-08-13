use byte_unit::Byte;

use crate::{
    manifest::Store,
    prelude::{Error, OperationStatus, OperationStepDecision, StrictPath},
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
            Error::UnableToOpenDir(path) => self.unable_to_open_dir(path),
            Error::UnableToOpenUrl(url) => self.unable_to_open_url(url),
        }
    }

    pub fn cli_backup_target_exists(&self, path: &StrictPath) -> String {
        match self.language {
            Language::English => format!("The backup target already exists ( {} ). Either choose a different --target or delete it with --force.", path.render()),
        }
    }

    pub fn cli_unrecognized_games(&self, games: &[String]) -> String {
        let prefix = match self.language {
            Language::English => "No info for these games:",
        };
        let lines: Vec<_> = games.iter().map(|x| format!("  - {}", x)).collect();
        format!("{}\n{}", prefix, lines.join("\n"))
    }

    pub fn cli_confirm_restoration(&self, path: &StrictPath) -> String {
        match self.language {
            Language::English => format!("Do you want to restore from {}?", path.render()),
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

    pub fn label_duplicates(&self) -> String {
        match self.language {
            Language::English => "[DUPLICATES]",
        }
        .into()
    }

    pub fn label_duplicated(&self) -> String {
        match self.language {
            Language::English => "[DUPLICATED]",
        }
        .into()
    }

    pub fn label_ignored(&self) -> String {
        match self.language {
            Language::English => "[IGNORED]",
        }
        .into()
    }

    pub fn badge_failed(&self) -> String {
        match self.language {
            Language::English => "FAILED",
        }
        .into()
    }

    pub fn badge_duplicates(&self) -> String {
        match self.language {
            Language::English => "DUPLICATES",
        }
        .into()
    }

    pub fn badge_duplicated(&self) -> String {
        match self.language {
            Language::English => "DUPLICATED",
        }
        .into()
    }

    pub fn badge_redirected_from(&self, original: &StrictPath) -> String {
        match self.language {
            Language::English => format!("FROM: {}", original.render()),
        }
    }

    pub fn badge_selected_games(&self, games: usize, bytes: u64) -> String {
        match self.language {
            Language::English => format!("SELECTING {} GAMES, {}", games, self.adjusted_size(bytes)),
        }
    }

    pub fn cli_game_header(
        &self,
        name: &str,
        bytes: u64,
        decision: &OperationStepDecision,
        duplicated: bool,
    ) -> String {
        let mut labels = vec![];
        if *decision == OperationStepDecision::Ignored {
            labels.push(self.label_ignored());
        }
        if duplicated {
            labels.push(self.label_duplicates());
        }

        if labels.is_empty() {
            match self.language {
                Language::English => format!("{} [{}]:", name, self.adjusted_size(bytes)),
            }
        } else {
            match self.language {
                Language::English => format!("{} [{}] {}:", name, self.adjusted_size(bytes), labels.join(" ")),
            }
        }
    }

    pub fn cli_game_line_item(&self, item: &str, successful: bool, duplicated: bool) -> String {
        let mut parts = vec![];
        if !successful {
            parts.push(self.label_failed());
        }
        if duplicated {
            parts.push(self.label_duplicated());
        }
        parts.push(item.to_string());

        match self.language {
            Language::English => format!("  - {}", parts.join(" ")),
        }
    }

    pub fn cli_game_line_item_redirected(&self, item: &str) -> String {
        match self.language {
            Language::English => format!("    - Redirected from: {}", item),
        }
    }

    pub fn cli_summary(&self, status: &OperationStatus, location: &StrictPath) -> String {
        if status.completed() {
            match self.language {
                Language::English => format!(
                    "\nOverall:\n  Games: {}\n  Size: {}\n  Location: {}",
                    status.total_games,
                    self.adjusted_size(status.total_bytes),
                    location.render()
                ),
            }
        } else {
            match self.language {
                Language::English => format!(
                    "\nOverall:\n  Games: {} of {}\n  Size: {} of {}\n  Location: {}",
                    status.processed_games,
                    status.total_games,
                    self.adjusted_size(status.processed_bytes),
                    self.adjusted_size(status.total_bytes),
                    location.render()
                ),
            }
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

    pub fn nav_custom_games_button(&self) -> String {
        match self.language {
            Language::English => "CUSTOM GAMES",
        }
        .into()
    }

    pub fn nav_other_button(&self) -> String {
        match self.language {
            Language::English => "OTHER",
        }
        .into()
    }

    pub fn add_root_button(&self) -> String {
        match self.language {
            Language::English => "Add root",
        }
        .into()
    }

    pub fn add_redirect_button(&self) -> String {
        match self.language {
            Language::English => "Add redirect",
        }
        .into()
    }

    pub fn add_game_button(&self) -> String {
        match self.language {
            Language::English => "Add game",
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

    pub fn enable_all_button(&self) -> String {
        match self.language {
            Language::English => "Enable all",
        }
        .into()
    }

    pub fn disable_all_button(&self) -> String {
        match self.language {
            Language::English => "Disable all",
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
            Language::English => {
                "Error: Unable to check for an update to the manifest file. Is your Internet connection down?"
            }
        }
        .into()
    }

    pub fn cannot_prepare_backup_target(&self, target: &StrictPath) -> String {
        match self.language {
            Language::English => format!("Error: Unable to prepare backup target (either creating or emptying the folder). If you have the folder open in your file browser, try closing it: {}", target.render()),
        }
    }

    pub fn restoration_source_is_invalid(&self, source: &StrictPath) -> String {
        match self.language {
            Language::English => {
                format!("Error: The restoration source is invalid (either doesn't exist or isn't a directory). Please double check the location: {}", source.render())
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

    pub fn unable_to_open_dir(&self, path: &StrictPath) -> String {
        match self.language {
            Language::English => format!("Error: Unable to open directory:\n\n{}", path.render()),
        }
    }

    pub fn unable_to_open_url(&self, url: &str) -> String {
        match self.language {
            Language::English => format!("Error: Unable to open URL:\n\n{}", url),
        }
    }

    pub fn adjusted_size(&self, bytes: u64) -> String {
        let byte = Byte::from_bytes(bytes.into());
        let adjusted_byte = byte.get_appropriate_unit(true);
        adjusted_byte.to_string()
    }

    pub fn processed_games(&self, status: &OperationStatus) -> String {
        if status.completed() {
            match self.language {
                Language::English => format!("{} games", status.total_games,),
            }
        } else {
            match self.language {
                Language::English => format!("{} of {} games", status.processed_games, status.total_games,),
            }
        }
    }

    pub fn processed_bytes(&self, status: &OperationStatus) -> String {
        if status.completed() {
            match self.language {
                Language::English => self.adjusted_size(status.total_bytes),
            }
        } else {
            match self.language {
                Language::English => format!(
                    "{} of {}",
                    self.adjusted_size(status.processed_bytes),
                    self.adjusted_size(status.total_bytes)
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

    pub fn backup_merge_label(&self) -> String {
        match self.language {
            Language::English => "Merge",
        }
        .into()
    }

    pub fn restore_source_label(&self) -> String {
        match self.language {
            Language::English => "Restore from:",
        }
        .into()
    }

    pub fn custom_files_label(&self) -> String {
        match self.language {
            Language::English => "Paths:",
        }
        .into()
    }

    pub fn custom_registry_label(&self) -> String {
        match self.language {
            Language::English => "Registry:",
        }
        .into()
    }

    pub fn search_label(&self) -> String {
        match self.language {
            Language::English => "Search:",
        }
        .into()
    }

    pub fn store(&self, store: &Store) -> String {
        match self.language {
            Language::English => match store {
                Store::Epic => "Epic",
                Store::Gog => "GOG",
                Store::GogGalaxy => "GOG Galaxy",
                Store::Microsoft => "Microsoft",
                Store::Origin => "Origin",
                Store::Steam => "Steam",
                Store::Uplay => "Uplay",
                Store::OtherHome => "Home folder",
                Store::OtherWine => "Wine prefix",
                Store::Other => "Other",
            },
        }
        .into()
    }

    pub fn redirect_source_placeholder(&self) -> String {
        match self.language {
            Language::English => "Source (original location)",
        }
        .into()
    }

    pub fn redirect_target_placeholder(&self) -> String {
        match self.language {
            Language::English => "Target (new location)",
        }
        .into()
    }

    pub fn custom_game_name_placeholder(&self) -> String {
        match self.language {
            Language::English => "Name",
        }
        .into()
    }

    pub fn search_game_name_placeholder(&self) -> String {
        match self.language {
            Language::English => "Name",
        }
        .into()
    }

    pub fn explanation_for_exclude_other_os_data(&self) -> String {
        match self.language {
            Language::English => "In backups, exclude save locations that have only been confirmed on another operating system. Some games always put saves in the same place, but the locations may have only been confirmed for a different OS, so it can help to check them anyway. Excluding that data may help to avoid false positives, but may also mean missing out on some saves. On Linux, Proton saves will still be backed up regardless of this setting.",
        }
        .into()
    }

    pub fn explanation_for_exclude_store_screenshots(&self) -> String {
        match self.language {
            Language::English => "In backups, exclude store-specific screenshots. Right now, this only applies to Steam screenshots that you've taken. If a game has its own built-in screenshot functionality, this setting will not affect whether those screenshots are backed up.",
        }
        .into()
    }

    pub fn modal_confirm_backup(&self, target: &StrictPath, target_exists: bool, merge: bool) -> String {
        match (self.language, target_exists, merge) {
            (Language::English, false, _) => format!("Are you sure you want to proceed with the backup? The target folder will be created:\n\n{}\n\n{}", target.render(), self.modal_consider_doing_a_preview()),
            (Language::English, true, false) => format!("Are you sure you want to proceed with the backup? The target folder will be deleted and recreated from scratch:\n\n{}\n\n{}", target.render(), self.modal_consider_doing_a_preview()),
            (Language::English, true, true) => format!("Are you sure you want to proceed with the backup? New save data will be merged into the target folder:\n\n{}\n\n{}", target.render(), self.modal_consider_doing_a_preview()),
        }
    }

    pub fn modal_confirm_restore(&self, source: &StrictPath) -> String {
        match self.language {
            Language::English => format!("Are you sure you want to proceed with the restoration? This will overwrite any current files with the backups from here:\n\n{}\n\n{}", source.render(), self.modal_consider_doing_a_preview()),
        }
    }

    pub fn modal_consider_doing_a_preview(&self) -> String {
        match self.language {
            Language::English => {
                "If you haven't already, consider doing a preview first so that there are no surprises."
            }
        }
        .into()
    }
}
