use std::collections::HashMap;

use crate::prelude::StrictPath;

use crate::{
    prelude::ENV_DEBUG,
    resource::{config::RootsConfig, manifest::Os},
    scan::{
        launchers::{heroic::find_prefix, LauncherGame},
        TitleFinder, TitleQuery,
    },
};

/// `gog_store/installed.json`
#[derive(serde::Deserialize)]
struct Installed {
    installed: Vec<InstalledGame>,
}

#[derive(serde::Deserialize)]
struct InstalledGame {
    /// This is an opaque ID, not the human-readable title.
    #[serde(rename = "appName")]
    app_name: String,
    platform: String,
    install_path: String,
}

/// `gog_store/library.json` or `store_cache/gog_library.json`
#[derive(serde::Deserialize)]
struct Library {
    games: Vec<LibraryGame>,
}

#[derive(serde::Deserialize)]
pub struct LibraryGame {
    /// This is an opaque ID, not the human-readable title.
    pub app_name: String,
    pub title: String,
}

pub fn scan(root: &RootsConfig, title_finder: &TitleFinder) -> HashMap<String, LauncherGame> {
    let mut games = HashMap::new();

    let game_titles: HashMap<String, String> = get_library(root)
        .iter()
        .map(|game| (game.app_name.clone(), game.title.clone()))
        .collect();

    if game_titles.is_empty() {
        return games;
    }

    // iterate over all games found in HEROCONFIGDIR/gog_store/installed.json and call find_prefix
    let installed_path = root.path.joined("gog_store").joined("installed.json");
    let content = installed_path.read();

    match serde_json::from_str::<Installed>(&content.unwrap_or_default()) {
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
                let prefix = find_prefix(&root.path, game_title, &game.platform, &game.app_name);
                games.insert(
                    official_title,
                    LauncherGame {
                        install_dir: Some(StrictPath::new(game.install_path.clone())),
                        prefix,
                        platform: Some(Os::from(game.platform.as_str())),
                    },
                );
            }
        }
        Err(e) => {
            log::warn!("Unable to parse installed list from {:?}: {}", &installed_path, e);
        }
    }

    games
}

pub fn get_library(root: &RootsConfig) -> Vec<LibraryGame> {
    let libraries = [
        root.path.joined("store_cache").joined("gog_library.json"),
        root.path.joined("gog_store").joined("library.json"),
    ];

    let library_path = 'outer: {
        for library in libraries {
            if library.is_file() {
                break 'outer library;
            }
        }
        log::warn!("Could not find library in {:?}", root);
        return vec![];
    };

    match serde_json::from_str::<Library>(&library_path.read().unwrap_or_default()) {
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
