use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::LazyLock,
};

use itertools::Itertools;
use regex::Regex;

use crate::{
    resource::{config::Config, manifest::Manifest},
    scan::ScanKind,
};

/// This covers any edition that is clearly separated by punctuation.
static RE_EDITION_PUNCTUATED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[™®©:-] .+ edition$"#).unwrap());
/// This covers specific, known editions that are not separated by punctuation.
static RE_EDITION_KNOWN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#" (game of the year) edition$"#).unwrap());
/// This covers any single-word editions that are not separated by punctuation.
/// We can't assume more than one word because it may be part of the main title.
static RE_EDITION_SHORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#" [^ ]+ edition$"#).unwrap());
static RE_YEAR_SUFFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" \(\d+\)$").unwrap());
static RE_SYMBOLS_GAP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[™®©:-]"#).unwrap());
static RE_SYMBOLS_NO_GAP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"['"‘’“”]"#).unwrap());
static RE_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#" {2,}"#).unwrap());

pub fn normalize_title(title: &str) -> String {
    let normalized = title.to_lowercase();
    let normalized = RE_YEAR_SUFFIX.replace_all(&normalized, "");
    let normalized = RE_EDITION_PUNCTUATED.replace_all(&normalized, "");
    let normalized = RE_EDITION_KNOWN.replace_all(&normalized, "");
    let normalized = RE_EDITION_SHORT.replace_all(&normalized, "");
    let normalized = RE_SYMBOLS_GAP.replace_all(&normalized, " ");
    let normalized = RE_SYMBOLS_NO_GAP.replace_all(&normalized, "");
    let normalized = RE_SPACES.replace_all(&normalized, " ");
    normalized.trim().to_string()
}

#[derive(Clone, Debug, Default)]
struct TitleGameInfo {
    backup: TitleGameOperationInfo,
    restore: TitleGameOperationInfo,
}

