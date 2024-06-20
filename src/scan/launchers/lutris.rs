use std::collections::{HashMap, HashSet};

use crate::{
    prelude::StrictPath,
    resource::{config::RootsConfig, manifest::Os},
    scan::{LauncherGame, TitleFinder, TitleQuery},
    wrap,
};

#[derive(Debug)]
enum Error {
    NoDatabase,
    #[allow(unused)]
    Sql(rusqlite::Error),
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

/// https://github.com/lutris/lutris/blob/e4ae3d7193da777ebb370603a9e20c435f725300/docs/installers.rst
mod spec {
    use super::*;

    /// For `games/foo.yml`, this would be `foo`.
    #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BareName(pub String);

    impl std::fmt::Display for BareName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", &self.0)
        }
    }

    #[derive(serde::Deserialize)]
    pub struct Data {
        pub game: Game,
        /// ID of the game itself.
        pub game_slug: Option<String>,
        /// Human-readable.
        pub name: Option<String>,
    }

    #[derive(serde::Deserialize)]
    pub struct Game {
        pub exe: Option<StrictPath>,
        pub prefix: Option<StrictPath>,
        pub working_dir: Option<StrictPath>,
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Pending {
    name: Option<String>,
    slug: Option<String>,
    prefix: Option<StrictPath>,
    platform: Option<Os>,
    install_dir: Option<StrictPath>,
}

impl Pending {
    fn merge(self, other: Option<Self>) -> Self {
        let Some(other) = other else {
            return self;
        };

        Self {
            name: self.name.or(other.name),
            slug: self.slug.or(other.slug),
            prefix: self.prefix.or(other.prefix),
            platform: self.platform.or(other.platform),
            install_dir: self.install_dir.or(other.install_dir),
        }
    }

    pub fn evaluate(self, title_finder: &TitleFinder) -> Option<(String, LauncherGame)> {
        let mut title = None;
        if let Some(slug) = self.slug {
            title = title_finder.find_one(TitleQuery {
                lutris_id: Some(slug),
                ..Default::default()
            });
        }
        if title.is_none() {
            if let Some(name) = self.name {
                title = title_finder.find_one_by_normalized_name(&name);
            }
        }
        let title = title?;

        let platform = if self.prefix.is_some() && self.platform.is_none() {
            Some(Os::Windows)
        } else {
            self.platform
        };

        Some((
            title,
            LauncherGame {
                install_dir: self.install_dir,
                prefix: self.prefix,
                platform,
            },
        ))
    }
}

pub fn scan(root: &RootsConfig, title_finder: &TitleFinder) -> HashMap<String, HashSet<LauncherGame>> {
    let mut games = HashMap::<String, HashSet<LauncherGame>>::new();

    log::trace!("Scanning Lutris root for games: {:?}", &root.path);

    match scan_db(root) {
        Ok(db_games) => {
            for (spec, db_pending) in db_games {
                let spec_pending = find_spec(&spec, &root.path);
                log::trace!(
                    "Evaluating game, bare name: {spec}, from DB: {db_pending:?} + from spec: {spec_pending:?}"
                );

                if let Some((title, game)) = db_pending.merge(spec_pending).evaluate(title_finder) {
                    log::trace!("Evaluated to '{title}': {game:?}");
                    games.entry(title).or_default().insert(game);
                } else {
                    log::trace!("Unable to determine game");
                }
            }
        }
        Err(e) => {
            log::error!("Failed to read database: {e:?}");

            for spec_path in root.path.joined("games/*.y*ml").glob() {
                let Some(spec_pending) = read_spec(&spec_path) else {
                    continue;
                };
                log::trace!("Evaluating game from spec only: {spec_pending:?}");

                if let Some((title, game)) = spec_pending.evaluate(title_finder) {
                    log::trace!("Evaluated to '{title}': {game:?}");
                    games.entry(title).or_default().insert(game);
                } else {
                    log::trace!("Unable to determine game");
                }
            }
        }
    }

    if let Some(metadata) = wrap::lutris::infer_metadata() {
        games.entry(metadata.title).or_default().insert(LauncherGame {
            platform: metadata.prefix.is_some().then_some(Os::Windows),
            install_dir: metadata.base,
            prefix: metadata.prefix,
        });
    }

    log::trace!("Finished scanning Lutris root for games: {:?}", &root.path);

    games
}

fn scan_db(root: &RootsConfig) -> Result<HashMap<spec::BareName, Pending>, Error> {
    #[derive(Debug)]
    struct Row {
        name: Option<String>,
        slug: Option<String>,
        platform: Option<String>,
        runner: Option<String>,
        directory: Option<String>,
        configpath: Option<String>,
    }

    let db_file = root.path.joined("pga.db");
    if !db_file.is_file() {
        return Err(Error::NoDatabase);
    }

    let mut games = HashMap::<spec::BareName, Pending>::new();

    let Ok(file) = db_file.as_std_path_buf() else {
        return Ok(games);
    };
    let conn = rusqlite::Connection::open(file)?;

    let mut stmt = conn.prepare("SELECT name, slug, platform, runner, directory, configpath FROM games")?;
    let rows = stmt.query_map([], |row| {
        Ok(Row {
            name: row.get(0)?,
            slug: row.get(1)?,
            platform: row.get(2)?,
            runner: row.get(3)?,
            directory: row.get(4)?,
            configpath: row.get(5)?,
        })
    })?;

    for row in rows {
        match row {
            Ok(row) => {
                log::trace!("Row = {row:?}");

                let spec = if let Some(spec) = row.configpath {
                    if spec.trim().is_empty() {
                        log::warn!("Ignoring row with empty `configpath`");
                        continue;
                    }
                    spec::BareName(spec)
                } else {
                    log::warn!("Ignoring row without `configpath`");
                    continue;
                };

                let mut pending = Pending {
                    name: row.name,
                    slug: row.slug,
                    prefix: None,
                    platform: row.platform.as_ref().map(|x| Os::from(x.as_str())),
                    install_dir: None,
                };

                if pending.platform.is_some_and(|x| x == Os::Windows) && row.runner.is_some_and(|x| x == "wine") {
                    if let Some(directory) = row.directory {
                        if !directory.trim().is_empty() {
                            pending.prefix = Some(StrictPath::new(directory));
                        }
                    }
                }

                games.insert(spec, pending);
            }
            Err(e) => {
                log::warn!("Row error: {e:?}");
            }
        }
    }

    Ok(games)
}

fn find_spec(name: &spec::BareName, root: &StrictPath) -> Option<Pending> {
    for candidate in root.joined(&format!("games/{name}.y*ml")).glob() {
        if candidate.is_file() {
            return read_spec(&candidate);
        }
    }

    None
}

fn read_spec(file: &StrictPath) -> Option<Pending> {
    log::debug!("Inspecting Lutris game file: {:?}", file);

    let Some(content) = file.read() else {
        log::warn!("Unable to read Lutris game file: {:?}", file);
        return None;
    };

    let spec = match serde_yaml::from_str::<spec::Data>(&content) {
        Ok(x) => x,
        Err(e) => {
            log::warn!("Unable to parse Lutris game file: {:?} | {e:?}", file);
            return None;
        }
    };

    scan_spec(spec, file)
}

fn scan_spec(spec: spec::Data, spec_path: &StrictPath) -> Option<Pending> {
    let mut pending = Pending {
        name: spec.name,
        slug: spec.game_slug,
        prefix: spec.game.prefix,
        platform: None,
        install_dir: None,
    };

    'wd: {
        if let Some(working_dir) = spec.game.working_dir {
            pending.install_dir = Some(working_dir);
        } else if let Some(exe) = spec.game.exe {
            let exe = if exe.is_absolute() {
                exe
            } else if let Some(prefix) = pending.prefix.as_ref() {
                prefix.joined(&exe.raw())
            } else {
                log::info!("Lutris game file has relative exe and no prefix: {:?}", spec_path);
                break 'wd;
            };

            if let Some(parent) = exe.parent_raw() {
                pending.install_dir = Some(parent)
            } else {
                log::info!(
                    "Lutris game file has indeterminate parent folder of exe: {:?}",
                    spec_path
                );
            }
        } else {
            log::info!(
                "Lutris game file does not have `working_dir` and `exe` fields: {:?}",
                spec_path
            );
        }
    }

    Some(pending)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{hash_map, hash_set};

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
            Windows Game 1:
              files:
                <base>/file1.txt: {}
            Windows Game 2:
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
    fn scan_finds_all_games_with_spec_files() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/lutris-spec", repo())),
            store: Store::Lutris,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "windows-game".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/home/deck/Games/service/windows-game/drive_c/game".to_string())),
                    prefix: Some(StrictPath::new("/home/deck/Games/service/windows-game".to_string())),
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }

    #[test]
    fn scan_finds_all_games_with_database() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/lutris-db", repo())),
            store: Store::Lutris,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "windows-game".to_string(): hash_set![LauncherGame {
                    install_dir: None,
                    prefix: Some(StrictPath::new("/home/deck/Games/service/windows-game".to_string())),
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }

    #[test]
    fn scan_finds_all_games_with_spec_and_database_merged() {
        let root = RootsConfig {
            path: StrictPath::new(format!("{}/tests/launchers/lutris-merged", repo())),
            store: Store::Lutris,
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "Windows Game 1".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/home/deck/Games/service/windows-game/drive_c/game".to_string())),
                    prefix: Some(StrictPath::new("/home/deck/Games/service/windows-game-1".to_string())),
                    platform: Some(Os::Windows),
                }],
                "Windows Game 2".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/home/deck/Games/service".to_string())),
                    prefix: Some(StrictPath::new("/home/deck/Games/service/windows-game-2".to_string())),
                    platform: Some(Os::Windows),
                }],
            },
            games,
        );
    }

    #[test]
    fn can_scan_spec_with_absolute_exe() {
        let spec = spec::Data {
            game: spec::Game {
                exe: Some(absolute_path("/install/drive_c/game/launcher.exe")),
                prefix: Some(absolute_path("/prefix")),
                working_dir: None,
            },
            game_slug: None,
            name: Some("Windows Game with Absolute Exe".into()),
        };
        assert_eq!(
            Some(Pending {
                name: Some("Windows Game with Absolute Exe".into()),
                slug: None,
                prefix: Some(absolute_path("/prefix")),
                platform: None,
                install_dir: Some(absolute_path("/install/drive_c/game")),
            }),
            scan_spec(spec, &absolute_path("/tmp")),
        );
    }

    #[test]
    fn can_scan_spec_with_relative_exe_but_prefix() {
        let spec = spec::Data {
            game: spec::Game {
                exe: Some(StrictPath::new("drive_c/game/launcher.exe".into())),
                prefix: Some(absolute_path("/prefix")),
                working_dir: None,
            },
            game_slug: None,
            name: Some("Windows Game with Relative Exe".into()),
        };
        assert_eq!(
            Some(Pending {
                name: Some("Windows Game with Relative Exe".into()),
                slug: None,
                prefix: Some(absolute_path("/prefix")),
                platform: None,
                install_dir: Some(absolute_path("/prefix/drive_c/game")),
            }),
            scan_spec(spec, &absolute_path("/tmp")),
        );
    }
}
