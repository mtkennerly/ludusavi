use iced::alignment;

use crate::{
    gui::{
        common::{
            BackupPhase, BrowseFileSubject, BrowseSubject, EditAction, Message, Operation, RestorePhase, Screen,
            ValidatePhase,
        },
        icon::Icon,
        style,
        widget::{Button, Element, IcedButtonExt, Text},
    },
    lang::TRANSLATOR,
    prelude::{Finality, SyncDirection},
};

fn template(content: Text, action: Option<Message>, style: Option<style::Button>) -> Element {
    Button::new(content.horizontal_alignment(alignment::Horizontal::Center))
        .on_press_some(action)
        .style(style.unwrap_or(style::Button::Primary))
        .into()
}

pub fn primary<'a>(content: String, action: Option<Message>) -> Element<'a> {
    Button::new(Text::new(content).horizontal_alignment(alignment::Horizontal::Center))
        .on_press_some(action)
        .style(style::Button::Primary)
        .width(125)
        .into()
}

pub fn negative<'a>(content: String, action: Option<Message>) -> Element<'a> {
    Button::new(Text::new(content).horizontal_alignment(alignment::Horizontal::Center))
        .on_press_some(action)
        .style(style::Button::Negative)
        .width(125)
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
    template(Icon::FolderOpen.as_text(), Some(Message::BrowseFile(subject)), None)
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

pub fn search<'a>(action: Message) -> Element<'a> {
    template(Icon::Search.as_text(), Some(action), None)
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

pub fn upload<'a>(operation: &Operation) -> Element<'a> {
    template(
        Icon::Upload.as_text(),
        match operation {
            Operation::Idle => Some(Message::ConfirmSynchronizeCloud {
                direction: SyncDirection::Upload,
            }),
            Operation::Cloud {
                direction: SyncDirection::Upload,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        match operation {
            Operation::Cloud {
                direction: SyncDirection::Upload,
                ..
            } => Some(style::Button::Negative),
            _ => None,
        },
    )
}

pub fn download<'a>(operation: &Operation) -> Element<'a> {
    template(
        Icon::Download.as_text(),
        match operation {
            Operation::Idle => Some(Message::ConfirmSynchronizeCloud {
                direction: SyncDirection::Download,
            }),
            Operation::Cloud {
                direction: SyncDirection::Download,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        match operation {
            Operation::Cloud {
                direction: SyncDirection::Download,
                ..
            } => Some(style::Button::Negative),
            _ => None,
        },
    )
}

pub fn backup<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        Text::new(match ongoing {
            Operation::Backup {
                finality: Finality::Final,
                cancelling: false,
                ..
            } => TRANSLATOR.cancel_button(),
            Operation::Backup {
                finality: Finality::Final,
                cancelling: true,
                ..
            } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.backup_button(),
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::Backup(BackupPhase::Confirm { games: None })),
            Operation::Backup {
                finality: Finality::Final,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(
            ongoing,
            Operation::Backup {
                finality: Finality::Final,
                ..
            }
        )
        .then_some(style::Button::Negative),
    )
}

pub fn backup_preview<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        Text::new(match ongoing {
            Operation::Backup {
                finality: Finality::Preview,
                cancelling: false,
                ..
            } => TRANSLATOR.cancel_button(),
            Operation::Backup {
                finality: Finality::Preview,
                cancelling: true,
                ..
            } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.preview_button(),
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::Backup(BackupPhase::Start {
                preview: true,
                repair: false,
                games: None,
            })),
            Operation::Backup {
                finality: Finality::Preview,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(
            ongoing,
            Operation::Backup {
                finality: Finality::Preview,
                ..
            }
        )
        .then_some(style::Button::Negative),
    )
}

pub fn restore<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        Text::new(match ongoing {
            Operation::Restore {
                finality: Finality::Final,
                cancelling: false,
                ..
            } => TRANSLATOR.cancel_button(),
            Operation::Restore {
                finality: Finality::Final,
                cancelling: true,
                ..
            } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.restore_button(),
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::Restore(RestorePhase::Confirm { games: None })),
            Operation::Restore {
                finality: Finality::Final,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(
            ongoing,
            Operation::Restore {
                finality: Finality::Final,
                ..
            }
        )
        .then_some(style::Button::Negative),
    )
}

pub fn restore_preview<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        Text::new(match ongoing {
            Operation::Restore {
                finality: Finality::Preview,
                cancelling: false,
                ..
            } => TRANSLATOR.cancel_button(),
            Operation::Restore {
                finality: Finality::Preview,
                cancelling: true,
                ..
            } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.preview_button(),
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::Restore(RestorePhase::Start {
                preview: true,
                games: None,
            })),
            Operation::Restore {
                finality: Finality::Preview,
                cancelling: false,
                ..
            } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(
            ongoing,
            Operation::Restore {
                finality: Finality::Preview,
                ..
            }
        )
        .then_some(style::Button::Negative),
    )
}

pub fn validate_backups<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        Text::new(match ongoing {
            Operation::ValidateBackups { cancelling: false, .. } => TRANSLATOR.cancel_button(),
            Operation::ValidateBackups { cancelling: true, .. } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.validate_button(),
        })
        .width(125)
        .horizontal_alignment(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::ValidateBackups(ValidatePhase::Start)),
            Operation::ValidateBackups { cancelling: false, .. } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(ongoing, Operation::ValidateBackups { .. }).then_some(style::Button::Negative),
    )
}
