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
        widget::{checkbox, pick_list, text, Column, Container, IcedParentExt, Row, Tooltip},
    },
    lang::TRANSLATOR,
    resource::{
        cache::Cache,
        config::{Config, RedirectKind},
        manifest::Store,
    },
};

pub fn root<'a>(config: &Config, histories: &TextHistories, modifiers: &keyboard::Modifiers) -> Container<'a> {
    let mut content = Column::new().width(Length::Fill).spacing(5);
    if config.roots.is_empty() {
        content = content.push(text(TRANSLATOR.no_roots_are_configured()));
    } else {
        content = config.roots.iter().enumerate().fold(content, |parent, (i, root)| {
            parent.push(
                Row::new()
                    .spacing(20)
                    .push(button::move_up(Message::EditedRoot, i))
                    .push(button::move_down(Message::EditedRoot, i, config.roots.len()))
                    .push(histories.input(UndoSubject::Root(i)))
                    .push(
                        pick_list(Store::ALL, Some(root.store), move |v| Message::SelectedRootStore(i, v))
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

pub fn manifest<'a>(config: &Config, cache: &'a Cache, histories: &TextHistories) -> Container<'a> {
    let label_width = Length::Fixed(160.0);
    let left_offset = Length::Fixed(70.0);
    let right_offset = Length::Fixed(70.0);

    let get_checked = |url: &str, cache: &'a Cache| {
        let cached = cache.manifests.get(url)?;
        let checked = match cached.checked {
            Some(x) => chrono::DateTime::<chrono::Local>::from(x)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            None => "?".to_string(),
        };
        Some(Container::new(text(checked)).width(label_width))
    };

    let get_updated = |url: &str, cache: &'a Cache| {
        let cached = cache.manifests.get(url)?;
        let updated = match cached.updated {
            Some(x) => chrono::DateTime::<chrono::Local>::from(x)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            None => "?".to_string(),
        };
        Some(Container::new(text(updated)).width(label_width))
    };

    let mut content = Column::new()
        .padding(5)
        .spacing(5)
        .push(
            Row::new()
                .spacing(20)
                .align_items(Alignment::Center)
                .push_if(
                    || !config.manifest.secondary.is_empty(),
                    || Space::with_width(left_offset),
                )
                .push(text(TRANSLATOR.url_label()).width(Length::Fill))
                .push(Container::new(text(TRANSLATOR.checked_label())).width(label_width))
                .push(Container::new(text(TRANSLATOR.updated_label())).width(label_width))
                .push_if(
                    || !config.manifest.secondary.is_empty(),
                    || Space::with_width(right_offset),
                ),
        )
        .push(
            Row::new()
                .spacing(20)
                .align_items(Alignment::Center)
                .push_if(
                    || !config.manifest.secondary.is_empty(),
                    || Space::with_width(left_offset),
                )
                .push(iced::widget::TextInput::new("", &config.manifest.url).width(Length::Fill))
                .push_some(|| get_checked(&config.manifest.url, cache))
                .push_some(|| get_updated(&config.manifest.url, cache))
                .push_if(
                    || !config.manifest.secondary.is_empty(),
                    || Space::with_width(right_offset),
                ),
        );

    content = config
        .manifest
        .secondary
        .iter()
        .enumerate()
        .fold(content, |column, (i, _)| {
            column.push(
                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(button::move_up(Message::EditedSecondaryManifest, i))
                    .push(button::move_down(
                        Message::EditedSecondaryManifest,
                        i,
                        config.manifest.secondary.len(),
                    ))
                    .push(histories.input(UndoSubject::SecondaryManifest(i)))
                    .push_some(|| get_checked(&config.manifest.secondary[i], cache))
                    .push_some(|| get_updated(&config.manifest.secondary[i], cache))
                    .push(button::remove(Message::EditedSecondaryManifest, i)),
            )
        });

    content = content.push(button::add(Message::EditedSecondaryManifest));

    Container::new(content).style(style::Container::GameListEntry)
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
                            pick_list(RedirectKind::ALL, Some(redirects[i].kind), move |v| {
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
                                            checkbox("", config.is_custom_game_enabled(i), move |enabled| {
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
                                        button::refresh_custom_game(
                                            Message::Backup(BackupPhase::Start {
                                                games: Some(vec![config.custom_games[i].name.clone()]),
                                                preview: true,
                                                repair: false,
                                            }),
                                            operating,
                                            config.is_custom_game_enabled(i),
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
                                .push(Column::new().width(130).push(text(TRANSLATOR.custom_files_label())))
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
                                .push(Column::new().width(130).push(text(TRANSLATOR.custom_registry_label())))
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
                .id(iced::widget::container::Id::new(config.custom_games[i].name.clone()))
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
                            .push(Column::new().width(100).push(text(TRANSLATOR.custom_files_label())))
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
                            .push(Column::new().width(100).push(text(TRANSLATOR.custom_registry_label())))
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
