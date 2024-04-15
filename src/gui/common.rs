use std::collections::BTreeSet;

use iced::Length;

use crate::{
    cloud::{rclone_monitor, Remote, RemoteChoice},
    gui::{
        icon::Icon,
        modal::{ModalField, ModalInputKind},
    },
    lang::{Language, TRANSLATOR},
    prelude::{CommandError, Error, Finality, Privacy, StrictPath, SyncDirection},
    resource::{
        config::{
            BackupFormat, CustomGameKind, RedirectKind, RootsConfig, SecondaryManifestConfigKind, SortKey, Theme,
            ZipCompression,
        },
        manifest::{Manifest, ManifestUpdate, Store},
    },
    scan::{
        game_filter,
        layout::{Backup, BackupLayout, GameLayout},
        registry_compat::RegistryItem,
        BackupInfo, Launchers, OperationStepDecision, ScanInfo, SteamShortcuts,
    },
};

#[derive(Clone, Debug, Default)]
pub struct Flags {
    pub update_manifest: bool,
}

#[derive(Debug, Clone)]
pub enum BackupPhase {
    Confirm {
        games: Option<Vec<String>>,
    },
    Start {
        preview: bool,
        /// Was this backup triggered by a validation check?
        repair: bool,
        games: Option<Vec<String>>,
    },
    CloudCheck,
    Load,
    RegisterCommands {
        subjects: Vec<String>,
        manifest: Manifest,
        layout: Box<BackupLayout>,
        steam: SteamShortcuts,
        launchers: Launchers,
    },
    GameScanned {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
    },
    CloudSync,
    Done,
}

#[derive(Debug, Clone)]
pub enum RestorePhase {
    Confirm {
        games: Option<Vec<String>>,
    },
    Start {
        preview: bool,
        games: Option<Vec<String>>,
    },
    CloudCheck,
    Load,
    RegisterCommands {
        restorables: Vec<String>,
        layout: BackupLayout,
    },
    GameScanned {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
        game_layout: Box<GameLayout>,
    },
    Done,
}

