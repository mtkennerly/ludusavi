use crate::{
    config::{Config, RootsConfig},
    lang::Translator,
    manifest::{Game, Manifest, SteamMetadata, Store},
    prelude::{
        app_dir, back_up_game, game_file_restoration_target, prepare_backup_target, restore_game,
        scan_dir_for_restorable_games, scan_dir_for_restoration, scan_game_for_backup, BackupInfo, Error,
        OperationStatus, OperationStepDecision, ScanInfo, StrictPath,
    },
    shortcuts::{Shortcut, TextHistory},
};

use iced::{
    button, executor,
    keyboard::{KeyCode, ModifiersState},
    scrollable, text_input, Align, Application, Button, Checkbox, Column, Command, Container, Element, Font,
    HorizontalAlignment, Length, ProgressBar, Radio, Row, Scrollable, Space, Subscription, Text, TextInput,
};
use native_dialog::Dialog;

const ICONS: Font = Font::External {
    name: "Material Icons",
    bytes: include_bytes!("../assets/MaterialIcons-Regular.ttf"),
};

enum Icon {
    AddCircle,
    RemoveCircle,
    FolderOpen,
}

impl Icon {
    fn as_text(&self) -> Text {
        let character = match self {
            Self::AddCircle => '\u{E147}',
            Self::RemoveCircle => '\u{E15C}',
            Self::FolderOpen => '\u{E2C8}',
        };
        Text::new(&character.to_string())
            .font(ICONS)
            .width(Length::Units(60))
            .horizontal_alignment(HorizontalAlignment::Center)
    }
}

#[realia::dep_from_registry("ludusavi", "iced_native")]
fn get_key_pressed(event: iced_native::input::keyboard::Event) -> Option<(KeyCode, ModifiersState)> {
    match event {
        iced_native::input::keyboard::Event::Input {
            state,
            key_code,
            modifiers,
        } if state == iced_native::input::ButtonState::Pressed => Some((key_code, modifiers)),
        _ => None,
    }
}

#[realia::not(dep_from_registry("ludusavi", "iced_native"))]
fn get_key_pressed(event: iced_native::keyboard::Event) -> Option<(KeyCode, ModifiersState)> {
    match event {
        iced_native::keyboard::Event::KeyPressed { key_code, modifiers } => Some((key_code, modifiers)),
        _ => None,
    }
}

#[realia::dep_from_registry("ludusavi", "iced")]
fn set_app_icon<T>(_settings: &mut iced::Settings<T>) {}

