use std::collections::HashMap;

use crate::{
    config::RootsConfig,
    manifest::{Manifest, Store},
    prelude::StrictPath,
};

#[derive(serde::Deserialize)]
struct HeroicGame {
    #[serde(rename = "appName")]
    game_id: String,
    platform: String,
}
#[derive(serde::Deserialize)]
struct HeroicInstalled {
    installed: Vec<HeroicGame>,
}

// TODO.2022-10-10 windows location for legendary
// TODO.2022-10-15 make relative to heroic root in question!
const LEGENDARY_PATHS: &[&str] = &[
    "~/.config/legendary",
    "~/.var/app/com.heroicgameslauncher.hgl/config/legendary",
];

#[derive(Clone, Default, Debug)]
pub struct HeroicGames(HashMap<String, StrictPath>);

impl HeroicGames {
    pub fn get(&self, game: &String) -> Option<&StrictPath> {
        self.0.get(game)
    }

    pub fn scan(roots: &[RootsConfig], _manifest: &Manifest) -> Self {
        let mut instance = HeroicGames::default();

        roots.iter().for_each(|root: &RootsConfig| {
            if root.store == Store::HeroicConfig {
                instance.detect_normal_roots(root);
                instance.detect_legendary_roots(root);
                instance.detect_gog_roots(root);
                println!("heroic::detect_heroic_roots found: {:#?}", instance);
            }
        });

        instance
    }

    fn detect_normal_roots(&mut self, _root: &RootsConfig) {
        // TODO.2022-10-15 handle games storing stuff in ~/.var/app/com.heroicgameslauncher.hglw
        // TODO.2022-10-15 for some games, .var/app/com.heroicgameslauncher.hglw is a root just like $HOME
        // NOP
    }

    /// Look for EPIC games installed with legendary
    fn detect_legendary_roots(&mut self, root: &RootsConfig) {
        if root.store == Store::HeroicConfig {
            println!("heroic::detect_legendary_roots searching for legendary config: {root:?}");

            for &legendary_path_candidate in LEGENDARY_PATHS {
                let legendary_path = StrictPath::new(legendary_path_candidate.to_string());
                if legendary_path.is_dir() {
                    println!(
                        "heroic::detect_legendary_roots found legendary configuration in {legendary_path:?}"
                    );
                    // read list of installed games
                    let mut legendary_installed = legendary_path.as_std_path_buf();
                    legendary_installed.push("installed.json");
                    if legendary_installed.is_file() {
                        // TODO.2022-10-18 use struct to have type safety
                        let v: serde_json::Value =
                            serde_json::from_str(&std::fs::read_to_string(legendary_installed).unwrap_or_default())
                                .unwrap_or_default();

                        // TODO.2022-10-17 try unwrap_or_default on v (as in detect_heroic_gog_roots)
                        v.as_object().unwrap().iter().for_each(|entry| {
                            let game_title = String::from(entry.1["title"].as_str().unwrap_or_default());
                            println!(
                                "heroic::detect_legendary_roots found game {}: {}",
                                entry.0, game_title
                            );
                            // process game from GamesConfig
                            if let Some(sp) = self.find_game_root(
                                root.path.interpret(),
                                &game_title,
                                &String::from(entry.1["platform"].as_str().unwrap_or_default().to_lowercase()),
                                &entry.0,
                            ) {
                                self.memorize_game_root(&game_title, &sp);
                            }
                        });
                    } else {
                        println!(
                            "heroic::detect_legendary_roots no such file '{:?}', legendary probably not used yet... skipping",
                            legendary_installed
                        );
                    }
                }
            }
        }
    }

