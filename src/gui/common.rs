use crate::{
    config::{BackupFormat, RootsConfig, SortKey, Theme, ZipCompression},
    gui::badge::Badge,
    lang::{Language, Translator},
    layout::Backup,
    manifest::Store,
    prelude::{BackupInfo, Error, OperationStatus, OperationStepDecision, RegistryItem, ScanInfo, StrictPath},
    shortcuts::{Shortcut, TextHistory},
};

use iced::{Alignment, Row, Text};

#[derive(Debug, Clone)]
pub enum Message {
    Idle,
    Ignore,
    Error(Error),
    ConfirmBackupStart {
        games: Option<Vec<String>>,
    },
    BackupPrep {
        preview: bool,
        games: Option<Vec<String>>,
    },
    BackupStart {
        preview: bool,
        games: Option<Vec<String>>,
    },
    ConfirmRestoreStart {
        games: Option<Vec<String>>,
    },
    RestoreStart {
        preview: bool,
        games: Option<Vec<String>>,
    },
    BackupStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
        preview: bool,
    },
    RestoreStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
    },
    CancelOperation,
    ProcessGameOnDemand {
        game: String,
        restore: bool,
    },
    EditedBackupTarget(String),
    EditedBackupMerge(bool),
    EditedRestoreSource(String),
    FindRoots,
    ConfirmAddMissingRoots(Vec<RootsConfig>),
    EditedRoot(EditAction),
    SelectedRootStore(usize, Store),
    EditedRedirect(EditAction, Option<RedirectEditActionField>),
    EditedCustomGame(EditAction),
    EditedCustomGameFile(usize, EditAction),
    EditedCustomGameRegistry(usize, EditAction),
    EditedExcludeOtherOsData(bool),
    EditedExcludeStoreScreenshots(bool),
    EditedBackupFilterIgnoredPath(EditAction),
    EditedBackupFilterIgnoredRegistry(EditAction),
    SwitchScreen(Screen),
    ToggleGameListEntryExpanded {
        name: String,
    },
    ToggleGameListEntryTreeExpanded {
        name: String,
        keys: Vec<String>,
    },
    ToggleGameListEntryEnabled {
        name: String,
        enabled: bool,
        restoring: bool,
    },
    ToggleSearch {
        screen: Screen,
    },
    ToggleSpecificBackupPathIgnored {
        name: String,
        path: StrictPath,
        enabled: bool,
    },
    ToggleSpecificBackupRegistryIgnored {
        name: String,
        path: RegistryItem,
        enabled: bool,
    },
    ToggleCustomGameEnabled {
        index: usize,
        enabled: bool,
    },
    EditedSearchGameName {
        screen: Screen,
        value: String,
    },
    EditedSortKey {
        screen: Screen,
        value: SortKey,
    },
    EditedSortReversed {
        screen: Screen,
        value: bool,
    },
    BrowseDir(BrowseSubject),
    BrowseDirFailure,
    SelectAllGames,
    DeselectAllGames,
    CustomizeGame {
        name: String,
    },
    OpenDir {
        path: StrictPath,
    },
    OpenDirFailure {
        path: StrictPath,
    },
    OpenWiki {
        game: String,
    },
    OpenUrlFailure {
        url: String,
    },
    SubscribedEvent(iced_native::Event),
    EditedFullRetention(u8),
    EditedDiffRetention(u8),
    SelectedBackupToRestore {
        game: String,
        backup: Backup,
    },
    SelectedLanguage(Language),
    SelectedTheme(Theme),
    SelectedBackupFormat(BackupFormat),
    SelectedBackupCompression(ZipCompression),
    ToggleBackupSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OngoingOperation {
    Backup,
    CancelBackup,
    PreviewBackup,
    CancelPreviewBackup,
    Restore,
    CancelRestore,
    PreviewRestore,
    CancelPreviewRestore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Backup,
    Restore,
    CustomGames,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    Add,
    Change(usize, String),
    Remove(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectEditActionField {
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseSubject {
    BackupTarget,
    RestoreSource,
    Root(usize),
    RedirectSource(usize),
    RedirectTarget(usize),
    CustomGameFile(usize, usize),
    BackupFilterIgnoredPath(usize),
}

impl Default for Screen {
    fn default() -> Self {
        Self::Backup
    }
}

pub fn apply_shortcut_to_strict_path_field(shortcut: &Shortcut, config: &mut StrictPath, history: &mut TextHistory) {
    match shortcut {
        Shortcut::Undo => {
            config.reset(history.undo());
        }
        Shortcut::Redo => {
            config.reset(history.redo());
        }
    }
}

pub fn apply_shortcut_to_registry_path_field(
    shortcut: &Shortcut,
    config: &mut RegistryItem,
    history: &mut TextHistory,
) {
    match shortcut {
        Shortcut::Undo => {
            config.reset(history.undo());
        }
        Shortcut::Redo => {
            config.reset(history.redo());
        }
    }
}

pub fn apply_shortcut_to_string_field(shortcut: &Shortcut, config: &mut String, history: &mut TextHistory) {
    match shortcut {
        Shortcut::Undo => {
            *config = history.undo();
        }
        Shortcut::Redo => {
            *config = history.redo();
        }
    }
}

pub fn make_status_row<'a>(
    translator: &Translator,
    status: &OperationStatus,
    found_any_duplicates: bool,
    theme: Theme,
) -> Row<'a, Message> {
    Row::new()
        .padding([0, 20, 0, 20])
        .align_items(Alignment::Center)
        .push(Text::new(translator.processed_games(status)).size(35))
        .push(Text::new("  |  ").size(35))
        .push(Text::new(translator.processed_bytes(status)).size(35))
        .push_if(
            || found_any_duplicates,
            || Badge::new(&translator.badge_duplicates()).left_margin(15).view(theme),
        )
}

pub trait IcedExtension<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<iced::Element<'a, Message>>;

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<iced::Element<'a, Message>>;
}

impl<'a> IcedExtension<'a> for iced::Column<'a, Message> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<iced::Element<'a, Message>>,
    {
        if condition() {
            self.push(element().into())
        } else {
            self
        }
    }

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<iced::Element<'a, Message>>,
    {
        if let Some(element) = element() {
            self.push(element.into())
        } else {
            self
        }
    }
}

impl<'a> IcedExtension<'a> for iced::Row<'a, Message> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<iced::Element<'a, Message>>,
    {
        if condition() {
            self.push(element().into())
        } else {
            self
        }
    }

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<iced::Element<'a, Message>>,
    {
        if let Some(element) = element() {
            self.push(element.into())
        } else {
            self
        }
    }
}

pub trait IcedButtonExt<'a> {
    fn on_press_some(self, msg: Option<Message>) -> Self;
}

impl<'a> IcedButtonExt<'a> for iced::Button<'a, Message> {
    fn on_press_some(self, msg: Option<Message>) -> Self {
        match msg {
            Some(msg) => self.on_press(msg),
            None => self,
        }
    }
}
