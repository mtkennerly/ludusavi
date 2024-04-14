use std::collections::{BTreeSet, HashMap};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::resource::{config::Config, manifest::Manifest};

/// This covers any edition that is clearly separated by punctuation.
static RE_EDITION_PUNCTUATED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[™®©:-] .+ edition$"#).unwrap());
/// This covers specific, known editions that are not separated by punctuation.
static RE_EDITION_KNOWN: Lazy<Regex> = Lazy::new(|| Regex::new(r#" (game of the year) edition$"#).unwrap());
/// This covers any single-word editions that are not separated by punctuation.
/// We can't assume more than one word because it may be part of the main title.
static RE_EDITION_SHORT: Lazy<Regex> = Lazy::new(|| Regex::new(r#" [^ ]+ edition$"#).unwrap());
static RE_YEAR_SUFFIX: Lazy<Regex> = Lazy::new(|| Regex::new(r" \(\d+\)$").unwrap());
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

#[derive(Clone, Debug, Default)]
struct TitleGameInfo {
    backup: TitleGameOperationInfo,
    restore: TitleGameOperationInfo,
}

#[derive(Clone, Debug, Default)]
struct TitleGameOperationInfo {
    known: bool,
    enabled: bool,
    complete: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TitleFinder {
    games: HashMap<String, TitleGameInfo>,
    steam_ids: HashMap<u32, String>,
    gog_ids: HashMap<u64, String>,
    normalized: HashMap<String, String>,
    aliases: HashMap<String, String>,
}

impl TitleFinder {
    pub fn new(config: &Config, manifest: &Manifest, restorables: BTreeSet<String>) -> Self {
        let mut games: HashMap<String, TitleGameInfo> = HashMap::new();
        for name in manifest.0.keys() {
            let info = games.entry(name.clone()).or_default();
            info.backup = TitleGameOperationInfo {
                known: true,
                enabled: config.is_game_enabled_for_backup(name),
                complete: !config.any_saves_ignored(name, false),
            };
        }
        for name in restorables {
            let info = games.entry(name.clone()).or_default();
            info.restore = TitleGameOperationInfo {
                known: true,
                enabled: config.is_game_enabled_for_restore(&name),
                complete: !config.any_saves_ignored(&name, true),
            };
        }

        let steam_ids = manifest.map_steam_ids_to_names();
        let gog_ids = manifest.map_gog_ids_to_names();
        let normalized: HashMap<_, _> = games
            .keys()
            .map(|title| (normalize_title(title), title.to_owned()))
            .collect();
        let aliases = manifest.aliases();

        Self {
            games,
            steam_ids,
            gog_ids,
            normalized,
            aliases,
        }
    }

    fn eligible(&self, game: &str, backup: bool, restore: bool) -> bool {
        let (can_backup, can_restore) = self
            .games
            .get(game)
            .map(|x| (x.backup.known, x.restore.known))
            .unwrap_or_default();

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

    pub fn find_one(&self, query: TitleQuery) -> Option<String> {
        self.find(query).into_iter().next()
    }

    pub fn find_one_by_name(&self, name: &str) -> Option<String> {
        self.find_one(TitleQuery {
            names: vec![name.to_string()],
            ..Default::default()
        })
    }

    pub fn find_one_by_normalized_name(&self, name: &str) -> Option<String> {
        self.find_one(TitleQuery {
            names: vec![name.to_string()],
            normalized: true,
            ..Default::default()
        })
    }

    /// Look up games based on certain criteria.
    /// Returns a set of matching game names.
    ///
    /// Only returns one result when querying for exact titles or store IDs.
    /// Precedence: Steam ID -> GOG ID -> exact title -> normalized title.
    ///
    /// Otherwise, returns all results that match the query.
    pub fn find(&self, query: TitleQuery) -> BTreeSet<String> {
        let TitleQuery {
            names,
            steam_id,
            gog_id,
            normalized,
            backup,
            restore,
            disabled,
            partial,
        } = query;

        let mut output = BTreeSet::new();
        let singular = !names.is_empty() || steam_id.is_some() || gog_id.is_some();

        'outer: {
            if singular {
                if let Some(steam_id) = steam_id {
                    if let Some(found) = self.steam_ids.get(&steam_id) {
                        if self.eligible(found, backup, restore) {
                            output.insert(found.to_owned());
                            break 'outer;
                        }
                    }
                }

                if let Some(gog_id) = gog_id {
                    if let Some(found) = self.gog_ids.get(&gog_id) {
                        if self.eligible(found, backup, restore) {
                            output.insert(found.to_owned());
                            break 'outer;
                        }
                    }
                }

                for name in &names {
                    if self.games.contains_key(name) && self.eligible(name, backup, restore) {
                        output.insert(name.to_owned());
                        break 'outer;
                    }
                }

                if normalized {
                    for name in &names {
                        if let Some(found) = self.normalized.get(&normalize_title(name)) {
                            if self.eligible(found, backup, restore) {
                                output.insert((*found).to_owned());
                                break 'outer;
                            }
                        }
                    }
                }
            } else {
                for (game, info) in &self.games {
                    if (backup && !info.backup.known) || (restore && !info.restore.known) {
                        continue;
                    }

                    if disabled {
                        let skip = match (backup, restore) {
                            (true, true) => info.backup.enabled || info.restore.enabled,
                            (true, false) => info.backup.enabled,
                            (false, true) => info.restore.enabled,
                            (false, false) => info.backup.enabled && info.restore.enabled,
                        };
                        if skip {
                            continue;
                        }
                    }

                    if partial {
                        let skip = match (backup, restore) {
                            (true, true) => info.backup.complete || info.restore.complete,
                            (true, false) => info.backup.complete,
                            (false, true) => info.restore.complete,
                            (false, false) => info.backup.complete && info.restore.complete,
                        };
                        if skip {
                            continue;
                        }
                    }

                    output.insert(game.to_owned());
                }
            }
        }

        // Resolve aliases to primary name.
        for name in output.clone() {
            if let Some(aliased) = self.aliases.get(&name) {
                output.remove(&name);
                output.insert(aliased.to_string());
            }
        }

        output
    }
}

#[derive(Clone, Debug, Default)]
pub struct TitleQuery {
    /// Search for exact titles or aliases.
    /// This will cause only one result to be returned.
    pub names: Vec<String>,
    // Search for a Steam ID.
    /// This will cause only one result to be returned.
    pub steam_id: Option<u32>,
    /// Search for a GOG ID.
    /// This will cause only one result to be returned.
    pub gog_id: Option<u64>,
    /// Search by normalizing the `names`.
    pub normalized: bool,
    /// Only return games that are possible to back up.
    pub backup: bool,
    /// Only return games that are possible to restore.
    pub restore: bool,
    /// Only return games that are disabled for processing.
    pub disabled: bool,
    /// Only return games that have some saves deselected.
    pub partial: bool,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::btree_set;

    use crate::resource::ResourceFile;

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

    #[test]
    fn can_find_one_title() {
        let manifest = Manifest::load_from_string(
            r#"
            by-name: {}
            by-name-alias:
                alias: by-name
            by-steam:
                steam:
                    id: 1
            by-gog:
                gog:
                    id: 2
            "#,
        )
        .unwrap();

        let finder = TitleFinder::new(&Default::default(), &manifest, Default::default());

        assert_eq!(
            btree_set!["by-name".to_string()],
            finder.find(TitleQuery {
                names: vec!["by-name".to_string()],
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set!["by-name".to_string()],
            finder.find(TitleQuery {
                names: vec!["by-name-alias".to_string()],
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set!["by-steam".to_string()],
            finder.find(TitleQuery {
                steam_id: Some(1),
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set!["by-gog".to_string()],
            finder.find(TitleQuery {
                gog_id: Some(2),
                ..Default::default()
            }),
        );
    }

    #[test]
    fn can_find_multiple_titles() {
        let config = Config::load_from_string(
            r#"
            manifest:
                url: foo
            roots: []
            backup:
                path: /backup
                ignoredGames:
                    - backup-disabled
                toggledPaths:
                    backup-partial:
                        /foo: false
            restore:
                path: /backup
                ignoredGames:
                    - restore-disabled
                toggledPaths:
                    restore-partial:
                        /foo: false
            "#,
        )
        .unwrap();

        let manifest = Manifest::load_from_string(
            r#"
            both: {}
            backup: {}
            backup-disabled: {}
            backup-partial: {}
            "#,
        )
        .unwrap();

        let restorables = btree_set![
            "both".to_string(),
            "restore".to_string(),
            "restore-disabled".to_string(),
            "restore-partial".to_string()
        ];

        let finder = TitleFinder::new(&config, &manifest, restorables);

        assert_eq!(
            btree_set![
                "both".to_string(),
                "backup".to_string(),
                "backup-disabled".to_string(),
                "backup-partial".to_string(),
                "restore".to_string(),
                "restore-disabled".to_string(),
                "restore-partial".to_string()
            ],
            finder.find(TitleQuery::default()),
        );

        assert_eq!(
            btree_set![
                "both".to_string(),
                "backup".to_string(),
                "backup-disabled".to_string(),
                "backup-partial".to_string()
            ],
            finder.find(TitleQuery {
                backup: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set![
                "both".to_string(),
                "restore".to_string(),
                "restore-disabled".to_string(),
                "restore-partial".to_string()
            ],
            finder.find(TitleQuery {
                restore: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set!["both".to_string()],
            finder.find(TitleQuery {
                backup: true,
                restore: true,
                ..Default::default()
            }),
        );

        assert_eq!(
            btree_set![
                "backup".to_string(),
                "backup-disabled".to_string(),
                "backup-partial".to_string(),
                "restore".to_string(),
                "restore-disabled".to_string(),
                "restore-partial".to_string()
            ],
            finder.find(TitleQuery {
                disabled: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set!["backup-disabled".to_string()],
            finder.find(TitleQuery {
                backup: true,
                disabled: true,
                ..Default::default()
            }),
        );

        assert_eq!(
            btree_set![
                "backup".to_string(),
                "backup-disabled".to_string(),
                "backup-partial".to_string(),
                "restore".to_string(),
                "restore-disabled".to_string(),
                "restore-partial".to_string()
            ],
            finder.find(TitleQuery {
                partial: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_set!["restore-partial".to_string()],
            finder.find(TitleQuery {
                restore: true,
                partial: true,
                ..Default::default()
            }),
        );
    }
}
