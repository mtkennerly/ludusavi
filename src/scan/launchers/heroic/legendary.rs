use std::collections::{HashMap, HashSet};

use crate::prelude::StrictPath;

use crate::{
    prelude::ENV_DEBUG,
    resource::{config::root, manifest::Os},
    scan::{
        launchers::{heroic::find_prefix, legendary as legendary_standalone, LauncherGame},
        TitleFinder,
    },
};

pub fn scan(
    root: &root::Heroic,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, HashSet<LauncherGame>> {
    let mut games = HashMap::<String, HashSet<LauncherGame>>::new();

    for game in get_installed(root, legendary) {
        let Some(official_title) = title_finder.find_one_by_normalized_name(&game.title) else {
            log::trace!("Ignoring unrecognized game: {}, app: {}", &game.title, &game.app_name);
            if std::env::var(ENV_DEBUG).is_ok() {
                eprintln!(
                    "Ignoring unrecognized game from Heroic/Legendary: {} (app = {})",
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
        let prefix = find_prefix(&root.path, &game.title, Some(&game.platform), &game.app_name);
        games.entry(official_title).or_default().insert(LauncherGame {
            install_dir: Some(StrictPath::new(game.install_path.clone())),
            prefix,
            platform: Some(Os::from(game.platform.as_str())),
        });
    }

    games
}

pub fn get_installed(
    root: &root::Heroic,
    legendary: Option<&StrictPath>,
) -> Vec<legendary_standalone::installed::Game> {
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
        out.extend(legendary_standalone::get_games(&legendary_path));
    }

    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{hash_map, hash_set};

    use super::*;
    use crate::{
        resource::{
            manifest::{Manifest, Os},
            ResourceFile,
        },
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
    fn scan_finds_all_games() {
        let root = root::Heroic {
            path: format!("{}/tests/launchers/heroic-gog-without-store-cache", repo()).into(),
        };
        let legendary = Some(StrictPath::new(format!("{}/tests/launchers/legendary", repo())));
        let games = scan(&root, &title_finder(), legendary.as_ref());
        assert_eq!(
            hash_map! {
                "game-1".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/games/game-1".to_string())),
                    prefix: Some(StrictPath::new("/prefixes/game-1".to_string())),
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }
}
