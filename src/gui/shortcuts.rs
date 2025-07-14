// Iced has built-in support for some keyboard shortcuts. This module provides
// support for implementing other shortcuts until Iced provides its own support.

use std::collections::{HashMap, VecDeque};

use iced::Length;

use crate::{
    cloud::Remote,
    gui::{
        common::{Message, UndoSubject, ERROR_ICON},
        modal::{ModalField, ModalInputKind},
        style,
        widget::{id, Element, TextInput, Undoable},
    },
    lang::TRANSLATOR,
    prelude::{EditAction, RedirectEditActionField, StrictPath},
    resource::config::{self, Config, CustomGame},
    scan::{game_filter, registry::RegistryItem},
};

fn path_appears_valid(path: &str) -> bool {
    !path.contains("://")
}

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

    pub fn apply_to_option_strict_path_field(&self, config: &mut Option<StrictPath>, history: &mut TextHistory) {
        let value = match self {
            Shortcut::Undo => history.undo(),
            Shortcut::Redo => history.redo(),
        };

        if value.is_empty() {
            *config = None;
        } else {
            match config {
                Some(config) => config.reset(value),
                None => *config = Some(value.into()),
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
        history.push_back(initial.raw().into());
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

    pub fn clear(&mut self) {
        self.initialize("".to_string());
    }

    pub fn initialize(&mut self, value: String) {
        self.history.clear();
        self.history.push_back(value);
        self.position = 0;
    }

    pub fn undo(&mut self) -> String {
        self.position = if self.position == 0 { 0 } else { self.position - 1 };
        self.current()
    }

    pub fn redo(&mut self) -> String {
        self.position = std::cmp::min(self.position + 1, self.history.len() - 1);
        self.current()
    }

    pub fn apply(&mut self, shortcut: Shortcut) {
        match shortcut {
            Shortcut::Undo => {
                self.undo();
            }
            Shortcut::Redo => {
                self.redo();
            }
        }
    }
}

#[derive(Default)]
pub struct RootHistory {
    pub path: TextHistory,
    pub lutris_database: TextHistory,
}

impl RootHistory {
    pub fn clear_secondary(&mut self) {
        self.lutris_database.clear();
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
    pub alias: TextHistory,
    pub files: Vec<TextHistory>,
    pub registry: Vec<TextHistory>,
    pub install_dir: Vec<TextHistory>,
    pub wine_prefix: Vec<TextHistory>,
}

#[derive(Default)]
pub struct ModalHistory {
    pub url: TextHistory,
    pub host: TextHistory,
    pub port: TextHistory,
    pub username: TextHistory,
    pub password: TextHistory,
}

#[derive(Default)]
pub struct TextHistories {
    pub backup_target: TextHistory,
    pub restore_source: TextHistory,
    pub backup_search_game_name: TextHistory,
    pub restore_search_game_name: TextHistory,
    pub custom_games_search_game_name: TextHistory,
    pub roots: Vec<RootHistory>,
    pub secondary_manifests: Vec<TextHistory>,
    pub redirects: Vec<RedirectHistory>,
    pub custom_games: Vec<CustomGameHistory>,
    pub backup_filter_ignored_paths: Vec<TextHistory>,
    pub backup_filter_ignored_registry: Vec<TextHistory>,
    pub rclone_executable: TextHistory,
    pub rclone_arguments: TextHistory,
    pub cloud_remote_id: TextHistory,
    pub cloud_path: TextHistory,
    pub modal: ModalHistory,
    pub backup_comments: HashMap<String, TextHistory>,
}

impl TextHistories {
    pub fn new(config: &Config) -> Self {
        let mut histories = Self {
            backup_target: TextHistory::path(&config.backup.path),
            restore_source: TextHistory::path(&config.restore.path),
            backup_search_game_name: TextHistory::raw(""),
            restore_search_game_name: TextHistory::raw(""),
            rclone_executable: TextHistory::path(&config.apps.rclone.path),
            rclone_arguments: TextHistory::raw(&config.apps.rclone.arguments),
            cloud_path: TextHistory::raw(&config.cloud.path),
            ..Default::default()
        };

        for x in &config.roots {
            histories.roots.push(RootHistory {
                path: TextHistory::path(x.path()),
                lutris_database: x.lutris_database().map(TextHistory::path).unwrap_or_default(),
            });
        }

        for x in &config.manifest.secondary {
            histories.secondary_manifests.push(TextHistory::raw(&x.value()));
        }

        for x in &config.redirects {
            histories.redirects.push(RedirectHistory {
                source: TextHistory::path(&x.source),
                target: TextHistory::path(&x.target),
            });
        }

        for x in &config.custom_games {
            histories.add_custom_game(x);
        }

        for x in &config.backup.filter.ignored_paths {
            histories.backup_filter_ignored_paths.push(TextHistory::path(x));
        }
        for x in &config.backup.filter.ignored_registry {
            histories
                .backup_filter_ignored_registry
                .push(TextHistory::raw(&x.raw()));
        }

        if let Some(Remote::Custom { id }) = &config.cloud.remote {
            histories.cloud_remote_id.push(id);
        }

        histories
    }

    pub fn add_custom_game(&mut self, game: &CustomGame) {
        let history = CustomGameHistory {
            name: TextHistory::raw(&game.name),
            alias: TextHistory::raw(&game.alias.clone().unwrap_or_default()),
            files: game.files.iter().map(|x| TextHistory::raw(x)).collect(),
            registry: game.registry.iter().map(|x| TextHistory::raw(x)).collect(),
            install_dir: game.install_dir.iter().map(|x| TextHistory::raw(x)).collect(),
            wine_prefix: game.wine_prefix.iter().map(|x| TextHistory::raw(x)).collect(),
        };
        self.custom_games.push(history);
    }

    pub fn clear_modal_fields(&mut self) {
        self.modal.url.clear();
        self.modal.host.clear();
        self.modal.port.clear();
        self.modal.username.clear();
        self.modal.password.clear();
    }

    pub fn input<'a>(&self, subject: UndoSubject) -> Element<'a> {
        let current = match &subject {
            UndoSubject::BackupTarget => self.backup_target.current(),
            UndoSubject::RestoreSource => self.restore_source.current(),
            UndoSubject::BackupSearchGameName => self.backup_search_game_name.current(),
            UndoSubject::RestoreSearchGameName => self.restore_search_game_name.current(),
            UndoSubject::CustomGamesSearchGameName => self.custom_games_search_game_name.current(),
            UndoSubject::RootPath(i) => self.roots.get(*i).map(|x| x.path.current()).unwrap_or_default(),
            UndoSubject::RootLutrisDatabase(i) => self
                .roots
                .get(*i)
                .map(|x| x.lutris_database.current())
                .unwrap_or_default(),
            UndoSubject::SecondaryManifest(i) => self
                .secondary_manifests
                .get(*i)
                .map(|x| x.current())
                .unwrap_or_default(),
            UndoSubject::RedirectSource(i) => self.redirects.get(*i).map(|x| x.source.current()).unwrap_or_default(),
            UndoSubject::RedirectTarget(i) => self.redirects.get(*i).map(|x| x.target.current()).unwrap_or_default(),
            UndoSubject::CustomGameName(i) => self.custom_games.get(*i).map(|x| x.name.current()).unwrap_or_default(),
            UndoSubject::CustomGameAlias(i) => self.custom_games.get(*i).map(|x| x.alias.current()).unwrap_or_default(),
            UndoSubject::CustomGameFile(i, j) => self
                .custom_games
                .get(*i)
                .and_then(|x| x.files.get(*j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::CustomGameRegistry(i, j) => self
                .custom_games
                .get(*i)
                .and_then(|x| x.registry.get(*j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::CustomGameInstallDir(i, j) => self
                .custom_games
                .get(*i)
                .and_then(|x| x.install_dir.get(*j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::CustomGameWinePrefix(i, j) => self
                .custom_games
                .get(*i)
                .and_then(|x| x.wine_prefix.get(*j).map(|y| y.current()))
                .unwrap_or_default(),
            UndoSubject::BackupFilterIgnoredPath(i) => self
                .backup_filter_ignored_paths
                .get(*i)
                .map(|x| x.current())
                .unwrap_or_default(),
            UndoSubject::BackupFilterIgnoredRegistry(i) => self
                .backup_filter_ignored_registry
                .get(*i)
                .map(|x| x.current())
                .unwrap_or_default(),
            UndoSubject::RcloneExecutable => self.rclone_executable.current(),
            UndoSubject::RcloneArguments => self.rclone_arguments.current(),
            UndoSubject::CloudRemoteId => self.cloud_remote_id.current(),
            UndoSubject::CloudPath => self.cloud_path.current(),
            UndoSubject::ModalField(field) => match field {
                ModalInputKind::Url => self.modal.url.current(),
                ModalInputKind::Host => self.modal.host.current(),
                ModalInputKind::Port => self.modal.port.current(),
                ModalInputKind::Username => self.modal.username.current(),
                ModalInputKind::Password => self.modal.password.current(),
            },
            UndoSubject::BackupComment(game) => self.backup_comments.get(game).map(|x| x.current()).unwrap_or_default(),
        };

        let event: Box<dyn Fn(String) -> Message> = match subject.clone() {
            UndoSubject::BackupTarget => Box::new(Message::config(config::Event::BackupTarget)),
            UndoSubject::RestoreSource => Box::new(Message::config(config::Event::RestoreSource)),
            UndoSubject::BackupSearchGameName => Box::new(|value| Message::Filter {
                event: game_filter::Event::EditedGameName(value),
            }),
            UndoSubject::RestoreSearchGameName => Box::new(|value| Message::Filter {
                event: game_filter::Event::EditedGameName(value),
            }),
            UndoSubject::CustomGamesSearchGameName => Box::new(|value| Message::Filter {
                event: game_filter::Event::EditedGameName(value),
            }),
            UndoSubject::RootPath(i) => Box::new(Message::config(move |value| {
                config::Event::Root(EditAction::Change(i, value))
            })),
            UndoSubject::RootLutrisDatabase(i) => Box::new(Message::config(move |value| {
                config::Event::RootLutrisDatabase(i, value)
            })),
            UndoSubject::SecondaryManifest(i) => Box::new(Message::config(move |value| {
                config::Event::SecondaryManifest(EditAction::Change(i, value))
            })),
            UndoSubject::RedirectSource(i) => Box::new(Message::config(move |value| {
                config::Event::Redirect(EditAction::Change(i, value), Some(RedirectEditActionField::Source))
            })),
            UndoSubject::RedirectTarget(i) => Box::new(Message::config(move |value| {
                config::Event::Redirect(EditAction::Change(i, value), Some(RedirectEditActionField::Target))
            })),
            UndoSubject::CustomGameName(i) => Box::new(Message::config(move |value| {
                config::Event::CustomGame(EditAction::Change(i, value))
            })),
            UndoSubject::CustomGameAlias(i) => {
                Box::new(Message::config(move |value| config::Event::CustomGameAlias(i, value)))
            }
            UndoSubject::CustomGameFile(i, j) => Box::new(Message::config(move |value| {
                config::Event::CustomGameFile(i, EditAction::Change(j, value))
            })),
            UndoSubject::CustomGameRegistry(i, j) => Box::new(Message::config(move |value| {
                config::Event::CustomGameRegistry(i, EditAction::Change(j, value))
            })),
            UndoSubject::CustomGameInstallDir(i, j) => Box::new(Message::config(move |value| {
                config::Event::CustomGameInstallDir(i, EditAction::Change(j, value))
            })),
            UndoSubject::CustomGameWinePrefix(i, j) => Box::new(Message::config(move |value| {
                config::Event::CustomGameWinePrefix(i, EditAction::Change(j, value))
            })),
            UndoSubject::BackupFilterIgnoredPath(i) => Box::new(Message::config(move |value| {
                config::Event::BackupFilterIgnoredPath(EditAction::Change(i, value))
            })),
            UndoSubject::BackupFilterIgnoredRegistry(i) => Box::new(Message::config(move |value| {
                config::Event::BackupFilterIgnoredRegistry(EditAction::Change(i, value))
            })),
            UndoSubject::RcloneExecutable => Box::new(Message::config(config::Event::RcloneExecutable)),
            UndoSubject::RcloneArguments => Box::new(Message::config(config::Event::RcloneArguments)),
            UndoSubject::CloudRemoteId => Box::new(Message::config(config::Event::CloudRemoteId)),
            UndoSubject::CloudPath => Box::new(Message::config(config::Event::CloudPath)),
            UndoSubject::ModalField(field) => Box::new(move |value| {
                Message::EditedModalField(match field {
                    ModalInputKind::Url => ModalField::Url(value),
                    ModalInputKind::Host => ModalField::Host(value),
                    ModalInputKind::Port => ModalField::Port(value),
                    ModalInputKind::Username => ModalField::Username(value),
                    ModalInputKind::Password => ModalField::Password(value),
                })
            }),
            // TODO: This is now handled separately with a `TextEditor`.
            UndoSubject::BackupComment(_) => Box::new(|_| Message::Ignore),
        };

        let placeholder = match &subject {
            UndoSubject::BackupTarget => "".to_string(),
            UndoSubject::RestoreSource => "".to_string(),
            UndoSubject::BackupSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::RestoreSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::CustomGamesSearchGameName => TRANSLATOR.search_game_name_placeholder(),
            UndoSubject::RootPath(_) => "".to_string(),
            UndoSubject::RootLutrisDatabase(_) => "".to_string(),
            UndoSubject::SecondaryManifest(_) => "".to_string(),
            UndoSubject::RedirectSource(_) => TRANSLATOR.redirect_source_placeholder(),
            UndoSubject::RedirectTarget(_) => TRANSLATOR.redirect_target_placeholder(),
            UndoSubject::CustomGameName(_) => TRANSLATOR.custom_game_name_placeholder(),
            UndoSubject::CustomGameAlias(_) => TRANSLATOR.custom_game_name_placeholder(),
            UndoSubject::CustomGameFile(_, _) => "".to_string(),
            UndoSubject::CustomGameRegistry(_, _) => "".to_string(),
            UndoSubject::CustomGameInstallDir(_, _) => "".to_string(),
            UndoSubject::CustomGameWinePrefix(_, _) => "".to_string(),
            UndoSubject::BackupFilterIgnoredPath(_) => "".to_string(),
            UndoSubject::BackupFilterIgnoredRegistry(_) => "".to_string(),
            UndoSubject::RcloneExecutable => TRANSLATOR.executable_label(),
            UndoSubject::RcloneArguments => TRANSLATOR.arguments_label(),
            UndoSubject::CloudRemoteId => "".to_string(),
            UndoSubject::CloudPath => "".to_string(),
            UndoSubject::ModalField(_) => "".to_string(),
            UndoSubject::BackupComment(_) => TRANSLATOR.comment_label(),
        };

        let icon = match &subject {
            UndoSubject::BackupTarget
            | UndoSubject::RestoreSource
            | UndoSubject::RootPath(_)
            | UndoSubject::RootLutrisDatabase(_)
            | UndoSubject::RedirectSource(_)
            | UndoSubject::RedirectTarget(_)
            | UndoSubject::CustomGameFile(_, _)
            | UndoSubject::CustomGameInstallDir(_, _)
            | UndoSubject::CustomGameWinePrefix(_, _)
            | UndoSubject::BackupFilterIgnoredPath(_)
            | UndoSubject::RcloneExecutable => (!path_appears_valid(&current)).then_some(ERROR_ICON),
            UndoSubject::CustomGameName(_) | UndoSubject::CustomGameAlias(_) => {
                (current.trim() != current).then_some(ERROR_ICON)
            }
            UndoSubject::SecondaryManifest(_)
            | UndoSubject::BackupSearchGameName
            | UndoSubject::RestoreSearchGameName
            | UndoSubject::CustomGamesSearchGameName
            | UndoSubject::CustomGameRegistry(_, _)
            | UndoSubject::BackupFilterIgnoredRegistry(_)
            | UndoSubject::RcloneArguments
            | UndoSubject::CloudRemoteId
            | UndoSubject::CloudPath
            | UndoSubject::ModalField(_)
            | UndoSubject::BackupComment(_) => None,
        };

        let id = match &subject {
            UndoSubject::BackupSearchGameName => Some(id::backup_search()),
            UndoSubject::RestoreSearchGameName => Some(id::restore_search()),
            UndoSubject::CustomGamesSearchGameName => Some(id::custom_games_search()),
            _ => None,
        };

        Undoable::new(
            {
                let mut input = TextInput::new(&placeholder, &current)
                    .on_input(event)
                    .class(style::TextInput)
                    .width(Length::Fill)
                    .padding(5);

                if let Some(icon) = icon {
                    input = input.icon(icon);
                }

                if subject.privacy().sensitive() {
                    input = input.secure(true);
                }

                if let Some(id) = id {
                    input = input.id(id);
                }

                input
            },
            move |action| Message::UndoRedo(action, subject.clone()),
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
