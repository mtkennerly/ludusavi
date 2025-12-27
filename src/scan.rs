pub mod backup;
pub mod change;
pub mod duplicate;
pub mod game_filter;
pub mod launchers;
pub mod layout;
pub mod preview;
pub mod registry;
pub mod saves;
pub mod steam;
pub mod title;

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use regex::Regex;

#[allow(unused)]
pub use self::{
    backup::{BackupError, BackupId, BackupInfo, OperationStatus, OperationStepDecision},
    change::{ScanChange, ScanChangeCount},
    duplicate::{DuplicateDetector, Duplication},
    launchers::{LauncherGame, Launchers},
    preview::ScanInfo,
    saves::{ScannedFile, ScannedRegistry, ScannedRegistryValue, ScannedRegistryValues},
    steam::{SteamShortcut, SteamShortcuts},
    title::{compare_ranked_titles, compare_ranked_titles_ref, TitleFinder, TitleMatch, TitleQuery},
};

use crate::{
    path::{CommonPath, StrictPath},
    prelude::{filter_map_walkdir, Error, SKIP},
    resource::{
        config::{
            root, BackupFilter, Config, RedirectConfig, RedirectKind, Root, SortKey, ToggledPaths, ToggledRegistry,
        },
        manifest::{Game, GameFileEntry, IdSet, Os, Store},
    },
    scan::layout::LatestBackup,
};

#[cfg(target_os = "windows")]
use crate::scan::registry::RegistryItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanKind {
    Backup,
    Restore,
}

impl ScanKind {
    pub fn is_backup(&self) -> bool {
        *self == Self::Backup
    }

    pub fn is_restore(&self) -> bool {
        *self == Self::Restore
    }
}

/// Returns the effective target, if different from the original
pub fn game_file_target(
    original: &StrictPath,
    redirects: &[RedirectConfig],
    reverse_redirects_on_restore: bool,
    scan_kind: ScanKind,
) -> Option<StrictPath> {
    if redirects.is_empty() {
        return None;
    }

    let mut redirected = original.clone();

    let redirects: &mut dyn Iterator<Item = &RedirectConfig> = if scan_kind.is_restore() && reverse_redirects_on_restore
    {
        &mut redirects.iter().rev()
    } else {
        &mut redirects.iter()
    };

    for redirect in redirects {
        if redirect.source.raw().trim().is_empty() || redirect.target.raw().trim().is_empty() {
            continue;
        }
        let (source, target) = match scan_kind {
            ScanKind::Backup => match redirect.kind {
                RedirectKind::Backup | RedirectKind::Bidirectional => (&redirect.source, &redirect.target),
                RedirectKind::Restore => continue,
            },
            ScanKind::Restore => match redirect.kind {
                RedirectKind::Backup => continue,
                RedirectKind::Restore => (&redirect.source, &redirect.target),
                RedirectKind::Bidirectional => (&redirect.target, &redirect.source),
            },
        };
        redirected = redirected.replace(source, target);
    }

    (original != &redirected).then_some(redirected)
}

fn check_windows_path(path: &str) -> &str {
    match Os::HOST {
        Os::Windows => path,
        _ => SKIP,
    }
}

fn check_nonwindows_path(path: &str) -> &str {
    match Os::HOST {
        Os::Windows => SKIP,
        _ => path,
    }
}

