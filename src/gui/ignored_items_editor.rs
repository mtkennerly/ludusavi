use iced::Length;

use crate::{
    config::Config,
    gui::{
        button,
        common::{BrowseSubject, Message, TextHistories, UndoSubject},
        style,
        widget::{Column, Container, Row, Text},
    },
    lang::Translator,
};

#[derive(Default)]
pub struct IgnoredItemsEditor {}

impl IgnoredItemsEditor {
    pub fn view<'a>(config: &Config, translator: &Translator, histories: &TextHistories) -> Container<'a> {
        Container::new({
            Column::new().width(Length::Fill).height(Length::Fill).spacing(10).push(
                Container::new(
                    Column::new()
                        .padding(5)
                        .spacing(5)
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(100)
                                        .push(Text::new(translator.custom_files_label())),
                                )
                                .push(
                                    config
                                        .backup
                                        .filter
                                        .ignored_paths
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(button::move_up(Message::EditedBackupFilterIgnoredPath, ii))
                                                    .push(button::move_down(
                                                        Message::EditedBackupFilterIgnoredPath,
                                                        ii,
                                                        config.backup.filter.ignored_paths.len(),
                                                    ))
                                                    .push(histories.input(UndoSubject::BackupFilterIgnoredPath(ii)))
                                                    .push(button::open_folder(BrowseSubject::BackupFilterIgnoredPath(
                                                        ii,
                                                    )))
                                                    .push(button::remove(Message::EditedBackupFilterIgnoredPath, ii)),
                                            )
                                        })
                                        .push(button::add(Message::EditedBackupFilterIgnoredPath)),
                                ),
                        )
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(100)
                                        .push(Text::new(translator.custom_registry_label())),
                                )
                                .push(
                                    config
                                        .backup
                                        .filter
                                        .ignored_registry
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .push(button::move_up(
                                                        Message::EditedBackupFilterIgnoredRegistry,
                                                        ii,
                                                    ))
                                                    .push(button::move_down(
                                                        Message::EditedBackupFilterIgnoredRegistry,
                                                        ii,
                                                        config.backup.filter.ignored_registry.len(),
                                                    ))
                                                    .push(histories.input(UndoSubject::BackupFilterIgnoredRegistry(ii)))
                                                    .push(button::remove(
                                                        Message::EditedBackupFilterIgnoredRegistry,
                                                        ii,
                                                    )),
                                            )
                                        })
                                        .push(button::add(Message::EditedBackupFilterIgnoredRegistry)),
                                ),
                        ),
                )
                .style(style::Container::GameListEntry),
            )
        })
    }
}
