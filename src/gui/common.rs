use std::collections::{BTreeSet, HashMap, HashSet};

use iced::{widget::text_input, Length};

use crate::{
    cloud::{rclone_monitor, Remote, RemoteChoice},
    gui::{
        icon::Icon,
        modal::{ModalField, ModalInputKind},
    },
    lang::TRANSLATOR,
    prelude::{CommandError, EditAction, Error, Finality, Privacy, RedirectEditActionField, StrictPath, SyncDirection},
    resource::{
        config::{self, Root},
        manifest::{self, Manifest, ManifestUpdate},
    },
    scan::{
        game_filter,
        layout::{Backup, BackupLayout, GameLayout},
        registry::RegistryItem,
        BackupInfo, Launchers, ScanInfo, ScanKind, SteamShortcuts,
    },
};

pub const ERROR_ICON: text_input::Icon<iced::Font> = text_input::Icon {
    font: crate::gui::font::ICONS,
    code_point: crate::gui::icon::Icon::Error.as_char(),
    size: None,
    spacing: 5.0,
    side: text_input::Side::Right,
};

#[derive(Clone, Debug, Default)]
pub struct Flags {
    pub update_manifest: bool,
    pub custom_game: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BackupPhase {
    Confirm {
        games: Option<GameSelection>,
    },
    Start {
        preview: bool,
        /// Was this backup triggered by a validation check?
        repair: bool,
        /// Jump to the first game in the list after executing.
        jump: bool,
        games: Option<GameSelection>,
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
    },
    CloudSync,
    Done,
}

#[derive(Debug, Clone)]
pub enum RestorePhase {
    Confirm {
        games: Option<GameSelection>,
    },
    Start {
        preview: bool,
        games: Option<GameSelection>,
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
    Config {
        event: config::Event,
    },
    CheckAppRelease,
    AppReleaseChecked(Result<crate::metadata::Release, String>),
    UpdateManifest {
        force: bool,
    },
    ManifestUpdated(Vec<Result<Option<ManifestUpdate>, Error>>),
    Backup(BackupPhase),
    Restore(RestorePhase),
    ValidateBackups(ValidatePhase),
    CancelOperation,
    FindRoots,
    ConfirmAddMissingRoots(Vec<Root>),
    SwitchScreen(Screen),
    ToggleGameListEntryExpanded {
        name: String,
    },
    ToggleGameListEntryTreeExpanded {
        name: String,
        keys: Vec<TreeNodeKey>,
    },
    ToggleCustomGameExpanded {
        index: usize,
        expanded: bool,
    },
    Filter {
        event: game_filter::Event,
    },
    BrowseDir(BrowseSubject),
    BrowseFile(BrowseFileSubject),
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
    SelectedBackupToRestore {
        game: String,
        backup: Backup,
    },
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
    ShowGameNotes {
        game: String,
        notes: Vec<manifest::Note>,
    },
    EditedBackupComment {
        game: String,
        action: iced::widget::text_editor::Action,
    },
    FilterDuplicates {
        scan_kind: ScanKind,
        game: Option<String>,
    },
    OpenUrl(String),
    OpenUrlAndCloseModal(String),
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
    ShowScanActiveGames,
    CopyText(String),
    OpenRegistry(RegistryItem),
}

impl Message {
    pub fn browsed_dir(subject: BrowseSubject, choice: Option<std::path::PathBuf>) -> Self {
        match choice {
            Some(path) => match subject {
                BrowseSubject::BackupTarget => config::Event::BackupTarget(crate::path::render_pathbuf(&path)),
                BrowseSubject::RestoreSource => config::Event::RestoreSource(crate::path::render_pathbuf(&path)),
                BrowseSubject::Root(i) => config::Event::Root(EditAction::Change(
                    i,
                    globetter::Pattern::escape(&crate::path::render_pathbuf(&path)),
                )),
                BrowseSubject::RedirectSource(i) => config::Event::Redirect(
                    EditAction::Change(i, crate::path::render_pathbuf(&path)),
                    Some(RedirectEditActionField::Source),
                ),
                BrowseSubject::RedirectTarget(i) => config::Event::Redirect(
                    EditAction::Change(i, crate::path::render_pathbuf(&path)),
                    Some(RedirectEditActionField::Target),
                ),
                BrowseSubject::CustomGameFile(i, j) => config::Event::CustomGameFile(
                    i,
                    EditAction::Change(j, globetter::Pattern::escape(&crate::path::render_pathbuf(&path))),
                ),
                BrowseSubject::BackupFilterIgnoredPath(i) => {
                    config::Event::BackupFilterIgnoredPath(EditAction::Change(i, crate::path::render_pathbuf(&path)))
                }
            }
            .into(),
            None => Message::Ignore,
        }
    }

