use iced::{alignment, keyboard, Length};

use crate::{
    gui::{
        common::{
            BackupPhase, BrowseFileSubject, BrowseSubject, Message, Operation, RestorePhase, Screen, ValidatePhase,
        },
        icon::Icon,
        style,
        widget::{text, Button, Container, Element, Row, Text, Tooltip},
    },
    lang::TRANSLATOR,
    prelude::{EditAction, Finality, SyncDirection},
    resource::{config, manifest},
    scan::game_filter,
};

const WIDTH: u32 = 125;

fn template(content: Text, action: Option<Message>, style: Option<style::Button>) -> Element {
    Button::new(content.align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style.unwrap_or(style::Button::Primary))
        .padding(5)
        .into()
}

fn template_bare(content: Text, action: Option<Message>, style: Option<style::Button>) -> Element {
    Button::new(content.align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style.unwrap_or(style::Button::Primary))
        .padding(0)
        .into()
}

fn template_extended(
    content: Text,
    action: Option<Message>,
    style: Option<style::Button>,
    icon: Option<Icon>,
    tooltip: Option<String>,
) -> Element {
    let button = match icon {
        Some(icon) => template_complex(
            Container::new(
                Row::new()
                    .spacing(5)
                    .push(icon.text_narrow())
                    .push(content.width(Length::Shrink)),
            )
            .center_x(WIDTH),
            action,
            style,
        ),
        None => template(content, action, style),
    };

    match tooltip {
        Some(tooltip) => Tooltip::new(button, text(tooltip), iced::widget::tooltip::Position::Top)
            .class(style::Container::Tooltip)
            .into(),
        None => button,
    }
}

fn template_complex<'a>(
    content: impl Into<Element<'a>>,
    action: Option<Message>,
    style: Option<style::Button>,
) -> Element<'a> {
    Button::new(content)
        .on_press_maybe(action)
        .class(style.unwrap_or(style::Button::Primary))
        .padding(5)
        .into()
}

pub fn primary<'a>(content: String, action: Option<Message>) -> Element<'a> {
    Button::new(text(content).align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style::Button::Primary)
        .padding(5)
        .width(WIDTH)
        .into()
}

pub fn negative<'a>(content: String, action: Option<Message>) -> Element<'a> {
    Button::new(text(content).align_x(alignment::Horizontal::Center))
        .on_press_maybe(action)
        .class(style::Button::Negative)
        .width(WIDTH)
        .padding(5)
        .into()
}

pub fn add<'a>(action: impl Fn(EditAction) -> Message) -> Element<'a> {
    template(Icon::AddCircle.text(), Some(action(EditAction::Add)), None)
}

pub fn add_nested<'a>(action: impl Fn(usize, EditAction) -> Message, parent: usize) -> Element<'a> {
    template(Icon::AddCircle.text(), Some(action(parent, EditAction::Add)), None)
}

