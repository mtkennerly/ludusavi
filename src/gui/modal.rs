use iced::{Alignment, Length};

use crate::{
    cloud::{CloudChange, Remote, RemoteChoice, WebDavProvider},
    gui::{
        badge::Badge,
        button,
        common::{BackupPhase, Message, RestorePhase, ScrollSubject, UndoSubject},
        shortcuts::TextHistories,
        style,
        widget::{Column, Container, Element, IcedParentExt, PickList, Row, Space, Text},
    },
    lang::TRANSLATOR,
    prelude::{Error, Finality, SyncDirection},
    resource::config::{Config, RootsConfig},
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
            ModalInputKind::Url => TRANSLATOR.url_label(),
            ModalInputKind::Host => TRANSLATOR.host_label(),
            ModalInputKind::Port => TRANSLATOR.port_label(),
            ModalInputKind::Username => TRANSLATOR.username_label(),
            ModalInputKind::Password => TRANSLATOR.password_label(),
        };

        Row::new()
            .align_items(Alignment::Center)
            .push(Text::new(label).width(150))
            .push(histories.input(UndoSubject::ModalField(kind)))
    }

    pub fn view_pick_list<'a, T>(label: String, value: &'a T, choices: &'a [T], change: fn(T) -> Self) -> Row<'a>
    where
        T: Copy + Eq + PartialEq + ToString + 'static,
    {
        Row::new()
            .align_items(Alignment::Center)
            .push(Text::new(label).width(150))
            .push(
                Container::new(PickList::new(choices, Some(*value), move |x| {
                    Message::EditedModalField(change(x))
                }))
                .width(Length::Fill),
            )
    }
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
        games: Option<Vec<String>>,
    },
    ConfirmRestore {
        games: Option<Vec<String>>,
    },
    NoMissingRoots,
    ConfirmAddMissingRoots(Vec<RootsConfig>),
    UpdatingManifest,
    ConfirmCloudSync {
        local: String,
        cloud: String,
        direction: SyncDirection,
        changes: Vec<CloudChange>,
        done: bool,
        page: usize,
        previewing: bool,
        syncing: bool,
    },
    ConfigureFtpRemote,
    ConfigureSmbRemote,
    ConfigureWebDavRemote {
        provider: WebDavProvider,
    },
}

