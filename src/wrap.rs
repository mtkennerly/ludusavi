use crate::{prelude::Error, resource::config::RootsConfig};

mod gogdl;
mod legendary;

/// Trait for command line argument parsers to determine the actual game name,
/// for implementations check the submodules.
trait LaunchParser {
    /// Determine game name from `commands`, return `None` if it fails.
    fn parse(&self, roots: &[RootsConfig], commands: &[String]) -> Option<String>;
}

pub fn get_game_name_from_heroic_launch_commands(roots: &[RootsConfig], commands: &[String]) -> Result<String, Error> {
    // I'd love to write let d = vec![Heroic{}, Legendary{}];
    //
    // Coming from OOP the code below seems a bit much of syntactical noise, but
    // it handles the fact that a trait is a compile time structure with unknown
    // size, so I "Box" it to put the actual objects on the heap.
    //
    // Taken from https://doc.rust-lang.org/book/ch17-02-trait-objects.html
    //
    // Also support for Amazon Prime is in the works for heroic:
    // https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/pull/2831
    let detectors: Vec<Box<dyn LaunchParser>> =
        vec![Box::new(gogdl::HeroicGogdl {}), Box::new(legendary::Legendary {})];

    // TODO.2023-07-19 check only applicable roots

    match detectors.iter().find_map(|parser| parser.parse(roots, commands)) {
        Some(game_name) => Ok(game_name),
        None => Err(Error::WrapCommandNotRecognized {
            msg: "get_game_name_from_heroic_launch_commands: could not detect any known launcher.".to_string(),
        }),
    }
}
