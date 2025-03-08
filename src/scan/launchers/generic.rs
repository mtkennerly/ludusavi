use std::collections::{HashMap, HashSet};

use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;

use crate::prelude::INVALID_FILE_CHARS;

use crate::{
    resource::{config::Root, manifest::Manifest},
    scan::launchers::LauncherGame,
};

fn make_fuzzy_matcher() -> fuzzy_matcher::skim::SkimMatcherV2 {
    fuzzy_matcher::skim::SkimMatcherV2::default()
        .ignore_case()
        .score_config(fuzzy_matcher::skim::SkimScoreConfig {
            penalty_case_mismatch: 0,
            ..Default::default()
        })
}

fn fuzzy_match(
    matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    reference: &str,
    candidate: &str,
    ideal: Option<i64>,
) -> Option<i64> {
    if reference == candidate {
        return Some(i64::MAX);
    }

    // A space-consolidating regex would be better, but is too much of a performance hit.
    // Also, this is used for files/folders, so we can always ignore illegal characters.
    let candidate = candidate
        .replace(['_', '-'], " ")
        .replace(INVALID_FILE_CHARS, " ")
        .replace("    ", " ")
        .replace("   ", " ")
        .replace("  ", " ");

    let actual = matcher.fuzzy_match(reference, &candidate);
    if let (Some(ideal), Some(actual)) = (ideal, actual) {
        if actual == ideal {
            return Some(i64::MAX);
        } else if actual > (ideal / 4 * 3) {
            return Some(actual);
        }
    }
    None
}

pub fn scan(root: &Root, manifest: &Manifest, subjects: &[String]) -> HashMap<String, HashSet<LauncherGame>> {
    log::debug!("ranking installations for root: {:?}", &root);

    let install_parent = root.games_path();
    let matcher = make_fuzzy_matcher();

    let actual_dirs: Vec<_> = install_parent
        .read_dir()
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| match entry.file_type() {
                    Ok(ft) if ft.is_dir() => Some(entry.file_name().to_string_lossy().to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    log::debug!("actual install folders: {}", actual_dirs.join(" | "));

    let scores: Vec<_> = subjects
        .into_par_iter()
        .filter_map(|name| {
            let expected_install_dirs = manifest.0[name].install_dir.keys().chain(std::iter::once(name));

            let mut best: Option<(i64, &String)> = None;
            'dirs: for expected_dir in expected_install_dirs {
                log::trace!("[{name}] looking for install dir: {expected_dir}");

                if expected_dir.contains(['/', '\\']) {
                    if root.path().joined(expected_dir).is_dir() {
                        log::trace!("[{name}] using exact nested install dir");
                        best = Some((i64::MAX, expected_dir));
                        break;
                    } else {
                        continue;
                    }
                }

                let ideal = matcher.fuzzy_match(expected_dir, expected_dir);
                for actual_dir in &actual_dirs {
                    let score = fuzzy_match(&matcher, expected_dir, actual_dir, ideal);
                    if let Some(score) = score {
                        if let Some((previous, _)) = best {
                            if score > previous {
                                log::trace!("[{name}] score {score} beats previous {previous}: {actual_dir}");
                                best = Some((score, actual_dir));
                            }
                        } else {
                            log::trace!("[{name}] new score {score}: {actual_dir}");
                            best = Some((score, actual_dir));
                        }
                    } else {
                        // irrelevant
                    }
                    if score == Some(i64::MAX) {
                        break 'dirs;
                    }
                }
            }
            best.map(|(score, subdir)| {
                log::debug!("[{name}] selecting subdir with score {score}: {subdir}");
                (score, name, subdir)
            })
        })
        .collect();

    let mut by_title = HashMap::<String, (i64, String)>::new();
    for (score, name, subdir) in &scores {
        by_title
            .entry(name.to_string())
            .and_modify(|(stored_score, stored_subdir)| {
                if score > stored_score {
                    *stored_score = *score;
                    *stored_subdir = subdir.to_string();
                }
            })
            .or_insert((*score, subdir.to_string()));
    }

    let mut by_subdir = HashMap::<String, Vec<String>>::new();
    for (_score, name, subdir) in &scores {
        by_subdir
            .entry(subdir.to_string())
            .and_modify(|names| {
                names.push(name.to_string());
            })
            .or_insert(vec![name.to_string()]);
    }

    subjects
        .iter()
        .filter_map(|name| {
            let (score, subdir) = by_title.get(name)?;

            if *score < i64::MAX {
                if let Some(competitors) = by_subdir.get(subdir) {
                    for competitor in competitors {
                        if let Some((competitor_score, _)) = by_title.get(competitor) {
                            if competitor_score > score {
                                log::debug!("[{name}] outranked by '{competitor}' for subdir '{subdir}'");
                                return None;
                            }
                        }
                    }
                }
            }

            Some((
                name.clone(),
                HashSet::from_iter([LauncherGame {
                    install_dir: Some(install_parent.joined(subdir)),
                    prefix: None,
                    platform: None,
                }]),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn fuzzy_matching() {
        let matcher = make_fuzzy_matcher();

        for (reference, candidate, output) in vec![
            ("a", "a", Some(i64::MAX)),
            ("a", "b", None),
            ("Something", "Something", Some(i64::MAX)),
            // Too short:
            ("ab", "a", None),
            ("ab", "b", None),
            ("abc", "ab", None),
            // Long enough:
            ("abcd", "abc", Some(71)),
            ("A Fun Game", "a fun game", Some(i64::MAX)),
            ("A Fun Game", "a  fun  game", Some(i64::MAX)),
            ("A Fun Game", "AFunGame", Some(171)),
            ("A Fun Game", "A_Fun_Game", Some(i64::MAX)),
            ("A Fun Game", "A _ Fun _ Game", Some(i64::MAX)),
            ("A Fun Game", "a-fun-game", Some(i64::MAX)),
            ("A Fun Game", "a - fun - game", Some(i64::MAX)),
            ("A Fun Game", "A FUN GAME", Some(i64::MAX)),
            ("A Fun Game!", "A Fun Game", Some(219)),
            ("A Funner Game", "A Fun Game", Some(209)),
            ("A Fun Game 2", "A Fun Game", Some(219)),
        ] {
            assert_eq!(
                output,
                fuzzy_match(
                    &matcher,
                    reference,
                    candidate,
                    matcher.fuzzy_match(reference, reference)
                )
            );
        }
    }
}