#[derive(Clone, Debug, Default)]
struct NormalizedTitleGameInfo {
    canonical: String,
    score: Option<f64>,
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
    lutris_ids: HashMap<String, String>,
    normalized: HashMap<String, NormalizedTitleGameInfo>,
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
                complete: !config.any_saves_ignored(name, ScanKind::Backup),
            };
        }
        for name in restorables {
            let info = games.entry(name.clone()).or_default();
            info.restore = TitleGameOperationInfo {
                known: true,
                enabled: config.is_game_enabled_for_restore(&name),
                complete: !config.any_saves_ignored(&name, ScanKind::Restore),
            };
        }

        let steam_ids = manifest.map_steam_ids_to_names();
        let gog_ids = manifest.map_gog_ids_to_names();
        let lutris_ids = manifest.map_lutris_ids_to_names();
        let aliases = manifest.aliases();

        let mut normalized: HashMap<String, NormalizedTitleGameInfo> = HashMap::new();
        for title in games.keys() {
            let norm = normalize_title(title);
            let entry = normalized.entry(norm.clone()).or_default();
            let new_score = strsim::jaro_winkler(title, &norm);
            match entry.score {
                Some(old_score) => {
                    if new_score > old_score {
                        entry.canonical = title.to_owned();
                        entry.score = Some(new_score);
                    }
                }
                None => {
                    entry.canonical = title.to_owned();
                    entry.score = Some(new_score);
                }
            }
        }

        Self {
            games,
            steam_ids,
            gog_ids,
            lutris_ids,
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
        self.find(query)
            .into_iter()
            .sorted_by(compare_ranked_titles)
            .map(|(name, _info)| name)
            .next()
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
    pub fn find(&self, query: TitleQuery) -> BTreeMap<String, TitleMatch> {
        let TitleQuery {
            multiple,
            names,
            steam_id,
            gog_id,
            lutris_id,
            normalized,
            fuzzy,
            backup,
            restore,
            disabled,
            partial,
        } = query;

        let mut output: BTreeMap<String, TitleMatch> = BTreeMap::new();
        let mut update = |title: String, info: TitleMatch| {
            output
                .entry(title)
                .and_modify(|entry| {
                    if entry.score.is_none() || entry.score.is_some_and(|old| info.score.is_some_and(|new| new > old)) {
                        entry.score = info.score;
                    }
                })
                .or_insert(info);
        };

        let singular = !names.is_empty() || steam_id.is_some() || gog_id.is_some() || lutris_id.is_some();

        'outer: {
            if singular {
                if let Some(steam_id) = steam_id {
                    if let Some(found) = self.steam_ids.get(&steam_id) {
                        if self.eligible(found, backup, restore) {
                            update(found.to_owned(), TitleMatch::perfect());
                            if !multiple {
                                break 'outer;
                            }
                        }
                    }
                }

                if let Some(gog_id) = gog_id {
                    if let Some(found) = self.gog_ids.get(&gog_id) {
                        if self.eligible(found, backup, restore) {
                            update(found.to_owned(), TitleMatch::perfect());
                            if !multiple {
                                break 'outer;
                            }
                        }
                    }
                }

                if let Some(lutris_id) = lutris_id {
                    if let Some(found) = self.lutris_ids.get(&lutris_id) {
                        if self.eligible(found, backup, restore) {
                            update(found.to_owned(), TitleMatch::perfect());
                            if !multiple {
                                break 'outer;
                            }
                        }
                    }
                }

                for name in &names {
                    if self.games.contains_key(name) && self.eligible(name, backup, restore) {
                        update(name.to_owned(), TitleMatch::perfect());
                        if !multiple {
                            break 'outer;
                        }
                    }
                }

                if normalized {
                    for name in &names {
                        if let Some(found) = self.normalized.get(&normalize_title(name)) {
                            if self.eligible(&found.canonical, backup, restore) {
                                update(found.canonical.to_owned(), TitleMatch { score: found.score });
                                if !multiple {
                                    break 'outer;
                                }
                            }
                        }
                    }
                }

                if fuzzy {
                    let mut matches = BTreeMap::new();

                    for name in &names {
                        for known in self.games.keys() {
                            let score = if normalized {
                                strsim::jaro_winkler(&normalize_title(known), &normalize_title(name))
                            } else {
                                strsim::jaro_winkler(known, name)
                            };

                            if score < 0.75 {
                                continue;
                            }

                            if self.eligible(known, backup, restore) {
                                matches.insert(known.to_string(), score);
                            }
                        }
                    }

                    let sorted: Vec<_> = matches
                        .into_iter()
                        .map(|(name, score)| (name, TitleMatch { score: Some(score) }))
                        .sorted_by(compare_ranked_titles)
                        .collect();

                    if multiple {
                        for (name, info) in sorted {
                            update(name, info);
                        }
                    } else if let Some((name, info)) = sorted.first() {
                        update(name.clone(), info.clone());
                        break 'outer;
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

                    output.insert(game.to_owned(), TitleMatch::default());
                }
            }
        }

        // Resolve aliases to primary name.
        output = output
            .into_iter()
            .map(|(name, info)| match self.aliases.get(&name) {
                Some(aliased) => (aliased.to_string(), info),
                None => (name, info),
            })
            .collect();

        output
    }
}

