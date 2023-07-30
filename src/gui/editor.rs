use iced::{
    keyboard,
    widget::{tooltip, Space},
    Alignment, Length,
};

use crate::{
    gui::{
        button,
        common::{BackupPhase, BrowseSubject, Message, ScrollSubject, UndoSubject},
        shortcuts::TextHistories,
        style,
        widget::{Checkbox, Column, Container, PickList, Row, Text, Tooltip},
    },
    lang::TRANSLATOR,
    resource::{
        config::{Config, RedirectKind},
        manifest::Store,
    },
};

pub fn root<'a>(config: &Config, histories: &TextHistories, modifiers: &keyboard::Modifiers) -> Container<'a> {
    let mut content = Column::new().width(Length::Fill).spacing(5);
    if config.roots.is_empty() {
        content = content.push(Text::new(TRANSLATOR.no_roots_are_configured()));
    } else {
        content = config.roots.iter().enumerate().fold(content, |parent, (i, root)| {
            parent.push(
                Row::new()
                    .spacing(20)
                    .push(button::move_up(Message::EditedRoot, i))
                    .push(button::move_down(Message::EditedRoot, i, config.roots.len()))
                    .push(histories.input(UndoSubject::Root(i)))
                    .push(
                        PickList::new(Store::ALL, Some(root.store), move |v| Message::SelectedRootStore(i, v))
                            .style(style::PickList::Primary),
                    )
                    .push(button::choose_folder(BrowseSubject::Root(i), modifiers))
                    .push(button::remove(Message::EditedRoot, i)),
            )
        });
    };

    content = content.push(
        Row::new()
            .spacing(20)
            .push(button::add(Message::EditedRoot))
            .push(button::search(Message::FindRoots)),
    );

    Container::new(content)
}

pub fn redirect<'a>(config: &Config, histories: &TextHistories, modifiers: &keyboard::Modifiers) -> Container<'a> {
    let redirects = config.get_redirects();

    let inner = Container::new({
        config
            .redirects
            .iter()
            .enumerate()
            .fold(Column::new().padding(5).spacing(4), |parent, (i, _)| {
                parent.push(
                    Row::new()
                        .spacing(20)
                        .push(button::move_up(|x| Message::EditedRedirect(x, None), i))
                        .push(button::move_down(
                            |x| Message::EditedRedirect(x, None),
                            i,
                            config.redirects.len(),
                        ))
                        .push(
                            PickList::new(RedirectKind::ALL, Some(redirects[i].kind), move |v| {
                                Message::SelectedRedirectKind(i, v)
                            })
                            .style(style::PickList::Primary),
                        )
                        .push(histories.input(UndoSubject::RedirectSource(i)))
                        .push(button::choose_folder(BrowseSubject::RedirectSource(i), modifiers))
                        .push(histories.input(UndoSubject::RedirectTarget(i)))
                        .push(button::choose_folder(BrowseSubject::RedirectTarget(i), modifiers))
                        .push(button::remove(|x| Message::EditedRedirect(x, None), i)),
                )
            })
            .push(button::add(|x| Message::EditedRedirect(x, None)))
    })
    .style(style::Container::GameListEntry);

    Container::new(inner)
}

