use crate::{
    config::{Config, RootsConfig},
    gui::{common::Message, style},
    lang::Translator,
    prelude::Error,
};

use crate::gui::widget::{Button, Column, Container, Row, Scrollable, Space, Text};
use iced::{alignment::Horizontal as HorizontalAlignment, Alignment, Length};

pub enum ModalVariant {
    Loading,
    Info,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalTheme {
    Error { variant: Error },
    ConfirmBackup { games: Option<Vec<String>> },
    ConfirmRestore { games: Option<Vec<String>> },
    NoMissingRoots,
    ConfirmAddMissingRoots(Vec<RootsConfig>),
    PreparingBackupDir,
    UpdatingManifest,
}

impl ModalTheme {
    pub fn variant(&self) -> ModalVariant {
        match self {
            Self::PreparingBackupDir | Self::UpdatingManifest => ModalVariant::Loading,
            Self::Error { .. } | Self::NoMissingRoots => ModalVariant::Info,
            Self::ConfirmBackup { .. } | Self::ConfirmRestore { .. } | Self::ConfirmAddMissingRoots(..) => {
                ModalVariant::Confirm
            }
        }
    }

    pub fn text(&self, config: &Config, translator: &Translator) -> String {
        match self {
            Self::Error { variant } => translator.handle_error(variant),
            Self::ConfirmBackup { .. } => translator.confirm_backup(
                &config.backup.path,
                config.backup.path.exists(),
                config.backup.merge,
                true,
            ),
            Self::ConfirmRestore { .. } => translator.confirm_restore(&config.restore.path, true),
            Self::NoMissingRoots => translator.no_missing_roots(),
            Self::ConfirmAddMissingRoots(missing) => translator.confirm_add_missing_roots(missing),
            Self::PreparingBackupDir => translator.preparing_backup_dir(),
            Self::UpdatingManifest => translator.updating_manifest(),
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
        }
    }
}

#[derive(Default)]
pub struct ModalComponent {}

impl ModalComponent {
    pub fn view(&self, theme: &ModalTheme, config: &Config, translator: &Translator) -> Container {
        let mut positive_button = Button::new(
            Text::new(match theme.variant() {
                ModalVariant::Loading => translator.okay_button(), // dummy
                ModalVariant::Info => translator.okay_button(),
                ModalVariant::Confirm => translator.continue_button(),
            })
            .horizontal_alignment(HorizontalAlignment::Center),
        )
        .width(125)
        .style(style::Button::Primary);

        if let Some(message) = theme.message() {
            positive_button = positive_button.on_press(message);
        }

        let negative_button =
            Button::new(Text::new(translator.cancel_button()).horizontal_alignment(HorizontalAlignment::Center))
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
                                .push(
                                    Scrollable::new(
                                        Column::new()
                                            .width(Length::Fill)
                                            .align_items(Alignment::Center)
                                            .push(Text::new(theme.text(config, translator))),
                                    )
                                    .height(Length::Fill)
                                    .style(style::Scrollable),
                                )
                                .height(Length::Fill),
                        )
                        .push(
                            match theme.variant() {
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
