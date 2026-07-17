use iced::{Alignment, Length, padding, widget::tooltip::Position};

use crate::{
    gui::{
        common::Message,
        icon::Icon,
        style,
        widget::{Button, Column, Container, Element, Row, Tooltip, checkbox, text},
    },
    resource::{
        config::Config,
        sync_state::GameSyncEntry,
    },
    scan::ScanInfo,
};

/// Steam Deck UI constants - larger touch targets for handheld use.
const CARD_MIN_HEIGHT: f32 = 80.0;
const BUTTON_SIZE: f32 = 48.0;
const ICON_SIZE: f32 = 26.0;
const CARD_PADDING: f32 = 12.0;
const CARD_SPACING: f32 = 10.0;

/// Sync state for a game, derived from comparing local and cloud info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    /// Never synced to cloud.
    NotSynced,
    /// Has local backup, not on cloud.
    LocalOnly,
    /// In sync with cloud.
    Synced,
    /// Cloud has newer version than local.
    CloudNewer,
    /// Local has newer version than cloud.
    LocalNewer,
}

impl SyncState {
    pub fn label(&self) -> &'static str {
        match self {
            SyncState::NotSynced => "Not synced",
            SyncState::LocalOnly => "Local only",
            SyncState::Synced => "Synced",
            SyncState::CloudNewer => "Cloud has newer",
            SyncState::LocalNewer => "Local has newer",
        }
    }
}

/// Data model for a single game card.
#[derive(Debug, Clone)]
pub struct GameCard {
    /// Game name as known to the manifest.
    pub name: String,
    /// Display name (may be alias or normalized).
    pub display_name: String,
    /// Whether this game is enabled for sync.
    pub enabled: bool,
    /// Scan info if the game has been scanned locally.
    pub scan_info: Option<ScanInfo>,
    /// Cloud sync info from settings.config.
    pub cloud_info: Option<GameSyncEntry>,
    /// Derived sync state.
    pub sync_state: SyncState,
    /// Whether a push/pull operation is currently in progress for this game.
    pub syncing: bool,
    /// Whether this game has a local backup.
    pub has_local_backup: bool,
}

impl GameCard {
    /// Create a new game card from a game name and config state.
    pub fn new(game_name: &str, config: &Config, cloud_info: Option<GameSyncEntry>) -> Self {
        let enabled = config.sync.enabled_games.contains(game_name);
        let display_name = config.display_name(game_name).to_string();

        // Check if game has a local backup folder
        let layout = crate::scan::layout::BackupLayout::new(config.backup.path.clone());
        let game_folder = layout.game_folder(game_name);
        let has_local_backup = game_folder.exists();

        let sync_state = match (&cloud_info, has_local_backup) {
            (None, false) => SyncState::NotSynced,
            (None, true) => SyncState::LocalOnly,
            (Some(_), false) => SyncState::CloudNewer,
            (Some(_), true) => SyncState::Synced,
        };

        Self {
            name: game_name.to_string(),
            display_name,
            enabled,
            scan_info: None,
            cloud_info,
            sync_state,
            syncing: false,
            has_local_backup,
        }
    }

    /// Render the game card with Steam Deck-optimized sizing.
    pub fn view(&self) -> Element {
        let card_content = Row::new()
            .spacing(CARD_SPACING)
            .align_y(Alignment::Center)
            .padding(padding::all(CARD_PADDING))
            // Enable/disable checkbox
            .push({
                let name = self.name.clone();
                checkbox("", self.enabled, move |_| {
                    Message::ToggleSyncGame { name: name.clone() }
                })
                .spacing(0)
            })
            // Game title
            .push(
                Button::new(
                    text(&self.display_name)
                        .size(18)
                        .align_x(iced::alignment::Horizontal::Center),
                )
                .on_press(Message::ScanSingleGame {
                    name: self.name.clone(),
                })
                .class(if self.enabled {
                    style::Button::GameListEntryTitle
                } else {
                    style::Button::GameListEntryTitleDisabled
                })
                .width(Length::Fill)
                .height(BUTTON_SIZE)
                .padding(5),
            )
            // Sync status badge
            .push(self.sync_badge())
            // Pull button (down arrow)
            .push(self.pull_button())
            // Push button (up arrow)
            .push(self.push_button());

        Container::new(card_content)
            .width(Length::Fill)
            .height(CARD_MIN_HEIGHT)
            .class(style::Container::Card)
            .into()
    }

    /// Create the sync status badge.
    fn sync_badge(&self) -> Element {
        let label = self.sync_state.label();

        Container::new(text(label).size(14))
            .padding(padding::horizontal(8))
            .height(BUTTON_SIZE)
            .center_y(Length::Fill)
            .class(style::Container::Badge)
            .into()
    }

    /// Create the pull button (down arrow for downloading from cloud).
    fn pull_button(&self) -> Element {
        let button = Button::new(
            text(Icon::ArrowDownward.as_char().to_string())
                .font(crate::gui::font::ICONS)
                .size(ICON_SIZE)
                .width(BUTTON_SIZE)
                .height(BUTTON_SIZE),
        )
        .on_press_maybe(if self.enabled && !self.syncing {
            Some(Message::PullGame {
                name: self.name.clone(),
            })
        } else {
            None
        })
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .padding(0)
        .class(style::Button::Primary);

        // Add tooltip
        Tooltip::new(button, text("Pull from cloud"), Position::Top)
            .class(style::Container::Tooltip)
            .into()
    }

    /// Create the push button (up arrow for uploading to cloud).
    fn push_button(&self) -> Element {
        let button = Button::new(
            text(Icon::ArrowUpward.as_char().to_string())
                .font(crate::gui::font::ICONS)
                .size(ICON_SIZE)
                .width(BUTTON_SIZE)
                .height(BUTTON_SIZE),
        )
        .on_press_maybe(if self.enabled && !self.syncing {
            Some(Message::PushGame {
                name: self.name.clone(),
            })
        } else {
            None
        })
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .padding(0)
        .class(style::Button::Primary);

        Tooltip::new(button, text("Push to cloud"), Position::Top)
            .class(style::Container::Tooltip)
            .into()
    }
}

/// Render the entire sync screen with all game cards.
pub fn view<'a>(
    cards: &'a [GameCard],
    _config: &Config,
    syncing_all: bool,
) -> Element<'a> {
    let mut content = Column::new()
        .spacing(CARD_SPACING)
        .padding(CARD_PADDING)
        .align_x(Alignment::Center);

    // "Sync All Enabled" button at the top
    let sync_all_button = Button::new(
        text("Sync All Enabled")
            .size(18)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press_maybe(if !syncing_all {
        Some(Message::SyncAllEnabled)
    } else {
        None
    })
    .width(250)
    .height(BUTTON_SIZE)
    .padding(10)
    .class(style::Button::Primary);

    content = content.push(
        Row::new()
            .spacing(CARD_SPACING)
            .align_y(Alignment::Center)
            .push(sync_all_button),
    );

    // Game cards
    let mut cards_column = Column::new().spacing(CARD_SPACING);
    for card in cards {
        cards_column = cards_column.push(card.view());
    }

    content = content.push(
        iced::widget::scrollable(cards_column)
            .height(Length::Fill)
            .width(Length::Fill),
    );

    content.into()
}
