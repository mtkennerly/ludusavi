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
    pub fn handle_error(&self, error: &Error) -> String {
        match error {
            Error::ConfigInvalid { why } => self.config_is_invalid(why),
            Error::ManifestInvalid { why } => self.manifest_is_invalid(why),
            Error::ManifestCannotBeUpdated => self.manifest_cannot_be_updated(),
            Error::CannotPrepareBackupTarget => self.cannot_prepare_backup_target(),
        }
    }

    pub fn backup_button(&self) -> String {
        match self.language {
            Language::English => "Back up",
        }
        .into()
    }

    pub fn scan_button(&self) -> String {
        match self.language {
            Language::English => "Scan",
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
            Language::English => "Error: Unable to prepare backup target by either cleaning or creating the folder).",
        }
        .into()
    }

    pub fn processed_games(&self, total: usize) -> String {
        match self.language {
            Language::English => format!("{} games", total),
        }
    }
}