#[derive(Debug, Clone)]
pub enum ValidatePhase {
    Start,
    Load,
    RegisterCommands {
        subjects: Vec<String>,
        layout: BackupLayout,
    },
    GameScanned {
        game: String,
        valid: bool,
    },
    Done,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ignore,
    Exit {
        user: bool,
    },
    Save,
    CloseModal,
    UpdateTime,
    PruneNotifications,
    UpdateManifest,
    ManifestUpdated(Vec<Result<Option<ManifestUpdate>, Error>>),
    Backup(BackupPhase),
    Restore(RestorePhase),
    ValidateBackups(ValidatePhase),
    CancelOperation,
    EditedBackupTarget(String),
    EditedRestoreSource(String),
    FindRoots,
    ConfirmAddMissingRoots(Vec<RootsConfig>),
    EditedRoot(EditAction),
    EditedSecondaryManifest(EditAction),
    SelectedRootStore(usize, Store),
    SelectedRedirectKind(usize, RedirectKind),
    SelectedSecondaryManifestKind(usize, SecondaryManifestConfigKind),
    SelectedCustomGameKind(usize, CustomGameKind),
    EditedRedirect(EditAction, Option<RedirectEditActionField>),
    EditedCustomGame(EditAction),
    EditedCustomGameAlias(usize, String),
    EditedCustomGaleAliasDisplay(usize, bool),
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
    ToggleSpecificGamePathIgnored {
        name: String,
        path: StrictPath,
        enabled: bool,
        restoring: bool,
    },
    ToggleSpecificGameRegistryIgnored {
        name: String,
        path: RegistryItem,
        value: Option<String>,
        enabled: bool,
        restoring: bool,
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
    EditedSearchFilterChange(game_filter::Change),
    EditedSortKey {
        screen: Screen,
        value: SortKey,
    },
    EditedSortReversed {
        screen: Screen,
        value: bool,
    },
    BrowseDir(BrowseSubject),
    BrowseFile(BrowseFileSubject),
    BrowseDirFailure,
    SelectedFile(BrowseFileSubject, StrictPath),
    SelectAllGames,
    DeselectAllGames,
    OpenDir {
        path: StrictPath,
    },
    OpenDirSubject(BrowseSubject),
    OpenFileSubject(BrowseFileSubject),
    OpenDirFailure {
        path: StrictPath,
    },
    OpenUrlFailure {
        url: String,
    },
    KeyboardEvent(iced::keyboard::Event),
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
    ToggleCloudSynchronize,
    GameAction {
        action: GameAction,
        game: String,
    },
    UndoRedo(crate::gui::undoable::Action, UndoSubject),
    Scrolled {
        subject: ScrollSubject,
        position: iced::widget::scrollable::AbsoluteOffset,
    },
    Scroll {
        subject: ScrollSubject,
        position: iced::widget::scrollable::AbsoluteOffset,
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
    EditedRcloneExecutable(String),
    EditedRcloneArguments(String),
    EditedCloudRemoteId(String),
    EditedCloudPath(String),
    OpenUrl(String),
    EditedCloudRemote(RemoteChoice),
    ConfigureCloudSuccess(Remote),
    ConfigureCloudFailure(CommandError),
    ConfirmSynchronizeCloud {
        direction: SyncDirection,
    },
    SynchronizeCloud {
        direction: SyncDirection,
        finality: Finality,
    },
    RcloneMonitor(rclone_monitor::Event),
    FinalizeRemote(Remote),
    EditedModalField(ModalField),
    ModalChangePage(usize),
    ShowCustomGame {
        name: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Operation {
    #[default]
    Idle,
    Backup {
        finality: Finality,
        cancelling: bool,
        checking_cloud: bool,
        syncing_cloud: bool,
        should_sync_cloud_after: bool,
        games: Option<Vec<String>>,
        errors: Vec<Error>,
        cloud_changes: i64,
        force_new_full_backup: bool,
    },
    Restore {
        finality: Finality,
        cancelling: bool,
        checking_cloud: bool,
        games: Option<Vec<String>>,
        errors: Vec<Error>,
        cloud_changes: i64,
    },
    ValidateBackups {
        cancelling: bool,
        faulty_games: BTreeSet<String>,
    },
    Cloud {
        direction: SyncDirection,
        finality: Finality,
        cancelling: bool,
        errors: Vec<Error>,
        cloud_changes: i64,
    },
}

impl Operation {
    pub fn idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn new_backup(finality: Finality, games: Option<Vec<String>>) -> Self {
        Self::Backup {
            finality,
            cancelling: false,
            checking_cloud: false,
            syncing_cloud: false,
            should_sync_cloud_after: false,
            games,
            errors: vec![],
            cloud_changes: 0,
            force_new_full_backup: false,
        }
    }

    pub fn new_restore(finality: Finality, games: Option<Vec<String>>) -> Self {
        Self::Restore {
            finality,
            cancelling: false,
            checking_cloud: false,
            games,
            errors: vec![],
            cloud_changes: 0,
        }
    }

    pub fn new_validate_backups() -> Self {
        Self::ValidateBackups {
            cancelling: false,
            faulty_games: Default::default(),
        }
    }

    pub fn new_cloud(direction: SyncDirection, finality: Finality) -> Self {
        Self::Cloud {
            direction,
            finality,
            cancelling: false,
            errors: vec![],
            cloud_changes: 0,
        }
    }

    pub fn preview(&self) -> bool {
        match self {
            Operation::Idle => true,
            Operation::Backup { finality, .. } => finality.preview(),
            Operation::Restore { finality, .. } => finality.preview(),
            Operation::ValidateBackups { .. } => true,
            Operation::Cloud { finality, .. } => finality.preview(),
        }
    }

    pub fn full(&self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup { games, .. } => games.is_none(),
            Operation::Restore { games, .. } => games.is_none(),
            Operation::ValidateBackups { .. } => true,
            Operation::Cloud { .. } => true,
        }
    }

    pub fn games(&self) -> Option<Vec<String>> {
        match self {
            Operation::Idle => None,
            Operation::Backup { games, .. } => games.clone(),
            Operation::Restore { games, .. } => games.clone(),
            Operation::ValidateBackups { .. } => None,
            Operation::Cloud { .. } => None,
        }
    }

    pub fn flag_cancel(&mut self) {
        match self {
            Operation::Idle => (),
            Operation::Backup { cancelling, .. } => *cancelling = true,
            Operation::Restore { cancelling, .. } => *cancelling = true,
            Operation::ValidateBackups { cancelling, .. } => *cancelling = true,
            Operation::Cloud { cancelling, .. } => *cancelling = true,
        }
    }

    pub fn errors(&self) -> Option<&Vec<Error>> {
        match self {
            Operation::Idle => None,
            Operation::Backup { errors, .. } => Some(errors),
            Operation::Restore { errors, .. } => Some(errors),
            Operation::ValidateBackups { .. } => None,
            Operation::Cloud { errors, .. } => Some(errors),
        }
    }

    pub fn push_error(&mut self, error: Error) {
        match self {
            Operation::Idle => (),
            Operation::Backup { errors, .. } => errors.push(error),
            Operation::Restore { errors, .. } => errors.push(error),
            Operation::ValidateBackups { .. } => (),
            Operation::Cloud { errors, .. } => errors.push(error),
        }
    }

    pub fn update_integrated_cloud(&mut self, finality: Finality) {
        match self {
            Operation::Idle => (),
            Operation::Backup {
                checking_cloud,
                syncing_cloud,
                ..
            } => match finality {
                Finality::Preview => *checking_cloud = true,
                Finality::Final => *syncing_cloud = true,
            },
            Operation::Restore { checking_cloud, .. } => match finality {
                Finality::Preview => *checking_cloud = true,
                Finality::Final => (),
            },
            Operation::ValidateBackups { .. } => (),
            Operation::Cloud { .. } => (),
        }
    }

    pub fn transition_from_cloud_step(&mut self, synced: bool) {
        let preview = self.preview();

        match self {
            Operation::Idle => (),
            Operation::Backup {
                checking_cloud,
                syncing_cloud,
                should_sync_cloud_after,
                ..
            } => {
                if *checking_cloud {
                    *checking_cloud = false;
                    *should_sync_cloud_after = synced && !preview;
                    if !synced {
                        self.push_error(Error::CloudConflict);
                    }
                } else if *syncing_cloud {
                    *syncing_cloud = false;
                }
            }
            Operation::Restore { checking_cloud, .. } => {
                if *checking_cloud {
                    *checking_cloud = false;
                }
            }
            Operation::ValidateBackups { .. } => (),
            Operation::Cloud { .. } => (),
        }
    }

    pub fn is_cloud_active(&self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup {
                checking_cloud,
                syncing_cloud,
                ..
            } => *checking_cloud || *syncing_cloud,
            Operation::Restore { checking_cloud, .. } => *checking_cloud,
            Operation::ValidateBackups { .. } => false,
            Operation::Cloud { .. } => true,
        }
    }

    pub fn integrated_checking_cloud(&self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup { checking_cloud, .. } => *checking_cloud,
            Operation::Restore { checking_cloud, .. } => *checking_cloud,
            Operation::ValidateBackups { .. } => false,
            Operation::Cloud { .. } => false,
        }
    }

    pub fn integrated_syncing_cloud(&self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup { syncing_cloud, .. } => *syncing_cloud,
            Operation::Restore { .. } => false,
            Operation::ValidateBackups { .. } => false,
            Operation::Cloud { .. } => false,
        }
    }

