use std::collections::HashMap;

use crate::prelude::StrictPath;

use crate::{
    prelude::ENV_DEBUG,
    resource::{config::RootsConfig, manifest::Os},
    scan::{
        launchers::{heroic::find_prefix, legendary as legendary_standalone, LauncherGame},
        TitleFinder,
    },
};

pub fn scan(
    root: &RootsConfig,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, LauncherGame> {
    let mut games = HashMap::new();

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
        games.insert(
            official_title,
            LauncherGame {
                install_dir: Some(StrictPath::new(game.install_path.clone())),
                prefix,
                platform: Some(Os::from(game.platform.as_str())),
            },
        );
    }

    games
}

pub fn get_installed(root: &RootsConfig, legendary: Option<&StrictPath>) -> Vec<legendary_standalone::Game> {
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
