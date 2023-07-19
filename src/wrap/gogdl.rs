use itertools::Itertools;

use super::LaunchParser;
use crate::{resource::config::RootsConfig, scan::launchers::heroic::get_gog_games_library};

pub struct HeroicGogdl;
impl LaunchParser for HeroicGogdl {
    fn parse(&self, roots: &[RootsConfig], commands: &[String]) -> Option<String> {
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
}
