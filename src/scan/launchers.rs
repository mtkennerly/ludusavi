mod generic;
pub mod heroic;
mod legendary;
mod lutris;

use std::collections::{HashMap, HashSet};

use crate::{
    prelude::StrictPath,
    resource::{
        config::RootsConfig,
        manifest::{Manifest, Os, Store},
    },
    scan::TitleFinder,
};

#[derive(Clone, Default, Debug)]
pub struct Launchers {
    games: HashMap<RootsConfig, HashMap<String, HashSet<LauncherGame>>>,
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
}

impl Launchers {
    pub fn get_game(&self, root: &RootsConfig, game: &str) -> impl Iterator<Item = &LauncherGame> {
        self.games
            .get(root)
            .and_then(|root| root.get(game))
            .unwrap_or(&self.empty)
            .iter()
    }

    pub fn scan(
        roots: &[RootsConfig],
        manifest: &Manifest,
        subjects: &[String],
        title_finder: &TitleFinder,
        legendary: Option<StrictPath>,
    ) -> Self {
        let mut instance = Self::default();

        for root in roots {
            log::debug!("Scanning launcher info: {:?} - {}", root.store, root.path.render());
            let mut found = match root.store {
                Store::Heroic => heroic::scan(root, title_finder, legendary.as_ref()),
                Store::Legendary => legendary::scan(root, title_finder),
                Store::Lutris => lutris::scan(root, title_finder),
                _ => generic::scan(root, manifest, subjects),
            };
            found.retain(|_k, v| {
                v.retain(|x| !x.is_empty());
                !v.is_empty()
            });
            log::debug!(
                "launcher games found ({:?} - {}): {:#?}",
                root.store,
                root.path.raw(),
                &found
            );
            if !found.is_empty() {
                instance.games.entry(root.clone()).or_default().extend(found);
            }
        }

        instance
    }

    #[cfg(test)]
    pub fn scan_dirs(roots: &[RootsConfig], manifest: &Manifest, subjects: &[String]) -> Self {
        Self::scan(roots, manifest, subjects, &TitleFinder::default(), None)
    }
}
