use iced::{Alignment, Length};

use crate::{
    cache::Cache,
    config::Config,
    gui::{
        common::{
            make_status_row, operation_button, BrowseSubject, CommonButton, Message, OngoingOperation, Screen,
            UndoSubject,
        },
        game_list::GameList,
        shortcuts::TextHistory,
        style,
        widget::{Column, Container, Row, Text, TextInput, Undoable},
    },
    lang::Translator,
    manifest::Manifest,
    scan::DuplicateDetector,
};

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
                        .push(operation_button(OngoingOperation::PreviewRestore, operation.to_owned()))
                        .push(operation_button(OngoingOperation::Restore, operation.to_owned()))
                        .push(CommonButton::ToggleAllScannedGames {
                            all_enabled: self.log.all_entries_selected(config, true),
                        })
                        .push(CommonButton::Search {
                            screen: Screen::Restore,
                            open: self.log.search.show,
                        }),
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
                        .push(CommonButton::OpenFolder {
                            subject: BrowseSubject::RestoreSource,
                        }),
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
