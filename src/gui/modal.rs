use crate::{
    config::Config,
    gui::{common::Message, style},
    lang::Translator,
    prelude::Error,
};

use iced::{button, Align, Button, Column, Container, HorizontalAlignment, Length, Row, Space, Text};

#[derive(Debug, Clone, PartialEq)]
pub enum ModalTheme {
    Error { variant: Error },
    ConfirmBackup,
    ConfirmRestore,
}

#[derive(Default)]
pub struct ModalComponent {
    positive_button: button::State,
    negative_button: button::State,
}

impl ModalComponent {
    pub fn view(&mut self, theme: &ModalTheme, config: &Config, translator: &Translator) -> Container<Message> {
        let positive_button = Button::new(
            &mut self.positive_button,
            Text::new(match theme {
                ModalTheme::Error { .. } => translator.okay_button(),
                _ => translator.continue_button(),
            })
            .horizontal_alignment(HorizontalAlignment::Center),
        )
        .on_press(match theme {
            ModalTheme::Error { .. } => Message::Idle,
            ModalTheme::ConfirmBackup => Message::BackupStart { preview: false },
            ModalTheme::ConfirmRestore => Message::RestoreStart { preview: false },
        })
        .width(Length::Units(125))
        .style(style::Button::Primary);

        let negative_button = Button::new(
            &mut self.negative_button,
            Text::new(translator.cancel_button()).horizontal_alignment(HorizontalAlignment::Center),
        )
        .on_press(Message::Idle)
        .width(Length::Units(125))
        .style(style::Button::Negative);

        Container::new(
            Column::new()
                .padding(5)
                .width(Length::Fill)
                .align_items(Align::Center)
                .push(
                    Container::new(Space::new(Length::Shrink, Length::Shrink))
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                        .style(style::Container::ModalBackground),
                )
                .push(
                    Column::new()
                        .height(Length::FillPortion(2))
                        .align_items(Align::Center)
                        .push(
                            Row::new()
                                .padding(20)
                                .align_items(Align::Center)
                                .push(Text::new(match theme {
                                    ModalTheme::Error { variant } => translator.handle_error(variant),
                                    ModalTheme::ConfirmBackup => translator.modal_confirm_backup(
                                        &config.backup.path,
                                        config.backup.path.exists(),
                                        config.backup.merge,
                                    ),
                                    ModalTheme::ConfirmRestore => {
                                        translator.modal_confirm_restore(&config.restore.path)
                                    }
                                }))
                                .height(Length::Fill),
                        )
                        .push(
                            match theme {
                                ModalTheme::Error { .. } => Row::new().push(positive_button),
                                _ => Row::new().push(positive_button).push(negative_button),
                            }
                            .padding(20)
                            .spacing(20)
                            .height(Length::Fill)
                            .align_items(Align::Center),
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
