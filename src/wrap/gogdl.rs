use std::{env, str::FromStr};

use itertools::Itertools;

use crate::{
    prelude::StrictPath,
    resource::{config::RootsConfig, manifest::Store},
    scan::launchers::heroic::get_gog_games_library,
};

fn find_in_roots(roots: &[RootsConfig], game_id: &str) -> Option<String> {
    roots
        .iter()
        .filter(|root| root.store == Store::Heroic)
        .find_map(|root| {
            log::debug!("HeroicGogdl::find_in_roots: checking root {:?}", root);
            match get_gog_games_library(root) {
                Some(gog_games) => gog_games.iter().find_map(|g| match g.app_name == *game_id {
                    true => Some(g.title.clone()),
                    false => None,
                }),
                None => None,
            }
        })
}

/// Parsing of command line for Heroic 2.9.x, returns a game name or None
pub fn parse_heroic_2_9(roots: &[RootsConfig], commands: &[String]) -> Option<String> {
    let mut game_id = String::default();

    // Is this a Linux native game?
    // Try checking dirname(commands[0])/game/__game/goggame-GAME_ID.id
    let id_files = StrictPath::from(commands[0].as_str())
        .parent_if_file()
        .joined("game")
        .joined("__game")
        .joined("goggame-*.id")
        .glob();
    if id_files.len() == 1 {
        let id_file = id_files[0].as_std_path_buf();
        log::debug!(
            "HeroicGogdl::parse_heroic_2_9: found Linux native goggame-*.id: {:?}",
            id_file
        );
        game_id = String::from_str(
            id_file
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .strip_prefix("goggame-")
                .unwrap(),
        )
        .unwrap();
    }

    if game_id.is_empty() {
        // Is this a Windows game?
        //
        // check environment variable
        // env(STEAM_COMPAT_INSTALL_PATH)/goggame-GAME_ID.id
        if let Ok(env_install_path) = env::var("STEAM_COMPAT_INSTALL_PATH") {
            let id_files = StrictPath::from(env_install_path.as_str())
                .joined("goggame-*.id")
                .glob();
            if id_files.len() == 1 {
                let id_file = id_files[0].as_std_path_buf();
                log::debug!(
                    "HeroicGogdl::parse_heroic_2_9: found Windows goggame-*.id: {:?}",
                    id_file
                );
                game_id = String::from_str(
                    id_file
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .strip_prefix("goggame-")
                        .unwrap(),
                )
                .unwrap();
            }
        }
    }

    if game_id.is_empty() {
        log::debug!("HeroicGogdl::parse_heroic_2_9: no goggame-*.id found, neither for Linux native nor Windows");
        None
    } else {
        find_in_roots(roots, &game_id)
    }
}

/// Parsing of command line for Heroic 2.8.x (and probably earlier versions),
/// returns a game name or None
pub fn parse_heroic_2_8(roots: &[RootsConfig], commands: &[String]) -> Option<String> {
    let mut iter = commands.iter();

    if iter.find_position(|p| p.ends_with("gogdl")).is_none() {
        log::debug!("HeroicGogdl::parse_heroic_2_8: gogdl not found");
        return None;
    }
    log::debug!("HeroicGogdl::parse_heroic_2_8: gogdl found");

    if iter.find_position(|p| p.ends_with("launch")).is_none() {
        log::debug!("HeroicGogdl::parse_heroic_2_8: launch not found");
        return None;
    }
    // TODO.2023-07-19 fails if user selects different game exe in heroic
    let game_dir = iter.next().unwrap();
    let game_id = iter.next().unwrap();
    log::debug!(
        "HeroicGogdl::parse_heroic_2_8: gogdl launch found: dir = {}, id = {}",
        game_dir,
        game_id
    );

    find_in_roots(roots, game_id)
}