    pub fn should_sync_cloud_after(&self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup {
                should_sync_cloud_after,
                ..
            } => *should_sync_cloud_after,
            Operation::Restore { .. } => false,
            Operation::ValidateBackups { .. } => false,
            Operation::Cloud { .. } => false,
        }
    }

    pub fn cloud_changes(&self) -> i64 {
        match self {
            Operation::Idle => 0,
            Operation::Backup { cloud_changes, .. } => *cloud_changes,
            Operation::Restore { cloud_changes, .. } => *cloud_changes,
            Operation::ValidateBackups { .. } => 0,
            Operation::Cloud { cloud_changes, .. } => *cloud_changes,
        }
    }

    pub fn add_cloud_change(&mut self) {
        match self {
            Operation::Idle => (),
            Operation::Backup { cloud_changes, .. } => *cloud_changes += 1,
            Operation::Restore { cloud_changes, .. } => *cloud_changes += 1,
            Operation::ValidateBackups { .. } => (),
            Operation::Cloud { cloud_changes, .. } => *cloud_changes += 1,
        }
    }

    pub fn should_force_new_full_backups(&mut self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup {
                force_new_full_backup, ..
            } => *force_new_full_backup,
            Operation::Restore { .. } => false,
            Operation::ValidateBackups { .. } => false,
            Operation::Cloud { .. } => false,
        }
    }

    pub fn set_force_new_full_backups(&mut self, value: bool) {
        match self {
            Operation::Idle => (),
            Operation::Backup {
                force_new_full_backup, ..
            } => *force_new_full_backup = value,
            Operation::Restore { .. } => (),
            Operation::ValidateBackups { .. } => (),
            Operation::Cloud { .. } => (),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseFileSubject {
    RcloneExecutable,
    SecondaryManifest(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UndoSubject {
    BackupTarget,
    RestoreSource,
    BackupSearchGameName,
    RestoreSearchGameName,
    Root(usize),
    SecondaryManifest(usize),
    RedirectSource(usize),
    RedirectTarget(usize),
    CustomGameName(usize),
    CustomGameAlias(usize),
    CustomGameFile(usize, usize),
    CustomGameRegistry(usize, usize),
    BackupFilterIgnoredPath(usize),
    BackupFilterIgnoredRegistry(usize),
    RcloneExecutable,
    RcloneArguments,
    CloudRemoteId,
    CloudPath,
    ModalField(ModalInputKind),
    BackupComment(String),
}

impl UndoSubject {
    pub fn privacy(&self) -> Privacy {
        match self {
            UndoSubject::BackupTarget
            | UndoSubject::RestoreSource
            | UndoSubject::BackupSearchGameName
            | UndoSubject::RestoreSearchGameName
            | UndoSubject::Root(_)
            | UndoSubject::SecondaryManifest(_)
            | UndoSubject::RedirectSource(_)
            | UndoSubject::RedirectTarget(_)
            | UndoSubject::CustomGameName(_)
            | UndoSubject::CustomGameAlias(_)
            | UndoSubject::CustomGameFile(_, _)
            | UndoSubject::CustomGameRegistry(_, _)
            | UndoSubject::BackupFilterIgnoredPath(_)
            | UndoSubject::BackupFilterIgnoredRegistry(_)
            | UndoSubject::RcloneExecutable
            | UndoSubject::RcloneArguments
            | UndoSubject::CloudRemoteId
            | UndoSubject::CloudPath
            | UndoSubject::BackupComment(_) => Privacy::Public,
            UndoSubject::ModalField(field) => match field {
                ModalInputKind::Url | ModalInputKind::Host | ModalInputKind::Port | ModalInputKind::Username => {
                    Privacy::Public
                }
                ModalInputKind::Password => Privacy::Private,
            },
        }
    }
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

    pub fn id(&self) -> iced::widget::scrollable::Id {
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
            .on_scroll(move |viewport| Message::Scrolled {
                subject: self,
                position: viewport.absolute_offset(),
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
    Lock,
    Unlock,
    MakeAlias,
}

impl GameAction {
    pub fn options(
        restoring: bool,
        operating: bool,
        customized: bool,
        invented: bool,
        has_backups: bool,
        locked: bool,
    ) -> Vec<Self> {
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

        options.push(Self::MakeAlias);

        if restoring && has_backups {
            options.push(Self::Comment);

            if locked {
                options.push(Self::Unlock);
            } else {
                options.push(Self::Lock);
            }
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
            GameAction::Lock => Icon::Lock,
            GameAction::Unlock => Icon::LockOpen,
            GameAction::MakeAlias => Icon::Edit,
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
            Self::Lock => TRANSLATOR.lock_button(),
            Self::Unlock => TRANSLATOR.unlock_button(),
            Self::MakeAlias => TRANSLATOR.alias_label(),
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
