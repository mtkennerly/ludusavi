use iced::{alignment::Horizontal as HorizontalAlignment, Alignment, Length};

use crate::{
    cloud::{Remote, RemoteChoice, WebDavProvider},
    gui::{
        common::{Message, ScrollSubject},
        style,
        widget::{Button, Column, Container, PickList, Row, Space, Text, TextInput},
    },
    lang::TRANSLATOR,
    prelude::{Error, Privacy},
    resource::config::{Config, RootsConfig},
};

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
    ConfirmUploadToCloud {
        local: String,
        cloud: String,
    },
    ConfirmDownloadFromCloud {
        local: String,
        cloud: String,
    },
    ConfigureFtpRemote {
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
            Self::PreparingBackupDir | Self::UpdatingManifest => ModalVariant::Loading,
            Self::Error { .. } | Self::NoMissingRoots => ModalVariant::Info,
            Self::ConfirmBackup { .. }
            | Self::ConfirmRestore { .. }
            | Self::ConfirmAddMissingRoots(..)
            | Self::ConfirmUploadToCloud { .. }
            | Self::ConfirmDownloadFromCloud { .. }
            | Self::ConfigureFtpRemote { .. }
            | Self::ConfigureWebDavRemote { .. } => ModalVariant::Confirm,
        }
    }

    pub fn text(&self, config: &Config) -> String {
        match self {
            Self::Error { variant } => TRANSLATOR.handle_error(variant),
            Self::ConfirmBackup { .. } => TRANSLATOR.confirm_backup(
                &config.backup.path,
                config.backup.path.exists(),
                config.backup.merge,
                true,
            ),
            Self::ConfirmRestore { .. } => TRANSLATOR.confirm_restore(&config.restore.path, true),
            Self::NoMissingRoots => TRANSLATOR.no_missing_roots(),
            Self::ConfirmAddMissingRoots(missing) => TRANSLATOR.confirm_add_missing_roots(missing),
            Self::PreparingBackupDir => TRANSLATOR.preparing_backup_dir(),
            Self::UpdatingManifest => TRANSLATOR.updating_manifest(),
            Self::ConfirmUploadToCloud { local, cloud } => TRANSLATOR.confirm_cloud_upload(local, cloud),
            Self::ConfirmDownloadFromCloud { local, cloud } => TRANSLATOR.confirm_cloud_download(local, cloud),
            Self::ConfigureFtpRemote { .. } => RemoteChoice::Ftp.to_string(),
            Self::ConfigureWebDavRemote { .. } => RemoteChoice::WebDav.to_string(),
        }
    }

    pub fn message(&self) -> Option<Message> {
        match self {
            Self::Error { .. } | Self::NoMissingRoots => Some(Message::CloseModal),
            Self::ConfirmBackup { games } => Some(Message::BackupPrep {
                preview: false,
                games: games.clone(),
            }),
            Self::ConfirmRestore { games } => Some(Message::RestoreStart {
                preview: false,
                games: games.clone(),
            }),
            Self::ConfirmAddMissingRoots(missing) => Some(Message::ConfirmAddMissingRoots(missing.clone())),
            Self::PreparingBackupDir | Self::UpdatingManifest => None,
            Self::ConfirmUploadToCloud { .. } => Some(Message::SynchronizeFromLocalToCloud),
            Self::ConfirmDownloadFromCloud { .. } => Some(Message::SynchronizeFromCloudToLocal),
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
                        url: url.clone(),
                        username: username.clone(),
                        password: password.clone(),
                        provider: *provider,
                    }))
                }
            }
        }
    }

    pub fn body(&self, config: &Config) -> Column {
        let mut col = Column::new()
            .width(Length::Fill)
            .spacing(10)
            .align_items(Alignment::Center)
            .push(Text::new(self.text(config)));

        if let Modal::ConfigureFtpRemote {
            host,
            port,
            username,
            password,
        } = self
        {
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

        if let Modal::ConfigureWebDavRemote {
            url,
            username,
            password,
            provider,
        } = self
        {
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
            // .push(PickList::new(WebDavProvider::ALL, Some(*provider), |x| Message::EditedModalField(ModalField::WebDavProvider(x))));
        }

        col
    }
}

impl Modal {
    pub fn view(&self, config: &Config) -> Container {
        let mut positive_button = Button::new(
            Text::new(match self.variant() {
                ModalVariant::Loading => TRANSLATOR.okay_button(), // dummy
                ModalVariant::Info => TRANSLATOR.okay_button(),
                ModalVariant::Confirm => TRANSLATOR.continue_button(),
            })
            .horizontal_alignment(HorizontalAlignment::Center),
        )
        .width(125)
        .style(style::Button::Primary);

        if let Some(message) = self.message() {
            positive_button = positive_button.on_press(message);
        }

        let negative_button =
            Button::new(Text::new(TRANSLATOR.cancel_button()).horizontal_alignment(HorizontalAlignment::Center))
                .on_press(Message::CloseModal)
                .width(125)
                .style(style::Button::Negative);

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
                        .height(Length::FillPortion(2))
                        .align_items(Alignment::Center)
                        .push(
                            Row::new()
                                .padding([40, 40, 0, 40])
                                .align_items(Alignment::Center)
                                .push(ScrollSubject::Modal.into_widget(self.body(config)))
                                .height(Length::Fill),
                        )
                        .push(
                            match self.variant() {
                                ModalVariant::Loading => Row::new(),
                                ModalVariant::Info => Row::new().push(positive_button),
                                ModalVariant::Confirm => Row::new().push(positive_button).push(negative_button),
                            }
                            .padding(40)
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
