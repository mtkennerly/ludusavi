use fuzzy_matcher::FuzzyMatcher;
use iced::Alignment;

use crate::{
    gui::{
        common::{Message, Screen, UndoSubject},
        shortcuts::TextHistories,
        widget::{checkbox, pick_list, text, Column, Element, IcedParentExt, Row},
    },
    lang::TRANSLATOR,
    scan::{
        game_filter::{self, FilterKind},
        Duplication, ScanInfo,
    },
};

#[derive(Default, Clone, Eq, PartialEq)]
pub struct Filter<T> {
    active: bool,
    pub choice: T,
}

#[derive(Default)]
pub struct FilterComponent {
    pub show: bool,
    pub game_name: String,
    pub uniqueness: Filter<game_filter::Uniqueness>,
    pub completeness: Filter<game_filter::Completeness>,
    pub enablement: Filter<game_filter::Enablement>,
    pub change: Filter<game_filter::Change>,
}

fn template<'a, T: 'static + Default + Copy + Eq + PartialEq + ToString>(
    filter: &'a Filter<T>,
    kind: FilterKind,
    options: &'a [T],
    message: fn(T) -> Message,
) -> Element<'a> {
    Row::new()
        .spacing(10)
        .align_items(Alignment::Center)
        .push(
            checkbox("", filter.active, move |enabled| Message::ToggledSearchFilter {
                filter: kind,
                enabled,
            })
            .spacing(0),
        )
        .push(pick_list(options, Some(filter.choice), message))
        .into()
}

impl FilterComponent {
    pub fn qualifies(
        &self,
        scan: &ScanInfo,
        enabled: bool,
        duplicated: Duplication,
        show_deselected_games: bool,
    ) -> bool {
        let fuzzy = self.game_name.is_empty()
            || fuzzy_matcher::skim::SkimMatcherV2::default()
                .fuzzy_match(&scan.game_name, &self.game_name)
                .is_some();
        let unique = !self.uniqueness.active || self.uniqueness.choice.qualifies(duplicated);
        let complete = !self.completeness.active || self.completeness.choice.qualifies(scan);
        let enable = !show_deselected_games || !self.enablement.active || self.enablement.choice.qualifies(enabled);
        let changed = !self.change.active || self.change.choice.qualifies(scan);

        fuzzy && unique && complete && changed && enable
    }

    pub fn toggle_filter(&mut self, filter: FilterKind, enabled: bool) {
        match filter {
            FilterKind::Uniqueness => self.uniqueness.active = enabled,
            FilterKind::Completeness => self.completeness.active = enabled,
            FilterKind::Enablement => self.enablement.active = enabled,
            FilterKind::Change => self.change.active = enabled,
        }
    }

    pub fn view(&self, screen: Screen, histories: &TextHistories, show_deselected_games: bool) -> Option<Element> {
        if !self.show {
            return None;
        }
        Some(
            Column::new()
                .push(
                    Row::new()
                        .padding([0, 20, 10, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(text(TRANSLATOR.filter_label()))
                        .push(histories.input(match screen {
                            Screen::Restore => UndoSubject::RestoreSearchGameName,
                            _ => UndoSubject::BackupSearchGameName,
                        })),
                )
                .push(
                    Row::new()
                        .padding([0, 20, 20, 20])
                        .spacing(20)
                        .align_items(Alignment::Center)
                        .push(template(
                            &self.uniqueness,
                            FilterKind::Uniqueness,
                            game_filter::Uniqueness::ALL,
                            Message::EditedSearchFilterUniqueness,
                        ))
                        .push(template(
                            &self.completeness,
                            FilterKind::Completeness,
                            game_filter::Completeness::ALL,
                            Message::EditedSearchFilterCompleteness,
                        ))
                        .push(template(
                            &self.change,
                            FilterKind::Change,
                            game_filter::Change::ALL,
                            Message::EditedSearchFilterChange,
                        ))
                        .push_if(show_deselected_games, || {
                            template(
                                &self.enablement,
                                FilterKind::Enablement,
                                game_filter::Enablement::ALL,
                                Message::EditedSearchFilterEnablement,
                            )
                        }),
                )
                .into(),
        )
    }
}
