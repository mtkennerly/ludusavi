// Iced has built-in support for some keyboard shortcuts. This module provides
// support for implementing other shortcuts until Iced provides its own support.

use std::collections::VecDeque;

use iced::Length;

use crate::{
    gui::{
        common::{EditAction, Message, RedirectEditActionField, Screen, UndoSubject},
        style,
        widget::{Element, TextInput, Undoable},
    },
    lang::TRANSLATOR,
    prelude::StrictPath,
    resource::config::Config,
    scan::registry_compat::RegistryItem,
};

pub enum Shortcut {
    Undo,
    Redo,
}

impl Shortcut {
    pub fn apply_to_strict_path_field(&self, config: &mut StrictPath, history: &mut TextHistory) {
        match self {
            Shortcut::Undo => {
                config.reset(history.undo());
            }
            Shortcut::Redo => {
                config.reset(history.redo());
            }
        }
    }

    pub fn apply_to_registry_path_field(&self, config: &mut RegistryItem, history: &mut TextHistory) {
        match self {
            Shortcut::Undo => {
                config.reset(history.undo());
            }
            Shortcut::Redo => {
                config.reset(history.redo());
            }
        }
    }

    pub fn apply_to_string_field(&self, config: &mut String, history: &mut TextHistory) {
        match self {
            Shortcut::Undo => {
                *config = history.undo();
            }
            Shortcut::Redo => {
                *config = history.redo();
            }
        }
    }
}

impl From<crate::gui::undoable::Action> for Shortcut {
    fn from(source: crate::gui::undoable::Action) -> Self {
        match source {
            crate::gui::undoable::Action::Undo => Self::Undo,
            crate::gui::undoable::Action::Redo => Self::Redo,
        }
    }
}

pub struct TextHistory {
    history: VecDeque<String>,
    limit: usize,
    position: usize,
}

impl Default for TextHistory {
    fn default() -> Self {
        Self::new("", 100)
    }
}

impl TextHistory {
    pub fn new(initial: &str, limit: usize) -> Self {
        let mut history = VecDeque::<String>::new();
        history.push_back(initial.to_string());
        Self {
            history,
            limit,
            position: 0,
        }
    }

    pub fn raw(initial: &str) -> Self {
        let mut history = VecDeque::<String>::new();
        history.push_back(initial.to_string());
        Self {
            history,
            limit: 100,
            position: 0,
        }
    }

    pub fn path(initial: &StrictPath) -> Self {
        let mut history = VecDeque::<String>::new();
        history.push_back(initial.raw());
        Self {
            history,
            limit: 100,
            position: 0,
        }
    }

    pub fn push(&mut self, text: &str) {
        if self.current() == text {
            return;
        }
        if self.position + 1 < self.history.len() {
            self.history.truncate(self.position + 1);
        }
        if self.position + 1 >= self.limit {
            self.history.pop_front();
        }
        self.history.push_back(text.to_string());
        self.position = self.history.len() - 1;
    }

    pub fn current(&self) -> String {
        match self.history.get(self.position) {
            Some(x) => x.to_string(),
            None => "".to_string(),
        }
    }

    pub fn undo(&mut self) -> String {
        self.position = if self.position == 0 { 0 } else { self.position - 1 };
        self.current()
    }

    pub fn redo(&mut self) -> String {
        self.position = std::cmp::min(self.position + 1, self.history.len() - 1);
        self.current()
    }
}

#[derive(Default)]
pub struct RedirectHistory {
    pub source: TextHistory,
    pub target: TextHistory,
}

#[derive(Default)]
pub struct CustomGameHistory {
    pub name: TextHistory,
    pub files: Vec<TextHistory>,
    pub registry: Vec<TextHistory>,
}

#[derive(Default)]
pub struct TextHistories {
    pub backup_target: TextHistory,
    pub restore_source: TextHistory,
    pub backup_search_game_name: TextHistory,
    pub restore_search_game_name: TextHistory,
    pub roots: Vec<TextHistory>,
    pub redirects: Vec<RedirectHistory>,
    pub custom_games: Vec<CustomGameHistory>,
    pub backup_filter_ignored_paths: Vec<TextHistory>,
    pub backup_filter_ignored_registry: Vec<TextHistory>,
}

impl TextHistories {
    pub fn new(config: &Config) -> Self {
        let mut histories = Self {
            backup_target: TextHistory::path(&config.backup.path),
            restore_source: TextHistory::path(&config.restore.path),
            backup_search_game_name: TextHistory::raw(""),
            restore_search_game_name: TextHistory::raw(""),
            ..Default::default()
        };

        for x in &config.roots {
            histories.roots.push(TextHistory::path(&x.path));
        }

        for x in &config.redirects {
            histories.redirects.push(RedirectHistory {
                source: TextHistory::path(&x.source),
                target: TextHistory::path(&x.target),
            });
        }

        for x in &config.custom_games {
            let mut custom_history = CustomGameHistory {
                name: TextHistory::raw(&x.name),
                ..Default::default()
            };
            for y in &x.files {
                custom_history.files.push(TextHistory::raw(y));
            }
            for y in &x.registry {
                custom_history.registry.push(TextHistory::raw(y));
            }
            histories.custom_games.push(custom_history);
        }

        for x in &config.backup.filter.ignored_paths {
            histories.backup_filter_ignored_paths.push(TextHistory::path(x));
        }
        for x in &config.backup.filter.ignored_registry {
            histories
                .backup_filter_ignored_registry
                .push(TextHistory::raw(&x.raw()));
        }

        histories
    }