/// Returns paths to check and whether they require case-sensitive matching.
pub fn parse_paths(
    path: &str,
    data: &GameFileEntry,
    root: &Root,
    install_dir: Option<&String>,
    full_install_dir: Option<&StrictPath>,
    ids: &IdSet,
    manifest_dir: &StrictPath,
    steam_shortcut: Option<&SteamShortcut>,
    platform: Os,
) -> HashSet<(StrictPath, bool)> {
    use crate::resource::manifest::placeholder as p;

    let mut paths = HashSet::new();

    macro_rules! add_path {
        ($path:expr) => {
            paths.insert(($path, platform.is_case_sensitive()))
        };
    }
    macro_rules! add_path_insensitive {
        ($path:expr) => {
            paths.insert(($path, false))
        };
    }

    // Since STORE_USER_ID becomes `*`, we don't want to end up with an invalid `**`.
    let path = path
        .replace(&format!("*{}", p::STORE_USER_ID), p::STORE_USER_ID)
        .replace(&format!("{}*", p::STORE_USER_ID), p::STORE_USER_ID);

    let install_dir = install_dir.map(|x| globset::escape(x)).unwrap_or(SKIP.to_string());
    let full_install_dir = full_install_dir
        .map(|x| x.globbable())
        .unwrap_or_else(|| SKIP.to_string());

    let root_globbable = root.path().globbable();
    let manifest_dir_globbable = manifest_dir.globbable();

    let data_dir = CommonPath::Data.get_globbable().unwrap_or(SKIP);
    let data_local_dir = CommonPath::DataLocal.get_globbable().unwrap_or(SKIP);
    let data_local_low_dir = CommonPath::DataLocalLow.get_globbable().unwrap_or(SKIP);
    let config_dir = CommonPath::Config.get_globbable().unwrap_or(SKIP);
    let home = CommonPath::Home.get_globbable().unwrap_or(SKIP);
    let document_dir = CommonPath::Document.get_globbable().unwrap_or(SKIP);
    let public_dir = CommonPath::Public.get_globbable().unwrap_or(SKIP);
    let saved_games_dir = CommonPath::SavedGames.get_globbable();

    add_path!(path
        .replace(p::ROOT, &root_globbable)
        .replace(p::GAME, &install_dir)
        .replace(p::BASE, &full_install_dir)
        .replace(p::HOME, home)
        .replace(p::STORE_USER_ID, "*")
        .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME)
        .replace(p::WIN_APP_DATA, check_windows_path(data_dir))
        .replace(p::WIN_LOCAL_APP_DATA, check_windows_path(data_local_dir))
        .replace(p::WIN_LOCAL_APP_DATA_LOW, check_windows_path(data_local_low_dir))
        .replace(p::WIN_DOCUMENTS, check_windows_path(document_dir))
        .replace(p::WIN_PUBLIC, check_windows_path(public_dir))
        .replace(p::WIN_PROGRAM_DATA, check_windows_path("C:/ProgramData"))
        .replace(p::WIN_DIR, check_windows_path("C:/Windows"))
        .replace(p::XDG_DATA, check_nonwindows_path(data_dir))
        .replace(p::XDG_CONFIG, check_nonwindows_path(config_dir)));

    match root.store() {
        Store::Gog => {
            if Os::HOST == Os::Linux {
                add_path!(path
                    .replace(p::GAME, &format!("{install_dir}/game"))
                    .replace(p::BASE, &format!("{}/{}/game", &root_globbable, install_dir)));
            }
        }
        Store::Heroic => {
            if Os::HOST == Os::Linux && root_globbable.ends_with(root::Heroic::FLATPAK_SUFFIX) {
                // Heroic is installed via Flatpak.
                add_path!(path
                    .replace(
                        p::XDG_DATA,
                        check_nonwindows_path(&format!("{}/../../data", &root_globbable)),
                    )
                    .replace(
                        p::XDG_CONFIG,
                        check_nonwindows_path(&format!("{}/../../config", &root_globbable)),
                    )
                    .replace(p::STORE_USER_ID, "*")
                    .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME));
            }
        }
        Store::Lutris => {
            if Os::HOST == Os::Linux
                && (root_globbable.ends_with(root::Lutris::FLATPAK_SUFFIX_DATA)
                    || root_globbable.ends_with(root::Lutris::FLATPAK_SUFFIX_CONFIG))
            {
                // Lutris is installed via Flatpak.
                add_path!(path
                    .replace(
                        p::XDG_DATA,
                        check_nonwindows_path(&format!("{}/../../data", &root_globbable)),
                    )
                    .replace(
                        p::XDG_CONFIG,
                        check_nonwindows_path(&format!("{}/../../config", &root_globbable)),
                    )
                    .replace(p::STORE_USER_ID, "*")
                    .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME));
            }
        }
        Store::Steam => {
            if let Some(steam_shortcut) = steam_shortcut {
                if let Some(start_dir) = &steam_shortcut.start_dir {
                    if let Ok(start_dir) = start_dir.interpret() {
                        add_path!(path.replace(p::BASE, &start_dir));
                    }
                }
            }

            if Os::HOST == Os::Linux {
                if root_globbable.ends_with(root::Steam::FLATPAK_SUFFIX) {
                    // Steam is installed via Flatpak.
                    add_path!(path
                        .replace(p::STORE_USER_ID, "*")
                        .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME)
                        .replace(p::XDG_DATA, &format!("{}../../.local/share", &root_globbable))
                        .replace(p::XDG_CONFIG, &format!("{}../../.config", &root_globbable)));
                }

                for id in ids.steam(steam_shortcut.map(|x| x.id)) {
                    let prefix = format!("{}/steamapps/compatdata/{}/pfx/drive_c", &root_globbable, id);
                    let path2 = path
                        .replace(p::ROOT, &root_globbable)
                        .replace(p::GAME, &install_dir)
                        .replace(p::BASE, &full_install_dir)
                        .replace(p::HOME, &format!("{prefix}/users/steamuser"))
                        .replace(p::STORE_USER_ID, "*")
                        .replace(p::OS_USER_NAME, "steamuser")
                        .replace(p::WIN_PUBLIC, &format!("{prefix}/users/Public"))
                        .replace(p::WIN_PROGRAM_DATA, &format!("{prefix}/ProgramData"))
                        .replace(p::WIN_DIR, &format!("{prefix}/windows"))
                        .replace(p::XDG_DATA, check_nonwindows_path(data_dir))
                        .replace(p::XDG_CONFIG, check_nonwindows_path(config_dir));
                    add_path_insensitive!(path2
                        .replace(p::WIN_DOCUMENTS, &format!("{prefix}/users/steamuser/Documents"))
                        .replace(p::WIN_APP_DATA, &format!("{prefix}/users/steamuser/AppData/Roaming"))
                        .replace(
                            p::WIN_LOCAL_APP_DATA,
                            &format!("{prefix}/users/steamuser/AppData/Local")
                        )
                        .replace(
                            p::WIN_LOCAL_APP_DATA_LOW,
                            &format!("{prefix}/users/steamuser/AppData/LocalLow")
                        ));
                    add_path_insensitive!(path2
                        .replace(p::WIN_DOCUMENTS, &format!("{prefix}/users/steamuser/My Documents"))
                        .replace(p::WIN_APP_DATA, &format!("{prefix}/users/steamuser/Application Data"))
                        .replace(
                            p::WIN_LOCAL_APP_DATA,
                            &format!("{prefix}/users/steamuser/Local Settings/Application Data"),
                        ));

                    if data.when.iter().any(|x| x.store == Some(Store::Uplay)) {
                        let ubisoft = format!("{prefix}/Program Files (x86)/Ubisoft/Ubisoft Game Launcher");
                        add_path!(path
                            .replace(p::ROOT, &ubisoft)
                            .replace(p::GAME, &install_dir)
                            .replace(p::BASE, &format!("{}/{}", &ubisoft, install_dir))
                            .replace(p::STORE_USER_ID, "*")
                            .replace(p::OS_USER_NAME, "steamuser"));
                    }
                }
            }
        }
        Store::OtherHome => {
            add_path!(path
                .replace(p::ROOT, &root_globbable)
                .replace(p::GAME, &install_dir)
                .replace(p::BASE, &format!("{}/{}", &root_globbable, install_dir))
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME)
                .replace(p::WIN_APP_DATA, check_windows_path("<home>/AppData/Roaming"))
                .replace(p::WIN_LOCAL_APP_DATA, check_windows_path("<home>/AppData/Local"))
                .replace(p::WIN_LOCAL_APP_DATA_LOW, check_windows_path("<home>/AppData/LocalLow"))
                .replace(p::WIN_DOCUMENTS, check_windows_path("<home>/Documents"))
                .replace(p::WIN_PUBLIC, check_windows_path(public_dir))
                .replace(p::WIN_PROGRAM_DATA, check_windows_path("C:/ProgramData"))
                .replace(p::WIN_DIR, check_windows_path("C:/Windows"))
                .replace(p::XDG_DATA, check_nonwindows_path("<home>/.local/share"))
                .replace(p::XDG_CONFIG, check_nonwindows_path("<home>/.config"))
                .replace(p::HOME, &root_globbable));
        }
        Store::OtherWine => {
            let prefix = format!("{}/drive_*", &root_globbable);
            let path2 = path
                .replace(p::ROOT, &root_globbable)
                .replace(p::GAME, &install_dir)
                .replace(p::BASE, &format!("{}/{}", &root_globbable, install_dir))
                .replace(p::HOME, &format!("{prefix}/users/*"))
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, "*")
                .replace(p::WIN_PUBLIC, &format!("{prefix}/users/Public"))
                .replace(p::WIN_PROGRAM_DATA, &format!("{prefix}/ProgramData"))
                .replace(p::WIN_DIR, &format!("{prefix}/windows"))
                .replace(p::XDG_DATA, check_nonwindows_path(data_dir))
                .replace(p::XDG_CONFIG, check_nonwindows_path(config_dir));
            add_path_insensitive!(path2
                .replace(p::WIN_DOCUMENTS, &format!("{prefix}/users/*/Documents"))
                .replace(p::WIN_APP_DATA, &format!("{prefix}/users/*/AppData/Roaming"))
                .replace(p::WIN_LOCAL_APP_DATA, &format!("{prefix}/users/*/AppData/Local"))
                .replace(p::WIN_LOCAL_APP_DATA_LOW, &format!("{prefix}/users/*/AppData/LocalLow")));
            add_path_insensitive!(path2
                .replace(p::WIN_DOCUMENTS, &format!("{prefix}/users/*/My Documents"))
                .replace(p::WIN_APP_DATA, &format!("{prefix}/users/*/Application Data"))
                .replace(
                    p::WIN_LOCAL_APP_DATA,
                    &format!("{prefix}/users/*/Local Settings/Application Data"),
                ));
        }
        Store::OtherWindows => {
            add_path!(path
                .replace(p::HOME, &format!("{}/Users/*", &root_globbable))
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, "*")
                .replace(p::WIN_APP_DATA, &format!("{}/Users/*/AppData/Roaming", &root_globbable))
                .replace(
                    p::WIN_LOCAL_APP_DATA,
                    &format!("{}/Users/*/AppData/Local", &root_globbable),
                )
                .replace(
                    p::WIN_LOCAL_APP_DATA_LOW,
                    &format!("{}/Users/*/AppData/LocalLow", &root_globbable),
                )
                .replace(p::WIN_DOCUMENTS, &format!("{}/Users/*/Documents", &root_globbable))
                .replace(p::WIN_PUBLIC, &format!("{}/Users/Public", &root_globbable))
                .replace(p::WIN_PROGRAM_DATA, &format!("{}/ProgramData", &root_globbable))
                .replace(p::WIN_DIR, &format!("{}/Windows", &root_globbable)));
        }
        Store::OtherLinux => {
            add_path!(path
                .replace(p::HOME, &format!("{}/home/*", &root_globbable))
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, "*")
                .replace(p::XDG_DATA, &format!("{}/home/*/.local/share", &root_globbable))
                .replace(p::XDG_CONFIG, &format!("{}/home/*/.config", &root_globbable)));
        }
        Store::OtherMac => {
            add_path!(path
                .replace(p::HOME, &format!("{}/Users/*", &root_globbable))
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, "*")
                .replace(p::XDG_DATA, &format!("{}/Users/*/Library", &root_globbable))
                .replace(
                    p::XDG_CONFIG,
                    &format!("{}/Users/*/Library/Preferences", &root_globbable),
                ));
        }
        Store::Ea
        | Store::Epic
        | Store::GogGalaxy
        | Store::Legendary
        | Store::Microsoft
        | Store::Origin
        | Store::Prime
        | Store::Uplay
        | Store::Other => {}
    }

    if Os::HOST == Os::Windows {
        if let Some(saved_games_dir) = saved_games_dir {
            add_path!(path
                .replace(p::GAME, &install_dir)
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME)
                .replace("<home>/Saved Games/", &format!("{saved_games_dir}/"))
                .replace("<home>\\Saved Games\\", &format!("{saved_games_dir}/"))
                .replace(p::HOME, home));
        }

        static VIRTUALIZED: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^C:[\\/](Program Files|Program Files \(x86\)|Windows|ProgramData)[\\/]"#).unwrap()
        });
        let expanded: HashSet<_> = paths
            .iter()
            .filter_map(
                |(p, c)| match VIRTUALIZED.replace(p, format!("{}/VirtualStore/${{1}}/", &data_local_dir)) {
                    std::borrow::Cow::Borrowed(_) => None,
                    std::borrow::Cow::Owned(p) => Some((p, *c)),
                },
            )
            .collect();
        paths.extend(expanded);
    } else {
        if Os::HOST == Os::Linux {
            // Default XDG paths, in case we're in a Flatpak context.
            add_path!(path
                .replace(p::GAME, &install_dir)
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, &crate::prelude::OS_USERNAME)
                .replace(p::XDG_DATA, "<home>/.local/share")
                .replace(p::XDG_CONFIG, "<home>/.config")
                .replace(p::HOME, home));
        }

        if let Some(flatpak_id) = ids.flatpak.as_ref() {
            add_path!(path
                .replace(p::HOME, home)
                .replace(p::STORE_USER_ID, "*")
                .replace(p::OS_USER_NAME, "*")
                .replace(p::XDG_DATA, &format!("{home}/.var/app/{flatpak_id}/data"))
                .replace(p::XDG_CONFIG, &format!("{home}/.var/app/{flatpak_id}/config")));

            if root.store() == Store::OtherHome {
                let home = &root_globbable;
                add_path!(path
                    .replace(p::HOME, home)
                    .replace(p::STORE_USER_ID, "*")
                    .replace(p::OS_USER_NAME, "*")
                    .replace(p::XDG_DATA, &format!("{home}/.var/app/{flatpak_id}/data"))
                    .replace(p::XDG_CONFIG, &format!("{home}/.var/app/{flatpak_id}/config")));
            }
        }
    }

    let paths = if path.contains(p::STORE_GAME_ID) {
        let mut expanded = HashSet::new();

        for (p, c) in paths {
            match root.store() {
                Store::Gog => {
                    for id in ids.gog() {
                        expanded.insert((p.replace(p::STORE_GAME_ID, &id.to_string()), c));
                    }
                }
                Store::Lutris => {
                    if let Some(id) = ids.lutris.as_ref() {
                        expanded.insert((p.replace(p::STORE_GAME_ID, id), c));
                    }
                }
                Store::Steam => {
                    for id in ids.steam(steam_shortcut.map(|x| x.id)) {
                        expanded.insert((p.replace(p::STORE_GAME_ID, &id.to_string()), c));
                    }
                }
                _ => continue,
            }
        }

        expanded
    } else {
        paths
    };

    paths
        .into_iter()
        // This excludes `SKIP` and any other unmatched placeholders.
        .filter(|(p, _)| !p.contains('<'))
        .map(|(p, c)| (StrictPath::relative(p, Some(manifest_dir_globbable.clone())), c))
        .collect()
}

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[Root],
    manifest_dir: &StrictPath,
    launchers: &Launchers,
    filter: &BackupFilter,
    wine_prefix: Option<&StrictPath>,
    ignored_paths: &ToggledPaths,
    #[cfg_attr(not(target_os = "windows"), allow(unused))] ignored_registry: &ToggledRegistry,
    previous: Option<&LatestBackup>,
    redirects: &[RedirectConfig],
    reverse_redirects_on_restore: bool,
    steam_shortcuts: &SteamShortcuts,
    only_constructive_backups: bool,
) -> ScanInfo {
    log::trace!("[{name}] beginning scan for backup");

    let mut found_files = HashMap::new();
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    let mut found_registry_keys = HashMap::new();
    #[allow(unused)]
    let mut dumped_registry = None;
    let has_backups = previous.is_some();

    let mut paths_to_check = HashSet::<(StrictPath, Option<bool>)>::new();

    // Add a dummy root for checking paths without `<root>`.
    let mut roots_to_check: Vec<Root> = vec![Root::new(SKIP, Store::Other)];
    roots_to_check.extend(roots.iter().cloned());

    let manifest_dir_globbable = manifest_dir.globbable();
    let all_ids = game.all_ids();
    let steam_shortcut = steam_shortcuts.get(name);

    for wp in &game.wine_prefix {
        if wp.trim().is_empty() {
            continue;
        }
        scan_game_for_backup_add_prefix(
            &mut roots_to_check,
            &mut paths_to_check,
            &StrictPath::new(wp),
            !game.registry.is_empty(),
        );
    }
    if let Some(wp) = wine_prefix {
        // We can add this for Wine prefixes from the CLI because they're
        // typically going to be used for only one or a few games at a time.
        // For other Wine roots, it would trigger for every game.
        scan_game_for_backup_add_prefix(&mut roots_to_check, &mut paths_to_check, wp, !game.registry.is_empty());
    }
    for root in roots {
        for wp in launchers.get_game(root, name).filter_map(|x| x.prefix.as_ref()) {
            scan_game_for_backup_add_prefix(&mut roots_to_check, &mut paths_to_check, wp, !game.registry.is_empty());

            let pfx = wp.joined("pfx");
            if pfx.exists() {
                scan_game_for_backup_add_prefix(
                    &mut roots_to_check,
                    &mut paths_to_check,
                    &pfx,
                    !game.registry.is_empty(),
                );
            }
        }
    }

    for root in roots_to_check {
        log::trace!("[{name}] adding candidates from root: {:?}", &root,);
        if root.path().raw().trim().is_empty() {
            continue;
        }
        let root_globbable = root.path().globbable();

        for (raw_path, path_data) in &game.files {
            log::trace!("[{name}] parsing candidates from: {}", raw_path);
            if raw_path.trim().is_empty() {
                continue;
            }

            let mut candidates = HashSet::new();
            let mut launcher_entries = launchers.get_game(&root, name).peekable();

            if launcher_entries.peek().is_none() {
                let platform = Os::HOST;
                let full_install_dir = None;
                let install_dir = None;

                candidates.extend(parse_paths(
                    raw_path,
                    path_data,
                    &root,
                    install_dir,
                    full_install_dir,
                    &all_ids,
                    manifest_dir,
                    steam_shortcut,
                    platform,
                ));
            } else {
                for launcher_entry in launcher_entries {
                    log::trace!("[{name}] parsing candidates with launcher info: {:?}", &launcher_entry);
                    let platform = launcher_entry.platform.unwrap_or(Os::HOST);
                    let full_install_dir = launcher_entry.install_dir.as_ref();
                    let install_dir = full_install_dir.and_then(|x| root.path().suffix_for(x));

                    candidates.extend(parse_paths(
                        raw_path,
                        path_data,
                        &root,
                        install_dir.as_ref(),
                        full_install_dir,
                        &all_ids,
                        manifest_dir,
                        steam_shortcut,
                        platform,
                    ));
                }
            }

            for (candidate, case_sensitive) in candidates {
                log::trace!("[{name}] parsed candidate: {candidate:?}");
                paths_to_check.insert((candidate, Some(case_sensitive)));
            }
        }
        if root.store() == Store::Steam {
            for id in all_ids.steam(steam_shortcut.map(|x| x.id)) {
                // Cloud saves:
                paths_to_check.insert((
                    StrictPath::relative(
                        format!("{}/userdata/*/{}/remote/", &root_globbable, id),
                        Some(manifest_dir_globbable.clone()),
                    ),
                    None,
                ));

                // Screenshots:
                if !filter.exclude_store_screenshots {
                    paths_to_check.insert((
                        StrictPath::relative(
                            format!("{}/userdata/*/760/remote/{}/screenshots/*.*", &root_globbable, id),
                            Some(manifest_dir_globbable.clone()),
                        ),
                        None,
                    ));
                }

                // Registry:
                if !game.registry.is_empty() {
                    let prefix = format!("{}/steamapps/compatdata/{}/pfx", &root_globbable, id);
                    paths_to_check.insert((
                        StrictPath::relative(format!("{prefix}/*.reg"), Some(manifest_dir_globbable.clone())),
                        None,
                    ));
                }
            }
        }
    }

    let previous_files: HashMap<&StrictPath, &String> = previous
        .as_ref()
        .map(|previous| {
            previous
                .scan
                .found_files
                .iter()
                .map(|(scan_key, x)| (x.original_path(scan_key), &x.hash))
                .collect()
        })
        .unwrap_or_default();

    for (path, case_sensitive) in paths_to_check {
        log::trace!("[{name}] checking: {path:?}");
        if filter.is_path_ignored(&path) {
            log::debug!("[{name}] excluded: {path:?}");
            continue;
        }
        let paths = match case_sensitive {
            None => path.glob(),
            Some(cs) => path.glob_case_sensitive(cs),
        };
        for p in paths {
            if p.is_file() {
                let Ok(scan_key) = p.interpreted().map(|x| x.rendered()) else {
                    continue;
                };
                if filter.is_path_ignored(&scan_key) {
                    log::debug!("[{name}] excluded: {scan_key:?}");
                    continue;
                }
                let ignored = ignored_paths.is_ignored(name, &scan_key);
                log::debug!("[{name}] found: {scan_key:?}");
                let size = scan_key.size();
                let hash = scan_key.sha1();
                let redirected = game_file_target(&scan_key, redirects, reverse_redirects_on_restore, ScanKind::Backup);
                let change =
                    ScanChange::evaluate_backup(&hash, previous_files.get(redirected.as_ref().unwrap_or(&scan_key)));
                found_files.insert(
                    scan_key,
                    ScannedFile {
                        change,
                        size,
                        hash,
                        redirected,
                        original_path: None,
                        ignored,
                        container: None,
                    },
                );
            } else if p.is_dir() {
                log::trace!("[{name}] looking for files in: {p:?}");
                for child in walkdir::WalkDir::new(p.as_std_path_buf().unwrap())
                    .max_depth(100)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|x| filter_map_walkdir(name, x))
                {
                    #[cfg(not(target_os = "windows"))]
                    if child.path().to_string_lossy().contains('\\') {
                        // TODO: Support names containing a slash.
                        continue;
                    }

                    if child.file_type().is_file() {
                        let Ok(scan_key) = StrictPath::from(&child).interpreted().map(|x| x.rendered()) else {
                            continue;
                        };

                        if filter.is_path_ignored(&scan_key) {
                            log::debug!("[{name}] excluded: {scan_key:?}");
                            continue;
                        }
                        let ignored = ignored_paths.is_ignored(name, &scan_key);
                        log::debug!("[{name}] found: {scan_key:?}");
                        let size = scan_key.size();
                        let hash = scan_key.sha1();
                        let redirected =
                            game_file_target(&scan_key, redirects, reverse_redirects_on_restore, ScanKind::Backup);
                        let change = ScanChange::evaluate_backup(
                            &hash,
                            previous_files.get(redirected.as_ref().unwrap_or(&scan_key)),
                        );
                        found_files.insert(
                            scan_key,
                            ScannedFile {
                                change,
                                size,
                                hash,
                                redirected,
                                original_path: None,
                                ignored,
                                container: None,
                            },
                        );
                    }
                }
            }
        }
    }

    // Mark removed files.
    let current_files: Vec<_> = found_files
        .iter()
        .map(|(scan_key, x)| x.redirected.as_ref().unwrap_or(scan_key).interpret())
        .collect();
    // But if a file is only "removed" because now it has a redirect,
    // then the removal isn't very interesting
    // and would lead to duplicate hash keys during reporting.
    let current_files_with_redirects: Vec<_> = found_files
        .iter()
        .filter(|(_, x)| x.redirected.is_some())
        .map(|(scan_key, _)| scan_key.interpret())
        .collect();
    for (previous_file, _) in previous_files {
        let previous_file_interpreted = previous_file.interpret();
        if !current_files.contains(&previous_file_interpreted)
            && !current_files_with_redirects.contains(&previous_file_interpreted)
        {
            found_files.insert(
                previous_file.to_owned(),
                ScannedFile {
                    change: ScanChange::Removed,
                    size: 0,
                    hash: "".to_string(),
                    redirected: None,
                    original_path: None,
                    ignored: ignored_paths.is_ignored(name, previous_file),
                    container: None,
                },
            );
        }
    }

    #[cfg(target_os = "windows")]
    {
        let previous_registry = previous.and_then(|x| x.registry_content.clone());
        let mut current_registry = registry::Hives::default();

        for key in game.registry.keys() {
            if key.trim().is_empty() {
                continue;
            }

            log::trace!("[{name}] computing candidates for registry: {key}");
            let mut candidates = vec![key.clone()];
            let normalized = key.replace('\\', "/").to_lowercase();
            if normalized.starts_with("hkey_local_machine/software/") && !normalized.contains("/wow6432node/") {
                let tail = &key[28..];
                candidates.push(format!("HKEY_LOCAL_MACHINE/SOFTWARE/Wow6432Node/{tail}"));
                candidates.push(format!(
                    "HKEY_CURRENT_USER/Software/Classes/VirtualStore/MACHINE/SOFTWARE/{tail}"
                ));
                candidates.push(format!(
                    "HKEY_CURRENT_USER/Software/Classes/VirtualStore/MACHINE/SOFTWARE/Wow6432Node/{tail}"
                ));
            }

            for candidate in &candidates {
                log::trace!("[{name}] checking registry: {candidate}");
                for (scan_key, mut scanned) in
                    registry::win::scan_registry(name, candidate, filter, ignored_registry, previous_registry.as_ref())
                        .unwrap_or_default()
                {
                    log::debug!("[{name}] found registry: {}", scan_key.raw());

                    // Mark removed registry values.
                    let previous_values = previous_registry
                        .as_ref()
                        .and_then(|x| x.get_path(&scan_key).map(|y| y.0.keys().cloned().collect::<Vec<_>>()))
                        .unwrap_or_default();
                    for previous_value in previous_values {
                        #[allow(clippy::map_entry)]
                        if !scanned.values.contains_key(&previous_value) {
                            let ignored = ignored_registry.is_ignored(name, &scan_key, Some(&previous_value));
                            scanned.values.insert(
                                previous_value,
                                ScannedRegistryValue {
                                    ignored,
                                    change: ScanChange::Removed,
                                },
                            );
                        }
                    }

                    let _ = current_registry.back_up_key(name, &scan_key, &scanned);

                    found_registry_keys.insert(scan_key, scanned);
                }
            }
        }

        // Mark removed registry keys.
        if let Some(previous_registry) = &previous_registry {
            let current_registry_keys: Vec<_> = found_registry_keys.keys().map(|x| x.interpret()).collect();
            for (previous_hive, previous_keys) in &previous_registry.0 {
                for previous_key in previous_keys.0.keys() {
                    let path = RegistryItem::from_hive_and_key(previous_hive, previous_key);
                    if !current_registry_keys.contains(&path.interpret()) {
                        let ignored = ignored_registry.is_ignored(name, &path, None);
                        found_registry_keys.insert(
                            path,
                            ScannedRegistry {
                                change: ScanChange::Removed,
                                ignored,
                                values: Default::default(),
                            },
                        );
                    }
                }
            }
        }

        dumped_registry = (!current_registry.is_empty()).then_some(current_registry);
    }

    log::trace!("[{name}] completed scan for backup");

    ScanInfo {
        game_name: name.to_string(),
        found_files,
        found_registry_keys,
        available_backups: vec![],
        backup: None,
        has_backups,
        dumped_registry,
        only_constructive_backups,
    }
}

