use crate::{
    gui::badge::Badge,
    lang::Translator,
    manifest::Store,
    prelude::{BackupInfo, OperationStatus, OperationStepDecision, ScanInfo, StrictPath},
    shortcuts::{Shortcut, TextHistory},
};

use iced::{Alignment, Row, Text};

#[derive(Debug, Clone)]
pub enum Message {
    Idle,
    Ignore,
    ConfirmBackupStart,
    BackupStart {
        preview: bool,
    },
    ConfirmRestoreStart,
    RestoreStart {
        preview: bool,
    },
    BackupStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
    },
    RestoreStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
    },
    CancelOperation,
    BackupComplete,
    RestoreComplete,
    EditedBackupTarget(String),
    EditedBackupMerge(bool),
    EditedRestoreSource(String),
    AddMissingRoots,
    EditedRoot(EditAction),
    SelectedRootStore(usize, Store),
    EditedRedirect(EditAction, Option<RedirectEditActionField>),
    EditedCustomGame(EditAction),
    EditedCustomGameFile(usize, EditAction),
    EditedCustomGameRegistry(usize, EditAction),
    EditedExcludeOtherOsData(bool),
    EditedExcludeStoreScreenshots(bool),
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
    ToggleCustomGameEnabled {
        index: usize,
        enabled: bool,
    },
    EditedSearchGameName {
        screen: Screen,
        value: String,
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
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Backup,
    Restore,
    CustomGames,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    Add,
    Change(usize, String),
    Remove(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectEditActionField {
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrowseSubject {
    BackupTarget,
    RestoreSource,
    Root(usize),
    RedirectSource(usize),
    RedirectTarget(usize),
    CustomGameFile(usize, usize),
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
    (selected_games, selected_bytes): (usize, u64),
    found_any_duplicates: bool,
) -> Row<'a, Message> {
    let show_selection = selected_games != status.processed_games || selected_bytes != status.processed_bytes;
    Row::new()
        .padding(20)
        .align_items(Alignment::Center)
        .push(Text::new(translator.processed_games(status)).size(35))
        .push(Text::new("  |  ").size(35))
        .push(Text::new(translator.processed_bytes(status)).size(35))
        .push(
            Badge::new(&translator.badge_duplicates())
                .left_margin(15)
                .view_if(found_any_duplicates),
        )
        .push(
            Badge::new(&translator.badge_selected_games(selected_games, selected_bytes))
                .left_margin(15)
                .view_if(show_selection),
        )
}
