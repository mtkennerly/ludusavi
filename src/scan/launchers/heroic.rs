use std::collections::HashMap;

use crate::prelude::{StrictPath, ENV_DEBUG};

use crate::{
    resource::{config::RootsConfig, manifest::Os},
    scan::{
        launchers::{legendary, LauncherGame},
        TitleFinder, TitleQuery,
    },
};

/// Deserialization of Heroic gog_store/installed.json
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

/// Deserialization of Heroic gog_store/library.json
#[derive(serde::Deserialize)]
pub struct GogLibraryGame {
    /// This is an opaque ID, not the human-readable title.
    pub app_name: String,
    pub title: String,
}
#[derive(serde::Deserialize)]
struct GogLibrary {
    games: Vec<GogLibraryGame>,
}

/// Deserialization of Heroic GamesConfig/*.json
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

pub fn scan(
    root: &RootsConfig,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, LauncherGame> {
    let mut games = HashMap::new();

    games.extend(detect_legendary_games(root, title_finder, legendary));
    games.extend(detect_gog_games(root, title_finder));

    games
}

pub fn get_legendary_installed_games(root: &RootsConfig, legendary: Option<&StrictPath>) -> Vec<legendary::Game> {
    log::trace!("detect_legendary_games searching for legendary config...");
    let mut out = vec![];

    let legendary_paths = match legendary {
        None => vec![
            root.path.popped().joined("legendary"),
            root.path.joined("legendaryConfig/legendary"),
            StrictPath::new("~/.config/legendary".to_string()),
        ],
        Some(x) => vec![x.clone()],
    };

    for legendary_path in legendary_paths {
        out.extend(legendary::get_games(&legendary_path));
    }

    out
}

fn detect_legendary_games(
    root: &RootsConfig,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, LauncherGame> {
    let mut games = HashMap::new();

    for game in get_legendary_installed_games(root, legendary) {
        log::trace!(
            "detect_legendary_games found legendary game {} ({})",
            game.title,
            game.app_name
        );
        let official_title = title_finder.find_one_by_normalized_name(&game.title);
        // process game from GamesConfig
        let prefix = find_prefix(&root.path, &game.title, &game.platform.to_lowercase(), &game.app_name);
        memorize_game(
            &mut games,
            &game.title,
            official_title,
            StrictPath::new(game.install_path.clone()),
            prefix,
            &game.platform,
        );
    }

    games
}

pub fn get_gog_games_library(root: &RootsConfig) -> Option<Vec<GogLibraryGame>> {
    log::trace!("get_gog_library searching for GOG information in {:?}", &root.path);

    // use library.json to build map .app_name -> .title
    let libraries = [
        root.path.joined("store_cache").joined("gog_library.json"),
        root.path.joined("gog_store").joined("library.json"),
    ];
    let library_path: StrictPath;
    'library: {
        for library in libraries {
            if library.is_file() {
                library_path = library;
                break 'library;
            }
        }
        log::info!("get_gog_library could not find GOG library");
        return None;
    }
    match serde_json::from_str::<GogLibrary>(&library_path.read().unwrap_or_default()) {
        Ok(gog_library) => {
            log::trace!(
                "get_gog_library found {} games in {:?}",
                gog_library.games.len(),
                &library_path
            );

            Some(gog_library.games)
        }
        Err(e) => {
            log::warn!(
                "get_gog_library returns None since it could not read {:?}: {}",
                &library_path,
                e
            );
            None
        }
    }
}

fn detect_gog_games(root: &RootsConfig, title_finder: &TitleFinder) -> HashMap<String, LauncherGame> {
    let mut games = HashMap::new();

    log::trace!("detect_gog_games searching for GOG information in {:?}", &root.path);

    let game_titles: HashMap<String, String> = match get_gog_games_library(root) {
        Some(gog_games_library) => gog_games_library
            .iter()
            .map(|game| (game.app_name.clone(), game.title.clone()))
            .collect(),
        None => {
            log::warn!("detect_gog_games aborting since gog_library was not read successfully.");
            return games;
        }
    };

    // iterate over all games found in HEROCONFIGDIR/gog_store/installed.json and call find_prefix
    let installed_path = root.path.joined("gog_store").joined("installed.json");
    let content = installed_path.read();
    if let Ok(installed_games) = serde_json::from_str::<HeroicInstalled>(&content.unwrap_or_default()) {
        for game in installed_games.installed {
            if let Some(game_title) = game_titles.get(&game.app_name) {
                let gog_id: Option<u64> = game.app_name.parse().ok();
                let official_title = title_finder.find_one(TitleQuery {
                    names: vec![game_title.to_owned()],
                    gog_id,
                    normalized: true,
                    ..Default::default()
                });
                let prefix = find_prefix(&root.path, game_title, &game.platform, &game.app_name);
                memorize_game(
                    &mut games,
                    game_title,
                    official_title,
                    StrictPath::new(game.install_path),
                    prefix,
                    &game.platform,
                );
            }
        }
    }

    games
}

fn memorize_game(
    games: &mut HashMap<String, LauncherGame>,
    heroic_title: &str,
    official_title: Option<String>,
    install_dir: StrictPath,
    prefix: Option<StrictPath>,
    platform: &str,
) {
    let platform = Some(Os::from(platform));

    log::trace!(
        "memorize_game memorizing info for '{:?}' (from: '{}'): install_dir={:?}, prefix={:?}",
        &official_title,
        heroic_title,
        &install_dir,
        &prefix
    );

    if let Some(official) = official_title {
        games.insert(
            official,
            LauncherGame {
                install_dir: Some(install_dir),
                prefix,
                platform,
            },
        );
    } else {
        // Handling game name mismatches, e.g. GRIP vs. GRIP: Combat Racing
        let log_message = format!("Ignoring unrecognized Heroic game: '{}'", heroic_title);
        if std::env::var(ENV_DEBUG).is_ok() {
            eprintln!("{}", &log_message);
        }
        log::info!("{}", &log_message);

        games.insert(
            heroic_title.to_string(),
            LauncherGame {
                install_dir: Some(install_dir),
                prefix,
                platform,
            },
        );
    }
}

fn find_prefix(heroic_path: &StrictPath, game_name: &str, platform: &str, app_name: &str) -> Option<StrictPath> {
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::hash_map;

    use super::*;
    use crate::{
        resource::{
            manifest::{Manifest, Store},
            ResourceFile,
        },
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
        TitleFinder::new(&Default::default(), &manifest(), Default::default())
    }

    #[test]
    fn scan_finds_nothing_when_folder_does_not_exist() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/nonexistent", repo())),
            store: Store::Heroic,
        };
        let legendary = Some(StrictPath::new(format!("{}/tests/nonexistent", repo())));
        let games = scan(&root, &title_finder(), legendary.as_ref());
        assert_eq!(HashMap::new(), games);
    }

    #[test]
    fn scan_finds_all_games_without_store_cache() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/heroic-without-store-cache", repo())),
            store: Store::Heroic,
        };
        let legendary = Some(StrictPath::new(format!("{}/tests/launchers/legendary", repo())));
        let games = scan(&root, &title_finder(), legendary.as_ref());
        assert_eq!(
            hash_map! {
                "windows-game".to_string(): LauncherGame {
                    install_dir: Some(StrictPath::new("C:\\Users\\me\\Games\\Heroic\\windows-game".to_string())),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/windows-game".to_string())),
                    platform: Some(Os::Windows),
                },
                "proton-game".to_string(): LauncherGame {
                    install_dir: Some(StrictPath::new("/home/root/Games/proton-game".to_string())),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/proton-game/pfx".to_string())),
                    platform: Some(Os::Windows),
                },
            },
            games,
        );
    }

    #[test]
    fn scan_finds_all_games_with_store_cache() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/heroic-with-store-cache", repo())),
            store: Store::Heroic,
        };
        let legendary = Some(StrictPath::new(format!("{}/tests/launchers/legendary", repo())));
        let games = scan(&root, &title_finder(), legendary.as_ref());
        assert_eq!(
            hash_map! {
                "windows-game".to_string(): LauncherGame {
                    install_dir: Some(StrictPath::new("C:\\Users\\me\\Games\\Heroic\\windows-game".to_string())),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/windows-game".to_string())),
                    platform: Some(Os::Windows),
                },
                "proton-game".to_string(): LauncherGame {
                    install_dir: Some(StrictPath::new("/home/root/Games/proton-game".to_string())),
                    prefix: Some(StrictPath::new("/home/root/Games/Heroic/Prefixes/proton-game/pfx".to_string())),
                    platform: Some(Os::Windows),
                },
            },
            games,
        );
    }
}
