use std::collections::BTreeSet;

use iced::{
    padding,
    widget::{mouse_area, opaque},
    Alignment, Length,
};
use itertools::Itertools;

use crate::{
    cloud::{CloudChange, Remote, RemoteChoice, WebDavProvider},
    gui::{
        badge::Badge,
        button,
        common::{BackupPhase, GameSelection, Message, Operation, RestorePhase, ScrollSubject, UndoSubject},
        icon::Icon,
        shortcuts::TextHistories,
        style,
        widget::{pick_list, text, Column, Container, Element, IcedParentExt, Row, Space},
    },
    lang::TRANSLATOR,
    prelude::{Error, Finality, SyncDirection},
    resource::{
        config::{Config, Root},
        manifest,
    },
};

const CHANGES_PER_PAGE: usize = 500;

pub enum ModalVariant {
    Loading,
    Info,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModalInputKind {
    Url,
    Host,
    Port,
    Username,
    Password,
}

#[derive(Debug, Clone)]
pub enum ModalField {
    Url(String),
    Host(String),
    Port(String),
    Username(String),
    Password(String),
    WebDavProvider(WebDavProvider),
}

impl ModalField {
    pub fn view<'a>(kind: ModalInputKind, histories: &TextHistories) -> Row<'a> {
        let label = match kind {
            ModalInputKind::Url => TRANSLATOR.url_field(),
            ModalInputKind::Host => TRANSLATOR.host_label(),
            ModalInputKind::Port => TRANSLATOR.port_label(),
            ModalInputKind::Username => TRANSLATOR.username_label(),
            ModalInputKind::Password => TRANSLATOR.password_label(),
        };

        Row::new()
            .align_y(Alignment::Center)
            .push(text(label).width(150))
            .push(histories.input(UndoSubject::ModalField(kind)))
    }