    pub fn browsed_file(subject: BrowseFileSubject, choice: Option<std::path::PathBuf>) -> Self {
        match choice {
            Some(path) => Message::SelectedFile(subject, StrictPath::from(path)),
            None => Message::Ignore,
        }
    }

    pub fn config<T>(convert: impl Fn(T) -> config::Event) -> impl Fn(T) -> Self {
        move |value: T| Self::Config { event: convert(value) }
    }

    pub fn config2<T, T2>(convert: impl Fn(T, T2) -> config::Event) -> impl Fn(T, T2) -> Self {
        move |v1: T, v2: T2| Self::Config { event: convert(v1, v2) }
    }
}

impl From<config::Event> for Message {
    fn from(event: config::Event) -> Self {
        Self::Config { event }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameSelection {
    Single { game: String },
    Group { games: HashSet<String> },
}

impl GameSelection {
    pub fn single(game: String) -> Self {
        Self::Single { game }
    }

    pub fn group(games: HashSet<String>) -> Self {
        Self::Group { games }
    }

    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single { .. })
    }

    pub fn contains(&self, query: &str) -> bool {
        match self {
            Self::Single { game } => game == query,
            Self::Group { games } => games.contains(query),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Single { .. } => false,
            Self::Group { games } => games.is_empty(),
        }
    }

    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a String> + 'a> {
        match self {
            Self::Single { game } => Box::new(std::iter::once(game)),
            Self::Group { games } => Box::new(games.iter()),
        }
    }
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
        games: Option<GameSelection>,
        errors: Vec<Error>,
        cloud_changes: i64,
        force_new_full_backup: bool,
        syncable_games: HashSet<String>,
        active_games: HashMap<String, chrono::DateTime<chrono::Utc>>,
    },
    Restore {
        finality: Finality,
        cancelling: bool,
        checking_cloud: bool,
        games: Option<GameSelection>,
        errors: Vec<Error>,
        cloud_changes: i64,
        active_games: HashMap<String, chrono::DateTime<chrono::Utc>>,
    },
    ValidateBackups {
        cancelling: bool,
        faulty_games: BTreeSet<String>,
        active_games: HashMap<String, chrono::DateTime<chrono::Utc>>,
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

    pub fn new_backup(finality: Finality, games: Option<GameSelection>) -> Self {
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
            syncable_games: HashSet::new(),
            active_games: HashMap::new(),
        }
    }

    pub fn new_restore(finality: Finality, games: Option<GameSelection>) -> Self {
        Self::Restore {
            finality,
            cancelling: false,
            checking_cloud: false,
            games,
            errors: vec![],
            cloud_changes: 0,
            active_games: HashMap::new(),
        }
    }