    // #94: add games installed with heroic roots
    fn detect_gog_roots(&mut self, root: &RootsConfig) {
        if root.store == Store::HeroicConfig {
            println!("heroic::detect_gog_roots found heroic config: {root:?}");

            // use HEROCONFIGDIR/gog_store/library.json to build map .app_name -> .title
            let mut game_titles = std::collections::HashMap::<String, String>::new();
            // TODO.2022-10-18 use struct for type safety
            let library_json: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(format!("{}/gog_store/library.json", root.path.interpret()))
                    .unwrap_or_default(),
            )
            .unwrap_or_default();
            library_json["games"].as_array().unwrap().iter().for_each(|lib| {
                game_titles.insert(
                    String::from(lib["app_name"].as_str().unwrap_or_default()),
                    String::from(lib["title"].as_str().unwrap_or_default()),
                );
            });
            println!(
                "heroic::detect_gog_roots found {} games in CONFIGDIR/gog_store/library.json",
                game_titles.len()
            );

            // iterate over all games found in HEROCONFIGDIR/gog_store/installed.json and call heroic_find_game_root
            let content = std::fs::read_to_string(format!("{}/gog_store/installed.json", root.path.interpret()));
            let installed_games = serde_json::from_str::<HeroicInstalled>(&content.unwrap_or_default());
            installed_games.unwrap().installed.iter().for_each(|game| {
                let game_title = game_titles.get(&game.game_id).unwrap();
                if let Some(sp) =
                    self.find_game_root(root.path.interpret(), &game_title, &game.platform, &game.game_id)
                {
                    self.memorize_game_root(&game_title, &sp);
                }
            });
        }
    }

    fn memorize_game_root(&mut self, title: &String, path: &StrictPath) {
        // TODO.2022-10-11 check against manifest, try name normalization like this:
        //
        // let normalized_to_official: HashMap<_> = manifest.keys().map(|title| (normalize_title(title), title)).collect();
        //
        // for candidate in heroic_games {
        //     let normalized = normalize_title(candidate.title);
        //     if let Some(official) = normalized_to_official.get(normalized) {
        //         // we found a match
        //     }
        // }
        println!(
            "heroic::memorize_game_root memorizing path {path:?} for {}",
            title
        );
        self.0.insert(title.clone(), path.clone());
    }

    fn find_game_root(
        &self,
        heroic_path: String,
        game_name: &String,
        platform: &String,
        game_id: &String,
    ) -> Option<StrictPath> {
        println!("heroic::find_game_root: {heroic_path} {game_name} {platform} {game_id}");
        match platform.as_str() {
            "windows" => {
                println!(
                    "heroic::find_game_root found Heroic Windows game {} ({}), checking...",
                    game_name, game_id
                );

                // TODO.2022-10-18 struct for type safety
                let v: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(format!("{}/GamesConfig/{}.json", heroic_path, game_id))
                        .unwrap_or_default(),
                )
                .unwrap_or_default();

                println!(
                    "heroic::find_game_root found Heroic Windows game {} ({}), checking... type: {}",
                    game_name, game_id, v[&game_id]["wineVersion"]["type"],
                );

                match v[&game_id]["wineVersion"]["type"].as_str().unwrap_or_default() {
                    "wine" => {
                        println!(
                            "heroic::find_game_root found Heroic Windows prefix for {} ({}), adding... -> {}",
                            game_name,
                            game_id,
                            v[&game_id]["winePrefix"].as_str().unwrap_or_default().to_string()
                        );

                        Some(StrictPath::new(
                            v[&game_id]["winePrefix"].as_str().unwrap_or_default().to_string(),
                        ))
                    }
                    "proton" => {
                        println!(
                            "heroic::find_game_root found Heroic Proton prefix for {} ({}), adding... -> {}",
                            game_name,
                            game_id,
                            format!("{}/pfx", v[&game_id]["winePrefix"].as_str().unwrap_or_default())
                        );

                        Some(StrictPath::new(format!(
                            "{}/pfx",
                            v[&game_id]["winePrefix"].as_str().unwrap_or_default()
                        )))
                    }
                    _ => {
                        // TODO.2022-10-07 handle unknown wine types, lutris?
                        println!(
                            "heroic::find_game_root found Heroic Windows game {} ({}), checking... unknown wine_type: {:#?}",
                            game_name,
                            game_id, v[&game_id]["wineVersion"]["type"]
                        );
                        None
                    }
                }
            }
            "linux" => {
                println!(
                    "heroic::find_game_root found Heroic Linux game {}, ignoring",
                    game_name
                );
                None
            }
            _ => {
                println!(
                    "heroic::find_game_root found Heroic game {} with unhandled platform {}, ignoring.",
                    game_name, platform,
                );
                None
            }
        }
    }
}
