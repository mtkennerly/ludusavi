use crate::{prelude::Error, resource::config::RootsConfig};

mod gogdl;
mod legendary;

pub fn get_game_name_from_heroic_launch_commands(roots: &[RootsConfig], commands: &[String]) -> Result<String, Error> {
    // TODO.2023-07-19 check only applicable roots

    let parsers = vec![
        gogdl::parse_heroic_2_9,
        gogdl::parse_heroic_2_8,
        legendary::parse_heroic_2_9,
        legendary::parse_heroic_2_8,
    ];
    match parsers.iter().find_map(|parser| parser(roots, commands)) {
        Some(game_name) => Ok(game_name),
        None => Err(Error::WrapCommandNotRecognized {
            msg: "get_game_name_from_heroic_launch_commands: could not detect any known launcher.".to_string(),
        }),
    }
}
