use std::collections::HashMap;

use crate::{
    prelude::StrictPath,
    resource::{config::RootsConfig, manifest::Store},
    scan::TitleFinder,
};

//
/// Deserialization of Heroic gog_store/installed.json
//
#[derive(serde::Deserialize)]
struct HeroicInstalledGame {
    /// This is an opaque ID, not the human-readable title.
    #[serde(rename = "appName")]
    app_name: String,
    platform: String,
    install_path: String,
}
#[derive(serde::Deserialize)]
struct HeroicInstalled {
    installed: Vec<HeroicInstalledGame>,
}

//
/// Deserialization of Heroic gog_store/library.json
//
#[derive(serde::Deserialize)]
struct GogLibraryGame {
    /// This is an opaque ID, not the human-readable title.
    app_name: String,
    title: String,
}
#[derive(serde::Deserialize)]
struct GogLibrary {
    games: Vec<GogLibraryGame>,
}

//
/// Deserialization of Legendary legendary/installed.json
//
#[derive(serde::Deserialize)]
struct LegendaryInstalledGame {
    /// This is an opaque ID, not the human-readable title.
    #[serde(rename = "app_name")]
    app_name: String,
    title: String,
    platform: String,
    install_path: String,
}
#[derive(serde::Deserialize)]
struct LegendaryInstalled(HashMap<String, LegendaryInstalledGame>);

//
/// Deserialization of Heroic GamesConfig/*.json
//
#[derive(serde::Deserialize, Debug)]
struct GamesConfigWrapper(HashMap<String, GamesConfig>);

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
enum GamesConfig {
    Config {
        #[serde(rename = "winePrefix")]
        wine_prefix: String,
        #[serde(rename = "wineVersion")]
        wine_version: GamesConfigWine,
    },
    IgnoreOther(serde::de::IgnoredAny),
}
#[derive(serde::Deserialize, Debug)]
struct GamesConfigWine {
    #[serde(rename = "type")]
    wine_type: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct MemorizedGame {
    install_dir: StrictPath,
    prefix: Option<StrictPath>,
}

//
/// Main structure where games installed with heroic are collected
//
#[derive(Clone, Default, Debug)]
pub struct HeroicGames {
    games: HashMap<(RootsConfig, String), MemorizedGame>,
}

impl HeroicGames {
    pub fn get_prefix(&self, root: &RootsConfig, game: &str) -> Option<&StrictPath> {
        // NOTE.2022-10-23 unusual to clone keys just for lookup, references
        // should be good enough
        self.games
            .get(&(root.clone(), game.to_string()))
            .and_then(|x| x.prefix.as_ref())
    }

    pub fn get_install_dir(&self, root: &RootsConfig, game: &str) -> Option<&StrictPath> {
        self.games
            .get(&(root.clone(), game.to_string()))
            .map(|x| &x.install_dir)
    }

    pub fn scan(roots: &[RootsConfig], title_finder: &TitleFinder, legendary: Option<StrictPath>) -> Self {
        let mut instance = HeroicGames::default();

        for root in roots {
            if root.store == Store::Heroic {
                instance.detect_legendary_games(root, title_finder, &legendary);
                instance.detect_gog_games(root, title_finder);
                log::trace!("scan found: {:#?}", instance.games);
            }
        }

        instance
    }

