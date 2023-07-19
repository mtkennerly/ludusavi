use itertools::Itertools;

use crate::{resource::config::RootsConfig, scan::heroic::get_legendary_installed_games};

use super::LaunchParser;

/// Deserialization of legendary game metadata from calling 'legendary list-installed --json'
#[derive(Debug, serde::Deserialize)]
pub struct LegendaryGameInfo {
    pub app_name: String,
    pub title: String,
    // ignore everything else
}

pub struct Legendary;
impl LaunchParser for Legendary {
    fn parse(&self, roots: &[RootsConfig], commands: &[String]) -> Option<String> {
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
}