pub fn remove<'a>(action: impl Fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::RemoveCircle.text(),
        Some(action(EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn remove_nested<'a>(action: impl Fn(usize, EditAction) -> Message, parent: usize, index: usize) -> Element<'a> {
    template(
        Icon::RemoveCircle.text(),
        Some(action(parent, EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn delete<'a>(action: impl Fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::Delete.text(),
        Some(action(EditAction::Remove(index))),
        Some(style::Button::Negative),
    )
}

pub fn hide<'a>(action: Message) -> Element<'a> {
    template(Icon::VisibilityOff.text_small(), Some(action), None)
}

pub fn choose_folder<'a>(subject: BrowseSubject, modifiers: &keyboard::Modifiers) -> Element<'a> {
    if modifiers.shift() {
        template(Icon::OpenInNew.text(), Some(Message::OpenDirSubject(subject)), None)
    } else {
        template(Icon::FolderOpen.text(), Some(Message::BrowseDir(subject)), None)
    }
}

pub fn choose_file<'a>(subject: BrowseFileSubject, modifiers: &keyboard::Modifiers) -> Element<'a> {
    if modifiers.shift() {
        template(Icon::OpenInNew.text(), Some(Message::OpenFileSubject(subject)), None)
    } else {
        template(Icon::FolderOpen.text(), Some(Message::BrowseFile(subject)), None)
    }
}

pub fn filter<'a>(open: bool) -> Element<'a> {
    template(
        Icon::Filter.text(),
        Some(Message::Filter {
            event: game_filter::Event::Toggled,
        }),
        open.then_some(style::Button::Negative),
    )
}

pub fn reset_filter<'a>(dirty: bool) -> Element<'a> {
    template(
        Icon::RemoveCircle.text(),
        dirty.then_some(Message::Filter {
            event: game_filter::Event::Reset,
        }),
        Some(style::Button::Negative),
    )
}

pub fn sort<'a>(message: impl Into<Message>) -> Element<'a> {
    template(text(TRANSLATOR.sort_button()).width(WIDTH), Some(message.into()), None)
}

pub fn sort_order<'a>(reversed: bool) -> Element<'a> {
    template(
        if reversed {
            Icon::ArrowDownward.text()
        } else {
            Icon::ArrowUpward.text()
        },
        Some(config::Event::SortReversed(!reversed).into()),
        None,
    )
}

pub fn refresh<'a>(action: Message, ongoing: bool) -> Element<'a> {
    template(Icon::Refresh.text(), (!ongoing).then_some(action), None)
}

pub fn refresh_custom_game<'a>(action: Message, ongoing: bool, enabled: bool) -> Element<'a> {
    template(Icon::Refresh.text(), (!ongoing && enabled).then_some(action), None)
}

pub fn search<'a>(action: Message) -> Element<'a> {
    template(Icon::Search.text(), Some(action), None)
}

pub fn move_up<'a>(action: impl Fn(EditAction) -> Message, index: usize) -> Element<'a> {
    template(
        Icon::ArrowUpward.text_small(),
        (index > 0).then(|| action(EditAction::move_up(index))),
        None,
    )
}

pub fn move_up_maybe<'a>(action: impl Fn(EditAction) -> Message, index: usize, enabled: bool) -> Element<'a> {
    template(
        Icon::ArrowUpward.text_small(),
        (enabled && index > 0).then(|| action(EditAction::move_up(index))),
        None,
    )
}

pub fn move_up_nested<'a>(action: impl Fn(usize, EditAction) -> Message, parent: usize, index: usize) -> Element<'a> {
    template(
        Icon::ArrowUpward.text_small(),
        (index > 0).then(|| action(parent, EditAction::move_up(index))),
        None,
    )
}

pub fn move_down<'a>(action: impl Fn(EditAction) -> Message, index: usize, max: usize) -> Element<'a> {
    template(
        Icon::ArrowDownward.text_small(),
        (index < max - 1).then(|| action(EditAction::move_down(index))),
        None,
    )
}

pub fn move_down_maybe<'a>(
    action: impl Fn(EditAction) -> Message,
    index: usize,
    max: usize,
    enabled: bool,
) -> Element<'a> {
    template(
        Icon::ArrowDownward.text_small(),
        (enabled && index < max - 1).then(|| action(EditAction::move_down(index))),
        None,
    )
}

pub fn move_down_nested<'a>(
    action: impl Fn(usize, EditAction) -> Message,
    parent: usize,
    index: usize,
    max: usize,
) -> Element<'a> {
    template(
        Icon::ArrowDownward.text_small(),
        (index < max - 1).then(|| action(parent, EditAction::move_down(index))),
        None,
    )
}

pub fn next_page<'a>(action: impl Fn(usize) -> Message, page: usize, pages: usize) -> Element<'a> {
    template(
        Icon::ArrowForward.text(),
        (page < pages).then(|| action(page + 1)),
        None,
    )
}

pub fn previous_page<'a>(action: impl Fn(usize) -> Message, page: usize) -> Element<'a> {
    template(Icon::ArrowBack.text(), (page > 0).then(|| action(page - 1)), None)
}

pub fn toggle_all_scanned_games<'a>(all_enabled: bool, filtered: bool) -> Element<'a> {
    if all_enabled {
        template_extended(
            text(TRANSLATOR.disable_all_button()).width(WIDTH),
            Some(Message::DeselectAllGames),
            None,
            filtered.then_some(Icon::Filter),
            filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
        )
    } else {
        template_extended(
            text(TRANSLATOR.enable_all_button()).width(WIDTH),
            Some(Message::SelectAllGames),
            None,
            filtered.then_some(Icon::Filter),
            filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
        )
    }
}

pub fn toggle_all_custom_games<'a>(all_enabled: bool, filtered: bool) -> Element<'a> {
    if all_enabled {
        template_extended(
            text(TRANSLATOR.disable_all_button()).width(WIDTH),
            Some(Message::DeselectAllGames),
            None,
            filtered.then_some(Icon::Filter),
            filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
        )
    } else {
        template_extended(
            text(TRANSLATOR.enable_all_button()).width(WIDTH),
            Some(Message::SelectAllGames),
            None,
            filtered.then_some(Icon::Filter),
            filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
        )
    }
}

