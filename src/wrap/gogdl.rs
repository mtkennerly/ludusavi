use itertools::Itertools;

use crate::{resource::config::RootsConfig, scan::launchers::heroic::get_gog_games_library};

// TODO.2023-07-30 - Linux Native: heroic 2.9.x changed invocation for gogdl
// based games and we no longer get the GAME_DIR ang GAME_ID information from
// the command line parameters.
//
// Instead we need to rely on GAMEDIR/goggame-GAME_ID.id (wine/proton) or
// GAMEDIR/game/__game/goggame-GAME_ID.id (Linux native) to get a game id
//
// Linux Native:
//
// commands is GAME_DIR/start.sh
//
// Wine/Proton:
//
// check environment variable
// STEAM_COMPAT_INSTALL_PATH='/home/saschal/Games/The Riftbreaker'

/// Parsing of command line for Heroic 2.8.x (and probably earlier versions)
pub fn parse_heroic_2_8(roots: &[RootsConfig], commands: &[String]) -> Option<String> {
    let mut iter = commands.iter();

    if iter.find_position(|p| p.ends_with("gogdl")).is_none() {
        log::debug!("HeroicGogdl::parse: gogdl not found");
        return None;
    }
    log::debug!("HeroicGogdl::parse: gogdl found");

    if iter.find_position(|p| p.ends_with("launch")).is_none() {
        log::debug!("HeroicGogdl::parse: launch not found");
        return None;
    }
    // TODO.2023-07-19 fails if user selects different game exe in heroic
    let game_dir = iter.next().unwrap();
    let game_id = iter.next().unwrap();
    log::debug!(
        "HeroicGogdl::parse: gogdl launch found: dir = {}, id = {}",
        game_dir,
        game_id
    );

    // TODO.2023-07-14 filter for root.type = Heroic
    roots.iter().find_map(|root| {
        log::debug!("HeroicGogdl::parse: checking root {:?}", root);
        match get_gog_games_library(root) {
            Some(gog_games) => gog_games.iter().find_map(|g| match g.app_name == *game_id {
                true => Some(g.title.clone()),
                false => None,
            }),
            None => None,
        }
    })
}
