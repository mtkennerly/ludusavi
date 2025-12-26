mod generic;
pub mod heroic;
mod legendary;
mod lutris;

use std::collections::{HashMap, HashSet};

use crate::{
    prelude::StrictPath,
    resource::{
        config::Root,
        manifest::{Manifest, Os},
    },
    scan::TitleFinder,
};

#[derive(Clone, Default, Debug)]
pub struct Launchers {
    games: HashMap<Root, HashMap<String, HashSet<LauncherGame>>>,
    empty: HashSet<LauncherGame>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LauncherGame {
    pub install_dir: Option<StrictPath>,
    pub prefix: Option<StrictPath>,
    pub platform: Option<Os>,
}

impl LauncherGame {
    pub fn is_empty(&self) -> bool {
        self.install_dir.is_none() && self.prefix.is_none() && self.platform.is_none()
    }

    pub fn replace_in_paths(&self, old: &StrictPath, new: &StrictPath) -> Self {
        Self {
            install_dir: self.install_dir.as_ref().map(|x| x.replace(old, new)),
            prefix: self.prefix.as_ref().map(|x| x.replace(old, new)),
            platform: self.platform,
        }
    }
}

impl Launchers {
    pub fn get_game(&self, root: &Root, game: &str) -> impl Iterator<Item = &LauncherGame> {
        self.games
            .get(root)
            .and_then(|root| root.get(game))
            .unwrap_or(&self.empty)
            .iter()
    }

    pub fn scan(
        roots: &[Root],
        manifest: &Manifest,
        subjects: &[String],
        title_finder: &TitleFinder,
        legendary: Option<StrictPath>,
    ) -> Self {
        let mut instance = Self::default();

        for root in roots {
            log::debug!("Scanning launcher info: {:?}", &root);
            let mut found = match root {
                Root::Heroic(root) => heroic::scan(root, title_finder, legendary.as_ref()),
                Root::Legendary(root) => legendary::scan(root, title_finder),
                Root::Lutris(root) => lutris::scan(root, title_finder),
                _ => generic::scan(root, manifest, subjects),
            };
            found.retain(|_k, v| {
                v.retain(|x| !x.is_empty());
                !v.is_empty()
            });
            log::debug!("launcher games found ({:?}): {:#?}", &root, &found);
            if !found.is_empty() {
                instance.games.entry(root.clone()).or_default().extend(found);
            }
        }

        instance
    }

    #[cfg(test)]
    pub fn scan_dirs(roots: &[Root], manifest: &Manifest, subjects: &[String]) -> Self {
        Self::scan(roots, manifest, subjects, &TitleFinder::default(), None)
    }
}