    pub fn input<'a>(&self, subject: UndoSubject) -> Element<'a> {
        let current = match subject {
            UndoSubject::BackupTarget => self.backup_target.current(),
            UndoSubject::RestoreSource => self.restore_source.current(),
            UndoSubject::BackupSearchGameName => self.backup_search_game_name.current(),
            UndoSubject::RestoreSearchGameName => self.restore_search_game_name.current(),
            UndoSubject::Root(i) => self.roots.get(i).map(|x| x.current()).unwrap_or_default(),
            UndoSubject::RedirectSource(i) => self.redirects.get(i).map(|x| x.source.current()).unwrap_or_default(),
            UndoSubject::RedirectTarget(i) => self.redirects.get(i).map(|x| x.target.current()).unwrap_or_default(),
            UndoSubject::CustomGameName(i) => self.custom_games.get(i).map(|x| x.name.current()).unwrap_or_default(),
            UndoSubject::CustomGameFile(i, j) => self
                .custom_games
                .get(i)
                .and_then(|x| x.files.get(j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::CustomGameRegistry(i, j) => self
                .custom_games
                .get(i)
                .and_then(|x| x.registry.get(j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::BackupFilterIgnoredPath(i) => self
                .backup_filter_ignored_paths
                .get(i)
                .map(|x| x.current())
                .unwrap_or_default(),
            UndoSubject::BackupFilterIgnoredRegistry(i) => self
                .backup_filter_ignored_registry
                .get(i)
                .map(|x| x.current())
                .unwrap_or_default(),
        };

        let event: Box<dyn Fn(String) -> Message> = match subject {
            UndoSubject::BackupTarget => Box::new(Message::EditedBackupTarget),
            UndoSubject::RestoreSource => Box::new(Message::EditedRestoreSource),
            UndoSubject::BackupSearchGameName => Box::new(|value| Message::EditedSearchGameName {
                screen: Screen::Backup,
                value,
            }),
            UndoSubject::RestoreSearchGameName => Box::new(|value| Message::EditedSearchGameName {
                screen: Screen::Restore,
                value,
            }),
            UndoSubject::Root(i) => Box::new(move |value| Message::EditedRoot(EditAction::Change(i, value))),
            UndoSubject::RedirectSource(i) => Box::new(move |value| {
                Message::EditedRedirect(EditAction::Change(i, value), Some(RedirectEditActionField::Source))
            }),
            UndoSubject::RedirectTarget(i) => Box::new(move |value| {
                Message::EditedRedirect(EditAction::Change(i, value), Some(RedirectEditActionField::Target))
            }),
            UndoSubject::CustomGameName(i) => {
                Box::new(move |value| Message::EditedCustomGame(EditAction::Change(i, value)))
            }
            UndoSubject::CustomGameFile(i, j) => {
                Box::new(move |value| Message::EditedCustomGameFile(i, EditAction::Change(j, value)))
            }
            UndoSubject::CustomGameRegistry(i, j) => {
                Box::new(move |value| Message::EditedCustomGameRegistry(i, EditAction::Change(j, value)))
            }
            UndoSubject::BackupFilterIgnoredPath(i) => {
                Box::new(move |value| Message::EditedBackupFilterIgnoredPath(EditAction::Change(i, value)))
            }
            UndoSubject::BackupFilterIgnoredRegistry(i) => {
                Box::new(move |value| Message::EditedBackupFilterIgnoredRegistry(EditAction::Change(i, value)))
            }
        };

        let placeholder = match subject {
            UndoSubject::BackupTarget => "".to_string(),
            UndoSubject::RestoreSource => "".to_string(),
            UndoSubject::BackupSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::RestoreSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::Root(_) => "".to_string(),
            UndoSubject::RedirectSource(_) => TRANSLATOR.redirect_source_placeholder(),
            UndoSubject::RedirectTarget(_) => TRANSLATOR.redirect_target_placeholder(),
            UndoSubject::CustomGameName(_) => TRANSLATOR.custom_game_name_placeholder(),
            UndoSubject::CustomGameFile(_, _) => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::CustomGameRegistry(_, _) => "".to_string(),
            UndoSubject::BackupFilterIgnoredPath(_) => "".to_string(),
            UndoSubject::BackupFilterIgnoredRegistry(_) => "".to_string(),
        };

        Undoable::new(
            TextInput::new(&placeholder, &current, event)
                .style(style::TextInput)
                .width(Length::Fill)
                .padding(5),
            move |action| Message::UndoRedo(action, subject),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_history() {
        let mut ht = TextHistory::new("initial", 3);

        assert_eq!(ht.current(), "initial");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.redo(), "initial");

        ht.push("a");
        assert_eq!(ht.current(), "a");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.redo(), "a");
        assert_eq!(ht.redo(), "a");

        // Duplicates are ignored:
        ht.push("a");
        ht.push("a");
        ht.push("a");
        assert_eq!(ht.undo(), "initial");

        // History is clipped at the limit:
        ht.push("b");
        ht.push("c");
        ht.push("d");
        assert_eq!(ht.undo(), "c");
        assert_eq!(ht.undo(), "b");
        assert_eq!(ht.undo(), "b");

        // Redos are lost on push:
        ht.push("e");
        assert_eq!(ht.current(), "e");
        assert_eq!(ht.redo(), "e");
        assert_eq!(ht.undo(), "b");
        assert_eq!(ht.undo(), "b");
    }
}
