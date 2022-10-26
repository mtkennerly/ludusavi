use std::collections::HashMap;

use crate::{
    config::RootsConfig,
    manifest::{Manifest, Store},
    prelude::{normalize_title, StrictPath},
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
    normalized_to_official: HashMap<String, String>,
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

    pub fn scan(roots: &[RootsConfig], manifest: &Manifest, legendary: Option<StrictPath>) -> Self {
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
                instance.detect_legendary_games(root, &legendary);
                instance.detect_gog_games(root);
                log::trace!("scan found: {:#?}", instance.games);
            }
        }

        instance
    }

    fn detect_legendary_games(&mut self, root: &RootsConfig, legendary: &Option<StrictPath>) {
        log::trace!("detect_legendary_games searching for legendary config...");

        // TODO.2022-10-10 heroic: windows location for legendary
        let legendary_path = match legendary {
            Some(x) => x.to_owned(),
            None => StrictPath::relative("../legendary".to_string(), Some(root.path.interpret())),
        };

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
                        // process game from GamesConfig
                        let prefix =
                            self.find_prefix(&root.path, &game.title, &game.platform.to_lowercase(), &game.app_name);
                        self.memorize_game(root, &game.title, StrictPath::new(game.install_path.clone()), prefix);
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
                    let prefix = self.find_prefix(&root.path, game_title, &game.platform, &game.app_name);
                    self.memorize_game(root, game_title, StrictPath::new(game.install_path), prefix);
                }
            }
        }
    }

    fn memorize_game(&mut self, root: &RootsConfig, title: &str, install_dir: StrictPath, prefix: Option<StrictPath>) {
        let normalized = normalize_title(title);
        if let Some(official) = self.normalized_to_official.get(&normalized) {
            log::trace!(
                "memorize_game memorizing info for {}: install_dir={:?}, prefix={:?}",
                official,
                &install_dir,
                &prefix
            );
            self.games
                .insert((root.clone(), official.clone()), MemorizedGame { install_dir, prefix });
        } else {
            // NOTE.2022-10-25 promoted to console error since this is something
            // which needs user attention (e.g. GRIP vs. GRIP: Combat Racing).
            // GUI might want to show a message / popup for this.
            eprintln!(
                "heroic::memorize_game did not find neither '{}' nor '{}' in ludusavi manifest, no backup/restore can done!",
                title, normalized
            );
            log::trace!(
                "memorize_game memorizing info for {}: install_dir={:?}, prefix={:?}",
                title,
                &install_dir,
                &prefix
            );
            self.games
                .insert((root.clone(), title.to_string()), MemorizedGame { install_dir, prefix });
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
    use super::*;
    use crate::testing::repo;
    use maplit::hashmap;
    use pretty_assertions::assert_eq;

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            windows-game:
              files:
                <base>/file1.txt: {}
            "#,
        )
        .unwrap()
    }

    #[test]
    fn scan_finds_nothing_when_folder_does_not_exist() {
        let roots = vec![RootsConfig {
            path: StrictPath::new(format!("{}/tests/nonexistent", repo())),
            store: Store::Heroic,
        }];
        let legendary = Some(StrictPath::new(format!("{}/tests/nonexistent", repo())));
        let prefixes = HeroicGames::scan(&roots, &manifest(), legendary);
        assert_eq!(HashMap::new(), prefixes.games);
    }

    #[test]
    fn scan_finds_all_games() {
        let roots = vec![RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/heroic", repo())),
            store: Store::Heroic,
        }];
        let legendary = Some(StrictPath::new(format!("{}/tests/launchers/legendary", repo())));
        let prefixes = HeroicGames::scan(&roots, &manifest(), legendary);
        assert_eq!(
            hashmap! {
                (roots[0].clone(), "windows-game".to_string()) => MemorizedGame {
                    install_dir: StrictPath::new("C:\\Users\\me\\Games\\Heroic\\windows-game".to_string()),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/windows-game".to_string())),
                },
            },
            prefixes.games,
        );
    }
}
