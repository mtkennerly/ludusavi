use std::collections::HashMap;

use crate::{
    config::RootsConfig,
    manifest::{Manifest, Store},
    prelude::{normalize_title, StrictPath},
};

// Deserialization of Heroic gog_store/installed.json
#[derive(serde::Deserialize)]
struct HeroicInstalledGame {
    #[serde(rename = "appName")]
    game_id: String,
    platform: String,
}
#[derive(serde::Deserialize)]
struct HeroicInstalled {
    installed: Vec<HeroicInstalledGame>,
}

// Deserialization of Heroic gog_store/library.json
#[derive(serde::Deserialize)]
struct GogLibraryGame {
    app_name: String,
    title: String,
}
#[derive(serde::Deserialize)]
struct GogLibrary {
    games: Vec<GogLibraryGame>,
}

// Deserialization of Legendary legendary/installed.json
#[derive(serde::Deserialize)]
struct LegendaryInstalledGame {
    #[serde(rename = "app_name")]
    game_id: String,
    title: String,
    platform: String,
}
#[derive(serde::Deserialize)]
struct LegendaryInstalled(HashMap<String, LegendaryInstalledGame>);

// TODO.2022-10-10 heroic: windows location for legendary
// TODO.2022-10-15 heroic: make relative to heroic root in question!
const LEGENDARY_PATHS: &[&str] = &[
    "~/.config/legendary",
    // TODO.2022-10-20 heroic: flatpak install is not supported yet
    // "~/.var/app/com.heroicgameslauncher.hgl/config/legendary",
];

#[derive(Clone, Default, Debug)]
pub struct HeroicGames {
    games: HashMap<String, StrictPath>,
    normalized_to_official: HashMap<String, String>,
}

impl HeroicGames {
    pub fn get(&self, game: &str) -> Option<&StrictPath> {
        self.games.get(game)
    }

    pub fn scan(roots: &[RootsConfig], manifest: &Manifest) -> Self {
        let mut instance = HeroicGames {
            normalized_to_official: manifest
                .0
                .keys()
                .map(|title| (normalize_title(title), title.clone()))
                .collect(),
            ..Default::default()
        };

        for root in roots {
            if root.store == Store::Heroic {
                instance.detect_legendary_games(root);
                instance.detect_gog_games(root);
                log::trace!("scan found: {:#?}", instance.games);
            }
        }

        instance
    }


    fn detect_legendary_games(&mut self, root: &RootsConfig) {
        log::trace!("detect_legendary_games searching for legendary config...");

        for &legendary_path_candidate in LEGENDARY_PATHS {
            let legendary_path = StrictPath::new(legendary_path_candidate.to_string());
            if !legendary_path.is_dir() {
                continue;
            }

            log::trace!(
                "detect_legendary_games found legendary configuration in {}",
                legendary_path.interpret()
            );

            let mut legendary_installed = legendary_path.as_std_path_buf();
            legendary_installed.push("installed.json");
            if legendary_installed.is_file() {
                // read list of installed games and call find_prefix for result
                if let Ok(installed_games) = serde_json::from_str::<LegendaryInstalled>(
                    &std::fs::read_to_string(legendary_installed).unwrap_or_default(),
                ) {
                    for game in installed_games.0.values() {
                        log::trace!("detect_legendary_games found game {} ({})", game.title, game.game_id);
                        // process game from GamesConfig
                        if let Some(sp) = self.find_prefix(
                            &root.path.interpret(),
                            &game.title,
                            &game.platform.to_lowercase(),
                            &game.game_id,
                        ) {
                            self.memorize_prefix(&game.title, &sp);
                        }
                    }
                }
            } else {
                log::trace!(
                    "detect_legendary_games no such file '{:?}', legendary probably not used yet... skipping",
                    legendary_installed
                );
            }
        }
    }

    fn detect_gog_games(&mut self, root: &RootsConfig) {
        log::trace!(
            "detect_gog_games searching for GOG information in {}",
            root.path.interpret()
        );

        // use gog_store/library.json to build map .app_name -> .title
        let game_titles: std::collections::HashMap<String, String>;
        let library_path = format!("{}/gog_store/library.json", root.path.interpret());
        match serde_json::from_str::<GogLibrary>(&std::fs::read_to_string(&library_path).unwrap_or_default()) {
            Ok(gog_library) => {
                game_titles = gog_library
                    .games
                    .iter()
                    .map(|game| (game.app_name.clone(), game.title.clone()))
                    .collect();
            }
            Err(e) => {
                log::warn!(
                    "detect_gog_games aborting since it could not read {}: {}",
                    library_path,
                    e
                );
                return;
            }
        }
        log::trace!("detect_gog_games found {} games in {}", game_titles.len(), library_path);

        // iterate over all games found in HEROCONFIGDIR/gog_store/installed.json and call find_prefix
        let content = std::fs::read_to_string(format!("{}/gog_store/installed.json", root.path.interpret()));
        if let Ok(installed_games) = serde_json::from_str::<HeroicInstalled>(&content.unwrap_or_default()) {
            for game in installed_games.installed {
                if let Some(game_title) = game_titles.get(&game.game_id) {
                    if let Some(sp) =
                        self.find_prefix(&root.path.interpret(), game_title, &game.platform, &game.game_id)
                    {
                        self.memorize_prefix(game_title, &sp);
                    }
                }
            }
        }
    }

    fn memorize_prefix(&mut self, title: &str, path: &StrictPath) {
        let normalized = normalize_title(title);
        if let Some(official) = self.normalized_to_official.get(&normalized) {
            log::trace!(
                "memorize_prefix memorizing path {} for {}",
                path.interpret(),
                official
            );
            self.games.insert(official.clone(), path.clone());
        } else {
            log::info!(
                "memorize_prefix did not find {} in manifest, no backup/restore will be done!",
                title
            );
            log::trace!("memorize_prefix memorizing path {} for {}", path.interpret(), title);
            self.games.insert(title.to_string(), path.clone());
        }
    }

    fn find_prefix(
        &self,
        heroic_path: &str,
        game_name: &str,
        platform: &str,
        game_id: &str,
    ) -> Option<StrictPath> {
        match platform {
            "windows" => {
                // no struct for type safety used here since GamesConfig use the game id as a key name
                let v: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(format!("{}/GamesConfig/{}.json", heroic_path, game_id))
                        .unwrap_or_default(),
                )
                .unwrap_or_default();

                match v[&game_id]["wineVersion"]["type"].as_str().unwrap_or_default() {
                    "wine" => {
                        log::trace!(
                            "find_prefix found Heroic Wine prefix for {} ({}) -> adding {}",
                            game_name,
                            game_id,
                            v[&game_id]["winePrefix"].as_str().unwrap_or_default().to_string()
                        );
                        Some(StrictPath::new(
                            v[&game_id]["winePrefix"].as_str().unwrap_or_default().to_string(),
                        ))
                    }

                    "proton" => {
                        log::trace!(
                            "find_prefix found Heroic Proton prefix for {} ({}), adding... -> {}",
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
                        log::warn!(
                            "find_prefix found Heroic Windows game {} ({}), checking... unknown wine_type: {:#?}",
                            game_name,
                            game_id,
                            v[&game_id]["wineVersion"]["type"]
                        );
                        None
                    }
                }
            }

            "linux" => {
                log::trace!("find_prefix found Heroic Linux game {}, ignoring", game_name);
                None
            }

            _ => {
                log::trace!(
                    "find_prefix found Heroic game {} with unhandled platform {}, ignoring.",
                    game_name,
                    platform,
                );
                None
            }
        }
    }
}
