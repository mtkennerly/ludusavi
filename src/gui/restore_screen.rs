use iced::{Alignment, Length};

use crate::{
    cache::Cache,
    config::Config,
    gui::{
        button,
        common::{make_status_row, BrowseSubject, OngoingOperation, Screen, TextHistories, UndoSubject},
        game_list::GameList,
        widget::{Column, Container, Row, Text},
    },
    lang::Translator,
    manifest::Manifest,
    scan::DuplicateDetector,
};

#[derive(Default)]
pub struct RestoreScreenComponent {
    pub log: GameList,
    pub duplicate_detector: DuplicateDetector,
}

impl RestoreScreenComponent {
    pub fn new(config: &Config, cache: &Cache) -> Self {
        Self {
            log: GameList::with_recent_games(true, config, cache),
            ..Default::default()
        }
    }

    pub fn view(
        &self,
        config: &Config,
        manifest: &Manifest,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
        histories: &TextHistories,
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
                        .push(button::operation(
                            OngoingOperation::PreviewRestore,
                            operation.to_owned(),
                        ))
                        .push(button::operation(OngoingOperation::Restore, operation.to_owned()))
                        .push(button::toggle_all_scanned_games(
                            self.log.all_entries_selected(config, true),
                        ))
                        .push(button::search(Screen::Restore, self.log.search.show)),
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
                        .push(histories.input(UndoSubject::RestoreSource))
                        .push(button::open_folder(BrowseSubject::RestoreSource)),
                )
                .push(self.log.view(
                    true,
                    translator,
                    config,
                    manifest,
                    &self.duplicate_detector,
                    operation,
                    histories,
                )),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
    }
}
