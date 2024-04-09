mod generic;
pub mod heroic;
mod legendary;
mod lutris;

use std::collections::HashMap;

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
    games: HashMap<RootsConfig, HashMap<String, LauncherGame>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LauncherGame {
    install_dir: Option<StrictPath>,
    prefix: Option<StrictPath>,
    platform: Option<Os>,
}

impl Launchers {
    fn get_game(&self, root: &RootsConfig, game: &str) -> Option<&LauncherGame> {
        self.games.get(root).and_then(|root| root.get(game))
    }

    pub fn get_prefix(&self, root: &RootsConfig, game: &str) -> Option<&StrictPath> {
        self.get_game(root, game).and_then(|x| x.prefix.as_ref())
    }

    pub fn get_install_dir_leaf(&self, root: &RootsConfig, game: &str) -> Option<String> {
        self.get_game(root, game)
            .and_then(|x| x.install_dir.as_ref())
            .and_then(|x| x.leaf())
    }

    pub fn get_install_dir(&self, root: &RootsConfig, game: &str) -> Option<&StrictPath> {
        self.get_game(root, game).and_then(|x| x.install_dir.as_ref())
    }

    pub fn get_platform(&self, root: &RootsConfig, game: &str) -> Option<Os> {
        self.get_game(root, game).and_then(|x| x.platform)
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
            let found = match root.store {
                Store::Heroic => heroic::scan(root, title_finder, legendary.as_ref()),
                Store::Legendary => legendary::scan(root, title_finder),
                Store::Lutris => lutris::scan(root, title_finder),
                _ => generic::scan(root, manifest, subjects),
            };
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