#[derive(Clone, Debug, Default)]
pub struct TitleQuery {
    /// Keep looking for all potential matches,
    /// instead of stopping at the first match.
    pub multiple: bool,
    /// Search for exact titles or aliases.
    /// This will cause only one result to be returned.
    pub names: Vec<String>,
    /// Search for a Steam ID.
    /// This will cause only one result to be returned.
    pub steam_id: Option<u32>,
    /// Search for a GOG ID.
    /// This will cause only one result to be returned.
    pub gog_id: Option<u64>,
    /// Search for a Lutris slug.
    /// This will cause only one result to be returned.
    pub lutris_id: Option<String>,
    /// Search by normalizing the `names`.
    pub normalized: bool,
    /// Search with fuzzy matching.
    pub fuzzy: bool,
    /// Only return games that are possible to back up.
    pub backup: bool,
    /// Only return games that are possible to restore.
    pub restore: bool,
    /// Only return games that are disabled for processing.
    pub disabled: bool,
    /// Only return games that have some saves deselected.
    pub partial: bool,
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TitleMatch {
    /// How well the title matches the query.
    /// Range: 0.0 to 1.0 (higher is better).
    pub score: Option<f64>,
}

impl TitleMatch {
    pub fn perfect() -> Self {
        Self { score: Some(1.0) }
    }
}

pub fn compare_ranked_titles(x: &(String, TitleMatch), y: &(String, TitleMatch)) -> std::cmp::Ordering {
    compare_ranked_titles_ref(&(&x.0, &x.1), &(&y.0, &y.1))
}

pub fn compare_ranked_titles_ref(x: &(&String, &TitleMatch), y: &(&String, &TitleMatch)) -> std::cmp::Ordering {
    y.1.score
        .partial_cmp(&x.1.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| x.0.to_lowercase().cmp(&y.0.to_lowercase()))
        .then_with(|| x.0.cmp(y.0))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, btree_set};

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
        assert_eq!("foo bar", normalize_title("Fo'o Bar"));
        assert_eq!("foo bar", normalize_title("Foo \"Bar\""));

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
            by-steam-extra:
                id:
                    steamExtra: [3]
            by-gog-extra:
                id:
                    gogExtra: [4]
            by-lutris:
                id:
                    lutris: slug
            "#,
        )
        .unwrap();

        let finder = TitleFinder::new(&Default::default(), &manifest, Default::default());

        assert_eq!(
            btree_map! { "by-name".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                names: vec!["by-name".to_string()],
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-name".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                names: vec!["by-name-alias".to_string()],
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-name".to_string(): TitleMatch { score: Some(0.9428571428571428) } },
            finder.find(TitleQuery {
                names: vec!["By Na".to_string()],
                normalized: true,
                fuzzy: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-steam".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                steam_id: Some(1),
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-gog".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                gog_id: Some(2),
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-steam-extra".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                steam_id: Some(3),
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-gog-extra".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                gog_id: Some(4),
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"by-lutris".to_string(): TitleMatch::perfect() },
            finder.find(TitleQuery {
                lutris_id: Some("slug".to_string()),
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
            btree_map! {
                "both".to_string(): TitleMatch::default(),
                "backup".to_string(): TitleMatch::default(),
                "backup-disabled".to_string(): TitleMatch::default(),
                "backup-partial".to_string(): TitleMatch::default(),
                "restore".to_string(): TitleMatch::default(),
                "restore-disabled".to_string(): TitleMatch::default(),
                "restore-partial".to_string(): TitleMatch::default(),
            },
            finder.find(TitleQuery::default()),
        );

        assert_eq!(
            btree_map! {
                "both".to_string(): TitleMatch::default(),
                "backup".to_string(): TitleMatch::default(),
                "backup-disabled".to_string(): TitleMatch::default(),
                "backup-partial".to_string(): TitleMatch::default(),
            },
            finder.find(TitleQuery {
                backup: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {
                "both".to_string(): TitleMatch::default(),
                "restore".to_string(): TitleMatch::default(),
                "restore-disabled".to_string(): TitleMatch::default(),
                "restore-partial".to_string(): TitleMatch::default(),
            },
            finder.find(TitleQuery {
                restore: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"both".to_string(): TitleMatch::default() },
            finder.find(TitleQuery {
                backup: true,
                restore: true,
                ..Default::default()
            }),
        );

        assert_eq!(
            btree_map! {
                "backup".to_string(): TitleMatch::default(),
                "backup-disabled".to_string(): TitleMatch::default(),
                "backup-partial".to_string(): TitleMatch::default(),
                "restore".to_string(): TitleMatch::default(),
                "restore-disabled".to_string(): TitleMatch::default(),
                "restore-partial".to_string(): TitleMatch::default(),
            },
            finder.find(TitleQuery {
                disabled: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"backup-disabled".to_string(): TitleMatch::default() },
            finder.find(TitleQuery {
                backup: true,
                disabled: true,
                ..Default::default()
            }),
        );

        assert_eq!(
            btree_map! {
                "backup".to_string(): TitleMatch::default(),
                "backup-disabled".to_string(): TitleMatch::default(),
                "backup-partial".to_string(): TitleMatch::default(),
                "restore".to_string(): TitleMatch::default(),
                "restore-disabled".to_string(): TitleMatch::default(),
                "restore-partial".to_string(): TitleMatch::default(),
            },
            finder.find(TitleQuery {
                partial: true,
                ..Default::default()
            }),
        );
        assert_eq!(
            btree_map! {"restore-partial".to_string(): TitleMatch::default() },
            finder.find(TitleQuery {
                restore: true,
                partial: true,
                ..Default::default()
            }),
        );

        assert_eq!(
            btree_map! {
                "backup".to_string(): TitleMatch { score: Some(0.8888888888888888) },
                "backup-disabled".to_string(): TitleMatch { score: Some(0.7555555555555555) },
                "backup-partial".to_string(): TitleMatch { score: Some(0.7619047619047619) },
            },
            finder.find(TitleQuery {
                names: vec!["acku".to_string()],
                multiple: true,
                normalized: true,
                fuzzy: true,
                ..Default::default()
            }),
        );
    }
}
