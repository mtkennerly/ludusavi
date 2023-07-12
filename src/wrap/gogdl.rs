use itertools::Itertools;

use super::LaunchParser;
use crate::prelude::StrictPath;

/// Deserialization of GOG Game info file "$GAME_DIR/goggame-$GAME_ID.info"
#[derive(serde::Deserialize)]
pub struct GogGameInfo {
    pub name: String,
    // ignore everything else
}

pub struct HeroicGogdl;
impl LaunchParser for HeroicGogdl {
    // TODO.2023-06-22 path separator linux specific
    // TODO.2023-06-23 refactor println into logs
    fn parse(&self, commands: &[String]) -> Option<String> {
        let mut iter = commands.iter();

        if iter.find_position(|p| p.ends_with("gogdl")).is_none() {
            println!("HeroicGogdl::parse: gogdl not found");
            return None;
        }
        println!("HeroicGogdl::parse: gogdl found");

        if iter.find_position(|p| p.ends_with("launch")).is_none() {
            println!("HeroicGogdl::parse: launch not found");
            return None;
        }
        let game_dir = iter.next().unwrap();
        let game_id = iter.next().unwrap();
        println!(
            "HeroicGogdl::parse: gogdl launch found: dir = {}, id = {}",
            game_dir, game_id
        );

        let gog_info_path_native = StrictPath::from(&format!("{}/gameinfo", game_dir));
        match gog_info_path_native.is_file() {
            true => {
                // GOG Linux native
                //     GAMENAME=`$HEAD -1 "$GAME_DIR/gameinfo"`
                let game_name = gog_info_path_native
                    .read()
                    .unwrap_or_default()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                if game_name.is_empty() {
                    println!("HeroicGogdl::parse: Error reading {}", gog_info_path_native.interpret());
                    None
                } else {
                    Some(game_name)
                }
            }
            false => {
                // GOG Windows game
                //     GAMENAME=`$JQ -r .name "$GAME_DIR/goggame-$GAME_ID.info"`
                let gog_info_path_windows = StrictPath::from(&format!("{}/goggame-{}.info", game_dir, game_id));

                match serde_json::from_str::<GogGameInfo>(&gog_info_path_windows.read().unwrap_or_default()) {
                    Ok(ggi) => {
                        let game_name = ggi.name;
                        match game_name.is_empty() {
                            true => {
                                println!(
                                    "HeroicGogdl::parse: Error reading {}, no name entry found.",
                                    gog_info_path_windows.interpret()
                                );
                                None
                            }
                            false => Some(game_name),
                        }
                    }
                    Err(e) => {
                        println!(
                            "HeroicGogdl::parse: Error reading {}: {:#?}",
                            gog_info_path_windows.interpret(),
                            e
                        );
                        None
                    }
                }
            }
        }
    }
}
