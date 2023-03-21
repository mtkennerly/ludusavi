use crate::{
    cache::Cache,
    config::Config,
    gui::{
        common::{make_status_row, BrowseSubject, IcedButtonExt, Message, OngoingOperation, Screen, UndoSubject},
        game_list::GameList,
        icon::Icon,
        shortcuts::TextHistory,
        style,
    },
    lang::Translator,
    manifest::Manifest,
    scan::DuplicateDetector,
};

use crate::gui::widget::{Button, Column, Container, Row, Text, TextInput, Undoable};
use iced::{alignment::Horizontal as HorizontalAlignment, Alignment, Length};

#[derive(Default)]
pub struct RestoreScreenComponent {
    pub log: GameList,
    pub restore_source_history: TextHistory,
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
        &self,
        config: &Config,
        manifest: &Manifest,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container {
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
                                Text::new(match operation {
                                    Some(OngoingOperation::PreviewRestore) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelPreviewRestore) => translator.cancelling_button(),
                                    _ => translator.preview_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press_some(match operation {
                                None => Some(Message::RestoreStart {
                                    preview: true,
                                    games: None,
                                }),
                                Some(OngoingOperation::PreviewRestore) => Some(Message::CancelOperation),
                                _ => None,
                            })
                            .width(125)
                            .style(match operation {
                                Some(OngoingOperation::PreviewRestore | OngoingOperation::CancelPreviewRestore) => {
                                    style::Button::Negative
                                }
                                _ => style::Button::Primary,
                            }),
                        )
                        .push(
                            Button::new(
                                Text::new(match operation {
                                    Some(OngoingOperation::Restore) => translator.cancel_button(),
                                    Some(OngoingOperation::CancelRestore) => translator.cancelling_button(),
                                    _ => translator.restore_button(),
                                })
                                .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press_some(match operation {
                                None => Some(Message::ConfirmRestoreStart { games: None }),
                                Some(OngoingOperation::Restore) => Some(Message::CancelOperation),
                                _ => None,
                            })
                            .width(125)
                            .style(match operation {
                                Some(OngoingOperation::Restore | OngoingOperation::CancelRestore) => {
                                    style::Button::Negative
                                }
                                _ => style::Button::Primary,
                            }),
                        )
                        .push({
                            let restoring = true;
                            Button::new(
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
                            .width(125)
                            .style(style::Button::Primary)
                        })
                        .push(
                            Button::new(Icon::Search.as_text())
                                .on_press(Message::ToggleSearch {
                                    screen: Screen::Restore,
                                })
                                .style(if self.log.search.show {
                                    style::Button::Negative
                                } else {
                                    style::Button::Primary
                                }),
                        ),
                )
                .push(make_status_row(
                    translator,
                    &self.log.compute_operation_status(config, true),
                    self.duplicate_detector.any_duplicates(),
                ))
                .push(
                    Row::new()
                        .padding([0, 20, 0, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(Text::new(translator.restore_source_label()))
                        .push(Undoable::new(
                            TextInput::new("", &config.restore.path.raw(), Message::EditedRestoreSource)
                                .style(style::TextInput)
                                .padding(5),
                            move |action| Message::UndoRedo(action, UndoSubject::RestoreSource),
                        ))
                        .push(
                            Button::new(Icon::FolderOpen.as_text())
                                .on_press(Message::BrowseDir(BrowseSubject::RestoreSource))
                                .style(style::Button::Primary),
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
