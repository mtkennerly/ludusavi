use crate::{
    lang::TRANSLATOR,
    scan::{Duplication, ScanInfo},
};

use super::ScanChange;

#[derive(Clone, Copy, Debug)]
pub enum FilterKind {
    Uniqueness,
    Completeness,
    Enablement,
    Change,
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
