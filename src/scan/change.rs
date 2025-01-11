use crate::{
    lang::{ADD_SYMBOL, CHANGE_SYMBOL, REMOVAL_SYMBOL},
    prelude::StrictPath,
    scan::ScanKind,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, schemars::JsonSchema)]
pub enum ScanChange {
    New,
    Different,
    Removed,
    Same,
    #[default]
    Unknown,
}

impl ScanChange {
    pub fn symbol(&self) -> &'static str {
        match self {
            ScanChange::New => ADD_SYMBOL,
            ScanChange::Different => CHANGE_SYMBOL,
            ScanChange::Removed => REMOVAL_SYMBOL,
            ScanChange::Same => "=",
            ScanChange::Unknown => "?",
        }
    }

    pub fn normalize(&self, ignored: bool, scan_kind: ScanKind) -> Self {
        match self {
            ScanChange::New if ignored => Self::Same,
            ScanChange::New => *self,
            ScanChange::Different if ignored && scan_kind.is_restore() => Self::Same,
            ScanChange::Different if ignored && scan_kind.is_backup() => Self::Removed,
            ScanChange::Different => Self::Different,
            ScanChange::Removed => *self,
            ScanChange::Same if ignored && scan_kind.is_backup() => Self::Removed,
            ScanChange::Same => *self,
            ScanChange::Unknown => *self,
        }
    }

    pub fn is_changed(&self) -> bool {
        match self {
            Self::New => true,
            Self::Different => true,
            Self::Removed => true,
            Self::Same => false,
            // This is because we want unchanged and unscanned games to be filtered differently:
            Self::Unknown => true,
        }
    }

    pub fn will_take_space(&self) -> bool {
        match self {
            Self::New => true,
            Self::Different => true,
            Self::Removed => false,
            Self::Same => true,
            Self::Unknown => true,
        }
    }

    pub fn is_inert(&self) -> bool {
        match self {
            ScanChange::New => false,
            ScanChange::Different => false,
            ScanChange::Removed => true,
            ScanChange::Same => false,
            ScanChange::Unknown => true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, schemars::JsonSchema)]
pub struct ScanChangeCount {
    pub new: usize,
    pub different: usize,
    #[serde(skip)]
    pub removed: usize,
    pub same: usize,
}

impl ScanChangeCount {
    pub fn new() -> Self {
        Self {
            new: 0,
            different: 0,
            removed: 0,
            same: 0,
        }
    }

    pub fn add(&mut self, change: ScanChange) {
        match change {
            ScanChange::New => self.new += 1,
            ScanChange::Different => self.different += 1,
            ScanChange::Removed => self.removed += 1,
            ScanChange::Same => self.same += 1,
            ScanChange::Unknown => (),
        }
    }

    pub fn brand_new(&self) -> bool {
        self.only(ScanChange::New)
    }

    pub fn updated(&self) -> bool {
        !self.brand_new() && (self.new > 0 || self.different > 0 || self.removed > 0)
    }

    fn only(&self, change: ScanChange) -> bool {
        let total = self.new + self.different + self.removed + self.same;
        let only = |count: usize| count > 0 && count == total;
        match change {
            ScanChange::New => only(self.new),
            ScanChange::Different => only(self.different),
            ScanChange::Removed => only(self.removed),
            ScanChange::Same => only(self.same),
            ScanChange::Unknown => false,
        }
    }

    pub fn overall(&self, only_constructive: bool) -> ScanChange {
        if self.brand_new() {
            ScanChange::New
        } else if self.only(ScanChange::Removed) {
            ScanChange::Removed
        } else if self.updated() {
            if only_constructive && self.new == 0 && self.different == 0 {
                ScanChange::Same
            } else {
                ScanChange::Different
            }
        } else if self.same != 0 {
            ScanChange::Same
        } else {
            ScanChange::Unknown
        }
    }
}

impl ScanChange {
    pub fn evaluate_backup(current_hash: &str, previous_hash: Option<&&String>) -> Self {
        match previous_hash {
            None => Self::New,
            Some(&previous) => {
                if current_hash == previous {
                    Self::Same
                } else {
                    Self::Different
                }
            }
        }
    }

    pub fn evaluate_restore(original_path: &StrictPath, previous_hash: &str) -> Self {
        match original_path.try_sha1() {
            Err(_) => Self::New,
            Ok(current_hash) => {
                if current_hash == previous_hash {
                    Self::Same
                } else {
                    Self::Different
                }
            }
        }
    }
}