fn scan_game_for_backup_add_prefix(
    roots_to_check: &mut Vec<Root>,
    paths_to_check: &mut HashSet<(StrictPath, Option<bool>)>,
    wp: &StrictPath,
    has_registry: bool,
) {
    roots_to_check.push(Root::new(wp.clone(), Store::OtherWine));
    if has_registry {
        paths_to_check.insert((wp.joined("*.reg"), None));
    }
}

pub fn prepare_backup_target(target: &StrictPath) -> Result<(), Error> {
    if target.exists() && !target.is_dir() {
        log::error!("Backup target exists, but is not a directory: {target:?}");
        return Err(Error::CannotPrepareBackupTarget { path: target.clone() });
    }

    target.create_dirs().map_err(|e| {
        log::error!("Failed to prepare backup target: {target:?} | {e:?}");
        Error::CannotPrepareBackupTarget { path: target.clone() }
    })?;

    Ok(())
}

pub fn compare_games(
    key: SortKey,
    config: &Config,
    display_title1: &str,
    scan_info1: &ScanInfo,
    backup_info1: Option<&BackupInfo>,
    display_title2: &str,
    scan_info2: &ScanInfo,
    backup_info2: Option<&BackupInfo>,
) -> std::cmp::Ordering {
    match key {
        SortKey::Name => compare_games_by_name(display_title1, display_title2),
        SortKey::Size => compare_games_by_size(scan_info1, backup_info1, scan_info2, backup_info2),
        SortKey::Status => compare_games_by_status(config, scan_info1, scan_info2),
    }
}