    pub fn new_validate_backups() -> Self {
        Self::ValidateBackups {
            cancelling: false,
            faulty_games: Default::default(),
            active_games: HashMap::new(),
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

    pub fn games(&self) -> Option<&GameSelection> {
        match self {
            Operation::Idle => None,
            Operation::Backup { games, .. } => games.as_ref(),
            Operation::Restore { games, .. } => games.as_ref(),
            Operation::ValidateBackups { .. } => None,
            Operation::Cloud { .. } => None,
        }
    }

    pub fn games_specified(&self) -> bool {
        match self {
            Operation::Idle => false,
            Operation::Backup { games, .. } => games.as_ref().is_some_and(|xs| !xs.is_empty()),
            Operation::Restore { games, .. } => games.as_ref().is_some_and(|xs| !xs.is_empty()),
            Operation::ValidateBackups { .. } => false,
            Operation::Cloud { .. } => false,
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

    pub fn syncable_games(&self) -> Option<&HashSet<String>> {
        match self {
            Operation::Idle => None,
            Operation::Backup { syncable_games, .. } => Some(syncable_games),
            Operation::Restore { .. } => None,
            Operation::ValidateBackups { .. } => None,
            Operation::Cloud { .. } => None,
        }
    }

    pub fn add_syncable_game(&mut self, title: String) {
        match self {
            Operation::Idle => {}
            Operation::Backup { syncable_games, .. } => {
                syncable_games.insert(title);
            }
            Operation::Restore { .. } => {}
            Operation::ValidateBackups { .. } => {}
            Operation::Cloud { .. } => {}
        }
    }

    pub fn active_games(&self) -> Option<&HashMap<String, chrono::DateTime<chrono::Utc>>> {
        match self {
            Operation::Idle => None,
            Operation::Backup { active_games, .. } => Some(active_games),
            Operation::Restore { active_games, .. } => Some(active_games),
            Operation::ValidateBackups { .. } => None,
            Operation::Cloud { .. } => None,
        }
    }

    pub fn add_active_game(&mut self, title: String) {
        match self {
            Operation::Idle | Operation::Cloud { .. } => {}
            Operation::Backup { active_games, .. }
            | Operation::Restore { active_games, .. }
            | Operation::ValidateBackups { active_games, .. } => {
                active_games.insert(title, chrono::Utc::now());
            }
        }
    }

    pub fn remove_active_game(&mut self, title: &str) {
        match self {
            Operation::Idle | Operation::Cloud { .. } => {}
            Operation::Backup { active_games, .. }
            | Operation::Restore { active_games, .. }
            | Operation::ValidateBackups { active_games, .. } => {
                active_games.remove(title);
            }
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
    RootLutrisDatabase(usize),
    SecondaryManifest(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UndoSubject {
    BackupTarget,
    RestoreSource,
    BackupSearchGameName,
    RestoreSearchGameName,
    CustomGamesSearchGameName,
    RootPath(usize),
    RootLutrisDatabase(usize),
    SecondaryManifest(usize),
    RedirectSource(usize),
    RedirectTarget(usize),
    CustomGameName(usize),
    CustomGameAlias(usize),
    CustomGameFile(usize, usize),
    CustomGameRegistry(usize, usize),
    CustomGameInstallDir(usize, usize),
    CustomGameWinePrefix(usize, usize),
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
            | UndoSubject::CustomGamesSearchGameName
            | UndoSubject::RootPath(_)
            | UndoSubject::RootLutrisDatabase(_)
            | UndoSubject::SecondaryManifest(_)
            | UndoSubject::RedirectSource(_)
            | UndoSubject::RedirectTarget(_)
            | UndoSubject::CustomGameName(_)
            | UndoSubject::CustomGameAlias(_)
            | UndoSubject::CustomGameFile(_, _)
            | UndoSubject::CustomGameRegistry(_, _)
            | UndoSubject::CustomGameInstallDir(_, _)
            | UndoSubject::CustomGameWinePrefix(_, _)
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
    pub fn game_list(scan_kind: ScanKind) -> Self {
        match scan_kind {
            ScanKind::Backup => Self::Backup,
            ScanKind::Restore => Self::Restore,
        }
    }

    pub fn id(&self) -> iced::widget::Id {
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
            .class(crate::gui::style::Scrollable)
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
        scan_kind: ScanKind,
        operating: bool,
        customized: bool,
        invented: bool,
        has_backups: bool,
        locked: bool,
    ) -> Vec<Self> {
        let mut options = vec![];

        if !operating {
            match scan_kind {
                ScanKind::Backup => {
                    options.push(Self::PreviewBackup);
                    options.push(Self::Backup { confirm: true });
                }
                ScanKind::Restore => {
                    options.push(Self::PreviewRestore);
                    options.push(Self::Restore { confirm: true });
                }
            }
        }

        if scan_kind.is_backup() && !customized {
            options.push(Self::Customize);
        }

        options.push(Self::MakeAlias);

        if scan_kind.is_restore() && has_backups {
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
