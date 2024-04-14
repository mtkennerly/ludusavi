use std::collections::HashMap;

use crate::{
    prelude::StrictPath,
    resource::{config::RootsConfig, manifest::Os},
    scan::{launchers::LauncherGame, TitleFinder},
};

#[derive(Clone, serde::Deserialize)]
pub struct Game {
    /// This is an opaque ID, not the human-readable title.
    pub app_name: String,
    pub title: String,
    pub platform: String,
    pub install_path: String,
}

/// installed.json
#[derive(serde::Deserialize)]
struct Library(HashMap<String, Game>);

pub fn scan(root: &RootsConfig, title_finder: &TitleFinder) -> HashMap<String, LauncherGame> {
    let mut out = HashMap::new();

    for game in get_games(&root.path) {
        let Some(official_title) = title_finder.find_one_by_normalized_name(&game.title) else {
            log::trace!("Ignoring unrecognized game: {}", &game.title);
            continue;
        };

        log::trace!(
            "Detected game: {} | app: {}, raw title: {}",
            &official_title,
            &game.app_name,
            &game.title
        );
        out.insert(
            official_title,
            LauncherGame {
                install_dir: Some(StrictPath::new(game.install_path)),
                prefix: None,
                platform: Some(Os::from(game.platform.as_str())),
            },
        );
    }

    out
}

pub fn get_games(source: &StrictPath) -> Vec<Game> {
    let mut out = vec![];

    let library = source.joined("installed.json");

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

    if let Ok(installed_games) = serde_json::from_str::<Library>(&content) {
        out.extend(installed_games.0.into_values());
    }

    out
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
            store: Store::Legendary,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(HashMap::new(), games);
    }

    #[test]
    fn scan_finds_all_games() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/legendary", repo())),
            store: Store::Legendary,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "windows-game".to_string(): LauncherGame {
                    install_dir: Some(StrictPath::new("C:\\Users\\me\\Games\\Heroic\\windows-game".to_string())),
                    prefix: None,
                    platform: Some(Os::Windows),
                },
            },
            games,
        );
    }
}
