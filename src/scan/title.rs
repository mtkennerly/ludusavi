use std::collections::{BTreeSet, HashMap, HashSet};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{resource::manifest::Manifest, scan::layout::BackupLayout};

/// This covers any edition that is clearly separated by punctuation.
static RE_EDITION_PUNCTUATED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[™®©:-] .+ edition$"#).unwrap());
/// This covers specific, known editions that are not separated by punctuation.
static RE_EDITION_KNOWN: Lazy<Regex> = Lazy::new(|| Regex::new(r#" (game of the year) edition$"#).unwrap());
/// This covers any single-word editions that are not separated by punctuation.
/// We can't assume more than one word because it may be part of the main title.
static RE_EDITION_SHORT: Lazy<Regex> = Lazy::new(|| Regex::new(r#" [^ ]+ edition$"#).unwrap());
static RE_YEAR_SUFFIX: Lazy<Regex> = Lazy::new(|| Regex::new(r#" \(\d+\)$"#).unwrap());
static RE_SYMBOLS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[™®©:-]"#).unwrap());
static RE_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r#" {2,}"#).unwrap());

pub fn normalize_title(title: &str) -> String {
    let normalized = title.to_lowercase();
    let normalized = RE_YEAR_SUFFIX.replace_all(&normalized, "");
    let normalized = RE_EDITION_PUNCTUATED.replace_all(&normalized, "");
    let normalized = RE_EDITION_KNOWN.replace_all(&normalized, "");
    let normalized = RE_EDITION_SHORT.replace_all(&normalized, "");
    let normalized = RE_SYMBOLS.replace_all(&normalized, " ");
    let normalized = RE_SPACES.replace_all(&normalized, " ");
    normalized.trim().to_string()
}

#[derive(Default)]
pub struct TitleFinder {
    all_games: HashSet<String>,
    can_backup: HashSet<String>,
    can_restore: HashSet<String>,
    steam_ids: HashMap<u32, String>,
    gog_ids: HashMap<u64, String>,
    normalized: HashMap<String, String>,
}

impl TitleFinder {
    pub fn new(manifest: &Manifest, layout: &BackupLayout) -> Self {
        let can_backup: HashSet<_> = manifest.0.keys().cloned().collect();
        let can_restore: HashSet<_> = layout.restorable_games().into_iter().collect();
        let all_games: HashSet<_> = can_backup.union(&can_restore).cloned().collect();
        let steam_ids = manifest.map_steam_ids_to_names();
        let gog_ids = manifest.map_gog_ids_to_names();
        let normalized: HashMap<_, _> = all_games
            .iter()
            .map(|title| (normalize_title(title), title.to_owned()))
            .collect();

        Self {
            all_games,
            can_backup,
            can_restore,
            steam_ids,
            gog_ids,
            normalized,
        }
    }

    fn eligible(&self, game: &str, backup: bool, restore: bool) -> bool {
        let can_backup = self.can_backup.contains(game);
        let can_restore = self.can_restore.contains(game);

        if backup && restore {
            can_backup && can_restore
        } else if backup {
            can_backup
        } else if restore {
            can_restore
        } else {
            true
        }
    }

    pub fn find_one(
        &self,
        names: &[String],
        steam_id: &Option<u32>,
        gog_id: &Option<u64>,
        normalized: bool,
        backup: bool,
        restore: bool,
    ) -> Option<String> {
        let found = self.find(names, steam_id, gog_id, normalized, backup, restore);
        found.iter().next().map(|x| x.to_owned())
    }

    pub fn find(
        &self,
        names: &[String],
        steam_id: &Option<u32>,
        gog_id: &Option<u64>,
        normalized: bool,
        backup: bool,
        restore: bool,
    ) -> BTreeSet<String> {
        let mut output = BTreeSet::new();

        if let Some(steam_id) = steam_id {
            if let Some(found) = self.steam_ids.get(steam_id) {
                if self.eligible(found, backup, restore) {
                    output.insert(found.to_owned());
                    return output;
                }
            }
        }

        if let Some(gog_id) = gog_id {
            if let Some(found) = self.gog_ids.get(gog_id) {
                if self.eligible(found, backup, restore) {
                    output.insert(found.to_owned());
                    return output;
                }
            }
        }

        for name in names {
            if self.all_games.contains(name) && self.eligible(name, backup, restore) {
                output.insert(name.to_owned());
                return output;
            }
        }

        if normalized {
            for name in names {
                if let Some(found) = self.normalized.get(&normalize_title(name)) {
                    if self.eligible(found, backup, restore) {
                        output.insert((*found).to_owned());
                        return output;
                    }
                }
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn can_normalize_title() {
        // capitalization
        assert_eq!("foo bar", normalize_title("foo bar"));
        assert_eq!("foo bar", normalize_title("Foo Bar"));

        // punctuated editions
        assert_eq!("foo bar", normalize_title("Foo Bar: Any Arbitrary Edition"));
        assert_eq!("foo bar", normalize_title("Foo Bar - Any Arbitrary Edition"));
        assert_eq!("foo bar", normalize_title("Foo Bar™ Any Arbitrary Edition"));
        assert_eq!("foo bar", normalize_title("Foo Bar® - Any Arbitrary Edition"));

        // special cased editions
        assert_eq!("foo bar", normalize_title("Foo Bar Game of the Year Edition"));

        // short editions
        assert_eq!("foo bar", normalize_title("Foo Bar Special Edition"));

        // year suffixes
        assert_eq!("foo bar", normalize_title("Foo Bar (2000)"));

        // symbols
        assert_eq!("foo bar", normalize_title("Foo:Bar"));
        assert_eq!("foo bar", normalize_title("Foo: Bar"));

        // spaces
        assert_eq!("foo bar", normalize_title("  Foo  Bar  "));
    }
}
