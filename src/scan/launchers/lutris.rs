use std::collections::HashMap;

use crate::{
    prelude::{StrictPath, ENV_DEBUG},
    resource::{config::RootsConfig, manifest::Os},
    scan::{LauncherGame, TitleFinder},
    wrap,
};

/// https://github.com/lutris/lutris/blob/e4ae3d7193da777ebb370603a9e20c435f725300/docs/installers.rst
#[derive(serde::Deserialize)]
struct LutrisGame {
    game: GameSection,
    /// ID of the game itself.
    game_slug: Option<String>,
    /// Human-readable.
    name: Option<String>,
}

#[derive(serde::Deserialize)]
struct GameSection {
    exe: Option<StrictPath>,
    prefix: Option<StrictPath>,
    working_dir: Option<StrictPath>,
}

pub fn scan(root: &RootsConfig, title_finder: &TitleFinder) -> HashMap<String, LauncherGame> {
    let mut games = HashMap::new();

    log::trace!("Scanning Lutris root for games: {:?}", &root.path);

    for spec_path in root.path.joined("games/*.y*ml").glob() {
        log::debug!("Inspecting Lutris game file: {}", spec_path.render());

        let Some(content) = spec_path.read() else {
            log::warn!("Unable to read Lutris game file: {}", spec_path.render());
            continue;
        };

        let Ok(spec) = serde_yaml::from_str::<LutrisGame>(&content) else {
            log::warn!("Unable to parse Lutris game file: {}", spec_path.render());
            continue;
        };

        if let Some((title, game)) = scan_spec(spec, &spec_path, title_finder) {
            games.insert(title, game);
        }
    }

    if let Some(metadata) = wrap::lutris::infer_metadata() {
        games.insert(
            metadata.title,
            LauncherGame {
                platform: metadata.prefix.is_some().then_some(Os::Windows),
                install_dir: metadata.base,
                prefix: metadata.prefix,
            },
        );
    }

    log::trace!("Finished scanning Lutris root for games: {:?}", &root.path);

    games
}

fn scan_spec(spec: LutrisGame, spec_path: &StrictPath, title_finder: &TitleFinder) -> Option<(String, LauncherGame)> {
    let Some(name) = spec.name.clone() else {
        log::info!("Skipping Lutris game file without `name` field: {}", spec_path.render());
        return None;
    };

    let official_title = title_finder.find_one_by_normalized_name(&name);
    let prefix = spec.game.prefix;
    let platform = Some(match &prefix {
        Some(_) => Os::Windows,
        None => Os::HOST,
    });

    let title = match official_title {
        Some(title) => {
            log::trace!(
                "Recognized Lutris game: '{title}' from '{}' (slug: '{:?}')",
                &name,
                spec.game_slug.as_ref(),
            );
            title
        }
        None => {
            let log_message = format!(
                "Unrecognized Lutris game: '{}' (slug: '{:?}')",
                &name,
                spec.game_slug.as_ref()
            );
            if std::env::var(ENV_DEBUG).is_ok() {
                eprintln!("{log_message}");
            }
            log::info!("{log_message}");
            name
        }
    };

    let install_dir = if let Some(working_dir) = spec.game.working_dir {
        working_dir
    } else if let Some(exe) = spec.game.exe {
        let exe = if exe.is_absolute() {
            exe
        } else if let Some(prefix) = &prefix {
            prefix.joined(&exe.raw())
        } else {
            log::info!(
                "Skipping Lutris game file with relative exe and no prefix: {}",
                spec_path.render()
            );
            return None;
        };

        if let Some(parent) = exe.parent_raw() {
            parent
        } else {
            log::info!(
                "Skipping Lutris game file with indeterminate parent folder of exe: {}",
                spec_path.render()
            );
            return None;
        }
    } else {
        log::info!(
            "Skipping Lutris game file without `working_dir` and `exe` fields: {}",
            spec_path.render()
        );
        return None;
    };

    Some((
        title,
        LauncherGame {
            install_dir: Some(install_dir),
            prefix,
            platform,
        },
    ))
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
        testing::{absolute_path, repo},
    };

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            windows-game:
              files:
                <base>/file1.txt: {}
            windows-game-with-absolute-exe:
              files:
                  <base>/file1.txt: {}
            windows-game-with-relative-exe:
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
            store: Store::Lutris,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(HashMap::new(), games);
    }

    #[test]
    fn scan_finds_all_games() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/lutris", repo())),
            store: Store::Lutris,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "windows-game".to_string(): LauncherGame {
                    install_dir: Some(StrictPath::new("/home/deck/Games/service/windows-game/drive_c/game".to_string())),
                    prefix: Some(StrictPath::new("/home/deck/Games/service/windows-game".to_string())),
                    platform: Some(Os::Windows),
                },
            },
            games,
        );
    }

    #[test]
    fn can_scan_spec_with_absolute_exe() {
        let spec = LutrisGame {
            game: GameSection {
                exe: Some(absolute_path("/install/drive_c/game/launcher.exe")),
                prefix: Some(absolute_path("/prefix")),
                working_dir: None,
            },
            game_slug: None,
            name: Some("Windows Game with Absolute Exe".into()),
        };
        assert_eq!(
            Some((
                "windows-game-with-absolute-exe".into(),
                LauncherGame {
                    install_dir: Some(absolute_path("/install/drive_c/game")),
                    prefix: Some(absolute_path("/prefix")),
                    platform: Some(Os::Windows),
                }
            )),
            scan_spec(spec, &absolute_path("/tmp"), &title_finder()),
        );
    }

    #[test]
    fn can_scan_spec_with_relative_exe_but_prefix() {
        let spec = LutrisGame {
            game: GameSection {
                exe: Some(StrictPath::new("drive_c/game/launcher.exe".into())),
                prefix: Some(absolute_path("/prefix")),
                working_dir: None,
            },
            game_slug: None,
            name: Some("Windows Game with Relative Exe".into()),
        };
        assert_eq!(
            Some((
                "windows-game-with-relative-exe".into(),
                LauncherGame {
                    install_dir: Some(absolute_path("/prefix/drive_c/game")),
                    prefix: Some(absolute_path("/prefix")),
                    platform: Some(Os::Windows),
                }
            )),
            scan_spec(spec, &absolute_path("/tmp"), &title_finder()),
        );
    }
}
