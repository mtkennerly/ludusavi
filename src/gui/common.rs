use iced::Length;

use crate::{
    gui::icon::Icon,
    lang::{Language, TRANSLATOR},
    prelude::{Error, StrictPath},
    resource::{
        config::{BackupFormat, RedirectKind, RootsConfig, SortKey, Theme, ZipCompression},
        manifest::{Manifest, ManifestUpdate, Store},
    },
    scan::{
        game_filter,
        heroic::HeroicGames,
        layout::{Backup, BackupLayout, GameLayout},
        registry_compat::RegistryItem,
        BackupInfo, InstallDirRanking, OperationStepDecision, ScanInfo, SteamShortcuts,
    },
};

#[derive(Debug, Clone)]
pub enum Message {
    Ignore,
    Error(Error),
    Exit,
    CloseModal,
    PruneNotifications,
    UpdateManifest,
    ManifestUpdated(Result<Option<ManifestUpdate>, Error>),
    ConfirmBackupStart {
        games: Option<Vec<String>>,
    },
    /// This phase is fallible and may need to show an error to the user.
    BackupPrep {
        preview: bool,
        games: Option<Vec<String>>,
    },
    /// This phase can be slow, so we keep it separate so things run smoothly.
    BackupStart {
        preview: bool,
        games: Option<Vec<String>>,
    },
    /// This phase is when we register a `Command` for each game.
    BackupPerform {
        preview: bool,
        full: bool,
        games: Option<Vec<String>>,
        subjects: Vec<String>,
        all_games: Manifest,
        layout: BackupLayout,
        ranking: InstallDirRanking,
        steam: SteamShortcuts,
        heroic: HeroicGames,
    },
    ConfirmRestoreStart {
        games: Option<Vec<String>>,
    },
    /// This phase can be slow, so we keep it separate so things run smoothly.
    RestoreStart {
        preview: bool,
        games: Option<Vec<String>>,
    },
    /// This phase is when we register a `Command` for each game.
    RestorePerform {
        preview: bool,
        full: bool,
        games: Option<Vec<String>>,
        restorables: Vec<String>,
        layout: BackupLayout,
    },
    BackupStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
        preview: bool,
        full: bool,
    },
    RestoreStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
        full: bool,
        game_layout: GameLayout,
    },
    CancelOperation,
    EditedBackupTarget(String),
    EditedBackupMerge(bool),
    EditedRestoreSource(String),
    FindRoots,
    ConfirmAddMissingRoots(Vec<RootsConfig>),
    EditedRoot(EditAction),
    SelectedRootStore(usize, Store),
    SelectedRedirectKind(usize, RedirectKind),
    EditedRedirect(EditAction, Option<RedirectEditActionField>),
    EditedCustomGame(EditAction),
    EditedCustomGameFile(usize, EditAction),
    EditedCustomGameRegistry(usize, EditAction),
    EditedExcludeStoreScreenshots(bool),
    EditedBackupFilterIgnoredPath(EditAction),
    EditedBackupFilterIgnoredRegistry(EditAction),
    SwitchScreen(Screen),
    ToggleGameListEntryExpanded {
        name: String,
    },
    ToggleGameListEntryTreeExpanded {
        name: String,
        keys: Vec<TreeNodeKey>,
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
        value: Option<String>,
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
    ToggledSearchFilter {
        filter: game_filter::FilterKind,
        enabled: bool,
    },
    EditedSearchFilterUniqueness(game_filter::Uniqueness),
    EditedSearchFilterCompleteness(game_filter::Completeness),
    EditedSearchFilterEnablement(game_filter::Enablement),
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
    OpenDir {
        path: StrictPath,
    },
    OpenDirFailure {
        path: StrictPath,
    },
    OpenUrlFailure {
        url: String,
    },
    KeyboardEvent(iced_native::keyboard::Event),
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
    EditedCompressionLevel(i32),
    ToggleBackupSettings,
    GameAction {
        action: GameAction,
        game: String,
    },
    UndoRedo(crate::gui::undoable::Action, UndoSubject),
    Scroll {
        subject: ScrollSubject,
        position: iced_native::widget::scrollable::RelativeOffset,
    },
    EditedBackupComment {
        game: String,
        comment: String,
    },
    SetShowDeselectedGames(bool),
    SetShowUnchangedGames(bool),
    SetShowUnscannedGames(bool),
    FilterDuplicates {
        restoring: bool,
        game: Option<String>,
    },
    OverrideMaxThreads(bool),
    EditedMaxThreads(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Backup,
    Restore,
    CustomGames,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditDirection {
    Up,
    Down,
}

impl EditDirection {
    pub fn shift(&self, index: usize) -> usize {
        match self {
            Self::Up => index - 1,
            Self::Down => index + 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    Add,
    Change(usize, String),
    Remove(usize),
    Move(usize, EditDirection),
}

impl EditAction {
    pub fn move_up(index: usize) -> Self {
        Self::Move(index, EditDirection::Up)
    }

    pub fn move_down(index: usize) -> Self {
        Self::Move(index, EditDirection::Down)
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UndoSubject {
    BackupTarget,
    RestoreSource,
    BackupSearchGameName,
    RestoreSearchGameName,
    Root(usize),
    RedirectSource(usize),
    RedirectTarget(usize),
    CustomGameName(usize),
    CustomGameFile(usize, usize),
    CustomGameRegistry(usize, usize),
    BackupFilterIgnoredPath(usize),
    BackupFilterIgnoredRegistry(usize),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ScrollSubject {
    Backup,
    Restore,
    CustomGames,
    Other,
    Modal,
}

impl ScrollSubject {
    pub fn game_list(restoring: bool) -> Self {
        if restoring {
            Self::Restore
        } else {
            Self::Backup
        }
    }

    pub fn id(&self) -> iced_native::widget::scrollable::Id {
        match self {
            Self::Backup => crate::gui::widget::id::backup_scroll(),
            Self::Restore => crate::gui::widget::id::restore_scroll(),
            Self::CustomGames => crate::gui::widget::id::custom_games_scroll(),
            Self::Other => crate::gui::widget::id::other_scroll(),
            Self::Modal => crate::gui::widget::id::modal_scroll(),
        }
    }

    pub fn into_widget<'a>(
        self,
        content: impl Into<crate::gui::widget::Element<'a>>,
    ) -> crate::gui::widget::Scrollable<'a> {
        crate::gui::widget::Scrollable::new(content)
            .height(Length::Fill)
            .style(crate::gui::style::Scrollable)
            .id(self.id())
            .on_scroll(move |position| Message::Scroll {
                subject: self,
                position,
            })
    }
}

impl From<Screen> for ScrollSubject {
    fn from(value: Screen) -> Self {
        match value {
            Screen::Backup => Self::Backup,
            Screen::Restore => Self::Restore,
            Screen::CustomGames => Self::CustomGames,
            Screen::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    Customize,
    PreviewBackup,
    Backup { confirm: bool },
    PreviewRestore,
    Restore { confirm: bool },
    Wiki,
    Comment,
}

impl GameAction {
    pub fn options(restoring: bool, operating: bool, customized: bool, invented: bool, has_backups: bool) -> Vec<Self> {
        let mut options = vec![];

        if !operating {
            if restoring {
                options.push(Self::PreviewRestore);
                options.push(Self::Restore { confirm: true });
            } else {
                options.push(Self::PreviewBackup);
                options.push(Self::Backup { confirm: true });
            }
        }

        if !restoring && !customized {
            options.push(Self::Customize);
        }

        if restoring && has_backups {
            options.push(Self::Comment);
        }

        if !invented {
            options.push(Self::Wiki);
        }

        options
    }

    pub fn icon(&self) -> Icon {
        match self {
            GameAction::Backup { confirm } | GameAction::Restore { confirm } => {
                if *confirm {
                    Icon::PlayCircleOutline
                } else {
                    Icon::FastForward
                }
            }
            GameAction::PreviewBackup | GameAction::PreviewRestore => Icon::Refresh,
            GameAction::Customize => Icon::Edit,
            GameAction::Wiki => Icon::Language,
            GameAction::Comment => Icon::Comment,
        }
    }
}

impl ToString for GameAction {
    fn to_string(&self) -> String {
        match self {
            Self::PreviewBackup | Self::PreviewRestore => TRANSLATOR.preview_button(),
            Self::Backup { confirm } => {
                if *confirm {
                    TRANSLATOR.backup_button()
                } else {
                    TRANSLATOR.backup_button_no_confirmation()
                }
            }
            Self::Restore { confirm } => {
                if *confirm {
                    TRANSLATOR.restore_button()
                } else {
                    TRANSLATOR.restore_button_no_confirmation()
                }
            }
            Self::Customize => TRANSLATOR.customize_button(),
            Self::Wiki => TRANSLATOR.pcgamingwiki(),
            Self::Comment => TRANSLATOR.comment_button(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TreeNodeKey {
    File(String),
    RegistryKey(String),
    RegistryValue(String),
}

impl TreeNodeKey {
    pub fn raw(&self) -> &str {
        match self {
            Self::File(x) => x,
            Self::RegistryKey(x) => x,
            Self::RegistryValue(x) => x,
        }
    }
}
