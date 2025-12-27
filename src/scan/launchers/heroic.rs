pub mod gog;
pub mod legendary;
pub mod nile;
pub mod sideload;

use std::collections::{HashMap, HashSet};

use crate::{path::CommonPath, prelude::StrictPath};

use crate::{
    resource::{config::root, manifest::Os},
    scan::{launchers::LauncherGame, TitleFinder},
};

mod games_config {
    use std::collections::HashMap;

    pub fn path(id: &str) -> String {
        format!("GamesConfig/{id}.json")
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Data(pub HashMap<String, Game>);

    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Game {
        #[serde(rename_all = "camelCase")]
        Config {
            wine_prefix: String,
            wine_version: Wine,
        },
        IgnoreOther(serde::de::IgnoredAny),
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Wine {
        #[serde(rename = "type")]
        pub wine_type: String,
    }
}

pub fn scan(
    root: &root::Heroic,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, HashSet<LauncherGame>> {
    let mut games = HashMap::<String, HashSet<LauncherGame>>::new();

    for (title, info) in legendary::scan(root, title_finder, legendary) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in gog::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in nile::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in sideload::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    if Os::HOST == Os::Linux {
        if let Some(normal_home) = CommonPath::Home.get().map(StrictPath::new) {
            if let Some(flatpak_home) = root.flatpak_home() {
                log::debug!("For Flatpak root {root:?}, translating home {normal_home:?} to {flatpak_home:?}");
                return games
                    .into_iter()
                    .map(|(title, mut info)| {
                        let flatpak: HashSet<_> = info
                            .iter()
                            .map(|info| info.replace_in_paths(&normal_home, &flatpak_home))
                            .collect();

                        info.extend(flatpak);

                        (title, info)
                    })
                    .collect();
            }
        }
    }

    games
}

fn find_prefix(
    heroic_path: &StrictPath,
    game_name: &str,
    platform: Option<&str>,
    app_name: &str,
) -> Option<StrictPath> {
    log::trace!(
        "Will try to find prefix for Heroic game: {} (app={}, platform={:?})",
        game_name,
        app_name,
        platform
    );

    let games_config_path = heroic_path.joined(games_config::path(app_name));

    let content = match games_config_path.try_read() {
        Ok(content) => content,
        Err(e) => {
            log::trace!("Failed to read {:?}: {}", &games_config_path, e);
            return None;
        }
    };

    match serde_json::from_str::<games_config::Data>(&content) {
        Ok(games_config_wrapper) => {
            let game_config = games_config_wrapper.0.get(app_name)?;

            match game_config {
                games_config::Game::Config {
                    wine_version,
                    wine_prefix,
                } => match wine_version.wine_type.as_str() {
                    "wine" => {
                        log::trace!(
                            "Found Heroic Wine prefix for {} ({}) -> adding {}",
                            game_name,
                            app_name,
                            wine_prefix
                        );
                        Some(StrictPath::new(wine_prefix.clone()))
                    }

                    "proton" => {
                        // This would usually be `{wine_prefix}/pfx`,
                        // but in at least one case in a Flatpak install,
                        // that was a broken link and the real prefix was just `wine_prefix`.
                        log::trace!(
                            "Found Heroic Proton prefix for {} ({}), adding {}",
                            game_name,
                            app_name,
                            wine_prefix
                        );
                        Some(StrictPath::new(wine_prefix.clone()))
                    }

                    _ => {
                        log::info!(
                            "Found Heroic Windows game {} ({}) with unknown wine_type: {:#?}",
                            game_name,
                            app_name,
                            wine_version.wine_type
                        );
                        None
                    }
                },
                games_config::Game::IgnoreOther(_) => None,
            }
        }
        Err(e) => {
            log::trace!("Failed to parse {:?}: {}", &games_config_path, e);
            None
        }
    }
}
