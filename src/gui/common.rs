use iced::Length;

use crate::{
    gui::{
        icon::Icon,
        shortcuts::{Shortcut, TextHistory},
        style,
        widget::{Button, Column, Element, Row, TextInput, Undoable},
    },
    lang::{Language, TRANSLATOR},
    prelude::{Error, StrictPath},
    resource::{
        config::{BackupFormat, Config, RedirectKind, RootsConfig, SortKey, Theme, ZipCompression},
        manifest::{ManifestUpdate, Store},
    },
    scan::{
        layout::{Backup, GameLayout},
        registry_compat::RegistryItem,
        BackupInfo, OperationStepDecision, ScanInfo,
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

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
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

#[derive(Default)]
pub struct RedirectHistory {
    pub source: TextHistory,
    pub target: TextHistory,
}

#[derive(Default)]
pub struct CustomGameHistory {
    pub name: TextHistory,
    pub files: Vec<TextHistory>,
    pub registry: Vec<TextHistory>,
}

#[derive(Default)]
pub struct TextHistories {
    pub backup_target: TextHistory,
    pub restore_source: TextHistory,
    pub backup_search_game_name: TextHistory,
    pub restore_search_game_name: TextHistory,
    pub roots: Vec<TextHistory>,
    pub redirects: Vec<RedirectHistory>,
    pub custom_games: Vec<CustomGameHistory>,
    pub backup_filter_ignored_paths: Vec<TextHistory>,
    pub backup_filter_ignored_registry: Vec<TextHistory>,
}

impl TextHistories {
    pub fn new(config: &Config) -> Self {
        let mut histories = Self {
            backup_target: TextHistory::path(&config.backup.path),
            restore_source: TextHistory::path(&config.restore.path),
            backup_search_game_name: TextHistory::raw(""),
            restore_search_game_name: TextHistory::raw(""),
            ..Default::default()
        };

        for x in &config.roots {
            histories.roots.push(TextHistory::path(&x.path));
        }

        for x in &config.redirects {
            histories.redirects.push(RedirectHistory {
                source: TextHistory::path(&x.source),
                target: TextHistory::path(&x.target),
            });
        }

        for x in &config.custom_games {
            let mut custom_history = CustomGameHistory {
                name: TextHistory::raw(&x.name),
                ..Default::default()
            };
            for y in &x.files {
                custom_history.files.push(TextHistory::raw(y));
            }
            for y in &x.registry {
                custom_history.registry.push(TextHistory::raw(y));
            }
            histories.custom_games.push(custom_history);
        }

        for x in &config.backup.filter.ignored_paths {
            histories.backup_filter_ignored_paths.push(TextHistory::path(x));
        }
        for x in &config.backup.filter.ignored_registry {
            histories
                .backup_filter_ignored_registry
                .push(TextHistory::raw(&x.raw()));
        }

        histories
    }

    pub fn input<'a>(&self, subject: UndoSubject) -> Element<'a> {
        let current = match subject {
            UndoSubject::BackupTarget => self.backup_target.current(),
            UndoSubject::RestoreSource => self.restore_source.current(),
            UndoSubject::BackupSearchGameName => self.backup_search_game_name.current(),
            UndoSubject::RestoreSearchGameName => self.restore_search_game_name.current(),
            UndoSubject::Root(i) => self.roots.get(i).map(|x| x.current()).unwrap_or_default(),
            UndoSubject::RedirectSource(i) => self.redirects.get(i).map(|x| x.source.current()).unwrap_or_default(),
            UndoSubject::RedirectTarget(i) => self.redirects.get(i).map(|x| x.target.current()).unwrap_or_default(),
            UndoSubject::CustomGameName(i) => self.custom_games.get(i).map(|x| x.name.current()).unwrap_or_default(),
            UndoSubject::CustomGameFile(i, j) => self
                .custom_games
                .get(i)
                .and_then(|x| x.files.get(j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::CustomGameRegistry(i, j) => self
                .custom_games
                .get(i)
                .and_then(|x| x.registry.get(j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::BackupFilterIgnoredPath(i) => self
                .backup_filter_ignored_paths
                .get(i)
                .map(|x| x.current())
                .unwrap_or_default(),
            UndoSubject::BackupFilterIgnoredRegistry(i) => self
                .backup_filter_ignored_registry
                .get(i)
                .map(|x| x.current())
                .unwrap_or_default(),
        };

        let event: Box<dyn Fn(String) -> Message> = match subject {
            UndoSubject::BackupTarget => Box::new(Message::EditedBackupTarget),
            UndoSubject::RestoreSource => Box::new(Message::EditedRestoreSource),
            UndoSubject::BackupSearchGameName => Box::new(|value| Message::EditedSearchGameName {
                screen: Screen::Backup,
                value,
            }),
            UndoSubject::RestoreSearchGameName => Box::new(|value| Message::EditedSearchGameName {
                screen: Screen::Restore,
                value,
            }),
            UndoSubject::Root(i) => Box::new(move |value| Message::EditedRoot(EditAction::Change(i, value))),
            UndoSubject::RedirectSource(i) => Box::new(move |value| {
                Message::EditedRedirect(EditAction::Change(i, value), Some(RedirectEditActionField::Source))
            }),
            UndoSubject::RedirectTarget(i) => Box::new(move |value| {
                Message::EditedRedirect(EditAction::Change(i, value), Some(RedirectEditActionField::Target))
            }),
            UndoSubject::CustomGameName(i) => {
                Box::new(move |value| Message::EditedCustomGame(EditAction::Change(i, value)))
            }
            UndoSubject::CustomGameFile(i, j) => {
                Box::new(move |value| Message::EditedCustomGameFile(i, EditAction::Change(j, value)))
            }
            UndoSubject::CustomGameRegistry(i, j) => {
                Box::new(move |value| Message::EditedCustomGameRegistry(i, EditAction::Change(j, value)))
            }
            UndoSubject::BackupFilterIgnoredPath(i) => {
                Box::new(move |value| Message::EditedBackupFilterIgnoredPath(EditAction::Change(i, value)))
            }
            UndoSubject::BackupFilterIgnoredRegistry(i) => {
                Box::new(move |value| Message::EditedBackupFilterIgnoredRegistry(EditAction::Change(i, value)))
            }
        };

        let placeholder = match subject {
            UndoSubject::BackupTarget => "".to_string(),
            UndoSubject::RestoreSource => "".to_string(),
            UndoSubject::BackupSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::RestoreSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::Root(_) => "".to_string(),
            UndoSubject::RedirectSource(_) => TRANSLATOR.redirect_source_placeholder(),
            UndoSubject::RedirectTarget(_) => TRANSLATOR.redirect_target_placeholder(),
            UndoSubject::CustomGameName(_) => TRANSLATOR.custom_game_name_placeholder(),
            UndoSubject::CustomGameFile(_, _) => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::CustomGameRegistry(_, _) => "".to_string(),
            UndoSubject::BackupFilterIgnoredPath(_) => "".to_string(),
            UndoSubject::BackupFilterIgnoredRegistry(_) => "".to_string(),
        };

        Undoable::new(
            TextInput::new(&placeholder, &current, event)
                .style(style::TextInput)
                .width(Length::Fill)
                .padding(5),
            move |action| Message::UndoRedo(action, subject),
        )
        .into()
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

pub trait IcedExtension<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>;

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<Element<'a>>;
}

impl<'a> IcedExtension<'a> for Column<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>,
    {
        if condition() {
            self.push(element().into())
        } else {
            self
        }
    }

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<Element<'a>>,
    {
        if let Some(element) = element() {
            self.push(element.into())
        } else {
            self
        }
    }
}

impl<'a> IcedExtension<'a> for Row<'a> {
    fn push_if<E>(self, condition: impl FnOnce() -> bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>,
    {
        if condition() {
            self.push(element().into())
        } else {
            self
        }
    }

    fn push_some<E>(self, element: impl FnOnce() -> Option<E>) -> Self
    where
        E: Into<Element<'a>>,
    {
        if let Some(element) = element() {
            self.push(element.into())
        } else {
            self
        }
    }
}

pub trait IcedButtonExt<'a> {
    fn on_press_if(self, condition: impl FnOnce() -> bool, msg: impl FnOnce() -> Message) -> Self;
    fn on_press_some(self, msg: Option<Message>) -> Self;
}

impl<'a> IcedButtonExt<'a> for Button<'a> {
    fn on_press_if(self, condition: impl FnOnce() -> bool, msg: impl FnOnce() -> Message) -> Self {
        if condition() {
            self.on_press(msg())
        } else {
            self
        }
    }

    fn on_press_some(self, msg: Option<Message>) -> Self {
        match msg {
            Some(msg) => self.on_press(msg),
            None => self,
        }
    }
}