pub fn add_game<'a>() -> Element<'a> {
    template(
        text(TRANSLATOR.add_game_button()).width(WIDTH),
        Some(config::Event::CustomGame(EditAction::Add).into()),
        None,
    )
}

pub fn open_url<'a>(label: String, url: String) -> Element<'a> {
    template(text(label).width(WIDTH), Some(Message::OpenUrl(url)), None)
}

pub fn open_url_icon<'a>(url: String) -> Element<'a> {
    template(Icon::OpenInBrowser.text(), Some(Message::OpenUrl(url)), None)
}

pub fn nav<'a>(screen: Screen, current_screen: Screen) -> Button<'a> {
    let label = match screen {
        Screen::Backup => TRANSLATOR.nav_backup_button(),
        Screen::Restore => TRANSLATOR.nav_restore_button(),
        Screen::CustomGames => TRANSLATOR.nav_custom_games_button(),
        Screen::Other => TRANSLATOR.nav_other_button(),
    };

    Button::new(text(label).size(14).align_x(alignment::Horizontal::Center))
        .on_press(Message::SwitchScreen(screen))
        .padding([5, 20])
        .class(if current_screen == screen {
            style::Button::NavButtonActive
        } else {
            style::Button::NavButtonInactive
        })
}

pub fn upload<'a>(operation: &Operation) -> Element<'a> {
    template(
        Icon::Upload.text(),
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
        Icon::Download.text(),
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

pub fn backup<'a>(ongoing: &Operation, filtered: bool) -> Element<'a> {
    template_extended(
        text(match ongoing {
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
        .width(WIDTH)
        .align_x(alignment::Horizontal::Center),
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
        filtered.then_some(Icon::Filter),
        filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
    )
}

pub fn backup_preview<'a>(ongoing: &Operation, filtered: bool) -> Element<'a> {
    template_extended(
        text(match ongoing {
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
        .width(WIDTH)
        .align_x(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::Backup(BackupPhase::Start {
                preview: true,
                repair: false,
                jump: false,
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
        filtered.then_some(Icon::Filter),
        filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
    )
}

pub fn restore<'a>(ongoing: &Operation, filtered: bool) -> Element<'a> {
    template_extended(
        text(match ongoing {
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
        .width(WIDTH)
        .align_x(alignment::Horizontal::Center),
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
        filtered.then_some(Icon::Filter),
        filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
    )
}

pub fn restore_preview<'a>(ongoing: &Operation, filtered: bool) -> Element<'a> {
    template_extended(
        text(match ongoing {
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
        .width(WIDTH)
        .align_x(alignment::Horizontal::Center),
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
        filtered.then_some(Icon::Filter),
        filtered.then(|| TRANSLATOR.operation_will_only_include_listed_games()),
    )
}

pub fn validate_backups<'a>(ongoing: &Operation) -> Element<'a> {
    template(
        text(match ongoing {
            Operation::ValidateBackups { cancelling: false, .. } => TRANSLATOR.cancel_button(),
            Operation::ValidateBackups { cancelling: true, .. } => TRANSLATOR.cancelling_button(),
            _ => TRANSLATOR.validate_button(),
        })
        .width(WIDTH)
        .align_x(alignment::Horizontal::Center),
        match ongoing {
            Operation::Idle => Some(Message::ValidateBackups(ValidatePhase::Start)),
            Operation::ValidateBackups { cancelling: false, .. } => Some(Message::CancelOperation),
            _ => None,
        },
        matches!(ongoing, Operation::ValidateBackups { .. }).then_some(style::Button::Negative),
    )
}

pub fn show_game_notes<'a>(game: String, notes: Vec<manifest::Note>) -> Element<'a> {
    template_bare(
        Icon::Info.text_narrow(),
        Some(Message::ShowGameNotes { game, notes }),
        Some(style::Button::Bare),
    )
}

pub fn expand<'a>(expanded: bool, on_press: Message) -> Element<'a> {
    Button::new(
        (if expanded {
            Icon::KeyboardArrowDown
        } else {
            Icon::KeyboardArrowRight
        })
        .text_small(),
    )
    .on_press(on_press)
    .class(style::Button::Primary)
    .padding(5)
    .height(25)
    .width(25)
    .into()
}
