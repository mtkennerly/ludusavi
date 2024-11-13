use std::env;

use crate::{
    resource::config::Root,
    scan::launchers::heroic::{gog, legendary, nile, sideload},
    wrap::WrapGameInfo,
};

/// Tries to find a game with ID `game_id` in the given game roots, actual
/// search algorithm used varies with `game_runner`.  Returns game name or None.
fn find_in_roots(roots: &[Root], game_id: &str, game_runner: &str) -> Option<String> {
    roots
        .iter()
        .filter_map(|root| match root {
            Root::Heroic(root) => Some(root),
            _ => None,
        })
        .find_map(|root| {
            log::debug!("Looking for game ID '{}' in root {:?}", game_id, root);

            match game_runner {
                "gog" => gog::get_library(root)
                    .iter()
                    .find_map(|g| match g.app_name == *game_id {
                        true => Some(g.title.clone()),
                        false => None,
                    }),
                "legendary" => legendary::get_installed(root, None).iter().find_map(|legendary_game| {
                    match legendary_game.app_name == *game_id {
                        true => Some(legendary_game.title.clone()),
                        false => None,
                    }
                }),

                "nile" => nile::get_library(&root.path).get(game_id).map(|x| x.title.clone()),
                "sideload" => sideload::get_library(&root.path).get(game_id).map(|x| x.title.clone()),
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
pub fn infer_game_from_heroic(roots: &[Root]) -> Option<WrapGameInfo> {
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
