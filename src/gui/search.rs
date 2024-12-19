use fuzzy_matcher::FuzzyMatcher;
use iced::{padding, Alignment};

use crate::{
    gui::{
        button,
        common::{Message, Screen, UndoSubject},
        shortcuts::TextHistories,
        style,
        widget::{checkbox, pick_list, text, Column, Container, Element, IcedParentExt, Row},
    },
    lang::TRANSLATOR,
    resource::{config::CustomGame, manifest::Manifest},
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
    pub manifest: Filter<game_filter::Manifest>,
}

fn template<'a, T: 'static + Default + Copy + Eq + PartialEq + ToString>(
    filter: &'a Filter<T>,
    kind: FilterKind,
    options: &'a [T],
    message: fn(T) -> Message,
) -> Element<'a> {
    Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .push(
            checkbox("", filter.active, move |enabled| Message::Filter {
                event: game_filter::Event::ToggledFilter { filter: kind, enabled },
            })
            .spacing(0),
        )
        .push(pick_list(options, Some(filter.choice), message))
        .into()
}

fn template_noncopy<T: 'static + Default + Clone + Eq + PartialEq + ToString>(
    filter: &Filter<T>,
    kind: FilterKind,
    options: Vec<T>,
    message: fn(T) -> Message,
) -> Element {
    Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .push(
            checkbox("", filter.active, move |enabled| Message::Filter {
                event: game_filter::Event::ToggledFilter { filter: kind, enabled },
            })
            .spacing(0),
        )
        .push(pick_list(options, Some(filter.choice.clone()), message))
        .into()
}

fn template_with_label<T: 'static + Default + Clone + Eq + PartialEq + ToString>(
    filter: &Filter<T>,
    label: String,
    kind: FilterKind,
    options: Vec<T>,
    message: fn(T) -> Message,
) -> Element {
    Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .push(checkbox(label, filter.active, move |enabled| Message::Filter {
            event: game_filter::Event::ToggledFilter { filter: kind, enabled },
        }))
        .push(pick_list(options, Some(filter.choice.clone()), message))
        .into()
}

impl FilterComponent {
    pub fn reset(&mut self) {
        self.game_name.clear();
        self.uniqueness.active = false;
        self.completeness.active = false;
        self.enablement.active = false;
        self.change.active = false;
        self.manifest.active = false;
    }

    pub fn is_dirty(&self) -> bool {
        !self.game_name.is_empty()
            || self.uniqueness.active
            || self.completeness.active
            || self.enablement.active
            || self.change.active
            || self.manifest.active
    }

    pub fn qualifies(
        &self,
        scan: &ScanInfo,
        manifest: &Manifest,
        enabled: bool,
        customized: bool,
        duplicated: Duplication,
        show_deselected_games: bool,
    ) -> bool {
        if !self.show {
            return true;
        }

        let fuzzy = self.game_name.is_empty()
            || fuzzy_matcher::skim::SkimMatcherV2::default()
                .fuzzy_match(&scan.game_name.to_lowercase(), &self.game_name.to_lowercase())
                .is_some();
        let unique = !self.uniqueness.active || self.uniqueness.choice.qualifies(duplicated);
        let complete = !self.completeness.active || self.completeness.choice.qualifies(scan);
        let enable = !show_deselected_games || !self.enablement.active || self.enablement.choice.qualifies(enabled);
        let changed = !self.change.active || self.change.choice.qualifies(scan);
        let manifest = !self.manifest.active
            || self
                .manifest
                .choice
                .qualifies(manifest.0.get(&scan.game_name), customized);

        fuzzy && unique && complete && changed && enable && manifest
    }

    pub fn toggle_filter(&mut self, filter: FilterKind, enabled: bool) {
        match filter {
            FilterKind::Uniqueness => self.uniqueness.active = enabled,
            FilterKind::Completeness => self.completeness.active = enabled,
            FilterKind::Enablement => self.enablement.active = enabled,
            FilterKind::Change => self.change.active = enabled,
            FilterKind::Manifest => self.manifest.active = enabled,
        }
    }