impl Modal {
    pub fn variant(&self) -> ModalVariant {
        match self {
            Self::Exiting | Self::UpdatingManifest => ModalVariant::Loading,
            Self::Error { .. } | Self::Errors { .. } | Self::NoMissingRoots => ModalVariant::Info,
            Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::ConfirmAddMissingRoots(..)
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => ModalVariant::Confirm,
            modal @ Self::ConfirmCloudSync { done, syncing, .. } => {
                if (*done && *syncing) || !modal.any_cloud_changes() {
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
            Self::Errors { errors } => {
                let messages: Vec<_> = errors.iter().map(|x| TRANSLATOR.handle_error(x)).collect();
                messages.join("\n\n")
            }
            Self::Exiting => TRANSLATOR.cancelling_button(),
            Self::ConfirmBackup { .. } => {
                TRANSLATOR.confirm_backup(&config.backup.path, config.backup.path.exists(), true)
            }
            Self::ConfirmRestore { .. } => TRANSLATOR.confirm_restore(&config.restore.path, true),
            Self::NoMissingRoots => TRANSLATOR.no_missing_roots(),
            Self::ConfirmAddMissingRoots(missing) => TRANSLATOR.confirm_add_missing_roots(missing),
            Self::UpdatingManifest => TRANSLATOR.updating_manifest(),
            modal @ Self::ConfirmCloudSync {
                local,
                cloud,
                direction,
                ..
            } => {
                if !modal.any_cloud_changes() {
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
        }
    }

    pub fn message(&self, histories: &TextHistories) -> Option<Message> {
        match self {
            Self::Error { .. } | Self::Errors { .. } | Self::NoMissingRoots => Some(Message::CloseModal),
            Self::Exiting => None,
            Self::ConfirmBackup { games } => Some(Message::Backup(BackupPhase::Start {
                preview: false,
                games: games.clone(),
            })),
            Self::ConfirmRestore { games } => Some(Message::Restore(RestorePhase::Start {
                preview: false,
                games: games.clone(),
            })),
            Self::ConfirmAddMissingRoots(missing) => Some(Message::ConfirmAddMissingRoots(missing.clone())),
            Self::UpdatingManifest => None,
            modal @ Self::ConfirmCloudSync {
                direction,
                syncing,
                done,
                ..
            } => {
                if (*done && *syncing) || !modal.any_cloud_changes() {
                    Some(Message::CloseModal)
                } else {
                    (!syncing).then_some(Message::SynchronizeCloud {
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
            modal @ Self::ConfirmCloudSync {
                direction,
                previewing,
                syncing,
                done,
                ..
            } => {
                if (*done && *syncing) || !modal.any_cloud_changes() {
                    vec![]
                } else {
                    vec![button::primary(
                        TRANSLATOR.preview_button(),
                        (!previewing && !syncing).then_some(Message::SynchronizeCloud {
                            direction: *direction,
                            finality: Finality::Preview,
                        }),
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
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => vec![],
        }
    }

    pub fn body(&self, config: &Config, histories: &TextHistories) -> Column {
        let mut col = Column::new()
            .width(Length::Fill)
            .spacing(15)
            .padding([0, 10, 0, 0])
            .align_items(Alignment::Center)
            .push(Text::new(self.text(config)));

        match self {
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::UpdatingManifest => (),
            modal @ Self::ConfirmCloudSync {
                changes,
                page,
                done,
                previewing,
                syncing,
                ..
            } => {
                if modal.any_cloud_changes() {
                    col = col
                        .push_if(
                            || *previewing || *syncing,
                            || {
                                Row::new()
                                    .spacing(20)
                                    .align_items(Alignment::Center)
                                    .push(Text::new(TRANSLATOR.change_count_label(changes.len())))
                                    .push_if(|| changes.is_empty() && !done, || Text::new(TRANSLATOR.loading()))
                                    .push(Space::new(Length::Fill, Length::Shrink))
                                    .push(button::previous_page(Message::ModalChangePage, *page))
                                    .push(button::next_page(
                                        Message::ModalChangePage,
                                        *page,
                                        changes.len() / CHANGES_PER_PAGE,
                                    ))
                            },
                        )
                        .push(
                            changes
                                .iter()
                                .skip(page * CHANGES_PER_PAGE)
                                .take(CHANGES_PER_PAGE)
                                .fold(
                                    Column::new().width(Length::Fill).align_items(Alignment::Start),
                                    |parent, CloudChange { change, path }| {
                                        parent.push(
                                            Row::new()
                                                .spacing(20)
                                                .align_items(Alignment::Start)
                                                .push(Badge::scan_change(*change).view())
                                                .push(Text::new(path)),
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
        }

        col
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
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => (),
        }
    }

    pub fn finish_cloud_scan(&mut self) {
        match self {
            Self::ConfirmCloudSync { done, .. } => {
                *done = true;
            }
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => (),
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
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => (),
        }
    }

    pub fn any_cloud_changes(&self) -> bool {
        match self {
            Self::ConfirmCloudSync { done, changes, .. } => !changes.is_empty() || !done,
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => false,
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
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => 2,
        }
    }

    pub fn view(&self, config: &Config, histories: &TextHistories) -> Container {
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
                .align_items(Alignment::Center)
                .push(
                    Container::new(Space::new(Length::Shrink, Length::Shrink))
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                        .style(style::Container::ModalBackground),
                )
                .push(
                    Column::new()
                        .height(Length::FillPortion(self.body_height_portion()))
                        .align_items(Alignment::Center)
                        .push(
                            Container::new(
                                ScrollSubject::Modal.into_widget(self.body(config, histories).padding([0, 30, 0, 30])),
                            )
                            .padding([30, 5, 0, 0])
                            .height(Length::Fill),
                        )
                        .push(
                            match self.variant() {
                                ModalVariant::Loading => Row::new(),
                                ModalVariant::Info => Row::with_children(self.extra_controls()).push(positive_button),
                                ModalVariant::Confirm => Row::with_children(self.extra_controls())
                                    .push(positive_button)
                                    .push(negative_button),
                            }
                            .padding([30, 0, 30, 0])
                            .spacing(20)
                            .height(Length::Shrink)
                            .align_items(Alignment::Center),
                        ),
                )
                .push(
                    Container::new(Space::new(Length::Shrink, Length::Shrink))
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                        .style(style::Container::ModalBackground),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
