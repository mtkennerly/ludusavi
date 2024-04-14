use std::env;

use crate::{
    resource::{config::RootsConfig, manifest::Store},
    scan::heroic::{get_gog_games_library, get_legendary_installed_games},
    wrap::WrapGameInfo,
};

/// Tries to find a game with ID `game_id` in the given game roots, actual
/// search algorithm used varies with `game_runner`.  Returns game name or None.
fn find_in_roots(roots: &[RootsConfig], game_id: &str, game_runner: &str) -> Option<String> {
    roots
        .iter()
        .filter(|root| root.store == Store::Heroic)
        .find_map(|root| {
            log::debug!("wrap::heroic::find_in_roots: checking root {:?}", root);

            match game_runner {
                "gog" => match get_gog_games_library(root) {
                    Some(gog_games) => gog_games.iter().find_map(|g| match g.app_name == *game_id {
                        true => Some(g.title.clone()),
                        false => None,
                    }),
                    None => None,
                },
                "legendary" => get_legendary_installed_games(root, None)
                    .iter()
                    .find_map(|legendary_game| match legendary_game.app_name == *game_id {
                        true => Some(legendary_game.title.clone()),
                        false => None,
                    }),

                "nile" => {
                    log::debug!("Ignoring Heroic game with unsupported runner 'nile'.");
                    None
                }
                "sideload" => {
                    log::debug!("Ignoring Heroic game with unsupported runner 'sideload'.");
                    None
                }
                value => {
                    log::debug!("Ignoring Heroic game with unknown runner '{}'.", value);
                    None
                }
            }
        })
}

/// Parse environment variables set by heroic (starting with 2.9.2):
///
/// HEROIC_APP_NAME (the ID, not the human-friendly title)
/// HEROIC_APP_RUNNER (one of: gog, legendary, nile, sideload)
/// HEROIC_APP_SOURCE (one of: gog, epic, amazon, sideload)
///
/// We rely on HEROIC_APP_NAME and HEROIC_APP_RUNNER only.
pub fn infer_game_from_heroic(roots: &[RootsConfig]) -> Option<WrapGameInfo> {
    let heroic_app_name = env::var("HEROIC_APP_NAME").ok()?;

    let heroic_app_runner = env::var("HEROIC_APP_RUNNER").ok()?;

    log::debug!(
        "Found Heroic environment variables: heroic_app_name={}, heroic_app_runner={}",
        heroic_app_name,
        heroic_app_runner,
    );

    let result = WrapGameInfo {
        name: find_in_roots(roots, &heroic_app_name, &heroic_app_runner),
        gog_id: match heroic_app_runner.as_str() {
            "gog" => heroic_app_name.parse().ok(),
            _ => None,
        },
        ..Default::default()
    };

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
