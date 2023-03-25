use iced::alignment;

use crate::{
    gui::{
        common::{BrowseSubject, EditAction, Message, OngoingOperation, Screen},
        icon::Icon,
        style,
        widget::{Button, Element, IcedButtonExt, Text},
    },
    lang::TRANSLATOR,
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

pub fn open_folder<'a>(subject: BrowseSubject) -> Element<'a> {
    template(Icon::FolderOpen.as_text(), Some(Message::BrowseDir(subject)), None)
}

pub fn search<'a>(screen: Screen, open: bool) -> Element<'a> {
    template(
        Icon::Search.as_text(),
        Some(Message::ToggleSearch { screen }),
        open.then_some(style::Button::Negative),
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
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Some(ongoing) => (action == ongoing).then_some(Message::CancelOperation),
            None => match action {
                Backup => Some(Message::ConfirmBackupStart { games: None }),
                PreviewBackup => Some(Message::BackupPrep {
                    preview: true,
                    games: None,
                }),
                Restore => Some(Message::ConfirmRestoreStart { games: None }),
                PreviewRestore => Some(Message::RestoreStart {
                    preview: true,
                    games: None,
                }),
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