#[realia::not(dep_from_registry("ludusavi", "iced"))]
fn set_app_icon<T>(settings: &mut iced::Settings<T>) {
    settings.window.icon = match image::load_from_memory(include_bytes!("../assets/icon.png")) {
        Ok(buffer) => {
            let buffer = buffer.to_rgba();
            let width = buffer.width();
            let height = buffer.height();
            let dynamic_image = image::DynamicImage::ImageRgba8(buffer);
            match iced::window::icon::Icon::from_rgba(dynamic_image.to_bytes(), width, height) {
                Ok(icon) => Some(icon),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

#[derive(Default)]
struct App {
    config: Config,
    manifest: Manifest,
    translator: Translator,
    operation: Option<OngoingOperation>,
    screen: Screen,
    modal_theme: Option<ModalTheme>,
    modal: ModalComponent,
    nav_to_backup_button: button::State,
    nav_to_restore_button: button::State,
    nav_to_custom_games_button: button::State,
    backup_screen: BackupScreenComponent,
    restore_screen: RestoreScreenComponent,
    custom_games_screen: CustomGamesScreenComponent,
    operation_should_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    progress: DisappearingProgress,
}

#[derive(Debug, Clone)]
enum Message {
    Idle,
    Ignore,
    ConfirmBackupStart,
    BackupStart {
        preview: bool,
    },
    ConfirmRestoreStart,
    RestoreStart {
        preview: bool,
    },
    BackupStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
    },
    RestoreStep {
        scan_info: Option<ScanInfo>,
        backup_info: Option<BackupInfo>,
        decision: OperationStepDecision,
    },
    CancelOperation,
    BackupComplete,
    RestoreComplete,
    EditedBackupTarget(String),
    EditedRestoreSource(String),
    EditedRoot(EditAction),
    SelectedRootStore(usize, Store),
    EditedRedirect(EditAction, Option<RedirectEditActionField>),
    EditedCustomGame(EditAction),
    EditedCustomGameFile(usize, EditAction),
    EditedCustomGameRegistry(usize, EditAction),
    SwitchScreen(Screen),
    ToggleGameListEntryExpanded {
        name: String,
    },
    ToggleGameListEntryEnabled {
        name: String,
        enabled: bool,
        restoring: bool,
    },
    BrowseDir(BrowseSubject),
    BrowseDirFailure,
    SelectAllGames,
    DeselectAllGames,
    SubscribedEvent(iced_native::Event),
}

#[derive(Debug, Clone, PartialEq)]
enum OngoingOperation {
    Backup,
    CancelBackup,
    PreviewBackup,
    CancelPreviewBackup,
    Restore,
    CancelRestore,
    PreviewRestore,
    CancelPreviewRestore,
}

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Backup,
    Restore,
    CustomGames,
}

#[derive(Debug, Clone, PartialEq)]
enum ModalTheme {
    Error { variant: Error },
    ConfirmBackup,
    ConfirmRestore,
}

#[derive(Debug, Clone, PartialEq)]
enum EditAction {
    Add,
    Change(usize, String),
    Remove(usize),
}

#[derive(Debug, Clone, PartialEq)]
enum RedirectEditActionField {
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq)]
enum BrowseSubject {
    BackupTarget,
    RestoreSource,
    Root(usize),
    RedirectSource(usize),
    RedirectTarget(usize),
    CustomGameFile(usize, usize),
}

impl Default for Screen {
    fn default() -> Self {
        Self::Backup
    }
}

fn apply_shortcut_to_strict_path_field(
    shortcut: &Shortcut,
    config: &mut StrictPath,
    state: &text_input::State,
    history: &mut TextHistory,
) {
    match shortcut {
        Shortcut::Undo => {
            config.reset(history.undo());
        }
        Shortcut::Redo => {
            config.reset(history.redo());
        }
        Shortcut::ClipboardCopy => {
            crate::shortcuts::copy_to_clipboard_from_iced(&config.raw(), &state.cursor());
        }
        Shortcut::ClipboardCut => {
            let modified = crate::shortcuts::cut_to_clipboard_from_iced(&config.raw(), &state.cursor());
            config.reset(modified);
            history.push(&config.raw());
        }
    }
}

fn apply_shortcut_to_string_field(
    shortcut: &Shortcut,
    config: &mut String,
    state: &text_input::State,
    history: &mut TextHistory,
) {
    match shortcut {
        Shortcut::Undo => {
            *config = history.undo();
        }
        Shortcut::Redo => {
            *config = history.redo();
        }
        Shortcut::ClipboardCopy => {
            crate::shortcuts::copy_to_clipboard_from_iced(&config, &state.cursor());
        }
        Shortcut::ClipboardCut => {
            let modified = crate::shortcuts::cut_to_clipboard_from_iced(&config, &state.cursor());
            *config = modified;
            history.push(&config);
        }
    }
}

#[derive(Default)]
struct ModalComponent {
    positive_button: button::State,
    negative_button: button::State,
}

impl ModalComponent {
    fn view(&mut self, theme: &ModalTheme, config: &Config, translator: &Translator) -> Container<Message> {
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
                                    ModalTheme::ConfirmBackup => translator
                                        .modal_confirm_backup(&config.backup.path, config.backup.path.exists()),
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

#[derive(Default)]
struct GameListEntry {
    scan_info: ScanInfo,
    backup_info: Option<BackupInfo>,
    button: button::State,
    expanded: bool,
}

impl GameListEntry {
    fn view(&mut self, restoring: bool, translator: &Translator, config: &Config) -> Container<Message> {
        let mut lines = Vec::<String>::new();
        let successful = match &self.backup_info {
            Some(x) => x.successful(),
            _ => true,
        };

        if self.expanded {
            for item in itertools::sorted(&self.scan_info.found_files) {
                let mut redirected_from = None;
                let mut line = item.path.render();
                if restoring {
                    if let Ok((original_target, redirected_target)) =
                        game_file_restoration_target(&item.path, &config.get_redirects())
                    {
                        if original_target != redirected_target {
                            redirected_from = Some(original_target);
                        }
                        line = redirected_target.render();
                    }
                }
                if let Some(backup_info) = &self.backup_info {
                    if backup_info.failed_files.contains(&item) {
                        line = translator.failed_file_entry_line(&line);
                    }
                }
                lines.push(line);
                if let Some(redirected_from) = redirected_from {
                    lines.push(translator.redirected_file_entry_line(&redirected_from));
                }
            }
            for item in itertools::sorted(&self.scan_info.found_registry_keys) {
                lines.push(item.clone());
            }
        }

        let enabled = if restoring {
            config.is_game_enabled_for_restore(&self.scan_info.game_name)
        } else {
            config.is_game_enabled_for_backup(&self.scan_info.game_name)
        };
        let name_for_checkbox = self.scan_info.game_name.clone();

        Container::new(
            Column::new()
                .padding(5)
                .spacing(5)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .push(Checkbox::new(enabled, "", move |enabled| {
                            Message::ToggleGameListEntryEnabled {
                                name: name_for_checkbox.clone(),
                                enabled,
                                restoring,
                            }
                        }))
                        .push(
                            Button::new(
                                &mut self.button,
                                Text::new(if successful {
                                    self.scan_info.game_name.clone()
                                } else {
                                    translator.game_list_entry_title_failed(&self.scan_info.game_name)
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::ToggleGameListEntryExpanded {
                                name: self.scan_info.game_name.clone(),
                            })
                            .style(if !enabled {
                                style::Button::GameListEntryTitleDisabled
                            } else if successful {
                                style::Button::GameListEntryTitle
                            } else {
                                style::Button::GameListEntryTitleFailed
                            })
                            .width(Length::Fill)
                            .padding(2),
                        )
                        .push(
                            Container::new(Text::new(
                                translator.mib(self.scan_info.sum_bytes(&self.backup_info), false),
                            ))
                            .width(Length::Units(115))
                            .center_x(),
                        ),
                )
                .push(
                    Row::new().push(
                        Container::new(Text::new(lines.join("\n")))
                            .width(Length::Fill)
                            .style(style::Container::GameListEntryBody),
                    ),
                ),
        )
        .style(style::Container::GameListEntry)
    }
}

#[derive(Default)]
struct GameList {
    entries: Vec<GameListEntry>,
    scroll: scrollable::State,
}

impl GameList {
    fn view(&mut self, restoring: bool, translator: &Translator, config: &Config) -> Container<Message> {
        self.entries.sort_by_key(|x| x.scan_info.game_name.clone());
        Container::new({
            self.entries.iter_mut().enumerate().fold(
                Scrollable::new(&mut self.scroll)
                    .width(Length::Fill)
                    .padding(10)
                    .style(style::Scrollable),
                |parent: Scrollable<'_, Message>, (_i, x)| {
                    parent
                        .push(x.view(restoring, translator, &config))
                        .push(Space::new(Length::Units(0), Length::Units(10)))
                },
            )
        })
    }

    fn all_entries_selected(&self, config: &Config, restoring: bool) -> bool {
        self.entries.iter().all(|x| {
            if restoring {
                config.is_game_enabled_for_restore(&x.scan_info.game_name)
            } else {
                config.is_game_enabled_for_backup(&x.scan_info.game_name)
            }
        })
    }
}

#[derive(Default)]
struct RootEditorRow {
    button_state: button::State,
    browse_button_state: button::State,
    text_state: text_input::State,
    text_history: TextHistory,
}

impl RootEditorRow {
    fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct RootEditor {
    scroll: scrollable::State,
    rows: Vec<RootEditorRow>,
}

impl RootEditor {
    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        let roots = config.roots.clone();
        if roots.is_empty() {
            Container::new(Text::new(translator.no_roots_are_configured()))
        } else {
            Container::new({
                self.rows.iter_mut().enumerate().fold(
                    Scrollable::new(&mut self.scroll)
                        .width(Length::Fill)
                        .max_height(100)
                        .style(style::Scrollable),
                    |parent: Scrollable<'_, Message>, (i, x)| {
                        parent
                            .push(
                                Row::new()
                                    .spacing(20)
                                    .push(Space::new(Length::Units(0), Length::Units(0)))
                                    .push(
                                        Button::new(&mut x.button_state, Icon::RemoveCircle.as_text())
                                            .on_press(Message::EditedRoot(EditAction::Remove(i)))
                                            .style(style::Button::Negative),
                                    )
                                    .push(
                                        TextInput::new(&mut x.text_state, "", &roots[i].path.raw(), move |v| {
                                            Message::EditedRoot(EditAction::Change(i, v))
                                        })
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push({
                                        Radio::new(
                                            Store::Steam,
                                            translator.store(&Store::Steam),
                                            Some(roots[i].store),
                                            move |v| Message::SelectedRootStore(i, v),
                                        )
                                    })
                                    .push({
                                        Radio::new(
                                            Store::Other,
                                            translator.store(&Store::Other),
                                            Some(roots[i].store),
                                            move |v| Message::SelectedRootStore(i, v),
                                        )
                                    })
                                    .push(
                                        Button::new(&mut x.browse_button_state, Icon::FolderOpen.as_text())
                                            .on_press(match operation {
                                                None => Message::BrowseDir(BrowseSubject::Root(i)),
                                                Some(_) => Message::Ignore,
                                            })
                                            .style(match operation {
                                                None => style::Button::Primary,
                                                Some(_) => style::Button::Disabled,
                                            }),
                                    )
                                    .push(Space::new(Length::Units(0), Length::Units(0))),
                            )
                            .push(Row::new().push(Space::new(Length::Units(0), Length::Units(5))))
                    },
                )
            })
        }
    }
}

#[derive(Default)]
struct RedirectEditorRow {
    button_state: button::State,
    source_text_state: text_input::State,
    source_text_history: TextHistory,
    source_browse_button_state: button::State,
    target_text_state: text_input::State,
    target_text_history: TextHistory,
    target_browse_button_state: button::State,
}

impl RedirectEditorRow {
    fn new(initial_source: &str, initial_target: &str) -> Self {
        Self {
            source_text_history: TextHistory::new(initial_source, 100),
            target_text_history: TextHistory::new(initial_target, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct RedirectEditor {
    scroll: scrollable::State,
    rows: Vec<RedirectEditorRow>,
}

impl RedirectEditor {
    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        let redirects = config.get_redirects();
        if redirects.is_empty() {
            Container::new(Space::new(Length::Units(0), Length::Units(0)))
        } else {
            Container::new({
                self.rows.iter_mut().enumerate().fold(
                    Scrollable::new(&mut self.scroll)
                        .width(Length::Fill)
                        .max_height(100)
                        .style(style::Scrollable),
                    |parent: Scrollable<'_, Message>, (i, x)| {
                        parent
                            .push(
                                Row::new()
                                    .spacing(20)
                                    .push(Space::new(Length::Units(0), Length::Units(0)))
                                    .push(
                                        Button::new(&mut x.button_state, Icon::RemoveCircle.as_text())
                                            .on_press(Message::EditedRedirect(EditAction::Remove(i), None))
                                            .style(style::Button::Negative),
                                    )
                                    .push(
                                        TextInput::new(
                                            &mut x.source_text_state,
                                            &translator.redirect_source_placeholder(),
                                            &redirects[i].source.raw(),
                                            move |v| {
                                                Message::EditedRedirect(
                                                    EditAction::Change(i, v),
                                                    Some(RedirectEditActionField::Source),
                                                )
                                            },
                                        )
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(
                                        Button::new(&mut x.source_browse_button_state, Icon::FolderOpen.as_text())
                                            .on_press(match operation {
                                                None => Message::BrowseDir(BrowseSubject::RedirectSource(i)),
                                                Some(_) => Message::Ignore,
                                            })
                                            .style(match operation {
                                                None => style::Button::Primary,
                                                Some(_) => style::Button::Disabled,
                                            }),
                                    )
                                    .push(
                                        TextInput::new(
                                            &mut x.target_text_state,
                                            &translator.redirect_target_placeholder(),
                                            &redirects[i].target.raw(),
                                            move |v| {
                                                Message::EditedRedirect(
                                                    EditAction::Change(i, v),
                                                    Some(RedirectEditActionField::Target),
                                                )
                                            },
                                        )
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(
                                        Button::new(&mut x.target_browse_button_state, Icon::FolderOpen.as_text())
                                            .on_press(match operation {
                                                None => Message::BrowseDir(BrowseSubject::RedirectTarget(i)),
                                                Some(_) => Message::Ignore,
                                            })
                                            .style(match operation {
                                                None => style::Button::Primary,
                                                Some(_) => style::Button::Disabled,
                                            }),
                                    )
                                    .push(Space::new(Length::Units(0), Length::Units(0))),
                            )
                            .push(Row::new().push(Space::new(Length::Units(0), Length::Units(5))))
                    },
                )
            })
        }
    }
}

#[derive(Default)]
struct CustomGamesEditorEntryRow {
    button_state: button::State,
    text_state: text_input::State,
    text_history: TextHistory,
    browse_button_state: button::State,
}

impl CustomGamesEditorEntryRow {
    fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct CustomGamesEditorEntry {
    remove_button_state: button::State,
    add_file_button_state: button::State,
    add_registry_button_state: button::State,
    text_state: text_input::State,
    text_history: TextHistory,
    files: Vec<CustomGamesEditorEntryRow>,
    registry: Vec<CustomGamesEditorEntryRow>,
}

impl CustomGamesEditorEntry {
    fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
struct CustomGamesEditor {
    scroll: scrollable::State,
    entries: Vec<CustomGamesEditorEntry>,
}

impl CustomGamesEditor {
    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        if config.custom_games.is_empty() {
            Container::new(Space::new(Length::Units(0), Length::Units(0)))
        } else {
            Container::new({
                self.entries.iter_mut().enumerate().fold(
                    Scrollable::new(&mut self.scroll)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .spacing(4)
                        .style(style::Scrollable),
                    |parent: Scrollable<'_, Message>, (i, x)| {
                        parent
                            .push(
                                Row::new()
                                    .push(Space::new(Length::Units(20), Length::Units(0)))
                                    .push(
                                        Column::new().width(Length::Units(100)).push(
                                            Button::new(&mut x.remove_button_state, Icon::RemoveCircle.as_text())
                                                .on_press(Message::EditedCustomGame(EditAction::Remove(i)))
                                                .style(style::Button::Negative),
                                        ),
                                    )
                                    .push(
                                        TextInput::new(
                                            &mut x.text_state,
                                            &translator.custom_game_name_placeholder(),
                                            &config.custom_games[i].name,
                                            move |v| Message::EditedCustomGame(EditAction::Change(i, v)),
                                        )
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(Space::new(Length::Units(20), Length::Units(0))),
                            )
                            .push(
                                Row::new()
                                    .push(Space::new(Length::Units(20), Length::Units(0)))
                                    .push(
                                        Column::new()
                                            .width(Length::Units(100))
                                            .push(Text::new(translator.custom_files_label())),
                                    )
                                    .push(
                                        x.files
                                            .iter_mut()
                                            .enumerate()
                                            .fold(Column::new().spacing(4), |column, (ii, xx)| {
                                                column.push(
                                                    Row::new()
                                                        .spacing(20)
                                                        .push(
                                                            TextInput::new(
                                                                &mut xx.text_state,
                                                                "",
                                                                &config.custom_games[i].files[ii],
                                                                move |v| {
                                                                    Message::EditedCustomGameFile(
                                                                        i,
                                                                        EditAction::Change(ii, v),
                                                                    )
                                                                },
                                                            )
                                                            .padding(5),
                                                        )
                                                        .push(
                                                            Button::new(
                                                                &mut xx.browse_button_state,
                                                                Icon::FolderOpen.as_text(),
                                                            )
                                                            .on_press(match operation {
                                                                None => Message::BrowseDir(
                                                                    BrowseSubject::CustomGameFile(i, ii),
                                                                ),
                                                                Some(_) => Message::Ignore,
                                                            })
                                                            .style(match operation {
                                                                None => style::Button::Primary,
                                                                Some(_) => style::Button::Disabled,
                                                            }),
                                                        )
                                                        .push(
                                                            Button::new(
                                                                &mut xx.button_state,
                                                                Icon::RemoveCircle.as_text(),
                                                            )
                                                            .on_press(Message::EditedCustomGameFile(
                                                                i,
                                                                EditAction::Remove(ii),
                                                            ))
                                                            .style(style::Button::Negative),
                                                        )
                                                        .push(Space::new(Length::Units(0), Length::Units(0))),
                                                )
                                            })
                                            .push(
                                                Button::new(&mut x.add_file_button_state, Icon::AddCircle.as_text())
                                                    .on_press(Message::EditedCustomGameFile(i, EditAction::Add))
                                                    .style(style::Button::Primary),
                                            ),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .push(Space::new(Length::Units(20), Length::Units(0)))
                                    .push(
                                        Column::new()
                                            .width(Length::Units(100))
                                            .push(Text::new(translator.custom_registry_label())),
                                    )
                                    .push(
                                        x.registry
                                            .iter_mut()
                                            .enumerate()
                                            .fold(Column::new().spacing(4), |column, (ii, xx)| {
                                                column.push(
                                                    Row::new()
                                                        .spacing(20)
                                                        .push(
                                                            TextInput::new(
                                                                &mut xx.text_state,
                                                                "",
                                                                &config.custom_games[i].registry[ii],
                                                                move |v| {
                                                                    Message::EditedCustomGameRegistry(
                                                                        i,
                                                                        EditAction::Change(ii, v),
                                                                    )
                                                                },
                                                            )
                                                            .padding(5),
                                                        )
                                                        .push(
                                                            Button::new(
                                                                &mut xx.button_state,
                                                                Icon::RemoveCircle.as_text(),
                                                            )
                                                            .on_press(Message::EditedCustomGameRegistry(
                                                                i,
                                                                EditAction::Remove(ii),
                                                            ))
                                                            .style(style::Button::Negative),
                                                        )
                                                        .push(Space::new(Length::Units(0), Length::Units(0))),
                                                )
                                            })
                                            .push(
                                                Button::new(
                                                    &mut x.add_registry_button_state,
                                                    Icon::AddCircle.as_text(),
                                                )
                                                .on_press(Message::EditedCustomGameRegistry(i, EditAction::Add))
                                                .style(style::Button::Primary),
                                            ),
                                    ),
                            )
                            .push(Row::new().push(Space::new(Length::Units(0), Length::Units(25))))
                    },
                )
            })
        }
    }
}

#[derive(Default)]
struct DisappearingProgress {
    max: f32,
    current: f32,
}

impl DisappearingProgress {
    fn view(&mut self) -> ProgressBar {
        let visible = self.current > 0.0 && self.current < self.max;
        ProgressBar::new(0.0..=self.max, self.current).height(Length::FillPortion(if visible { 100 } else { 1 }))
    }

    fn complete(&self) -> bool {
        self.current >= self.max
    }
}

#[derive(Default)]
struct BackupScreenComponent {
    status: OperationStatus,
    log: GameList,
    start_button: button::State,
    preview_button: button::State,
    add_root_button: button::State,
    select_all_button: button::State,
    backup_target_input: text_input::State,
    backup_target_history: TextHistory,
    backup_target_browse_button: button::State,
    root_editor: RootEditor,
}

impl BackupScreenComponent {
    fn new(config: &Config) -> Self {
        let mut root_editor = RootEditor::default();
        for root in &config.roots {
            root_editor.rows.push(RootEditorRow::new(&root.path.raw()))
        }

        Self {
            root_editor,
            backup_target_history: TextHistory::new(&config.backup.path.raw(), 100),
            ..Default::default()
        }
    }

    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.preview_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewBackup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewBackup) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::BackupStart { preview: true },
                                Some(OngoingOperation::PreviewBackup) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::PreviewBackup) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.start_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::Backup) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelBackup) => translator.cancelling_button(),
                                    _ => translator.backup_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::ConfirmBackupStart,
                                Some(OngoingOperation::Backup) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::Backup) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.add_root_button,
                                Text::new(translator.add_root_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::EditedRoot(EditAction::Add))
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push({
                            let restoring = false;
                            Button::new(
                                &mut self.select_all_button,
                                Text::new(if self.log.all_entries_selected(&config, restoring) {
                                    translator.deselect_all_button()
                                } else {
                                    translator.select_all_button()
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(if self.log.all_entries_selected(&config, restoring) {
                                Message::DeselectAllGames
                            } else {
                                Message::SelectAllGames
                            })
                            .width(Length::Units(125))
                            .style(style::Button::Primary)
                        }),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.processed_games(&self.status)).size(40)),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.backup_target_label()))
                        .push(
                            TextInput::new(
                                &mut self.backup_target_input,
                                "",
                                &config.backup.path.raw(),
                                Message::EditedBackupTarget,
                            )
                            .padding(5),
                        )
                        .push(
                            Button::new(&mut self.backup_target_browse_button, Icon::FolderOpen.as_text())
                                .on_press(match operation {
                                    None => Message::BrowseDir(BrowseSubject::BackupTarget),
                                    Some(_) => Message::Ignore,
                                })
                                .style(match operation {
                                    None => style::Button::Primary,
                                    Some(_) => style::Button::Disabled,
                                }),
                        ),
                )
                .push(self.root_editor.view(&config, &translator, &operation))
                .push(Space::new(Length::Units(0), Length::Units(30)))
                .push(self.log.view(false, translator, &config)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}

#[derive(Default)]
struct RestoreScreenComponent {
    status: OperationStatus,
    log: GameList,
    start_button: button::State,
    preview_button: button::State,
    add_redirect_button: button::State,
    select_all_button: button::State,
    restore_source_input: text_input::State,
    restore_source_history: TextHistory,
    restore_source_browse_button: button::State,
    redirect_editor: RedirectEditor,
}

impl RestoreScreenComponent {
    fn new(config: &Config) -> Self {
        let mut redirect_editor = RedirectEditor::default();
        for redirect in &config.get_redirects() {
            redirect_editor
                .rows
                .push(RedirectEditorRow::new(&redirect.source.raw(), &redirect.target.raw()))
        }

        Self {
            restore_source_history: TextHistory::new(&config.backup.path.raw(), 100),
            redirect_editor,
            ..Default::default()
        }
    }

    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(
                            Button::new(
                                &mut self.preview_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewRestore) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewRestore) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::RestoreStart { preview: true },
                                Some(OngoingOperation::PreviewRestore) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::PreviewRestore) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.start_button,
                                Text::new(match operation {
                                    Some(OngoingOperation::Restore) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelRestore) => translator.cancelling_button(),
                                    _ => translator.restore_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(match operation {
                                None => Message::ConfirmRestoreStart,
                                Some(OngoingOperation::Restore) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary,
                                Some(OngoingOperation::Restore) => style::Button::Negative,
                                _ => style::Button::Disabled,
                            }),
                        )
                        .push(
                            Button::new(
                                &mut self.add_redirect_button,
                                Text::new(translator.add_redirect_button())
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::EditedRedirect(EditAction::Add, None))
                            .width(Length::Units(125))
                            .style(style::Button::Primary),
                        )
                        .push({
                            let restoring = true;
                            Button::new(
                                &mut self.select_all_button,
                                Text::new(if self.log.all_entries_selected(&config, restoring) {
                                    translator.deselect_all_button()
                                } else {
                                    translator.select_all_button()
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(if self.log.all_entries_selected(&config, restoring) {
                                Message::DeselectAllGames
                            } else {
                                Message::SelectAllGames
                            })
                            .width(Length::Units(125))
                            .style(style::Button::Primary)
                        }),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.processed_games(&self.status)).size(40)),
                )
                .push(
                    Row::new()
                        .padding(20)
                        .spacing(20)
                        .align_items(Align::Center)
                        .push(Text::new(translator.restore_source_label()))
                        .push(
                            TextInput::new(
                                &mut self.restore_source_input,
                                "",
                                &config.restore.path.raw(),
                                Message::EditedRestoreSource,
                            )
                            .padding(5),
                        )
                        .push(
                            Button::new(&mut self.restore_source_browse_button, Icon::FolderOpen.as_text())
                                .on_press(match operation {
                                    None => Message::BrowseDir(BrowseSubject::RestoreSource),
                                    Some(_) => Message::Ignore,
                                })
                                .style(match operation {
                                    None => style::Button::Primary,
                                    Some(_) => style::Button::Disabled,
                                }),
                        ),
                )
                .push(self.redirect_editor.view(&config, &translator, &operation))
                .push(Space::new(Length::Units(0), Length::Units(30)))
                .push(self.log.view(true, translator, &config)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}

#[derive(Default)]
struct CustomGamesScreenComponent {
    add_game_button: button::State,
    games_editor: CustomGamesEditor,
}

impl CustomGamesScreenComponent {
    fn new(config: &Config) -> Self {
        let mut games_editor = CustomGamesEditor::default();
        for custom_game in &config.custom_games {
            let mut row = CustomGamesEditorEntry::new(&custom_game.name.to_string());
            for file in &custom_game.files {
                row.files.push(CustomGamesEditorEntryRow::new(&file))
            }
            for key in &custom_game.registry {
                row.registry.push(CustomGamesEditorEntryRow::new(&key))
            }
            games_editor.entries.push(row);
        }

        Self {
            games_editor,
            ..Default::default()
        }
    }

    fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .padding(5)
                .align_items(Align::Center)
                .push(
                    Row::new().padding(20).spacing(20).align_items(Align::Center).push(
                        Button::new(
                            &mut self.add_game_button,
                            Text::new(translator.add_game_button()).horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::EditedCustomGame(EditAction::Add))
                        .width(Length::Units(125))
                        .style(style::Button::Primary),
                    ),
                )
                .push(self.games_editor.view(&config, &translator, &operation)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let translator = Translator::default();
        let mut modal_theme: Option<ModalTheme> = None;
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(ModalTheme::Error { variant: x });
                Config::default()
            }
        };
        let manifest = match Manifest::load(&mut config, true) {
            Ok(x) => x,
            Err(x) => {
                modal_theme = Some(ModalTheme::Error { variant: x });
                Manifest::default()
            }
        };

        (
            Self {
                backup_screen: BackupScreenComponent::new(&config),
                restore_screen: RestoreScreenComponent::new(&config),
                custom_games_screen: CustomGamesScreenComponent::new(&config),
                translator,
                config,
                manifest,
                modal_theme,
                ..Self::default()
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        self.translator.window_title()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Idle => {
                self.operation = None;
                self.modal_theme = None;
                self.progress.current = 0.0;
                self.progress.max = 0.0;
                self.operation_should_cancel
                    .swap(false, std::sync::atomic::Ordering::Relaxed);
                Command::none()
            }
            Message::Ignore => Command::none(),
            Message::ConfirmBackupStart => {
                self.modal_theme = Some(ModalTheme::ConfirmBackup);
                Command::none()
            }
            Message::ConfirmRestoreStart => {
                self.modal_theme = Some(ModalTheme::ConfirmRestore);
                Command::none()
            }
            Message::BackupStart { preview } => {
                if self.operation.is_some() {
                    return Command::none();
                }

                let backup_path = &self.config.backup.path;
                if !preview {
                    if let Err(e) = prepare_backup_target(&backup_path) {
                        self.modal_theme = Some(ModalTheme::Error { variant: e });
                        return Command::none();
                    }
                }

                let mut all_games = self.manifest.0.clone();
                for custom_game in &self.config.custom_games {
                    all_games.insert(custom_game.name.clone(), Game::from(custom_game.to_owned()));
                }

                self.backup_screen.status.clear();
                self.backup_screen.log.entries.clear();
                self.modal_theme = None;
                self.progress.current = 0.0;
                self.progress.max = all_games.len() as f32;

                self.operation = Some(if preview {
                    OngoingOperation::PreviewBackup
                } else {
                    OngoingOperation::Backup
                });

                let mut commands: Vec<Command<Message>> = vec![];
                for key in all_games.iter().map(|(k, _)| k.clone()) {
                    let game = all_games[&key].clone();
                    let roots = self.config.roots.clone();
                    let backup_path2 = backup_path.clone();
                    let steam_id = game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;
                    let cancel_flag = self.operation_should_cancel.clone();
                    let ignored = !self.config.is_game_enabled_for_backup(&key);
                    commands.push(Command::perform(
                        async move {
                            if key.trim().is_empty() {
                                return (None, None, OperationStepDecision::Ignored);
                            }
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(std::time::Duration::from_millis(1));
                                return (None, None, OperationStepDecision::Cancelled);
                            }

                            let scan_info = scan_game_for_backup(
                                &game,
                                &key,
                                &roots,
                                &StrictPath::from_std_path_buf(&app_dir()),
                                &steam_id,
                            );
                            if ignored {
                                return (Some(scan_info), None, OperationStepDecision::Ignored);
                            }

                            let backup_info = if !preview {
                                Some(back_up_game(&scan_info, &backup_path2, &key))
                            } else {
                                None
                            };
                            (Some(scan_info), backup_info, OperationStepDecision::Processed)
                        },
                        move |(scan_info, backup_info, decision)| Message::BackupStep {
                            scan_info,
                            backup_info,
                            decision,
                        },
                    ));
                }

                Command::batch(commands)
            }
            Message::RestoreStart { preview } => {
                if self.operation.is_some() {
                    return Command::none();
                }

                let restore_path = &self.config.restore.path;
                if !restore_path.is_dir() {
                    self.modal_theme = Some(ModalTheme::Error {
                        variant: Error::RestorationSourceInvalid {
                            path: restore_path.clone(),
                        },
                    });
                    return Command::none();
                }

                let restorables = scan_dir_for_restorable_games(&restore_path);

                self.restore_screen.status.clear();
                self.restore_screen.log.entries.clear();
                self.modal_theme = None;

                if restorables.is_empty() {
                    return Command::none();
                }

                self.operation = Some(if preview {
                    OngoingOperation::PreviewRestore
                } else {
                    OngoingOperation::Restore
                });
                self.progress.current = 0.0;
                self.progress.max = restorables.len() as f32;

                let mut commands: Vec<Command<Message>> = vec![];
                for (name, subdir) in restorables {
                    let redirects = self.config.get_redirects();
                    let cancel_flag = self.operation_should_cancel.clone();
                    let ignored = !self.config.is_game_enabled_for_restore(&name);
                    commands.push(Command::perform(
                        async move {
                            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                // TODO: https://github.com/hecrj/iced/issues/436
                                std::thread::sleep(std::time::Duration::from_millis(1));
                                return (None, None, OperationStepDecision::Cancelled);
                            }

                            let scan_info = scan_dir_for_restoration(&subdir);
                            if ignored {
                                return (Some(scan_info), None, OperationStepDecision::Ignored);
                            }

                            let backup_info = if !preview {
                                Some(restore_game(&scan_info, &redirects))
                            } else {
                                None
                            };
                            (Some(scan_info), backup_info, OperationStepDecision::Processed)
                        },
                        move |(scan_info, backup_info, decision)| Message::RestoreStep {
                            scan_info,
                            backup_info,
                            decision,
                        },
                    ));
                }

                Command::batch(commands)
            }
            Message::BackupStep {
                scan_info,
                backup_info,
                decision,
            } => {
                self.progress.current += 1.0;
                if let Some(scan_info) = scan_info {
                    if scan_info.found_anything() {
                        self.backup_screen.status.add_game(
                            &scan_info,
                            &backup_info,
                            decision == OperationStepDecision::Processed,
                        );
                        self.backup_screen.log.entries.push(GameListEntry {
                            scan_info,
                            backup_info,
                            ..Default::default()
                        });
                    }
                }
                if self.progress.complete() {
                    Command::perform(async move {}, move |_| Message::BackupComplete)
                } else {
                    Command::none()
                }
            }
            Message::RestoreStep {
                scan_info,
                backup_info,
                decision,
            } => {
                self.progress.current += 1.0;
                if let Some(scan_info) = scan_info {
                    if scan_info.found_anything() {
                        self.restore_screen.status.add_game(
                            &scan_info,
                            &backup_info,
                            decision == OperationStepDecision::Processed,
                        );
                        self.restore_screen.log.entries.push(GameListEntry {
                            scan_info,
                            backup_info,
                            ..Default::default()
                        });
                    }
                }
                if self.progress.complete() {
                    Command::perform(async move {}, move |_| Message::RestoreComplete)
                } else {
                    Command::none()
                }
            }
            Message::CancelOperation => {
                self.operation_should_cancel
                    .swap(true, std::sync::atomic::Ordering::Relaxed);
                match self.operation {
                    Some(OngoingOperation::Backup) => {
                        self.operation = Some(OngoingOperation::CancelBackup);
                    }
                    Some(OngoingOperation::PreviewBackup) => {
                        self.operation = Some(OngoingOperation::CancelPreviewBackup);
                    }
                    Some(OngoingOperation::Restore) => {
                        self.operation = Some(OngoingOperation::CancelRestore);
                    }
                    Some(OngoingOperation::PreviewRestore) => {
                        self.operation = Some(OngoingOperation::CancelPreviewRestore);
                    }
                    _ => {}
                };
                Command::none()
            }
            Message::BackupComplete => {
                for entry in &self.backup_screen.log.entries {
                    if let Some(backup_info) = &entry.backup_info {
                        if !backup_info.successful() {
                            self.modal_theme = Some(ModalTheme::Error {
                                variant: Error::SomeEntriesFailed,
                            });
                            return Command::none();
                        }
                    }
                }
                Command::perform(async move {}, move |_| Message::Idle)
            }
            Message::RestoreComplete => {
                for entry in &self.restore_screen.log.entries {
                    if let Some(backup_info) = &entry.backup_info {
                        if !backup_info.successful() {
                            self.modal_theme = Some(ModalTheme::Error {
                                variant: Error::SomeEntriesFailed,
                            });
                            return Command::none();
                        }
                    }
                }
                Command::perform(async move {}, move |_| Message::Idle)
            }
            Message::EditedBackupTarget(text) => {
                self.backup_screen.backup_target_history.push(&text);
                self.config.backup.path.reset(text);
                self.config.save();
                Command::none()
            }
            Message::EditedRestoreSource(text) => {
                self.restore_screen.restore_source_history.push(&text);
                self.config.restore.path.reset(text);
                self.config.save();
                Command::none()
            }
            Message::EditedRoot(action) => {
                match action {
                    EditAction::Add => {
                        self.backup_screen.root_editor.rows.push(RootEditorRow::default());
                        self.config.roots.push(RootsConfig {
                            path: StrictPath::default(),
                            store: Store::Other,
                        });
                    }
                    EditAction::Change(index, value) => {
                        self.backup_screen.root_editor.rows[index].text_history.push(&value);
                        self.config.roots[index].path.reset(value);
                    }
                    EditAction::Remove(index) => {
                        self.backup_screen.root_editor.rows.remove(index);
                        self.config.roots.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::SelectedRootStore(index, store) => {
                self.config.roots[index].store = store;
                self.config.save();
                Command::none()
            }
            Message::EditedRedirect(action, field) => {
                match action {
                    EditAction::Add => {
                        self.restore_screen
                            .redirect_editor
                            .rows
                            .push(RedirectEditorRow::default());
                        self.config.add_redirect(&StrictPath::default(), &StrictPath::default());
                    }
                    EditAction::Change(index, value) => match field {
                        Some(RedirectEditActionField::Source) => {
                            self.restore_screen.redirect_editor.rows[index]
                                .source_text_history
                                .push(&value);
                            self.config.restore.redirects[index].source.reset(value);
                        }
                        Some(RedirectEditActionField::Target) => {
                            self.restore_screen.redirect_editor.rows[index]
                                .target_text_history
                                .push(&value);
                            self.config.restore.redirects[index].target.reset(value);
                        }
                        _ => {}
                    },
                    EditAction::Remove(index) => {
                        self.restore_screen.redirect_editor.rows.remove(index);
                        self.config.restore.redirects.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedCustomGame(action) => {
                match action {
                    EditAction::Add => {
                        self.custom_games_screen
                            .games_editor
                            .entries
                            .push(CustomGamesEditorEntry::default());
                        self.config.add_custom_game();
                    }
                    EditAction::Change(index, value) => {
                        self.custom_games_screen.games_editor.entries[index]
                            .text_history
                            .push(&value);
                        self.config.custom_games[index].name = value;
                    }
                    EditAction::Remove(index) => {
                        self.custom_games_screen.games_editor.entries.remove(index);
                        self.config.custom_games.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedCustomGameFile(game_index, action) => {
                match action {
                    EditAction::Add => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .files
                            .push(CustomGamesEditorEntryRow::default());
                        self.config.custom_games[game_index].files.push("".to_string());
                    }
                    EditAction::Change(index, value) => {
                        self.custom_games_screen.games_editor.entries[game_index].files[index]
                            .text_history
                            .push(&value);
                        self.config.custom_games[game_index].files[index] = value;
                    }
                    EditAction::Remove(index) => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .files
                            .remove(index);
                        self.config.custom_games[game_index].files.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::EditedCustomGameRegistry(game_index, action) => {
                match action {
                    EditAction::Add => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .registry
                            .push(CustomGamesEditorEntryRow::default());
                        self.config.custom_games[game_index].registry.push("".to_string());
                    }
                    EditAction::Change(index, value) => {
                        self.custom_games_screen.games_editor.entries[game_index].registry[index]
                            .text_history
                            .push(&value);
                        self.config.custom_games[game_index].registry[index] = value;
                    }
                    EditAction::Remove(index) => {
                        self.custom_games_screen.games_editor.entries[game_index]
                            .registry
                            .remove(index);
                        self.config.custom_games[game_index].registry.remove(index);
                    }
                }
                self.config.save();
                Command::none()
            }
            Message::SwitchScreen(screen) => {
                self.screen = screen;
                Command::none()
            }
            Message::ToggleGameListEntryExpanded { name } => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &mut self.backup_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                entry.expanded = !entry.expanded;
                            }
                        }
                    }
                    Screen::Restore => {
                        for entry in &mut self.restore_screen.log.entries {
                            if entry.scan_info.game_name == name {
                                entry.expanded = !entry.expanded;
                            }
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::ToggleGameListEntryEnabled {
                name,
                enabled,
                restoring,
            } => {
                match (restoring, enabled) {
                    (false, false) => self.config.disable_game_for_backup(&name),
                    (false, true) => self.config.enable_game_for_backup(&name),
                    (true, false) => self.config.disable_game_for_restore(&name),
                    (true, true) => self.config.enable_game_for_restore(&name),
                };
                self.config.save();
                Command::none()
            }
            Message::BrowseDir(subject) => Command::perform(
                async move { native_dialog::OpenSingleDir { dir: None }.show() },
                move |choice| match choice {
                    Ok(Some(path)) => match subject {
                        BrowseSubject::BackupTarget => Message::EditedBackupTarget(path),
                        BrowseSubject::RestoreSource => Message::EditedRestoreSource(path),
                        BrowseSubject::Root(i) => Message::EditedRoot(EditAction::Change(i, path)),
                        BrowseSubject::RedirectSource(i) => {
                            Message::EditedRedirect(EditAction::Change(i, path), Some(RedirectEditActionField::Source))
                        }
                        BrowseSubject::RedirectTarget(i) => {
                            Message::EditedRedirect(EditAction::Change(i, path), Some(RedirectEditActionField::Target))
                        }
                        BrowseSubject::CustomGameFile(i, j) => {
                            Message::EditedCustomGameFile(i, EditAction::Change(j, path))
                        }
                    },
                    Ok(None) => Message::Ignore,
                    Err(_) => Message::BrowseDirFailure,
                },
            ),
            Message::BrowseDirFailure => {
                self.modal_theme = Some(ModalTheme::Error {
                    variant: Error::UnableToBrowseFileSystem,
                });
                Command::none()
            }
            Message::SelectAllGames => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &self.backup_screen.log.entries {
                            self.config.enable_game_for_backup(&entry.scan_info.game_name);
                        }
                    }
                    Screen::Restore => {
                        for entry in &self.restore_screen.log.entries {
                            self.config.enable_game_for_restore(&entry.scan_info.game_name);
                        }
                    }
                    _ => {}
                }
                self.config.save();
                Command::none()
            }
            Message::DeselectAllGames => {
                match self.screen {
                    Screen::Backup => {
                        for entry in &self.backup_screen.log.entries {
                            self.config.disable_game_for_backup(&entry.scan_info.game_name);
                        }
                    }
                    Screen::Restore => {
                        for entry in &self.restore_screen.log.entries {
                            self.config.disable_game_for_restore(&entry.scan_info.game_name);
                        }
                    }
                    _ => {}
                }
                self.config.save();
                Command::none()
            }
            Message::SubscribedEvent(event) => {
                if let iced_native::Event::Keyboard(key) = event {
                    if let Some((key_code, modifiers)) = get_key_pressed(key) {
                        let activated = if cfg!(target_os = "mac") {
                            modifiers.logo || modifiers.control
                        } else {
                            modifiers.control
                        };
                        let shortcut = match (key_code, activated, modifiers.shift) {
                            (KeyCode::Z, true, false) => Some(Shortcut::Undo),
                            (KeyCode::Y, true, false) | (KeyCode::Z, true, true) => Some(Shortcut::Redo),
                            (KeyCode::C, true, false) => Some(Shortcut::ClipboardCopy),
                            (KeyCode::X, true, false) => Some(Shortcut::ClipboardCut),
                            _ => None,
                        };

                        if let Some(shortcut) = shortcut {
                            let mut matched = false;

                            if self.backup_screen.backup_target_input.is_focused() {
                                apply_shortcut_to_strict_path_field(
                                    &shortcut,
                                    &mut self.config.backup.path,
                                    &self.backup_screen.backup_target_input,
                                    &mut self.backup_screen.backup_target_history,
                                );
                                matched = true;
                            } else if self.restore_screen.restore_source_input.is_focused() {
                                apply_shortcut_to_strict_path_field(
                                    &shortcut,
                                    &mut self.config.restore.path,
                                    &self.restore_screen.restore_source_input,
                                    &mut self.restore_screen.restore_source_history,
                                );
                                matched = true;
                            } else {
                                for (i, root) in self.backup_screen.root_editor.rows.iter_mut().enumerate() {
                                    if root.text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.roots[i].path,
                                            &root.text_state,
                                            &mut root.text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                }
                                for (i, redirect) in self.restore_screen.redirect_editor.rows.iter_mut().enumerate() {
                                    if redirect.source_text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.restore.redirects[i].source,
                                            &redirect.source_text_state,
                                            &mut redirect.source_text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                    if redirect.target_text_state.is_focused() {
                                        apply_shortcut_to_strict_path_field(
                                            &shortcut,
                                            &mut self.config.restore.redirects[i].target,
                                            &redirect.target_text_state,
                                            &mut redirect.target_text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                }
                                for (i, game) in self.custom_games_screen.games_editor.entries.iter_mut().enumerate() {
                                    if matched {
                                        break;
                                    }
                                    if game.text_state.is_focused() {
                                        apply_shortcut_to_string_field(
                                            &shortcut,
                                            &mut self.config.custom_games[i].name,
                                            &game.text_state,
                                            &mut game.text_history,
                                        );
                                        matched = true;
                                        break;
                                    }
                                    for (j, file_row) in game.files.iter_mut().enumerate() {
                                        if file_row.text_state.is_focused() {
                                            apply_shortcut_to_string_field(
                                                &shortcut,
                                                &mut self.config.custom_games[i].files[j],
                                                &file_row.text_state,
                                                &mut file_row.text_history,
                                            );
                                            matched = true;
                                            break;
                                        }
                                    }
                                    for (j, registry_row) in game.registry.iter_mut().enumerate() {
                                        if registry_row.text_state.is_focused() {
                                            apply_shortcut_to_string_field(
                                                &shortcut,
                                                &mut self.config.custom_games[i].registry[j],
                                                &registry_row.text_state,
                                                &mut registry_row.text_history,
                                            );
                                            matched = true;
                                            break;
                                        }
                                    }
                                }
                            }

                            if matched {
                                self.config.save();
                            }
                        }
                    }
                };
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events().map(Message::SubscribedEvent)
    }

    fn view(&mut self) -> Element<Message> {
        if let Some(m) = &self.modal_theme {
            return self.modal.view(m, &self.config, &self.translator).into();
        }

        Column::new()
            .align_items(Align::Center)
            .push(
                Row::new()
                    .spacing(20)
                    .push(
                        Button::new(
                            &mut self.nav_to_backup_button,
                            Text::new(self.translator.nav_backup_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::Backup))
                        .width(Length::Units(200))
                        .style(match self.screen {
                            Screen::Backup => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    )
                    .push(
                        Button::new(
                            &mut self.nav_to_restore_button,
                            Text::new(self.translator.nav_restore_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::Restore))
                        .width(Length::Units(200))
                        .style(match self.screen {
                            Screen::Restore => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    )
                    .push(
                        Button::new(
                            &mut self.nav_to_custom_games_button,
                            Text::new(self.translator.nav_custom_games_button())
                                .size(16)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .on_press(Message::SwitchScreen(Screen::CustomGames))
                        .width(Length::Units(200))
                        .style(match self.screen {
                            Screen::CustomGames => style::NavButton::Active,
                            _ => style::NavButton::Inactive,
                        }),
                    ),
            )
            .push(
                match self.screen {
                    Screen::Backup => self.backup_screen.view(&self.config, &self.translator, &self.operation),
                    Screen::Restore => self
                        .restore_screen
                        .view(&self.config, &self.translator, &self.operation),
                    Screen::CustomGames => {
                        self.custom_games_screen
                            .view(&self.config, &self.translator, &self.operation)
                    }
                }
                .height(Length::FillPortion(10_000)),
            )
            .push(self.progress.view())
            .into()
    }
}

mod style {
    use iced::{button, container, scrollable, Background, Color, Vector};

    pub enum Button {
        Primary,
        Disabled,
        Negative,
        GameListEntryTitle,
        GameListEntryTitleFailed,
        GameListEntryTitleDisabled,
    }
    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: match self {
                    Self::Primary => Some(Background::Color(Color::from_rgb8(28, 107, 223))),
                    Self::GameListEntryTitle => Some(Background::Color(Color::from_rgb8(77, 127, 201))),
                    Self::GameListEntryTitleFailed => Some(Background::Color(Color::from_rgb8(201, 77, 77))),
                    Self::GameListEntryTitleDisabled => Some(Background::Color(Color::from_rgb8(230, 230, 230))),
                    Self::Disabled => Some(Background::Color(Color::from_rgb8(169, 169, 169))),
                    Self::Negative => Some(Background::Color(Color::from_rgb8(255, 0, 0))),
                },
                border_radius: match self {
                    Self::GameListEntryTitle | Self::GameListEntryTitleFailed | Self::GameListEntryTitleDisabled => 10,
                    _ => 4,
                },
                shadow_offset: Vector::new(1.0, 1.0),
                text_color: match self {
                    Self::GameListEntryTitleDisabled => Color::from_rgb8(0x44, 0x44, 0x44),
                    _ => Color::from_rgb8(0xEE, 0xEE, 0xEE),
                },
                ..button::Style::default()
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                text_color: match self {
                    Self::GameListEntryTitleDisabled => Color::BLACK,
                    _ => Color::WHITE,
                },
                shadow_offset: Vector::new(1.0, 2.0),
                ..self.active()
            }
        }
    }

    pub enum NavButton {
        Active,
        Inactive,
    }
    impl button::StyleSheet for NavButton {
        fn active(&self) -> button::Style {
            button::Style {
                background: match self {
                    Self::Active => Some(Background::Color(Color::from_rgba8(136, 0, 219, 0.9))),
                    Self::Inactive => Some(Background::Color(Color::TRANSPARENT)),
                },
                border_radius: 10,
                border_width: 1,
                border_color: Color::from_rgb8(136, 0, 219),
                text_color: match self {
                    Self::Active => Color::WHITE,
                    Self::Inactive => Color::BLACK,
                },
                ..button::Style::default()
            }
        }

        fn hovered(&self) -> button::Style {
            button::Style {
                background: match self {
                    Self::Active => Some(Background::Color(Color::from_rgba8(136, 0, 219, 0.95))),
                    Self::Inactive => Some(Background::Color(Color::from_rgba8(136, 0, 219, 0.2))),
                },
                ..self.active()
            }
        }
    }

    pub enum Container {
        ModalBackground,
        GameListEntry,
        GameListEntryBody,
    }

    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: match self {
                    Self::ModalBackground => Some(Background::Color(Color::from_rgb8(230, 230, 230))),
                    _ => None,
                },
                border_color: match self {
                    Self::GameListEntry => Color::from_rgb8(230, 230, 230),
                    _ => Color::BLACK,
                },
                border_width: match self {
                    Self::GameListEntry => 1,
                    _ => 0,
                },
                border_radius: match self {
                    Self::GameListEntry => 10,
                    _ => 0,
                },
                ..container::Style::default()
            }
        }
    }

    pub struct Scrollable;
    impl scrollable::StyleSheet for Scrollable {
        fn active(&self) -> scrollable::Scrollbar {
            scrollable::Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border_radius: 5,
                border_width: 0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: Color::from_rgba8(0, 0, 0, 0.7),
                    border_radius: 5,
                    border_width: 0,
                    border_color: Color::TRANSPARENT,
                },
            }
        }

        fn hovered(&self) -> scrollable::Scrollbar {
            let active = self.active();

            scrollable::Scrollbar {
                background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.4))),
                scroller: scrollable::Scroller {
                    color: Color::from_rgba8(0, 0, 0, 0.8),
                    ..active.scroller
                },
                ..active
            }
        }
    }
}

pub fn run_gui() {
    let mut settings = iced::Settings::default();
    set_app_icon(&mut settings);
    App::run(settings)
}
