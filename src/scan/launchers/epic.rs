use std::collections::{HashMap, HashSet};

use crate::{
    path::CommonPath,
    prelude::StrictPath,
    resource::{config::root, manifest::Os},
    scan::{LauncherGame, TitleFinder},
};

type ItemId = String;
type Title = String;

pub fn is_v2(root: &root::Epic) -> bool {
    root.path.joined(library::PATH).is_file() && root.path.joined(catalog::PATH).is_file()
}

mod library {
    use crate::prelude::AnyError;

    use super::*;

    pub const PATH: &str = "installations.jsonl";

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Game {
        /// Used for catalog lookups.
        pub item_id: Option<ItemId>,
        /// Used in prefix path.
        pub artifact_id: Option<ItemId>,
        /// Path to install folder.
        pub dir: Option<StrictPath>,
        /// Installed?
        pub complete: Option<bool>,
        /// Uninstalled?
        pub deleted: Option<bool>,
        /// OS.
        pub platform: Option<String>,
    }

    pub fn load(root: &StrictPath) -> Result<HashMap<ItemId, Game>, AnyError> {
        let content = root.joined(PATH).try_read()?;

        let mut games = HashMap::<ItemId, Game>::new();

        // First line is file metadata
        for line in content.lines().skip(1) {
            let Ok(game) = serde_json::from_str::<Game>(line) else {
                continue;
            };

            if game.complete.unwrap_or(true)
                && !game.deleted.unwrap_or(false)
                && let Some(id) = &game.item_id
            {
                games.insert(id.clone(), game);
            }
        }

        Ok(games)
    }
}

mod catalog {
    pub const PATH: &str = "cache/prod/catalog-cache.json";

    use crate::prelude::AnyError;

    use super::*;

    #[derive(Debug, serde::Deserialize)]
    pub struct Data {
        pub items: HashMap<ItemId, ItemWrapper>,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct ItemWrapper {
        pub item: Item,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Item {
        /// Item ID.
        pub id: ItemId,
        /// Human-readable title.
        pub title: Title,
    }

    pub fn load(root: &StrictPath) -> Result<HashMap<ItemId, Title>, AnyError> {
        let content = root.joined(PATH).try_read()?;
        let parsed = serde_json::from_str::<Data>(&content)?;

        Ok(parsed
            .items
            .into_values()
            .map(|wrapper| (wrapper.item.id, wrapper.item.title))
            .collect())
    }
}

pub fn scan(root: &root::Epic, title_finder: &TitleFinder) -> HashMap<String, HashSet<LauncherGame>> {
    log::trace!("Epic: Scanning root for games: {root:?}");

    let library = match library::load(&root.path) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Epic: Failed to load library: {e:?}");
            return HashMap::new();
        }
    };
    let catalog = match catalog::load(&root.path) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Epic: Failed to load catalog: {e:?}");
            return HashMap::new();
        }
    };

    let games = scan_spec(&root.path, library, catalog, title_finder);

    if Os::HOST == Os::Linux
        && let Some(normal_home) = CommonPath::Home.get().map(StrictPath::new)
        && let Some(flatpak_home) = root.flatpak_home()
    {
        log::debug!("Epic: For Flatpak root {root:?}, translating home {normal_home:?} to {flatpak_home:?}");
        return games
            .into_iter()
            .map(|(title, mut info)| {
                let flatpak: HashSet<_> = info
                    .iter()
                    .map(|info| info.replace_in_paths(&normal_home, &flatpak_home))
                    .collect();

                info.extend(flatpak);

                (title, info)
            })
            .collect();
    }

    log::trace!("Epic: Finished scanning root for games: {root:?} => {games:?}");

    games
}

fn scan_spec(
    root: &StrictPath,
    library: HashMap<Title, library::Game>,
    catalog: HashMap<ItemId, Title>,
    title_finder: &TitleFinder,
) -> HashMap<String, HashSet<LauncherGame>> {
    log::debug!("Epic: Inspecting library content");

    let mut out = HashMap::<String, HashSet<LauncherGame>>::new();

    for (id, info) in library.into_iter() {
        let title = match catalog.get(&id) {
            Some(title) => {
                let Some(title) = title_finder.find_one_by_normalized_name(title) else {
                    log::info!("Epic: Ignoring unrecognized title '{title}' for {id:?}");
                    continue;
                };
                title
            }
            None => {
                log::debug!("Epic: Skipping game {id:?} without title");
                continue;
            }
        };

        let install_dir = info.dir;

        let prefix = info
            .artifact_id
            .map(|id| root.joined(format!("compat/compatdata/{id}/pfx")))
            .filter(|path| path.exists());

        let platform = info
            .platform
            .as_ref()
            .is_some_and(|raw| raw.to_lowercase() == "windows")
            .then_some(Os::Windows);

        let entry = LauncherGame {
            install_dir,
            prefix,
            platform,
        };

        out.entry(title)
            .and_modify(|xs| {
                xs.insert(entry.clone());
            })
            .or_insert(HashSet::from_iter([entry]));
    }

    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{hash_map, hash_set};

    use super::*;
    use crate::{
        resource::{ResourceFile, manifest::Manifest},
        testing::repo,
    };

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            Exact Match:
              files:
                <base>/file1.txt: {}
            Fuzzy Match:
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
        let root = root::Epic {
            path: format!("{}/tests/nonexistent", repo()).into(),
        };
        let games = scan(&root, &title_finder());
        assert_eq!(HashMap::new(), games);
    }

    #[test]
    fn scan_finds_all_games() {
        let root = root::Epic {
            path: format!("{}/tests/launchers/epic-v2", repo()).into(),
        };
        let games = scan(&root, &title_finder());
        assert_eq!(
            hash_map! {
                "Exact Match".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/exact".to_string())),
                    prefix: Some(root.path.joined(format!("compat/compatdata/exact/pfx"))),
                    platform: Some(Os::Windows),
                }],
                "Fuzzy Match".to_string(): hash_set![LauncherGame {
                    install_dir: Some(StrictPath::new("/fuzzy".to_string())),
                    prefix: None,
                    platform: None,
                }],
            },
            games,
        );
    }
}