fn compare_games_by_name(name1: &str, name2: &str) -> std::cmp::Ordering {
    name1.to_lowercase().cmp(&name2.to_lowercase()).then(name1.cmp(name2))
}

fn compare_games_by_size(
    scan_info1: &ScanInfo,
    backup_info1: Option<&BackupInfo>,
    scan_info2: &ScanInfo,
    backup_info2: Option<&BackupInfo>,
) -> std::cmp::Ordering {
    scan_info1
        .sum_bytes(backup_info1)
        .cmp(&scan_info2.sum_bytes(backup_info2))
        .then_with(|| compare_games_by_name(&scan_info1.game_name, &scan_info2.game_name))
}

fn compare_games_by_status(config: &Config, scan_info1: &ScanInfo, scan_info2: &ScanInfo) -> std::cmp::Ordering {
    let evaluate = |scan_info: &ScanInfo| {
        let change = scan_info.overall_change();
        match change {
            ScanChange::Unknown => ScanChange::Unknown,
            change => {
                if !config.is_game_enabled_for_operation(&scan_info.game_name, scan_info.scan_kind()) {
                    ScanChange::Same
                } else {
                    change
                }
            }
        }
    };

    let change1 = evaluate(scan_info1);
    let change2 = evaluate(scan_info2);

    change1
        .cmp(&change2)
        .then_with(|| compare_games_by_name(&scan_info1.game_name, &scan_info2.game_name))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, hash_map};

    use super::*;
    #[cfg(target_os = "windows")]
    use crate::resource::config::ToggledRegistryEntry;
    use crate::{
        resource::{config::Config, manifest::Manifest, ResourceFile},
        testing::{repo, s, EMPTY_HASH},
    };

    const ONLY_CONSTRUCTIVE: bool = false;

    fn config() -> Config {
        Config::load_from_string(&format!(
            r#"
            manifest:
              url: example.com
              etag: null
            roots:
              - path: {0}/tests/root1
                store: other
              - path: {0}/tests/root2
                store: other
            backup:
              path: ~/backup
            restore:
              path: ~/restore
            "#,
            repo()
        ))
        .unwrap()
    }

    fn manifest() -> Manifest {
        Manifest::load_from_string(
            r#"
            game1:
              files:
                <base>/file1.txt: {}
                <base>/subdir: {}
            game 2:
              files:
                <root>/<game>: {}
              installDir:
                game2: {}
            game3:
              registry:
                HKEY_CURRENT_USER/Software/Ludusavi/game3: {}
                HKEY_CURRENT_USER/Software/Ludusavi/fake: {}
            game3-outer:
              registry:
                HKEY_CURRENT_USER/Software/Ludusavi: {}
            game4:
              files:
                <home>/data.txt: {}
                <winAppData>/winAppData.txt: {}
                <winLocalAppData>/winLocalAppData.txt: {}
                <winDocuments>/winDocuments.txt: {}
                <xdgConfig>/xdgConfig.txt: {}
                <xdgData>/xdgData.txt: {}
            game5:
              files:
                <base>: {}
            fake-registry:
              registry:
                HKEY_CURRENT_USER/Software/Ludusavi/fake: {}
            install-dir-with-glob-characters:
              installDir:
                'game-[not]-glob': {}
              files:
                <base>/file1.txt: {}
                <root>/<game>/file2.txt: {}
            "#,
        )
        .unwrap()
    }

    #[test]
    fn can_compute_game_file_target() {
        // No redirects
        assert_eq!(
            None,
            game_file_target(&StrictPath::new("/foo"), &[], false, ScanKind::Backup)
        );

        // Match - backup
        assert_eq!(
            Some(StrictPath::new("/quux")),
            game_file_target(
                &StrictPath::new("/foo"),
                &[
                    RedirectConfig {
                        kind: RedirectKind::Backup,
                        source: StrictPath::new("/foo"),
                        target: StrictPath::new("/bar"),
                    },
                    RedirectConfig {
                        kind: RedirectKind::Restore,
                        source: StrictPath::new("/bar"),
                        target: StrictPath::new("/baz"),
                    },
                    RedirectConfig {
                        kind: RedirectKind::Bidirectional,
                        source: StrictPath::new("/bar"),
                        target: StrictPath::new("/quux"),
                    },
                ],
                false,
                ScanKind::Backup,
            ),
        );

        // Match - restore
        assert_eq!(
            Some(StrictPath::new("/foo")),
            game_file_target(
                &StrictPath::new("/quux"),
                &[
                    RedirectConfig {
                        kind: RedirectKind::Bidirectional,
                        source: StrictPath::new("/bar"),
                        target: StrictPath::new("/quux"),
                    },
                    RedirectConfig {
                        kind: RedirectKind::Restore,
                        source: StrictPath::new("/bar"),
                        target: StrictPath::new("/foo"),
                    },
                    RedirectConfig {
                        kind: RedirectKind::Backup,
                        source: StrictPath::new("/foo"),
                        target: StrictPath::new("/baz"),
                    },
                ],
                false,
                ScanKind::Restore,
            ),
        );

        // Match - restore, reversed
        assert_eq!(
            Some(StrictPath::new("/bar")),
            game_file_target(
                &StrictPath::new("/quux"),
                &[
                    RedirectConfig {
                        kind: RedirectKind::Bidirectional,
                        source: StrictPath::new("/bar"),
                        target: StrictPath::new("/quux"),
                    },
                    RedirectConfig {
                        kind: RedirectKind::Restore,
                        source: StrictPath::new("/bar"),
                        target: StrictPath::new("/foo"),
                    },
                    RedirectConfig {
                        kind: RedirectKind::Backup,
                        source: StrictPath::new("/foo"),
                        target: StrictPath::new("/baz"),
                    },
                ],
                true,
                ScanKind::Restore,
            ),
        );

        // Mismatch - partial name
        assert_eq!(
            None,
            game_file_target(
                &StrictPath::new("/foo"),
                &[RedirectConfig {
                    kind: RedirectKind::Backup,
                    source: StrictPath::new("/f"),
                    target: StrictPath::new("/b"),
                },],
                false,
                ScanKind::Backup,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches() {
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hash_map! {
                    format!("{}/tests/root1/game1/subdir/file2.txt", repo()).into(): ScannedFile::new(2, "9d891e731f75deae56884d79e9816736b7488080").change_new(),
                    format!("{}/tests/root2/game1/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );

        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hash_map! {
                    format!("{}/tests/root2/game2/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game 2".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_deduplicating_symlinks() {
        let roots = &[Root::new(format!("{}/tests/root3", repo()), Store::Other)];
        assert_eq!(
            ScanInfo {
                game_name: s("game5"),
                found_files: hash_map! {
                    format!("{}/tests/root3/game5/data/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game5"],
                "game5",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game5".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_redirect_to_symlink() {
        let roots = &[Root::new(format!("{}/tests/root3", repo()), Store::Other)];
        assert_eq!(
            ScanInfo {
                game_name: s("game5"),
                found_files: hash_map! {
                    format!("{}/tests/root3/game5/data/file1.txt", repo()).into(): ScannedFile {
                        size: 1,
                        hash: "3a52ce780950d4d969792a2559cd519d7ee8c727".to_string(),
                        original_path: None,
                        ignored: false,
                        change: ScanChange::New,
                        container: None,
                        redirected: Some(StrictPath::new(format!("{}/tests/root3/game5/data-symlink/file1.txt", repo()))),
                    },
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game5"],
                "game5",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game5".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[RedirectConfig {
                    kind: RedirectKind::Bidirectional,
                    source: StrictPath::new(format!("{}/tests/root3/game5/data", repo())),
                    target: StrictPath::new(format!("{}/tests/root3/game5/data-symlink", repo())),
                }],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_fuzzy_matched_install_dir() {
        let roots = &[Root::new(format!("{}/tests/root3", repo()), Store::Other)];
        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hash_map! {
                    format!("{}/tests/root3/game_2/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game 2".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_escaped_glob_characters() {
        let config = Config::load_from_string(&format!(
            r#"
            roots:
              - path: {0}/tests/root-[[]not[]]-glob
                store: other
            "#,
            repo()
        ))
        .unwrap();

        dbg!(&config.roots);
        let roots = config.expanded_roots();
        dbg!(&roots);

        assert_eq!(
            ScanInfo {
                game_name: s("install-dir-with-glob-characters"),
                found_files: hash_map! {
                    format!("{}/tests/root-[not]-glob/game-[not]-glob/file1.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                    format!("{}/tests/root-[not]-glob/game-[not]-glob/file2.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["install-dir-with-glob-characters"],
                "install-dir-with-glob-characters",
                &roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&roots, &manifest(), &["install-dir-with-glob-characters".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_file_matches_in_custom_home_folder() {
        let roots = &[Root::new(format!("{}/tests/home", repo()), Store::OtherHome)];
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hash_map! {
                    format!("{}/tests/home/data.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                    format!("{}/tests/home/AppData/Roaming/winAppData.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                    format!("{}/tests/home/AppData/Local/winLocalAppData.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                    format!("{}/tests/home/Documents/winDocuments.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game4".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn can_scan_game_for_backup_with_file_matches_in_custom_home_folder() {
        let roots = &[Root::new(format!("{}/tests/home", repo()), Store::OtherHome)];
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hash_map! {
                    format!("{}/tests/home/data.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                    format!("{}/tests/home/.config/xdgConfig.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                    format!("{}/tests/home/.local/share/xdgData.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game4".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_in_wine_prefix() {
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hash_map! {
                    format!("{}/tests/wine-prefix/drive_c/users/anyone/data.txt", repo()).into(): ScannedFile::new(0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game4".to_string()]),
                &BackupFilter::default(),
                Some(&StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_registry_files_in_wine_prefix() {
        assert_eq!(
            ScanInfo {
                game_name: s("fake-registry"),
                found_files: hash_map! {
                    format!("{}/tests/wine-prefix/user.reg", repo()).into(): ScannedFile::new(37, "4a5b7e9de7d84ffb4bb3e9f38667f85741d5fbc0",).change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["fake-registry"],
                "fake-registry",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["fake-registry".to_string()]),
                &BackupFilter::default(),
                Some(&StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_and_ignored_directory() {
        let mut filter = BackupFilter {
            ignored_paths: vec![StrictPath::new(format!("{}\\tests/root1/game1/subdir", repo()))],
            ..Default::default()
        };
        let ignored = ToggledPaths::default();
        let found = hash_map! {
            format!("{}/tests/root2/game1/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
        };

        filter.build_globs();
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: found,
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                None,
                &ignored,
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_and_toggled_directory() {
        let mut filter = BackupFilter::default();
        let ignored = ToggledPaths::new(btree_map! {
            s("game1"): btree_map! {
                StrictPath::new(format!("{}\\tests/root1/game1/subdir", repo())): false
            }
        });
        let found = hash_map! {
            format!("{}/tests/root1/game1/subdir/file2.txt", repo()).into(): ScannedFile::new(2, "9d891e731f75deae56884d79e9816736b7488080").change_new().ignored(),
            format!("{}/tests/root2/game1/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
        };

        filter.build_globs();
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: found,
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                None,
                &ignored,
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_and_toggled_file() {
        let mut filter = BackupFilter::default();
        let ignored = ToggledPaths::new(btree_map! {
            s("game1"): btree_map! {
                StrictPath::new(format!("{}\\tests/root1/game1/subdir/file2.txt", repo())): false
            }
        });
        let found = hash_map! {
            format!("{}/tests/root1/game1/subdir/file2.txt", repo()).into(): ScannedFile::new(2, "9d891e731f75deae56884d79e9816736b7488080").change_new().ignored(),
            format!("{}/tests/root2/game1/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
        };

        filter.build_globs();
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: found,
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                None,
                &ignored,
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_leaf_key_with_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3"),
                found_files: hash_map! {},
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi/game3".into(): ScannedRegistry::new().change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz")
                },
                dumped_registry: Some(registry::Hives(btree_map! {
                    r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                        r"Software\Ludusavi\game3".into(): registry::Entries(btree_map! {
                            "binary".into(): registry::Entry::Binary(vec![65]),
                            "dword".into(): registry::Entry::Dword(1),
                            "expandSz".into(): registry::Entry::ExpandSz("baz".to_string()),
                            "multiSz".into(): registry::Entry::MultiSz("bar".to_string()),
                            "qword".into(): registry::Entry::Qword(2),
                            "sz".into(): registry::Entry::Sz("foo".to_string()),
                        }),
                    })
                })),
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3"],
                "game3",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game3".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_parent_key_without_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3-outer"),
                found_files: hash_map! {},
                found_registry_keys: hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi".into(): ScannedRegistry::new().change_as(ScanChange::New),
                    "HKEY_CURRENT_USER/Software/Ludusavi/game3".into(): ScannedRegistry::new().change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz"),
                    "HKEY_CURRENT_USER/Software/Ludusavi/invalid".into(): ScannedRegistry::new().change_as(ScanChange::New)
                        .with_value_new("dword"),
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().change_as(ScanChange::New),
                },
                dumped_registry: Some(registry::Hives(btree_map! {
                    r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                        r"Software\Ludusavi".into(): registry::Entries(btree_map! {}),
                        r"Software\Ludusavi\game3".into(): registry::Entries(btree_map! {
                            "binary".into(): registry::Entry::Binary(vec![65]),
                            "dword".into(): registry::Entry::Dword(1),
                            "expandSz".into(): registry::Entry::ExpandSz("baz".to_string()),
                            "multiSz".into(): registry::Entry::MultiSz("bar".to_string()),
                            "qword".into(): registry::Entry::Qword(2),
                            "sz".into(): registry::Entry::Sz("foo".to_string()),
                        }),
                        r"Software\Ludusavi\invalid".into(): registry::Entries(btree_map! {
                            "dword".into(): registry::Entry::Raw { kind: registry::RegistryKind::Dword, data: vec![0, 0, 0, 0, 0, 0, 0, 0] },
                        }),
                        r"Software\Ludusavi\other".into(): registry::Entries(btree_map! {}),
                    })
                })),
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3-outer"],
                "game3-outer",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game3-outer".to_string()]),
                &BackupFilter::default(),
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_and_ignores() {
        let cases = vec![
            (
                BackupFilter {
                    ignored_registry: vec![
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/invalid")),
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/other")),
                    ],
                    ..Default::default()
                },
                ToggledRegistry::default(),
                hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi".into(): ScannedRegistry::new().change_as(ScanChange::New),
                    "HKEY_CURRENT_USER/Software/Ludusavi/game3".into(): ScannedRegistry::new().change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz"),
                },
                Some(registry::Hives(btree_map! {
                    r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                        r"Software\Ludusavi".into(): registry::Entries(btree_map! {}),
                        r"Software\Ludusavi\game3".into(): registry::Entries(btree_map! {
                            "binary".into(): registry::Entry::Binary(vec![65]),
                            "dword".into(): registry::Entry::Dword(1),
                            "expandSz".into(): registry::Entry::ExpandSz("baz".to_string()),
                            "multiSz".into(): registry::Entry::MultiSz("bar".to_string()),
                            "qword".into(): registry::Entry::Qword(2),
                            "sz".into(): registry::Entry::Sz("foo".to_string()),
                        }),
                    })
                })),
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btree_map! {
                    s("game3-outer"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi")): ToggledRegistryEntry::Key(false)
                    }
                }),
                hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi".into(): ScannedRegistry::new().ignored().change_as(ScanChange::New),
                    "HKEY_CURRENT_USER/Software/Ludusavi/game3".into():  ScannedRegistry::new().ignored().change_as(ScanChange::New)
                        .with_value("binary", ScanChange::New, true)
                        .with_value("dword", ScanChange::New, true)
                        .with_value("expandSz", ScanChange::New, true)
                        .with_value("multiSz", ScanChange::New, true)
                        .with_value("qword", ScanChange::New, true)
                        .with_value("sz", ScanChange::New, true),
                    "HKEY_CURRENT_USER/Software/Ludusavi/invalid".into(): ScannedRegistry::new().ignored().change_as(ScanChange::New)
                        .with_value("dword", ScanChange::New, true),
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().ignored().change_as(ScanChange::New),
                },
                None,
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btree_map! {
                    s("game3-outer"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/game3")): ToggledRegistryEntry::Complex {
                            key: None,
                            values: btree_map! {
                                s("qword"): false,
                            },
                        },
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi/other")): ToggledRegistryEntry::Key(false),
                    }
                }),
                hash_map! {
                    "HKEY_CURRENT_USER/Software/Ludusavi".into(): ScannedRegistry::new().change_as(ScanChange::New),
                    "HKEY_CURRENT_USER/Software/Ludusavi/game3".into(): ScannedRegistry::new().change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value("qword", ScanChange::New, true)
                        .with_value_new("sz"),
                    "HKEY_CURRENT_USER/Software/Ludusavi/invalid".into(): ScannedRegistry::new().change_as(ScanChange::New)
                        .with_value_new("dword"),
                    "HKEY_CURRENT_USER/Software/Ludusavi/other".into(): ScannedRegistry::new().ignored().change_as(ScanChange::New),
                },
                Some(registry::Hives(btree_map! {
                    r"HKEY_CURRENT_USER".into(): registry::Keys(btree_map! {
                        r"Software\Ludusavi".into(): registry::Entries(btree_map! {}),
                        r"Software\Ludusavi\game3".into(): registry::Entries(btree_map! {
                            "binary".into(): registry::Entry::Binary(vec![65]),
                            "dword".into(): registry::Entry::Dword(1),
                            "expandSz".into(): registry::Entry::ExpandSz("baz".to_string()),
                            "multiSz".into(): registry::Entry::MultiSz("bar".to_string()),
                            "sz".into(): registry::Entry::Sz("foo".to_string()),
                        }),
                        r"Software\Ludusavi\invalid".into(): registry::Entries(btree_map! {
                            "dword".into(): registry::Entry::Raw { kind: registry::RegistryKind::Dword, data: vec![0, 0, 0, 0, 0, 0, 0, 0] },
                        }),
                    })
                })),
            ),
        ];

        for (filter, ignored, found, dumped_registry) in cases {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3-outer"),
                    found_files: hash_map! {},
                    found_registry_keys: found,
                    dumped_registry,
                    ..Default::default()
                },
                scan_game_for_backup(
                    &manifest().0["game3-outer"],
                    "game3-outer",
                    &config().roots,
                    &StrictPath::new(repo()),
                    &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                    &filter,
                    None,
                    &ToggledPaths::default(),
                    &ignored,
                    None,
                    &[],
                    false,
                    &Default::default(),
                    ONLY_CONSTRUCTIVE,
                ),
            );
        }
    }

    #[test]
    fn can_scan_game_for_backup_with_exact_exclusions() {
        let mut filter = BackupFilter {
            ignored_paths: vec![format!("{}/tests/root1/game1/subdir/file2.txt", repo()).into()],
            ..Default::default()
        };
        filter.build_globs();

        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hash_map! {
                    format!("{}/tests/root2/game1/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_glob_exclusions() {
        let mut filter = BackupFilter {
            ignored_paths: vec!["**/*2.txt".into()],
            ..Default::default()
        };
        filter.build_globs();

        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hash_map! {
                    format!("{}/tests/root2/game1/file1.txt", repo()).into(): ScannedFile::new(1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_map! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                false,
                &Default::default(),
                ONLY_CONSTRUCTIVE,
            ),
        );
    }
}