    fn detect_legendary_games(
        &mut self,
        root: &RootsConfig,
        title_finder: &TitleFinder,
        legendary: &Option<StrictPath>,
    ) {
        log::trace!("detect_legendary_games searching for legendary config...");

        let legendary_paths = match legendary {
            None => vec![
                StrictPath::relative("../legendary".to_string(), Some(root.path.interpret())),
                StrictPath::new("~/.config/legendary".to_string()),
            ],
            Some(x) => vec![x.clone()],
        };

        for legendary_path in legendary_paths {
            if legendary_path.is_dir() {
                log::trace!(
                    "detect_legendary_games checking for legendary configuration in {}",
                    legendary_path.interpret()
                );

                let legendary_installed = legendary_path.joined("installed.json");
                if legendary_installed.is_file() {
                    // read list of installed games and call find_prefix for result
                    if let Ok(installed_games) =
                        serde_json::from_str::<LegendaryInstalled>(&legendary_installed.read().unwrap_or_default())
                    {
                        for game in installed_games.0.values() {
                            log::trace!(
                                "detect_legendary_games found legendary game {} ({})",
                                game.title,
                                game.app_name
                            );
                            let official_title =
                                title_finder.find_one(&[game.title.to_owned()], &None, &None, true, true, false);
                            // process game from GamesConfig
                            let prefix = self.find_prefix(
                                &root.path,
                                &game.title,
                                &game.platform.to_lowercase(),
                                &game.app_name,
                            );
                            self.memorize_game(
                                root,
                                &game.title,
                                official_title,
                                StrictPath::new(game.install_path.clone()),
                                prefix,
                            );
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
    }

    fn detect_gog_games(&mut self, root: &RootsConfig, title_finder: &TitleFinder) {
        log::trace!(
            "detect_gog_games searching for GOG information in {}",
            root.path.interpret()
        );

        // use gog_store/library.json to build map .app_name -> .title
        let library_path = root.path.joined("gog_store").joined("library.json");
        let game_titles: std::collections::HashMap<String, String> =
            match serde_json::from_str::<GogLibrary>(&library_path.read().unwrap_or_default()) {
                Ok(gog_library) => gog_library
                    .games
                    .iter()
                    .map(|game| (game.app_name.clone(), game.title.clone()))
                    .collect(),
                Err(e) => {
                    log::warn!(
                        "detect_gog_games aborting since it could not read {}: {}",
                        library_path.interpret(),
                        e
                    );
                    return;
                }
            };
        log::trace!(
            "detect_gog_games found {} games in {}",
            game_titles.len(),
            library_path.interpret()
        );

        // iterate over all games found in HEROCONFIGDIR/gog_store/installed.json and call find_prefix
        let installed_path = root.path.joined("gog_store").joined("installed.json");
        let content = installed_path.read();
        if let Ok(installed_games) = serde_json::from_str::<HeroicInstalled>(&content.unwrap_or_default()) {
            for game in installed_games.installed {
                if let Some(game_title) = game_titles.get(&game.app_name) {
                    let gog_id: Option<u64> = game.app_name.parse().ok();
                    let official_title =
                        title_finder.find_one(&[game_title.to_owned()], &None, &gog_id, true, true, false);
                    let prefix = self.find_prefix(&root.path, game_title, &game.platform, &game.app_name);
                    self.memorize_game(
                        root,
                        game_title,
                        official_title,
                        StrictPath::new(game.install_path),
                        prefix,
                    );
                }
            }
        }
    }

    fn memorize_game(
        &mut self,
        root: &RootsConfig,
        heroic_title: &str,
        official_title: Option<String>,
        install_dir: StrictPath,
        prefix: Option<StrictPath>,
    ) {
        if let Some(official) = official_title {
            log::trace!(
                "memorize_game memorizing info for '{}' (from: '{}'): install_dir={:?}, prefix={:?}",
                official,
                heroic_title,
                &install_dir,
                &prefix
            );
            self.games
                .insert((root.clone(), official), MemorizedGame { install_dir, prefix });
        } else {
            // Handling game name mismatches, e.g. GRIP vs. GRIP: Combat Racing
            let log_message = format!("Ignoring unrecognized Heroic game: '{}'", heroic_title);
            if std::env::var("LUDUSAVI_DEBUG").is_ok() {
                eprintln!("{}", &log_message);
            }
            log::info!("{}", &log_message);

            log::trace!(
                "memorize_game memorizing info for '{}': install_dir={:?}, prefix={:?}",
                heroic_title,
                &install_dir,
                &prefix
            );
            self.games.insert(
                (root.clone(), heroic_title.to_string()),
                MemorizedGame { install_dir, prefix },
            );
        }
    }

    fn find_prefix(
        &self,
        heroic_path: &StrictPath,
        game_name: &str,
        platform: &str,
        app_name: &str,
    ) -> Option<StrictPath> {
        match platform {
            "windows" => {
                log::trace!(
                    "find_prefix found Heroic Windows game {}, looking closer ...",
                    game_name
                );

                let games_config_path = heroic_path.joined("GamesConfig").joined(&format!("{app_name}.json"));
                match serde_json::from_str::<GamesConfigWrapper>(&games_config_path.read().unwrap_or_default()) {
                    Ok(games_config_wrapper) => {
                        if let Some(game_config) = games_config_wrapper.0.get(app_name) {
                            match game_config {
                                GamesConfig::Config {
                                    wine_version,
                                    wine_prefix,
                                } => match wine_version.wine_type.as_str() {
                                    "wine" => {
                                        log::trace!(
                                            "find_prefix found Heroic Wine prefix for {} ({}) -> adding {}",
                                            game_name,
                                            app_name,
                                            wine_prefix
                                        );
                                        Some(StrictPath::new(wine_prefix.clone()))
                                    }

                                    "proton" => {
                                        log::trace!(
                                            "find_prefix found Heroic Proton prefix for {} ({}), adding... -> {}",
                                            game_name,
                                            app_name,
                                            format!("{}/pfx", wine_prefix)
                                        );
                                        Some(StrictPath::new(format!("{}/pfx", wine_prefix)))
                                    }

                                    _ => {
                                        log::info!(
                                                "find_prefix found Heroic Windows game {} ({}), checking... unknown wine_type: {:#?} -> ignored",
                                                game_name,
                                                app_name,
                                                wine_version.wine_type
                                            );
                                        None
                                    }
                                },
                                GamesConfig::IgnoreOther(_) => None,
                            }
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        log::trace!("find_prefix error: '{}', ignoring", e);
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

#[cfg(test)]
mod tests {
    use maplit::hashmap;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        resource::{manifest::Manifest, ResourceFile},
        testing::repo,
    };

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            windows-game:
              files:
                <base>/file1.txt: {}
            proton-game:
              files:
                <base>/file1.txt: {}
            "#,
        )
        .unwrap()
    }

    fn title_finder() -> TitleFinder {
        TitleFinder::new(&manifest(), &Default::default())
    }

    #[test]
    fn scan_finds_nothing_when_folder_does_not_exist() {
        let roots = vec![RootsConfig {
            path: StrictPath::new(format!("{}/tests/nonexistent", repo())),
            store: Store::Heroic,
        }];
        let legendary = Some(StrictPath::new(format!("{}/tests/nonexistent", repo())));
        let prefixes = HeroicGames::scan(&roots, &title_finder(), legendary);
        assert_eq!(HashMap::new(), prefixes.games);
    }

    #[test]
    fn scan_finds_all_games() {
        let roots = vec![RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/heroic", repo())),
            store: Store::Heroic,
        }];
        let legendary = Some(StrictPath::new(format!("{}/tests/launchers/legendary", repo())));
        let prefixes = HeroicGames::scan(&roots, &title_finder(), legendary);
        assert_eq!(
            hashmap! {
                (roots[0].clone(), "windows-game".to_string()) => MemorizedGame {
                    install_dir: StrictPath::new("C:\\Users\\me\\Games\\Heroic\\windows-game".to_string()),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/windows-game".to_string())),
                },
                (roots[0].clone(), "proton-game".to_string()) => MemorizedGame {
                    install_dir: StrictPath::new("/home/root/Games/proton-game".to_string()),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/proton-game/pfx".to_string())),
                },
            },
            prefixes.games,
        );
    }
}
