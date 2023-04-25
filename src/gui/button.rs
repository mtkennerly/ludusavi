use iced::alignment;

use crate::{
    gui::{
        common::{
            BackupPhase, BrowseFileSubject, BrowseSubject, EditAction, Message, OngoingOperation, RestorePhase, Screen,
        },
        icon::Icon,
        style,
        widget::{Button, Element, IcedButtonExt, Text},
    },
    lang::TRANSLATOR,
    prelude::SyncDirection,
};

fn template(content: Text, action: Option<Message>, style: Option<style::Button>) -> Element {
    Button::new(content.horizontal_alignment(alignment::Horizontal::Center))
        .on_press_some(action)
        .style(style.unwrap_or(style::Button::Primary))
        .into()
}

pub fn add<'a>(action: fn(EditAction) -> Message) -> Element<'a> {
    template(Icon::AddCircle.as_text(), Some(action(EditAction::Add)), None)
}

pub fn add_nested<'a>(action: fn(usize, EditAction) -> Message, parent: usize) -> Element<'a> {
    template(Icon::AddCircle.as_text(), Some(action(parent, EditAction::Add)), None)
}

pub fn remove<'a>(action: fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::RemoveCircle.as_text(),
        Some(action(EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn remove_nested<'a>(action: fn(usize, EditAction) -> Message, parent: usize, index: usize) -> Element<'a> {
    template(
        Icon::RemoveCircle.as_text(),
        Some(action(parent, EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn delete<'a>(action: fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::Delete.as_text(),
        Some(action(EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn close<'a>(action: Message) -> Element<'a> {
    template(
        Icon::Close.as_text().width(15).size(15),
        Some(action),
        Some(style::Button::Negative),
    )
}

pub fn choose_folder<'a>(subject: BrowseSubject) -> Element<'a> {
    template(Icon::FolderOpen.as_text(), Some(Message::BrowseDir(subject)), None)
}

pub fn choose_file<'a>(subject: BrowseFileSubject) -> Element<'a> {
    template(Icon::FileOpen.as_text(), Some(Message::BrowseFile(subject)), None)
}

pub fn filter<'a>(screen: Screen, open: bool) -> Element<'a> {
    template(
        Icon::Filter.as_text(),
        Some(Message::ToggleSearch { screen }),
        open.then_some(style::Button::Negative),
    )
}

pub fn sort_order<'a>(screen: Screen, reversed: bool) -> Element<'a> {
    template(
        if reversed {
            Icon::ArrowDownward.as_text()
        } else {
            Icon::ArrowUpward.as_text()
        },
        Some(Message::EditedSortReversed {
            screen,
            value: !reversed,
        }),
        None,
    )
}

pub fn refresh<'a>(action: Message, ongoing: bool) -> Element<'a> {
    template(Icon::Refresh.as_text(), (!ongoing).then_some(action), None)
}

pub fn settings<'a>(open: bool) -> Element<'a> {
    template(
        Icon::Settings.as_text(),
        Some(Message::ToggleBackupSettings),
        open.then_some(style::Button::Negative),
    )
}

pub fn move_up<'a>(action: fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::ArrowUpward.as_text().width(15).size(15),
        (index > 0).then(|| action(EditAction::move_up(index))),
        None,
    )
}

pub fn move_up_nested<'a>(action: fn(usize, EditAction) -> Message, parent: usize, index: usize) -> Element<'a> {
    template(
        Icon::ArrowUpward.as_text().width(15).size(15),
        (index > 0).then(|| action(parent, EditAction::move_up(index))),
        None,
    )
}

pub fn move_down<'a>(action: fn(EditAction) -> Message, index: usize, max: usize) -> Element<'a> {
    template(
        Icon::ArrowDownward.as_text().width(15).size(15),
        (index < max - 1).then(|| action(EditAction::move_down(index))),
        None,
    )
}

pub fn move_down_nested<'a>(
    action: fn(usize, EditAction) -> Message,
    parent: usize,
    index: usize,
    max: usize,
) -> Element<'a> {
    template(
        Icon::ArrowDownward.as_text().width(15).size(15),
        (index < max - 1).then(|| action(parent, EditAction::move_down(index))),
        None,
    )
}

pub fn next_page<'a>(action: fn(usize) -> Message, page: usize, pages: usize) -> Element<'a> {
    template(
        Icon::ArrowForward.as_text(),
        (page < pages).then(|| action(page + 1)),
        None,
    )
}

pub fn previous_page<'a>(action: fn(usize) -> Message, page: usize) -> Element<'a> {
    template(Icon::ArrowBack.as_text(), (page > 0).then(|| action(page - 1)), None)
}

pub fn toggle_all_scanned_games<'a>(all_enabled: bool) -> Element<'a> {
    if all_enabled {
        template(
            Text::new(TRANSLATOR.deselect_all_button()).width(125),
            Some(Message::DeselectAllGames),
            None,
        )
    } else {
        template(
            Text::new(TRANSLATOR.select_all_button()).width(125),
            Some(Message::SelectAllGames),
            None,
        )
    }
}

pub fn toggle_all_custom_games<'a>(all_enabled: bool) -> Element<'a> {
    if all_enabled {
        template(
            Text::new(TRANSLATOR.disable_all_button()).width(125),
            Some(Message::DeselectAllGames),
            None,
        )
    } else {
        template(
            Text::new(TRANSLATOR.enable_all_button()).width(125),
            Some(Message::SelectAllGames),
            None,
        )
    }
}

pub fn add_game<'a>() -> Element<'a> {
    template(
        Text::new(TRANSLATOR.add_game_button()).width(125),
        Some(Message::EditedCustomGame(EditAction::Add)),
        None,
    )
}

pub fn open_url<'a>(label: String, url: String) -> Element<'a> {
    template(Text::new(label).width(125), Some(Message::OpenUrl(url)), None)
}

pub fn nav<'a>(screen: Screen, current_screen: Screen) -> Button<'a> {
    let text = match screen {
        Screen::Backup => TRANSLATOR.nav_backup_button(),
        Screen::Restore => TRANSLATOR.nav_restore_button(),
        Screen::CustomGames => TRANSLATOR.nav_custom_games_button(),
        Screen::Other => TRANSLATOR.nav_other_button(),
    };

    Button::new(
        Text::new(text)
            .size(16)
            .horizontal_alignment(alignment::Horizontal::Center),
    )
    .on_press(Message::SwitchScreen(screen))
    .padding([5, 20, 5, 20])
    .style(if current_screen == screen {
        style::Button::NavButtonActive
    } else {
        style::Button::NavButtonInactive
    })
}

pub fn upload<'a>(operation: &Option<OngoingOperation>) -> Element<'a> {
    template(
        Icon::Upload.as_text(),
        match operation {
            None => Some(Message::ConfirmSynchronizeCloud {
                direction: SyncDirection::Upload,
            }),
            Some(OngoingOperation::CloudSync {
                direction: SyncDirection::Upload,
                ..
            }) => Some(Message::CancelOperation),
            _ => None,
        },
        match operation {
            Some(
                OngoingOperation::CloudSync {
                    direction: SyncDirection::Upload,
                    ..
                }
                | OngoingOperation::CancelCloudSync {
                    direction: SyncDirection::Upload,
                },
            ) => Some(style::Button::Negative),
            _ => None,
        },
    )
}

pub fn download<'a>(operation: &Option<OngoingOperation>) -> Element<'a> {
    template(
        Icon::Download.as_text(),
        match operation {
            None => Some(Message::ConfirmSynchronizeCloud {
                direction: SyncDirection::Download,
            }),
            Some(OngoingOperation::CloudSync {
                direction: SyncDirection::Download,
                ..
            }) => Some(Message::CancelOperation),
            _ => None,
        },
        match operation {
            Some(
                OngoingOperation::CloudSync {
                    direction: SyncDirection::Download,
                    ..
                }
                | OngoingOperation::CancelCloudSync {
                    direction: SyncDirection::Download,
                },
            ) => Some(style::Button::Negative),
            _ => None,
        },
    )
}

pub fn operation<'a>(action: OngoingOperation, ongoing: Option<OngoingOperation>) -> Element<'a> {
    use OngoingOperation::*;

    template(
        Text::new(match action {
            Backup => match ongoing {
                Some(Backup) => TRANSLATOR.cancel_button(),
                Some(CancelBackup) => TRANSLATOR.cancelling_button(),
                _ => TRANSLATOR.backup_button(),
            },
            PreviewBackup => match ongoing {
                Some(PreviewBackup) => TRANSLATOR.cancel_button(),
                Some(CancelPreviewBackup) => TRANSLATOR.cancelling_button(),
                _ => TRANSLATOR.preview_button(),
            },
            Restore => match ongoing {
                Some(Restore) => TRANSLATOR.cancel_button(),
                Some(CancelRestore) => TRANSLATOR.cancelling_button(),
                _ => TRANSLATOR.restore_button(),
            },
            PreviewRestore => match ongoing {
                Some(PreviewRestore) => TRANSLATOR.cancel_button(),
                Some(CancelPreviewRestore) => TRANSLATOR.cancelling_button(),
                _ => TRANSLATOR.preview_button(),
            },
            CancelBackup | CancelPreviewBackup | CancelRestore | CancelPreviewRestore => TRANSLATOR.cancel_button(),
            CloudSync { .. } | CancelCloudSync { .. } => "".to_string(),
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Some(ongoing) => (action == ongoing).then_some(Message::CancelOperation),
            None => match action {
                Backup => Some(Message::Backup(BackupPhase::Confirm { games: None })),
                PreviewBackup => Some(Message::Backup(BackupPhase::Start {
                    preview: true,
                    games: None,
                })),
                Restore => Some(Message::Restore(RestorePhase::Confirm { games: None })),
                PreviewRestore => Some(Message::Restore(RestorePhase::Start {
                    preview: true,
                    games: None,
                })),
                _ => None,
            },
        },
        match (action, ongoing) {
            (Backup, Some(Backup | CancelBackup)) => Some(style::Button::Negative),
            (PreviewBackup, Some(PreviewBackup | CancelPreviewBackup)) => Some(style::Button::Negative),
            (Restore, Some(Restore | CancelRestore)) => Some(style::Button::Negative),
            (PreviewRestore, Some(PreviewRestore | CancelPreviewRestore)) => Some(style::Button::Negative),
            _ => None,
        },
    )
}