    pub fn view_pick_list<'a, T>(label: String, value: &'a T, choices: &'a [T], change: fn(T) -> Self) -> Row<'a>
    where
        T: Copy + Eq + PartialEq + ToString + 'static,
    {
        Row::new().align_y(Alignment::Center).push(text(label).width(150)).push(
            Container::new(pick_list(choices, Some(*value), move |x| {
                Message::EditedModalField(change(x))
            }))
            .width(Length::Fill),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudModalState {
    Initial,
    Previewing,
    Previewed,
    Syncing,
    Synced,
    NoChanges,
}

impl CloudModalState {
    pub fn idle(&self) -> bool {
        match self {
            Self::Initial => true,
            Self::Previewing => false,
            Self::Previewed => true,
            Self::Syncing => false,
            Self::Synced => true,
            Self::NoChanges => true,
        }
    }

    pub fn done(&self) -> bool {
        match self {
            Self::Initial => false,
            Self::Previewing => false,
            Self::Previewed => false,
            Self::Syncing => false,
            Self::Synced => true,
            Self::NoChanges => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Error,
    Errors,
    Exiting,
    ConfirmBackup,
    ConfirmRestore,
    NoMissingRoots,
    ConfirmAddMissingRoots,
    BackupValidation,
    AppUpdate,
    UpdatingManifest,
    ConfirmCloudSync,
    ConfigureFtpRemote,
    ConfigureSmbRemote,
    ConfigureWebDavRemote,
    GameNotes,
    ActiveScanGames,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    Error {
        variant: Error,
    },
    Errors {
        errors: Vec<Error>,
    },
    Exiting,
    ConfirmBackup {
        games: Option<GameSelection>,
    },
    ConfirmRestore {
        games: Option<GameSelection>,
    },
    NoMissingRoots,
    ConfirmAddMissingRoots(Vec<Root>),
    BackupValidation {
        /// Any games with invalid backups.
        games: BTreeSet<String>,
    },
    AppUpdate {
        release: crate::metadata::Release,
    },
    UpdatingManifest,
    ConfirmCloudSync {
        local: String,
        cloud: String,
        direction: SyncDirection,
        changes: Vec<CloudChange>,
        page: usize,
        state: CloudModalState,
    },
    ConfigureFtpRemote,
    ConfigureSmbRemote,
    ConfigureWebDavRemote {
        provider: WebDavProvider,
    },
    GameNotes {
        game: String,
        notes: Vec<manifest::Note>,
    },
    ActiveScanGames,
}

impl Modal {
    pub fn kind(&self) -> Kind {
        match self {
            Modal::Error { .. } => Kind::Error,
            Modal::Errors { .. } => Kind::Errors,
            Modal::Exiting => Kind::Exiting,
            Modal::ConfirmBackup { .. } => Kind::ConfirmBackup,
            Modal::ConfirmRestore { .. } => Kind::ConfirmRestore,
            Modal::NoMissingRoots => Kind::NoMissingRoots,
            Modal::ConfirmAddMissingRoots(..) => Kind::ConfirmAddMissingRoots,
            Modal::BackupValidation { .. } => Kind::BackupValidation,
            Modal::AppUpdate { .. } => Kind::AppUpdate,
            Modal::UpdatingManifest => Kind::UpdatingManifest,
            Modal::ConfirmCloudSync { .. } => Kind::ConfirmCloudSync,
            Modal::ConfigureFtpRemote => Kind::ConfigureFtpRemote,
            Modal::ConfigureSmbRemote => Kind::ConfigureSmbRemote,
            Modal::ConfigureWebDavRemote { .. } => Kind::ConfigureWebDavRemote,
            Modal::GameNotes { .. } => Kind::GameNotes,
            Modal::ActiveScanGames => Kind::ActiveScanGames,
        }
    }

    /// Should we allow two of the same kind back-to-back in the list?
    pub fn stackable(&self) -> bool {
        match self {
            Modal::Error { .. } => true,
            Modal::Errors { .. } => true,
            Modal::Exiting => false,
            Modal::ConfirmBackup { .. } => false,
            Modal::ConfirmRestore { .. } => false,
            Modal::NoMissingRoots => false,
            Modal::ConfirmAddMissingRoots(..) => false,
            Modal::BackupValidation { .. } => false,
            Modal::AppUpdate { .. } => false,
            Modal::UpdatingManifest => false,
            Modal::ConfirmCloudSync { .. } => false,
            Modal::ConfigureFtpRemote => false,
            Modal::ConfigureSmbRemote => false,
            Modal::ConfigureWebDavRemote { .. } => false,
            Modal::GameNotes { .. } => false,
            Modal::ActiveScanGames => false,
        }
    }

    pub fn variant(&self) -> ModalVariant {
        match self {
            Self::Exiting | Self::UpdatingManifest => ModalVariant::Loading,
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::NoMissingRoots
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => ModalVariant::Info,
            Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::ConfirmAddMissingRoots(..)
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::AppUpdate { .. } => ModalVariant::Confirm,
            Self::BackupValidation { games } => {
                if games.is_empty() {
                    ModalVariant::Info
                } else {
                    ModalVariant::Confirm
                }
            }
            Self::ConfirmCloudSync { state, .. } => {
                if state.done() {
                    ModalVariant::Info
                } else {
                    ModalVariant::Confirm
                }
            }
        }
    }

    pub fn text(&self, config: &Config) -> String {
        match self {
            Self::Error { variant } => TRANSLATOR.handle_error(variant),
            Self::Errors { errors } => errors.iter().map(|x| TRANSLATOR.handle_error(x)).join("\n\n"),
            Self::Exiting => TRANSLATOR.cancelling_button(),
            Self::ConfirmBackup { .. } => {
                TRANSLATOR.confirm_backup(&config.backup.path, config.backup.path.exists(), true)
            }
            Self::ConfirmRestore { .. } => TRANSLATOR.confirm_restore(&config.restore.path, true),
            Self::NoMissingRoots => TRANSLATOR.no_missing_roots(),
            Self::ConfirmAddMissingRoots(missing) => TRANSLATOR.confirm_add_missing_roots(missing),
            Self::AppUpdate { release } => TRANSLATOR.new_version_available(release.version.to_string().as_str()),
            Self::UpdatingManifest => TRANSLATOR.updating_manifest(),
            Self::BackupValidation { games } => {
                if games.is_empty() {
                    TRANSLATOR.backups_are_valid()
                } else {
                    TRANSLATOR.backups_are_invalid()
                }
            }
            Self::ConfirmCloudSync {
                local,
                cloud,
                direction,
                state,
                ..
            } => {
                if *state == CloudModalState::NoChanges {
                    TRANSLATOR.no_cloud_changes()
                } else {
                    match direction {
                        SyncDirection::Upload => TRANSLATOR.confirm_cloud_upload(local, cloud),
                        SyncDirection::Download => TRANSLATOR.confirm_cloud_download(local, cloud),
                    }
                }
            }
            Self::ConfigureFtpRemote { .. } => RemoteChoice::Ftp.to_string(),
            Self::ConfigureSmbRemote { .. } => RemoteChoice::Smb.to_string(),
            Self::ConfigureWebDavRemote { .. } => RemoteChoice::WebDav.to_string(),
            Self::GameNotes { game, .. } => game.clone(),
            Self::ActiveScanGames => "".to_string(),
        }
    }

    pub fn message(&self, histories: &TextHistories) -> Option<Message> {
        match self {
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::NoMissingRoots
            | Self::BackupValidation { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => Some(Message::CloseModal),
            Self::Exiting => None,
            Self::ConfirmBackup { games } => Some(Message::Backup(BackupPhase::Start {
                preview: false,
                repair: false,
                jump: false,
                games: games.clone(),
            })),
            Self::ConfirmRestore { games } => Some(Message::Restore(RestorePhase::Start {
                preview: false,
                games: games.clone(),
            })),
            Self::ConfirmAddMissingRoots(missing) => Some(Message::ConfirmAddMissingRoots(missing.clone())),
            Self::AppUpdate { release } => Some(Message::OpenUrlAndCloseModal(release.url.clone())),
            Self::UpdatingManifest => None,
            Self::ConfirmCloudSync { direction, state, .. } => {
                if state.done() {
                    Some(Message::CloseModal)
                } else {
                    state.idle().then_some(Message::SynchronizeCloud {
                        direction: *direction,
                        finality: Finality::Final,
                    })
                }
            }
            Self::ConfigureFtpRemote => {
                let host = histories.modal.host.current();
                let port = histories.modal.port.current();
                let username = histories.modal.username.current();
                let password = histories.modal.password.current();

                let Ok(port) = port.parse::<i32>() else { return None };
                if host.is_empty() || username.is_empty() {
                    None
                } else {
                    Some(Message::FinalizeRemote(Remote::Ftp {
                        id: Remote::generate_id(),
                        host,
                        port,
                        username,
                        password,
                    }))
                }
            }
            Self::ConfigureSmbRemote => {
                let host = histories.modal.host.current();
                let port = histories.modal.port.current();
                let username = histories.modal.username.current();
                let password = histories.modal.password.current();

                let Ok(port) = port.parse::<i32>() else { return None };
                if host.is_empty() || username.is_empty() {
                    None
                } else {
                    Some(Message::FinalizeRemote(Remote::Smb {
                        id: Remote::generate_id(),
                        host,
                        port,
                        username,
                        password,
                    }))
                }
            }
            Self::ConfigureWebDavRemote { provider } => {
                let url = histories.modal.url.current();
                let username = histories.modal.username.current();
                let password = histories.modal.password.current();

                if url.is_empty() || username.is_empty() {
                    None
                } else {
                    Some(Message::FinalizeRemote(Remote::WebDav {
                        id: Remote::generate_id(),
                        url,
                        username,
                        password,
                        provider: *provider,
                    }))
                }
            }
        }
    }

    fn extra_controls(&self) -> Vec<Element> {
        match self {
            Self::ConfirmCloudSync { direction, state, .. } => {
                if state.done() {
                    vec![]
                } else {
                    vec![button::primary(
                        TRANSLATOR.preview_button(),
                        state.idle().then_some(Message::SynchronizeCloud {
                            direction: *direction,
                            finality: Finality::Preview,
                        }),
                    )]
                }
            }
            Self::BackupValidation { games } => {
                if games.is_empty() {
                    vec![]
                } else {
                    vec![button::primary(
                        TRANSLATOR.backup_button(),
                        Some(Message::Backup(BackupPhase::Start {
                            preview: false,
                            repair: true,
                            jump: false,
                            games: Some(GameSelection::group(games.iter().cloned().collect())),
                        })),
                    )]
                }
            }
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::UpdatingManifest
            | Self::AppUpdate { .. }
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => vec![],
        }
    }

    pub fn body(&self, config: &Config, histories: &TextHistories, operation: &Operation) -> Column {
        let mut col = Column::new()
            .width(Length::Fill)
            .spacing(15)
            .padding(padding::right(10))
            .align_x(Alignment::Center)
            .push(text(self.text(config)));

        match self {
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::AppUpdate { .. }
            | Self::UpdatingManifest => (),
            Self::BackupValidation { games } => {
                for game in games.iter().sorted() {
                    col = col.push(text(game))
                }
            }
            Self::ConfirmCloudSync {
                changes, page, state, ..
            } => {
                if !changes.is_empty() || !state.idle() {
                    col = col
                        .push_if(!state.idle(), || {
                            Row::new()
                                .spacing(20)
                                .align_y(Alignment::Center)
                                .push(text(TRANSLATOR.change_count_label(changes.len())))
                                .push_if(changes.is_empty(), || text(TRANSLATOR.loading()))
                                .push(Space::new().width(Length::Fill))
                                .push(button::previous_page(Message::ModalChangePage, *page))
                                .push(button::next_page(
                                    Message::ModalChangePage,
                                    *page,
                                    changes.len() / CHANGES_PER_PAGE,
                                ))
                        })
                        .push(
                            changes
                                .iter()
                                .skip(page * CHANGES_PER_PAGE)
                                .take(CHANGES_PER_PAGE)
                                .fold(
                                    Column::new().width(Length::Fill).align_x(Alignment::Start),
                                    |parent, CloudChange { change, path }| {
                                        parent.push(
                                            Row::new()
                                                .spacing(20)
                                                .align_y(Alignment::Start)
                                                .push(Badge::scan_change(*change).view())
                                                .push(text(path)),
                                        )
                                    },
                                ),
                        );
                }
            }
            Self::ConfigureFtpRemote { .. } | Self::ConfigureSmbRemote { .. } => {
                col = col
                    .width(500)
                    .push(ModalField::view(ModalInputKind::Host, histories))
                    .push(ModalField::view(ModalInputKind::Port, histories))
                    .push(ModalField::view(ModalInputKind::Username, histories))
                    .push(ModalField::view(ModalInputKind::Password, histories));
            }
            Self::ConfigureWebDavRemote { provider, .. } => {
                col = col
                    .width(500)
                    .push(ModalField::view(ModalInputKind::Url, histories))
                    .push(ModalField::view(ModalInputKind::Username, histories))
                    .push(ModalField::view(ModalInputKind::Password, histories))
                    .push(ModalField::view_pick_list(
                        TRANSLATOR.provider_label(),
                        provider,
                        WebDavProvider::ALL,
                        ModalField::WebDavProvider,
                    ));
            }
            Self::GameNotes { notes, .. } => {
                col = notes.iter().fold(col, |parent, note| {
                    parent.push(
                        Row::new()
                            .push(Container::new(Icon::Info.text_narrow()).padding(padding::top(2).left(5).right(10)))
                            .push(
                                Column::new()
                                    .spacing(5)
                                    .push(text(&note.message).size(16))
                                    .push(note.source.as_ref().map(|source| text(source).size(12))),
                            ),
                    )
                });
            }
            Self::ActiveScanGames => {
                if let Some(games) = operation.active_games() {
                    let now = chrono::Utc::now();
                    col = games
                        .iter()
                        .sorted_by_key(|(_, v)| *v)
                        .fold(col, |parent, (game, started)| {
                            let elapsed = now - started;
                            let readable = format!(
                                "{:0>2}:{:0>2}:{:0>2}.{:0>3}",
                                elapsed.num_hours(),
                                elapsed.num_minutes() % 60,
                                elapsed.num_seconds() % 60,
                                elapsed.num_milliseconds() % 1000,
                            );
                            parent.push(text(format!("{readable} - {game}")))
                        });
                    col = col.align_x(Alignment::Start).spacing(2);
                }
            }
        }

        col
    }

    fn content(&self, config: &Config, histories: &TextHistories, operation: &Operation) -> Container {
        let positive_button = button::primary(
            match self.variant() {
                ModalVariant::Loading => TRANSLATOR.okay_button(), // dummy
                ModalVariant::Info => TRANSLATOR.okay_button(),
                ModalVariant::Confirm => TRANSLATOR.continue_button(),
            },
            self.message(histories),
        );

        let negative_button = button::negative(TRANSLATOR.cancel_button(), Some(Message::CloseModal));

        Container::new(
            Column::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(
                    Container::new(
                        ScrollSubject::Modal.into_widget(self.body(config, histories, operation).padding([0, 30])),
                    )
                    .padding(padding::top(30).right(5))
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .push(
                    Container::new(
                        match self.variant() {
                            ModalVariant::Loading => Row::new(),
                            ModalVariant::Info => Row::with_children(self.extra_controls()).push(positive_button),
                            ModalVariant::Confirm => Row::with_children(self.extra_controls())
                                .push_if(!matches!(self, Modal::BackupValidation { .. }), || positive_button)
                                .push(negative_button),
                        }
                        .padding([30, 0])
                        .spacing(20)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .center_x(Length::Fill),
                ),
        )
        .class(style::Container::ModalForeground)
        .center_x(Length::Fill)
        .height(Length::Fill)
    }

    pub fn add_cloud_change(&mut self, change: CloudChange) {
        match self {
            Self::ConfirmCloudSync { changes, .. } => {
                changes.push(change);
                changes.sort();
            }
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::BackupValidation { .. }
            | Self::AppUpdate { .. }
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => (),
        }
    }

    pub fn finish_cloud_scan(&mut self) {
        match self {
            Self::ConfirmCloudSync { state, changes, .. } => {
                *state = match *state {
                    CloudModalState::Previewing => {
                        if changes.is_empty() {
                            CloudModalState::NoChanges
                        } else {
                            CloudModalState::Previewed
                        }
                    }
                    CloudModalState::Syncing => {
                        if changes.is_empty() {
                            CloudModalState::NoChanges
                        } else {
                            CloudModalState::Synced
                        }
                    }
                    x => x,
                };
            }
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::BackupValidation { .. }
            | Self::AppUpdate { .. }
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => (),
        }
    }

    pub fn set_page(&mut self, new_page: usize) {
        match self {
            Self::ConfirmCloudSync { page, .. } => {
                *page = new_page;
            }
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::BackupValidation { .. }
            | Self::AppUpdate { .. }
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => (),
        }
    }

    pub fn is_cloud_active(&self) -> bool {
        match self {
            Self::ConfirmCloudSync { state, .. } => !state.idle(),
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::BackupValidation { .. }
            | Self::AppUpdate { .. }
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => false,
        }
    }

    pub fn body_height_portion(&self) -> u16 {
        match self {
            Self::ConfirmCloudSync { .. } => 4,
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::BackupValidation { .. }
            | Self::AppUpdate { .. }
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. }
            | Self::GameNotes { .. }
            | Self::ActiveScanGames => 2,
        }
    }

    pub fn view(&self, config: &Config, histories: &TextHistories, operation: &Operation) -> Element {
        let horizontal = || {
            Container::new(Space::new().width(Length::FillPortion(1)).height(Length::Fill))
                .class(style::Container::ModalBackground)
        };

        let vertical = || {
            Container::new(Space::new())
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .class(style::Container::ModalBackground)
        };

        let modal = Container::new(
            Row::new()
                .push(horizontal())
                .push(
                    Column::new()
                        .width(Length::FillPortion(8))
                        .push(vertical())
                        .push(
                            Container::new(opaque(self.content(config, histories, operation)))
                                .class(style::Container::ModalBackground)
                                .width(Length::Fill)
                                .height(Length::FillPortion(self.body_height_portion())),
                        )
                        .push(vertical()),
                )
                .push(horizontal()),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        opaque({
            let mut area = mouse_area(modal);

            match self.variant() {
                ModalVariant::Loading => {}
                ModalVariant::Info | ModalVariant::Confirm => {
                    area = area.on_press(Message::CloseModal);
                }
            }

            area
        })
    }
}
