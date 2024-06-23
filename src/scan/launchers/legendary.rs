use std::collections::{HashMap, HashSet};

use crate::{
    prelude::{StrictPath, ENV_DEBUG},
    resource::{config::root, manifest::Os},
    scan::{launchers::LauncherGame, TitleFinder},
};

pub mod installed {
    use std::collections::HashMap;

    pub const PATH: &str = "installed.json";

    #[derive(serde::Deserialize)]
    pub struct Data(pub HashMap<String, Game>);

    #[derive(Clone, serde::Deserialize)]
    pub struct Game {
        /// This is an opaque ID, not the human-readable title.
        pub app_name: String,
        pub title: String,
        pub platform: String,
        pub install_path: String,
    }
}

pub fn scan(root: &root::Legendary, title_finder: &TitleFinder) -> HashMap<String, HashSet<LauncherGame>> {
    let mut out = HashMap::<String, HashSet<LauncherGame>>::new();

    for game in get_games(&root.path) {
        let Some(official_title) = title_finder.find_one_by_normalized_name(&game.title) else {
            log::trace!("Ignoring unrecognized game: {}", &game.title);
            if std::env::var(ENV_DEBUG).is_ok() {
                eprintln!(
                    "Ignoring unrecognized game from Legendary: {} (app = {})",
                    &game.title, &game.app_name
                );
            }
            continue;
        };

        log::trace!(
            "Detected game: {} | app: {}, raw title: {}",
            &official_title,
            &game.app_name,
            &game.title
        );
        out.entry(official_title).or_default().insert(LauncherGame {
            install_dir: Some(StrictPath::new(game.install_path)),
            prefix: None,
            platform: Some(Os::from(game.platform.as_str())),
        });
    }

    out
}

pub fn get_games(source: &StrictPath) -> Vec<installed::Game> {
    let mut out = vec![];

    let library = source.joined(installed::PATH);

    let content = match library.try_read() {
        Ok(content) => content,
        Err(e) => {
            log::debug!(
                "In Legendary source '{:?}', unable to read installed.json | {:?}",
                &library,
                e,
            );
            return out;
        }
    };

    if let Ok(installed_games) = serde_json::from_str::<installed::Data>(&content) {
        out.extend(installed_games.0.into_values());
    }

    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{hash_map, hash_set};

    use super::*;
    use crate::{
        resource::{manifest::Manifest, ResourceFile},
        testing::repo,
    };

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            game-1:
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
        let root = root::Legendary {
            path: format!("{}/tests/nonexistent", repo()).into(),
        };
        let games = scan(&root, &title_finder());
        assert_eq!(HashMap::new(), games);
    }

    #[test]
    fn scan_finds_all_games() {
        let root = root::Legendary {
            path: format!("{}/tests/launchers/legendary", repo()).into(),
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "game-1".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/games/game-1".to_string())),
                    prefix: None,
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }
}
