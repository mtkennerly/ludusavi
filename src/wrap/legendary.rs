use itertools::Itertools;

use crate::{resource::config::RootsConfig, scan::heroic::get_legendary_installed_games};

// Heroic 2.9:
//
// GAME_ID found in command line parameter
//     "-epicapp=d8a4c98b5020483881eb7f0c3fc4cea3",

/// Parsing of command line for Heroic 2.8.x (and probably earlier versions)
pub fn parse_heroic_2_8(roots: &[RootsConfig], commands: &[String]) -> Option<String> {
    let mut iter = commands.iter();

    let legendary_command = match iter.find_position(|p| p.ends_with("legendary")) {
        None => {
            log::debug!("Legendary::parse: legendary not found");
            return None;
        }
        Some(cmd) => cmd.1,
    };
    log::debug!("Legendary::parse: legendary found: {}", legendary_command);

    if iter.find_position(|p| p.ends_with("launch")).is_none() {
        log::debug!("Legendary::parse: launch not found");
        return None;
    }
    let game_id = iter.next().unwrap();
    log::debug!("Legendary::parse: legendary launch found: id = {}", game_id);

    // TODO.2023-07-14 filter for root.type?
    roots.iter().find_map(|root| {
        // TODO.2023-07-19 use some valid value for legendary parameter instead of None
        get_legendary_installed_games(root, None)
            .iter()
            .find_map(|legendary_game| match legendary_game.app_name == *game_id {
                true => Some(legendary_game.title.clone()),
                false => None,
            })
    })
}
