use std::{env, str::FromStr};

use itertools::Itertools;

use crate::{
    prelude::StrictPath,
    resource::{config::RootsConfig, manifest::Store},
    scan::launchers::heroic::get_gog_games_library,
};

/// Deserialization of GOG goggame-SOME_ID.info
#[derive(Debug, serde::Deserialize)]
struct GogGameInfo {
    #[serde(rename = "rootGameId")]
    root_game_id: String,
}

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

/// Parsing of command line for Heroic 2.9.x, returns a game name or None taken
/// from any goggame-SOME_ID.info found
pub fn parse_heroic_2_9_goggame_info(roots: &[RootsConfig], commands: &[String]) -> Option<String> {
    let mut game_id = String::default();

    // Lets find a useable goggame-GAME_ID.info file
    //
    // len()-1 accomodates for mangohud and other tools
    let info_files: Vec<_> = [
        // Linux native games like Stellaris, Dead Cells or Terraria
        StrictPath::from(commands[commands.len() - 1].as_str())
            .parent_if_file()
            .unwrap_or_default()
            .joined("game")
            .joined("goggame-*.info"),
        // Windows games like Blasphemous, Desperados 3 or The Witcher 3 Wild Hunt GOTY
        StrictPath::from(env::var("STEAM_COMPAT_INSTALL_PATH").unwrap_or_default().as_str()).joined("goggame-*.info"),
    ]
    .iter()
    .flat_map(|sp| sp.glob())
    .collect();

    if !info_files.is_empty() {
        let info_file = &info_files[0];
        log::debug!(
            "HeroicGogdl::parse_heroic_2_9_goggame_info: found goggame-*.info: {:?}",
            info_file
        );
        let content = info_file.read();
        if let Ok(ggi) = serde_json::from_str::<GogGameInfo>(&content.unwrap_or_default()) {
            log::debug!(
                "HeroicGogdl::parse_heroic_2_9_goggame_info: read goggame-SOME_ID.info: {:#?}",
                ggi
            );
            game_id = ggi.root_game_id;
        }
    }

    if game_id.is_empty() {
        log::debug!(
            "HeroicGogdl::parse_heroic_2_9_goggame_info: no goggame-*.info found, neither for Linux native nor Windows"
        );
        None
    } else {
        find_in_roots(roots, &game_id)
    }
}

// TODO.2023-08-07 this might be obsolete since parse_heroic_2_9_goggame_info
// probably always returns a proper value
/// Parsing of command line for Heroic 2.9.x, returns a game name if a
/// goggame-GAME_ID.id file was found or None
pub fn parse_heroic_2_9_goggame_id(roots: &[RootsConfig], commands: &[String]) -> Option<String> {
    let mut game_id = String::default();

    // Is this a Linux native game?
    // Try checking dirname(commands[0])/game/__game/goggame-GAME_ID.id
    let id_files = StrictPath::from(commands[0].as_str())
        .parent_if_file()
        .unwrap_or_default()
        .joined("game")
        .joined("__game")
        .joined("goggame-*.id")
        .glob();
    if id_files.len() == 1 {
        let id_file = id_files[0].as_std_path_buf();
        log::debug!(
            "HeroicGogdl::parse_heroic_2_9_goggame_id: found Linux native goggame-*.id: {:?}",
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
                    "HeroicGogdl::parse_heroic_2_9_goggame_id: found Windows goggame-*.id: {:?}",
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
        log::debug!(
            "HeroicGogdl::parse_heroic_2_9_goggame_id: no goggame-*.id found, neither for Linux native nor Windows"
        );
        None
    } else {
        find_in_roots(roots, &game_id)
    }
}

/// Parsing of command line for Heroic 2.8.x (and probably earlier versions),
/// returns a game name or None
///
/// NOTE: fails if user selects different game executable in heroic
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
