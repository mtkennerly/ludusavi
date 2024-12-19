use crate::{
    lang::TRANSLATOR,
    resource::manifest,
    scan::{Duplication, ScanInfo},
};

use super::ScanChange;

#[derive(Clone, Debug)]
pub enum Event {
    Toggled,
    ToggledFilter { filter: FilterKind, enabled: bool },
    EditedGameName(String),
    Reset,
    EditedFilterUniqueness(Uniqueness),
    EditedFilterCompleteness(Completeness),
    EditedFilterEnablement(Enablement),
    EditedFilterChange(Change),
    EditedFilterManifest(Manifest),
}

#[derive(Clone, Copy, Debug)]
pub enum FilterKind {
    Uniqueness,
    Completeness,
    Enablement,
    Change,
    Manifest,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Uniqueness {
    Unique,
    #[default]
    Duplicate,
}

impl Uniqueness {
    pub const ALL: &'static [Self] = &[Self::Unique, Self::Duplicate];

    pub fn qualifies(&self, duplicated: Duplication) -> bool {
        match self {
            Self::Unique => duplicated.unique(),
            Self::Duplicate => !duplicated.unique(),
        }
    }
}

impl ToString for Uniqueness {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_uniqueness(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Completeness {
    Complete,
    #[default]
    Partial,
}

impl Completeness {
    pub const ALL: &'static [Self] = &[Self::Complete, Self::Partial];

    pub fn qualifies(&self, scan: &ScanInfo) -> bool {
        match self {
            Self::Complete => !scan.any_ignored(),
            Self::Partial => scan.any_ignored(),
        }
    }
}

impl ToString for Completeness {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_completeness(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Enablement {
    Enabled,
    #[default]
    Disabled,
}

impl Enablement {
    pub const ALL: &'static [Self] = &[Self::Enabled, Self::Disabled];

    pub fn qualifies(&self, enabled: bool) -> bool {
        match self {
            Self::Enabled => enabled,
            Self::Disabled => !enabled,
        }
    }
}

impl ToString for Enablement {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_enablement(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Change {
    New,
    Updated,
    Unchanged,
    #[default]
    Unscanned,
}

impl ToString for Change {
    fn to_string(&self) -> String {
        TRANSLATOR.filter_freshness(*self)
    }
}

impl Change {
    pub const ALL: &'static [Self] = &[Self::New, Self::Updated, Self::Unchanged, Self::Unscanned];

    pub fn qualifies(&self, scan: &ScanInfo) -> bool {
        match self {
            Change::New => scan.overall_change() == ScanChange::New,
            Change::Updated => scan.overall_change() == ScanChange::Different,
            Change::Unchanged => scan.overall_change() == ScanChange::Same,
            Change::Unscanned => scan.overall_change() == ScanChange::Unknown,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Manifest {
    source: manifest::Source,
}

impl Manifest {
    pub fn new(source: manifest::Source) -> Self {
        Self { source }
    }
}

impl ToString for Manifest {
    fn to_string(&self) -> String {
        match &self.source {
            manifest::Source::Primary => TRANSLATOR.primary_manifest_label(),
            manifest::Source::Custom => TRANSLATOR.custom_games_label(),
            manifest::Source::Secondary(id) => id.to_string(),
        }
    }
}

impl Manifest {
    pub fn qualifies(&self, game: Option<&manifest::Game>, customized: bool) -> bool {
        game.map(|game| game.sources.contains(&self.source)).unwrap_or_default()
            || (self.source == manifest::Source::Custom && customized)
    }
}
