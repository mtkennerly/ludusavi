use std::collections::HashMap;

use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;

use crate::{
    prelude::INVALID_FILE_CHARS,
    resource::{
        config::RootsConfig,
        manifest::{Manifest, Store},
    },
};

fn make_fuzzy_matcher() -> fuzzy_matcher::skim::SkimMatcherV2 {
    fuzzy_matcher::skim::SkimMatcherV2::default()
        .ignore_case()
        .score_config(fuzzy_matcher::skim::SkimScoreConfig {
            penalty_case_mismatch: 0,
            ..Default::default()
        })
}

pub fn fuzzy_match(
    matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    reference: &str,
    candidate: &str,
    ideal: &Option<i64>,
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
        if actual == *ideal {
            return Some(i64::MAX);
        } else if actual > (ideal / 4 * 3) {
            return Some(actual);
        }
    }
    None
}

#[derive(Clone, Debug, Default)]
pub struct InstallDirRanking(HashMap<(RootsConfig, String), (i64, String)>);

impl InstallDirRanking {
    /// Get the installation directory for some root/game combination.
    pub fn get(&self, root: &RootsConfig, name: &str) -> Option<String> {
        self.0.get(&(root.to_owned(), name.to_owned())).and_then(|candidate| {
            if candidate.0 == i64::MAX {
                return Some(candidate.1.to_owned());
            }
            for ((other_root, other_game), (other_score, other_subdir)) in &self.0 {
                if other_root == root && other_subdir == &candidate.1 && other_score > &candidate.0 {
                    log::info!("[{name}] outranked by '{other_game}' for subdir '{other_subdir}'");
                    return None;
                }
            }
            Some(candidate.1.to_owned())
        })
    }

    pub fn scan(roots: &[RootsConfig], manifest: &Manifest, subjects: &[String]) -> Self {
        let mut ranking = Self::default();
        for root in roots {
            if root.store == Store::Heroic {
                // We handle this separately in the Heroic scan.
                continue;
            }
            ranking.scan_root(root, manifest, subjects);
        }
        ranking
    }

    fn scan_root(&mut self, root: &RootsConfig, manifest: &Manifest, subjects: &[String]) {
        log::debug!("ranking installations for {:?}: {}", root.store, root.path.raw());

        let install_parent = match root.store {
            Store::Steam => root.path.joined("steamapps/common"),
            _ => root.path.clone(),
        };
        let matcher = make_fuzzy_matcher();

        let actual_dirs: Vec<_> = std::fs::read_dir(install_parent.interpret())
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

        let scores: Vec<_> = subjects
            .into_par_iter()
            .filter_map(|name| {
                let manifest_install_dirs: Vec<_> = manifest.0[name]
                    .install_dir
                    .as_ref()
                    .map(|x| x.keys().collect())
                    .unwrap_or_default();
                let default_install_dir = name.to_string();
                let expected_install_dirs = &[manifest_install_dirs, vec![&default_install_dir]].concat();

                let mut best: Option<(i64, &String)> = None;
                'dirs: for expected_dir in expected_install_dirs {
                    log::trace!("[{name}] looking for install dir: {expected_dir}");
                    let ideal = matcher.fuzzy_match(expected_dir, expected_dir);
                    for actual_dir in &actual_dirs {
                        let score = fuzzy_match(&matcher, expected_dir, actual_dir, &ideal);
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
                            log::trace!("[{name}] irrelevant: {actual_dir}");
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

        for (score, name, subdir) in scores {
            self.0
                .insert((root.clone(), name.to_owned()), (score, subdir.to_owned()));
        }
    }
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
                    &matcher.fuzzy_match(reference, reference)
                )
            );
        }
    }
}
