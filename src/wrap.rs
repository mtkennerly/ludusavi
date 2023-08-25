use crate::{prelude::Error, resource::config::RootsConfig};

mod gogdl;
mod legendary;
pub(crate) mod ui;

/// Determine game name from heroic launch commands.  Game name is returned raw
/// (just like e.g. legendary or gogdl know them) and not yet checked with
/// TitleFinder or normalized in any way.
pub fn get_game_name_from_heroic_launch_commands(roots: &[RootsConfig], commands: &[String]) -> Result<String, Error> {
    // TODO.2023-08-01 support Amazon Games (supported since Heroic 2.9)
    let parsers = [
        gogdl::parse_heroic_2_9_goggame_info,
        gogdl::parse_heroic_2_9_goggame_id,
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