pub fn custom_games<'a>(
    config: &Config,
    operating: bool,
    histories: &TextHistories,
    modifiers: &keyboard::Modifiers,
) -> Container<'a> {
    if config.custom_games.is_empty() {
        return Container::new(Space::new(Length::Shrink, Length::Shrink));
    }

    let content = config.custom_games.iter().enumerate().fold(
        Column::new().width(Length::Fill).padding([0, 15, 5, 15]).spacing(10),
        |parent, (i, x)| {
            parent.push(
                Container::new(
                    Column::new()
                        .padding(5)
                        .spacing(5)
                        .push(
                            Row::new()
                                .spacing(20)
                                .align_items(iced::Alignment::Center)
                                .push(
                                    Row::new()
                                        .width(110)
                                        .spacing(20)
                                        .align_items(Alignment::Center)
                                        .push(
                                            Checkbox::new("", config.is_custom_game_enabled(i), move |enabled| {
                                                Message::ToggleCustomGameEnabled { index: i, enabled }
                                            })
                                            .spacing(0)
                                            .style(style::Checkbox),
                                        )
                                        .push(button::move_up(Message::EditedCustomGame, i))
                                        .push(button::move_down(
                                            Message::EditedCustomGame,
                                            i,
                                            config.custom_games.len(),
                                        )),
                                )
                                .push(histories.input(UndoSubject::CustomGameName(i)))
                                .push(
                                    Tooltip::new(
                                        button::refresh(
                                            Message::Backup(BackupPhase::Start {
                                                games: Some(vec![config.custom_games[i].name.clone()]),
                                                preview: true,
                                                repair: false,
                                            }),
                                            operating,
                                        ),
                                        TRANSLATOR.preview_button_in_custom_mode(),
                                        tooltip::Position::Top,
                                    )
                                    .size(16)
                                    .gap(5)
                                    .style(style::Container::Tooltip),
                                )
                                .push(button::delete(Message::EditedCustomGame, i)),
                        )
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(130)
                                        .push(Text::new(TRANSLATOR.custom_files_label())),
                                )
                                .push(
                                    x.files
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .align_items(Alignment::Center)
                                                    .spacing(20)
                                                    .push(button::move_up_nested(Message::EditedCustomGameFile, i, ii))
                                                    .push(button::move_down_nested(
                                                        Message::EditedCustomGameFile,
                                                        i,
                                                        ii,
                                                        x.files.len(),
                                                    ))
                                                    .push(histories.input(UndoSubject::CustomGameFile(i, ii)))
                                                    .push(button::choose_folder(
                                                        BrowseSubject::CustomGameFile(i, ii),
                                                        modifiers,
                                                    ))
                                                    .push(button::remove_nested(Message::EditedCustomGameFile, i, ii)),
                                            )
                                        })
                                        .push(button::add_nested(Message::EditedCustomGameFile, i)),
                                ),
                        )
                        .push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .width(130)
                                        .push(Text::new(TRANSLATOR.custom_registry_label())),
                                )
                                .push(
                                    x.registry
                                        .iter()
                                        .enumerate()
                                        .fold(Column::new().spacing(4), |column, (ii, _)| {
                                            column.push(
                                                Row::new()
                                                    .spacing(20)
                                                    .align_items(Alignment::Center)
                                                    .push(button::move_up_nested(
                                                        Message::EditedCustomGameRegistry,
                                                        i,
                                                        ii,
                                                    ))
                                                    .push(button::move_down_nested(
                                                        Message::EditedCustomGameRegistry,
                                                        i,
                                                        ii,
                                                        x.registry.len(),
                                                    ))
                                                    .push(histories.input(UndoSubject::CustomGameRegistry(i, ii)))
                                                    .push(button::remove_nested(
                                                        Message::EditedCustomGameRegistry,
                                                        i,
                                                        ii,
                                                    )),
                                            )
                                        })
                                        .push(button::add_nested(Message::EditedCustomGameRegistry, i)),
                                ),
                        ),
                )
                .style(style::Container::GameListEntry),
            )
        },
    );

    Container::new(ScrollSubject::CustomGames.into_widget(content))
}

pub fn ignored_items<'a>(config: &Config, histories: &TextHistories, modifiers: &keyboard::Modifiers) -> Container<'a> {
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
                                    .push(Text::new(TRANSLATOR.custom_files_label())),
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
                                                .push(button::choose_folder(
                                                    BrowseSubject::BackupFilterIgnoredPath(ii),
                                                    modifiers,
                                                ))
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
                                    .push(Text::new(TRANSLATOR.custom_registry_label())),
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
                                                .push(button::move_up(Message::EditedBackupFilterIgnoredRegistry, ii))
                                                .push(button::move_down(
                                                    Message::EditedBackupFilterIgnoredRegistry,
                                                    ii,
                                                    config.backup.filter.ignored_registry.len(),
                                                ))
                                                .push(histories.input(UndoSubject::BackupFilterIgnoredRegistry(ii)))
                                                .push(button::remove(Message::EditedBackupFilterIgnoredRegistry, ii)),
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
