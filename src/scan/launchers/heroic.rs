pub mod gog;
pub mod legendary;
pub mod nile;
pub mod sideload;

use std::collections::HashMap;

use crate::prelude::StrictPath;

use crate::{
    resource::{config::RootsConfig, manifest::Os},
    scan::{launchers::LauncherGame, TitleFinder},
};

/// `GamesConfig/*.json`
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

    games.extend(legendary::scan(root, title_finder, legendary));
    games.extend(gog::scan(root, title_finder));
    games.extend(nile::scan(root, title_finder));
    games.extend(sideload::scan(root, title_finder));

    games
}

fn find_prefix(
    heroic_path: &StrictPath,
    game_name: &str,
    platform: Option<&str>,
    app_name: &str,
) -> Option<StrictPath> {
    let platform = platform?;

    match Os::from(platform) {
        Os::Windows => {
            log::trace!(
                "Will try to find prefix for Heroic Windows game: {} ({})",
                game_name,
                app_name
            );

            let games_config_path = heroic_path.joined("GamesConfig").joined(&format!("{app_name}.json"));
            match serde_json::from_str::<GamesConfigWrapper>(&games_config_path.read().unwrap_or_default()) {
                Ok(games_config_wrapper) => {
                    let game_config = games_config_wrapper.0.get(app_name)?;

                    match game_config {
                        GamesConfig::Config {
                            wine_version,
                            wine_prefix,
                        } => match wine_version.wine_type.as_str() {
                            "wine" => {
                                log::trace!(
                                    "Found Heroic Wine prefix for {} ({}) -> adding {}",
                                    game_name,
                                    app_name,
                                    wine_prefix
                                );
                                Some(StrictPath::new(wine_prefix.clone()))
                            }

                            "proton" => {
                                let prefix = format!("{}/pfx", wine_prefix);
                                log::trace!(
                                    "Found Heroic Proton prefix for {} ({}), adding {}",
                                    game_name,
                                    app_name,
                                    &prefix
                                );
                                Some(StrictPath::new(prefix))
                            }

                            _ => {
                                log::info!(
                                    "Found Heroic Windows game {} ({}) with unknown wine_type: {:#?}",
                                    game_name,
                                    app_name,
                                    wine_version.wine_type
                                );
                                None
                            }
                        },
                        GamesConfig::IgnoreOther(_) => None,
                    }
                }
                Err(e) => {
                    log::trace!("Failed to read {:?}: {}", &games_config_path, e);
                    None
                }
            }
        }

        Os::Linux => {
            log::trace!("Found Heroic Linux game {}, ignoring prefix", game_name);
            None
        }

        Os::Mac => {
            log::trace!("Found Heroic Mac game {}, ignoring prefix", game_name);
            None
        }

        _ => {
            log::trace!(
                "Found Heroic game {} with unhandled platform {}, ignoring prefix",
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
            manifest::{Manifest, Os, Store},
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
