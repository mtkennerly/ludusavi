use std::collections::{HashMap, HashSet};

use crate::prelude::StrictPath;

use crate::{
    prelude::ENV_DEBUG,
    resource::{config::root, manifest::Os},
    scan::{
        launchers::{heroic::find_prefix, LauncherGame},
        TitleFinder, TitleQuery,
    },
};

pub mod installed {
    pub const PATH: &str = "gog_store/installed.json";

    #[derive(serde::Deserialize)]
    pub struct Data {
        pub installed: Vec<Game>,
    }

    #[derive(serde::Deserialize)]
    pub struct Game {
        /// This is an opaque ID, not the human-readable title.
        #[serde(rename = "appName")]
        pub app_name: String,
        pub platform: String,
        pub install_path: String,
    }
}

pub mod library {
    pub const PATH: &str = "store_cache/gog_library.json";
    pub const PATH_LEGACY: &str = "gog_store/library.json";

    #[derive(serde::Deserialize)]
    pub struct Data {
        pub games: Vec<Game>,
    }

    #[derive(serde::Deserialize)]
    pub struct Game {
        /// This is an opaque ID, not the human-readable title.
        pub app_name: String,
        pub title: String,
    }
}

pub fn scan(root: &root::Heroic, title_finder: &TitleFinder) -> HashMap<String, HashSet<LauncherGame>> {
    let mut games = HashMap::<String, HashSet<LauncherGame>>::new();

    let game_titles: HashMap<String, String> = get_library(root)
        .iter()
        .map(|game| (game.app_name.clone(), game.title.clone()))
        .collect();

    if game_titles.is_empty() {
        return games;
    }

    let installed_path = root.path.joined(installed::PATH);
    let content = installed_path.read();

    match serde_json::from_str::<installed::Data>(&content.unwrap_or_default()) {
        Ok(installed_games) => {
            for game in installed_games.installed {
                let Some(game_title) = game_titles.get(&game.app_name) else {
                    continue;
                };

                let gog_id: Option<u64> = game.app_name.parse().ok();

                let query = TitleQuery {
                    names: vec![game_title.to_owned()],
                    gog_id,
                    normalized: true,
                    ..Default::default()
                };
                let Some(official_title) = title_finder.find_one(query) else {
                    log::trace!("Ignoring unrecognized game: {}, app: {}", &game_title, &game.app_name);
                    if std::env::var(ENV_DEBUG).is_ok() {
                        eprintln!(
                            "Ignoring unrecognized game from Heroic/GOG: {} (app = {})",
                            &game_title, &game.app_name
                        );
                    }
                    continue;
                };

                log::trace!(
                    "Detected game: {} | app: {}, raw title: {}",
                    &official_title,
                    &game.app_name,
                    &game_title
                );
                let prefix = find_prefix(&root.path, game_title, Some(&game.platform), &game.app_name);
                games.entry(official_title).or_default().insert(LauncherGame {
                    install_dir: Some(StrictPath::new(game.install_path.clone())),
                    prefix,
                    platform: Some(Os::from(game.platform.as_str())),
                });
            }
        }
        Err(e) => {
            log::warn!("Unable to parse installed list from {:?}: {}", &installed_path, e);
        }
    }

    games
}

pub fn get_library(root: &root::Heroic) -> Vec<library::Game> {
    let libraries = [root.path.joined(library::PATH), root.path.joined(library::PATH_LEGACY)];

    let library_path = 'outer: {
        for library in libraries {
            if library.is_file() {
                break 'outer library;
            }
        }
        log::warn!("Could not find library in {:?}", root);
        return vec![];
    };

    match serde_json::from_str::<library::Data>(&library_path.read().unwrap_or_default()) {
        Ok(gog_library) => {
            log::trace!("Found {} games in {:?}", gog_library.games.len(), &library_path);

            gog_library.games
        }
        Err(e) => {
            log::warn!("Unable to parse library in {:?}: {}", &library_path, e);
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{hash_map, hash_set};

    use super::*;
    use crate::{
        resource::{
            manifest::{Manifest, Os},
            ResourceFile,
        },
        testing::repo,
    };

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            game-1:
              files:
                <base>/file1.txt: {}
            "#,
        )
        .unwrap()
    }

    fn title_finder() -> TitleFinder {
        TitleFinder::new(&Default::default(), &manifest(), Default::default())
    }

    #[test]
    fn scan_finds_all_games_without_store_cache() {
        let root = root::Heroic {
            path: format!("{}/tests/launchers/heroic-gog-without-store-cache", repo()).into(),
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "game-1".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/games/game-1".to_string())),
                    prefix: Some(StrictPath::new("/prefixes/game-1".to_string())),
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }

    #[test]
    fn scan_finds_all_games_with_store_cache() {
        let root = root::Heroic {
            path: format!("{}/tests/launchers/heroic-gog-with-store-cache", repo()).into(),
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "game-1".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/games/game-1".to_string())),
                    prefix: Some(StrictPath::new("/prefixes/game-1".to_string())),
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }
}
