use crate::manifest::Store;
use crate::prelude::Error;

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
        format!("Ludusavi v{}", env!("CARGO_PKG_VERSION"))
    }

    pub fn handle_error(&self, error: &Error) -> String {
        match error {
            Error::ConfigInvalid { why } => self.config_is_invalid(why),
            Error::ManifestInvalid { why } => self.manifest_is_invalid(why),
            Error::ManifestCannotBeUpdated => self.manifest_cannot_be_updated(),
            Error::CannotPrepareBackupTarget => self.cannot_prepare_backup_target(),
            Error::RestorationSourceInvalid => self.restoration_source_is_invalid(),
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
            Language::English => "=> Backup",
        }
        .into()
    }

    pub fn nav_restore_button(&self) -> String {
        match self.language {
            Language::English => "=> Restore",
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

    pub fn no_roots_are_configured(&self) -> String {
        match self.language {
            Language::English => "Add some roots (e.g., Steam installation directory) to back up more data.",
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

    pub fn cannot_prepare_backup_target(&self) -> String {
        match self.language {
            Language::English => "Error: Unable to prepare backup target (either creating or emptying the folder).",
        }
        .into()
    }

    pub fn restoration_source_is_invalid(&self) -> String {
        match self.language {
            Language::English => {
                "Error: The restoration source is invalid (either doesn't exist or isn't a directory)."
            }
        }
        .into()
    }

    pub fn processed_games(&self, total: usize) -> String {
        match self.language {
            Language::English => format!("{} games", total),
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

    pub fn start_of_backup(&self) -> String {
        match self.language {
            Language::English => "[ Backup ]",
        }
        .into()
    }

    pub fn start_of_backup_preview(&self) -> String {
        match self.language {
            Language::English => "[ Backup Preview ]",
        }
        .into()
    }

    pub fn start_of_restore(&self) -> String {
        match self.language {
            Language::English => "[ Restore ]",
        }
        .into()
    }

    pub fn start_of_restore_preview(&self) -> String {
        match self.language {
            Language::English => "[ Restore Preview ]",
        }
        .into()
    }
}
