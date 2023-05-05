use iced::{Alignment, Length};

use crate::{
    cloud::{CloudChange, Remote, RemoteChoice, WebDavProvider},
    gui::{
        badge::Badge,
        button,
        common::{BackupPhase, Message, RestorePhase, ScrollSubject},
        style,
        widget::{Column, Container, Element, IcedParentExt, PickList, Row, Space, Text, TextInput},
    },
    lang::TRANSLATOR,
    prelude::{Error, Finality, Privacy, SyncDirection},
    resource::config::{Config, RootsConfig},
};

const CHANGES_PER_PAGE: usize = 500;

pub enum ModalVariant {
    Loading,
    Info,
    Confirm,
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
    pub fn view<'a>(label: String, value: &str, change: fn(String) -> Self, privacy: Privacy) -> Row<'a> {
        Row::new()
            .align_items(Alignment::Center)
            .push(Text::new(label).width(150))
            .push({
                let input = TextInput::new("", value, move |x| Message::EditedModalField(change(x)));

                if privacy.sensitive() {
                    input.password()
                } else {
                    input
                }
            })
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
    PreparingBackupDir,
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
    ConfigureFtpRemote {
        host: String,
        port: String,
        username: String,
        password: String,
    },
    ConfigureSmbRemote {
        host: String,
        port: String,
        username: String,
        password: String,
    },
    ConfigureWebDavRemote {
        url: String,
        username: String,
        password: String,
        provider: WebDavProvider,
    },
}

impl Modal {
    pub fn variant(&self) -> ModalVariant {
        match self {
            Self::Exiting | Self::PreparingBackupDir | Self::UpdatingManifest => ModalVariant::Loading,
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
            Self::PreparingBackupDir => TRANSLATOR.preparing_backup_dir(),
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

    pub fn message(&self) -> Option<Message> {
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
            Self::PreparingBackupDir | Self::UpdatingManifest => None,
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
            Self::ConfigureFtpRemote {
                host,
                port,
                username,
                password,
            } => {
                let Ok(port) = port.parse::<i32>() else { return None };
                if host.is_empty() || username.is_empty() {
                    None
                } else {
                    Some(Message::FinalizeRemote(Remote::Ftp {
                        id: Remote::generate_id(),
                        host: host.clone(),
                        port,
                        username: username.clone(),
                        password: password.clone(),
                    }))
                }
            }
            Self::ConfigureSmbRemote {
                host,
                port,
                username,
                password,
            } => {
                let Ok(port) = port.parse::<i32>() else { return None };
                if host.is_empty() || username.is_empty() {
                    None
                } else {
                    Some(Message::FinalizeRemote(Remote::Smb {
                        id: Remote::generate_id(),
                        host: host.clone(),
                        port,
                        username: username.clone(),
                        password: password.clone(),
                    }))
                }
            }
            Self::ConfigureWebDavRemote {
                url,
                username,
                password,
                provider,
            } => {
                if url.is_empty() || username.is_empty() {
                    None
                } else {
                    Some(Message::FinalizeRemote(Remote::WebDav {
                        id: Remote::generate_id(),
                        url: url.clone(),
                        username: username.clone(),
                        password: password.clone(),
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
            | Self::PreparingBackupDir
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => vec![],
        }
    }

    pub fn body(&self, config: &Config) -> Column {
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
            | Self::PreparingBackupDir
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
                                    .padding([0, 20, 0, 0])
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
            Self::ConfigureFtpRemote {
                host,
                port,
                username,
                password,
            }
            | Self::ConfigureSmbRemote {
                host,
                port,
                username,
                password,
            } => {
                col = col
                    .width(500)
                    .push(ModalField::view(
                        TRANSLATOR.host_label(),
                        host,
                        ModalField::Host,
                        Privacy::Public,
                    ))
                    .push(ModalField::view(
                        TRANSLATOR.port_label(),
                        port,
                        ModalField::Port,
                        Privacy::Public,
                    ))
                    .push(ModalField::view(
                        TRANSLATOR.username_label(),
                        username,
                        ModalField::Username,
                        Privacy::Public,
                    ))
                    .push(ModalField::view(
                        TRANSLATOR.password_label(),
                        password,
                        ModalField::Password,
                        Privacy::Private,
                    ));
            }
            Self::ConfigureWebDavRemote {
                url,
                username,
                password,
                provider,
            } => {
                col = col
                    .width(500)
                    .push(ModalField::view(
                        TRANSLATOR.url_label(),
                        url,
                        ModalField::Url,
                        Privacy::Public,
                    ))
                    .push(ModalField::view(
                        TRANSLATOR.username_label(),
                        username,
                        ModalField::Username,
                        Privacy::Public,
                    ))
                    .push(ModalField::view(
                        TRANSLATOR.password_label(),
                        password,
                        ModalField::Password,
                        Privacy::Private,
                    ))
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

    pub fn edit(&mut self, field: ModalField) {
        match self {
            Modal::ConfigureFtpRemote {
                host,
                port,
                username,
                password,
            }
            | Modal::ConfigureSmbRemote {
                host,
                port,
                username,
                password,
            } => match field {
                ModalField::Url(_) => (),
                ModalField::Host(new) => *host = new,
                ModalField::Port(new) => *port = new,
                ModalField::Username(new) => *username = new,
                ModalField::Password(new) => *password = new,
                ModalField::WebDavProvider(_) => (),
            },
            Self::ConfigureWebDavRemote {
                url,
                username,
                password,
                provider,
            } => match field {
                ModalField::Url(new) => *url = new,
                ModalField::Host(_) => (),
                ModalField::Port(_) => (),
                ModalField::Username(new) => *username = new,
                ModalField::Password(new) => *password = new,
                ModalField::WebDavProvider(new) => *provider = new,
            },
            Self::Error { .. }
            | Self::Errors { .. }
            | Self::Exiting
            | Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::NoMissingRoots
            | Self::ConfirmAddMissingRoots(_)
            | Self::PreparingBackupDir
            | Self::UpdatingManifest
            | Self::ConfirmCloudSync { .. } => (),
        }
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
            | Self::PreparingBackupDir
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
            | Self::PreparingBackupDir
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
            | Self::PreparingBackupDir
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
            | Self::PreparingBackupDir
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
            | Self::PreparingBackupDir
            | Self::UpdatingManifest
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureSmbRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => 2,
        }
    }

    pub fn view(&self, config: &Config) -> Container {
        let positive_button = button::primary(
            match self.variant() {
                ModalVariant::Loading => TRANSLATOR.okay_button(), // dummy
                ModalVariant::Info => TRANSLATOR.okay_button(),
                ModalVariant::Confirm => TRANSLATOR.continue_button(),
            },
            self.message(),
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
                            Container::new(ScrollSubject::Modal.into_widget(self.body(config)))
                                .padding([30, 20, 0, 30])
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
                            .padding(30)
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
