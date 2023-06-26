use itertools::Itertools;

use crate::{
    prelude::{Error, StrictPath},
    wrap::gog::GogGameInfo,
};

mod gog;

// TODO.2023-06-23 refactor println into logs
// TODO.2023-06-23 legendary .. launch LEGENDARY_GAME_ID
// TODO.2023-06-23 legendary (EPIC) sample command:
// Launch Command: LD_PRELOAD= WINEPREFIX=/home/saschal/Games/Heroic/Prefixes/Slain WINEDLLOVERRIDES=winemenubuilder.exe=d ORIG_LD_LIBRARY_PATH= LD_LIBRARY_PATH=/home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/lib64:/home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/lib GST_PLUGIN_SYSTEM_PATH_1_0=/home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/lib64/gstreamer-1.0:/home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/lib/gstreamer-1.0 WINEDLLPATH=/home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/lib64/wine:/home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/lib/wine /usr/bin/mangohud --dlsym /opt/Heroic/resources/app.asar.unpacked/build/bin/linux/legendary launch d8a4c98b5020483881eb7f0c3fc4cea3 --language en --wine /home/saschal/.config/heroic/tools/wine/Wine-GE-Proton7-31/bin/wine
// read from /home/saschal/.config/legendary/metadata/d8a4c98b5020483881eb7f0c3fc4cea3.json
pub fn get_game_name_from_launch_commands(commands: &Vec<String>) -> Result<String, Error> {
    let mut wrap_error: Option<String> = None;
    let game_dir;
    let game_id;
    let mut game_name = String::default();

    println!("wrap commands: {:#?}", commands);

    let mut iter = commands.iter();
    if iter.find_position(|p| p.ends_with("gogdl")).is_some() {
        println!("wrap: gogdl found");
        if iter.find_position(|p| p.ends_with("launch")).is_some() {
            game_dir = iter.next().unwrap();
            game_id = iter.next().unwrap();
            let gog_info_path_native = StrictPath::from(&format!("{}/gameinfo", game_dir));
            match gog_info_path_native.is_file() {
                true => {
                    // GOG Linux native
                    //     GAMENAME=`$HEAD -1 "$GAME_DIR/gameinfo"`
                    game_name = gog_info_path_native
                        .read()
                        .unwrap_or_default()
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_string();
                    if game_name.is_empty() {
                        wrap_error = Some(format!("Error reading {}", gog_info_path_native.interpret()));
                    }
                }
                false => {
                    // GOG Windows game
                    //     GAMENAME=`$JQ -r .name "$GAME_DIR/goggame-$GAME_ID.info"`
                    let gog_info_path_windows = StrictPath::from(&format!("{}/goggame-{}.info", game_dir, game_id));

                    match serde_json::from_str::<GogGameInfo>(&gog_info_path_windows.read().unwrap_or_default()) {
                        Ok(ggi) => {
                            game_name = ggi.name;
                            if game_name.is_empty() {
                                wrap_error = Some(format!(
                                    "Error reading {}, no name entry found.",
                                    gog_info_path_windows.interpret()
                                ));
                            }
                        }
                        Err(e) => {
                            wrap_error = Some(format!("Error reading {}: {:#?}", gog_info_path_windows.interpret(), e));
                        }
                    }
                }
            }
            println!(
                "wrap: gogdl launch found: {} - {}, name: {}",
                game_dir, game_id, game_name
            );
        } else {
            wrap_error = Some("gogdl launch parameter not found".to_string());
        }
    } else {
        // TODO.2023-06-23 handle other launchers (legendary, ...) here
        wrap_error = Some("unknown launcher in command line parameters".to_string());
    }

    match wrap_error {
        Some(msg) => Err(Error::WrapCommandNotRecognized { msg }),
        None => Ok(game_name),
    }
}
