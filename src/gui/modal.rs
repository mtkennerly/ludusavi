use crate::{
    config::{Config, RootsConfig},
    gui::{common::Message, style},
    lang::Translator,
    prelude::Error,
};

use iced::{
    alignment::Horizontal as HorizontalAlignment, button, scrollable, Alignment, Button, Column, Container, Length,
    Row, Scrollable, Space, Text,
};

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
            Self::Error { .. } | Self::NoMissingRoots => Some(Message::Idle),
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
pub struct ModalComponent {
    positive_button: button::State,
    negative_button: button::State,
    scroll: scrollable::State,
}

impl ModalComponent {
    pub fn view(&mut self, theme: &ModalTheme, config: &Config, translator: &Translator) -> Container<Message> {
        let mut positive_button = Button::new(
            &mut self.positive_button,
            Text::new(match theme.variant() {
                ModalVariant::Loading => translator.okay_button(), // dummy
                ModalVariant::Info => translator.okay_button(),
                ModalVariant::Confirm => translator.continue_button(),
            })
            .horizontal_alignment(HorizontalAlignment::Center),
        )
        .width(Length::Units(125))
        .style(style::Button::Primary(config.theme));

        if let Some(message) = theme.message() {
            positive_button = positive_button.on_press(message);
        }

        let negative_button = Button::new(
            &mut self.negative_button,
            Text::new(translator.cancel_button()).horizontal_alignment(HorizontalAlignment::Center),
        )
        .on_press(Message::Idle)
        .width(Length::Units(125))
        .style(style::Button::Negative(config.theme));

        Container::new(
            Column::new()
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .push(
                    Container::new(Space::new(Length::Shrink, Length::Shrink))
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                        .style(style::Container::ModalBackground(config.theme)),
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
                                    Scrollable::new(&mut self.scroll)
                                        .width(Length::Fill)
                                        .height(Length::Fill)
                                        .style(style::Scrollable(config.theme))
                                        .push(Text::new(theme.text(config, translator)))
                                        .align_items(Alignment::Center),
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
                        .style(style::Container::ModalBackground(config.theme)),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