    pub fn view(
        &self,
        screen: Screen,
        histories: &TextHistories,
        show_deselected_games: bool,
        manifests: Vec<game_filter::Manifest>,
    ) -> Option<Element> {
        if !self.show {
            return None;
        }

        let content = Column::new()
            .padding(padding::left(5).right(5))
            .spacing(15)
            .push(
                Row::new()
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(text(TRANSLATOR.filter_label()))
                    .push(histories.input(match screen {
                        Screen::Restore => UndoSubject::RestoreSearchGameName,
                        _ => UndoSubject::BackupSearchGameName,
                    }))
                    .push(button::reset_filter(self.is_dirty())),
            )
            .push(
                Row::new()
                    .spacing(15)
                    .align_y(Alignment::Center)
                    .push(template(
                        &self.uniqueness,
                        FilterKind::Uniqueness,
                        game_filter::Uniqueness::ALL,
                        move |value| Message::Filter {
                            event: game_filter::Event::EditedFilterUniqueness(value),
                        },
                    ))
                    .push(template(
                        &self.completeness,
                        FilterKind::Completeness,
                        game_filter::Completeness::ALL,
                        move |value| Message::Filter {
                            event: game_filter::Event::EditedFilterCompleteness(value),
                        },
                    ))
                    .push(template(
                        &self.change,
                        FilterKind::Change,
                        game_filter::Change::ALL,
                        move |value| Message::Filter {
                            event: game_filter::Event::EditedFilterChange(value),
                        },
                    ))
                    .push_if(show_deselected_games, || {
                        template(
                            &self.enablement,
                            FilterKind::Enablement,
                            game_filter::Enablement::ALL,
                            move |value| Message::Filter {
                                event: game_filter::Event::EditedFilterEnablement(value),
                            },
                        )
                    })
                    .push_if(manifests.len() == 2, || {
                        template_noncopy(&self.manifest, FilterKind::Manifest, manifests.clone(), move |value| {
                            Message::Filter {
                                event: game_filter::Event::EditedFilterManifest(value),
                            }
                        })
                    })
                    .push_if(manifests.len() > 2, || {
                        template_with_label(
                            &self.manifest,
                            TRANSLATOR.source_field(),
                            FilterKind::Manifest,
                            manifests,
                            move |value| Message::Filter {
                                event: game_filter::Event::EditedFilterManifest(value),
                            },
                        )
                    })
                    .wrap(),
            );

        Some(
            Container::new(
                Container::new(content)
                    .class(style::Container::GameListEntry)
                    .padding(padding::top(5).bottom(5)),
            )
            .padding(padding::left(15).right(15))
            .into(),
        )
    }
}

#[derive(Default)]
pub struct CustomGamesFilter {
    pub enabled: bool,
    pub name: String,
}

impl CustomGamesFilter {
    pub fn reset(&mut self) {
        self.name.clear();
    }

    pub fn is_dirty(&self) -> bool {
        !self.name.is_empty()
    }

    pub fn qualifies(&self, game: &CustomGame) -> bool {
        !self.enabled
            || self.name.is_empty()
            || fuzzy_matcher::skim::SkimMatcherV2::default()
                .fuzzy_match(&game.name.to_lowercase(), &self.name.to_lowercase())
                .is_some()
    }

    pub fn view<'a>(&'a self, histories: &TextHistories) -> Option<Element<'a>> {
        if !self.enabled {
            return None;
        }

        let content = Row::new()
            .padding(padding::left(5).right(5))
            .spacing(20)
            .align_y(Alignment::Center)
            .push(text(TRANSLATOR.filter_label()))
            .push(histories.input(UndoSubject::CustomGamesSearchGameName))
            .push(button::reset_filter(self.is_dirty()));

        Some(
            Container::new(
                Container::new(content)
                    .class(style::Container::GameListEntry)
                    .padding(padding::top(5).bottom(5)),
            )
            .padding(padding::left(15).right(15))
            .into(),
        )
    }
}
