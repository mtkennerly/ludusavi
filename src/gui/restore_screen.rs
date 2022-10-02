use crate::{
    cache::Cache,
    config::Config,
    gui::{
        common::OngoingOperation,
        common::{make_status_row, BrowseSubject, Message, Screen},
        game_list::GameList,
        icon::Icon,
        style,
    },
    lang::Translator,
    manifest::Manifest,
    prelude::DuplicateDetector,
    shortcuts::TextHistory,
};

use iced::{
    alignment::Horizontal as HorizontalAlignment, button, text_input, Alignment, Button, Column, Container, Length,
    Row, Text, TextInput,
};

#[derive(Default)]
pub struct RestoreScreenComponent {
    pub log: GameList,
    start_button: button::State,
    preview_button: button::State,
    select_all_button: button::State,
    toggle_search_button: button::State,
    pub restore_source_input: text_input::State,
    pub restore_source_history: TextHistory,
    restore_source_browse_button: button::State,
    pub duplicate_detector: DuplicateDetector,
}

impl RestoreScreenComponent {
    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(true, config, cache),
            restore_source_history: TextHistory::new(&config.backup.path.raw(), 100),
            ..Default::default()
        }
    }

    pub fn view(
        &mut self,
        config: &Config,
        manifest: &Manifest,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        Container::new(
            Column::new()
                .align_items(Alignment::Center)
                .spacing(20)
                .push(
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
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
                                None => Message::RestoreStart {
                                    preview: true,
                                    games: None,
                                },
                                Some(OngoingOperation::PreviewRestore) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary(config.theme),
                                Some(OngoingOperation::PreviewRestore) => style::Button::Negative(config.theme),
                                _ => style::Button::Disabled(config.theme),
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
                                None => Message::ConfirmRestoreStart { games: None },
                                Some(OngoingOperation::Restore) => Message::CancelOperation,
                                _ => Message::Ignore,
                            })
                            .width(Length::Units(125))
                            .style(match operation {
                                None => style::Button::Primary(config.theme),
                                Some(OngoingOperation::Restore) => style::Button::Negative(config.theme),
                                _ => style::Button::Disabled(config.theme),
                            }),
                        )
                        .push({
                            let restoring = true;
                            Button::new(
                                &mut self.select_all_button,
                                Text::new(if self.log.all_entries_selected(config, restoring) {
                                    translator.deselect_all_button()
                                } else {
                                    translator.select_all_button()
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(if self.log.all_entries_selected(config, restoring) {
                                Message::DeselectAllGames
                            } else {
                                Message::SelectAllGames
                            })
                            .width(Length::Units(125))
                            .style(style::Button::Primary(config.theme))
                        })
                        .push(
                            Button::new(&mut self.toggle_search_button, Icon::Search.as_text())
                                .on_press(Message::ToggleSearch {
                                    screen: Screen::Restore,
                                })
                                .style(if self.log.search.show {
                                    style::Button::Negative(config.theme)
                                } else {
                                    style::Button::Primary(config.theme)
                                }),
                        ),
                )
                .push(make_status_row(
                    translator,
                    &self.log.compute_operation_status(config, true),
                    self.duplicate_detector.any_duplicates(),
                    config.theme,
                ))
                .push(
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(Text::new(translator.restore_source_label()))
                        .push(
                            TextInput::new(
                                &mut self.restore_source_input,
                                "",
                                &config.restore.path.raw(),
                                Message::EditedRestoreSource,
                            )
                            .style(style::TextInput(config.theme))
                            .padding(5),
                        )
                        .push(
                            Button::new(&mut self.restore_source_browse_button, Icon::FolderOpen.as_text())
                                .on_press(Message::BrowseDir(BrowseSubject::RestoreSource))
                                .style(style::Button::Primary(config.theme)),
                        ),
                )
                .push(
                    self.log
                        .view(true, translator, config, manifest, &self.duplicate_detector, operation),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
