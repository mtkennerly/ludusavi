mod backup;
mod change;
mod duplicate;
pub mod game_filter;
pub mod launchers;
pub mod layout;
mod preview;
pub mod registry_compat;
mod saves;
mod steam;
mod title;

#[cfg(target_os = "windows")]
pub mod registry;

use std::collections::{HashMap, HashSet};

pub use self::{backup::*, change::*, duplicate::*, launchers::*, preview::*, saves::*, steam::*, title::*};

use crate::{
    path::{CommonPath, StrictPath},
    prelude::{filter_map_walkdir, Error, SKIP},
    resource::{
        config::{BackupFilter, RedirectConfig, RedirectKind, RootsConfig, SortKey, ToggledPaths, ToggledRegistry},
        manifest::{Game, GameFileEntry, IdMetadata, Os, Store},
    },
    scan::layout::LatestBackup,
};

#[cfg(target_os = "windows")]
use crate::scan::registry_compat::RegistryItem;

/// Returns the effective target, if different from the original
pub fn game_file_target(
    original_target: &StrictPath,
    redirects: &[RedirectConfig],
    restoring: bool,
) -> Option<StrictPath> {
    if redirects.is_empty() {
        return None;
    }

    let mut redirected_target = original_target.render();
    for redirect in redirects {
        if redirect.source.raw().trim().is_empty() || redirect.target.raw().trim().is_empty() {
            continue;
        }
        let (source, target) = if !restoring {
            match redirect.kind {
                RedirectKind::Backup | RedirectKind::Bidirectional => {
                    (redirect.source.render(), redirect.target.render())
                }
                RedirectKind::Restore => continue,
            }
        } else {
            match redirect.kind {
                RedirectKind::Backup => continue,
                RedirectKind::Restore => (redirect.source.render(), redirect.target.render()),
                RedirectKind::Bidirectional => (redirect.target.render(), redirect.source.render()),
            }
        };
        if !source.is_empty() && !target.is_empty() && redirected_target.starts_with(&source) {
            redirected_target = redirected_target.replacen(&source, &target, 1);
        }
    }

    let redirected_target = StrictPath::new(redirected_target);
    if original_target.render() != redirected_target.render() {
        Some(redirected_target)
    } else {
        None
    }
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

pub fn steam_ids(game: &Game, shortcut: Option<&SteamShortcut>) -> Vec<u32> {
    let mut ids = vec![];
    if let Some(steam_id) = game.steam.as_ref().and_then(|x| x.id) {
        ids.push(steam_id);
    }
    if let Some(id_section) = game.id.as_ref() {
        for extra in &id_section.steam_extra {
            ids.push(*extra);
        }
    }
    if let Some(shortcut) = shortcut {
        ids.push(shortcut.id);
    }
    ids
}

/// Returns paths to check and whether they require case-sensitive matching.
pub fn parse_paths(
    path: &str,
    data: &GameFileEntry,
    root: &RootsConfig,
    install_dir: &Option<String>,
    full_install_dir: &Option<&StrictPath>,
    steam_ids: &[u32],
    ids: Option<&IdMetadata>,
    manifest_dir: &StrictPath,
    steam_shortcut: Option<&SteamShortcut>,
    platform: Os,
) -> HashSet<(StrictPath, bool)> {
    use crate::resource::manifest::placeholder::*;

    let mut paths = HashSet::new();

    // Since STORE_USER_ID becomes `*`, we don't want to end up with an invalid `**`.
    let path = path
        .replace(&format!("*{}", STORE_USER_ID), STORE_USER_ID)
        .replace(&format!("{}*", STORE_USER_ID), STORE_USER_ID);

    let install_dir = match install_dir {
        Some(d) => d,
        None => SKIP,
    };

    let Ok(root_interpreted) = root.path.interpret_unless_skip() else {
        return HashSet::new();
    };
    let data_dir = CommonPath::Data.get_or_skip();
    let data_local_dir = CommonPath::DataLocal.get_or_skip();
    let config_dir = CommonPath::Config.get_or_skip();
    let home = CommonPath::Home.get_or_skip();

    #[cfg(target_os = "windows")]
    let saved_games_dir = known_folders::get_known_folder_path(known_folders::KnownFolder::SavedGames)
        .map(|x| x.to_string_lossy().trim_end_matches(['/', '\\']).to_string());
    #[cfg(not(target_os = "windows"))]
    let saved_games_dir: Option<String> = None;

    paths.insert((
        path.replace(ROOT, &root_interpreted)
            .replace(GAME, install_dir)
            .replace(
                BASE,
                &match root.store {
                    Store::Steam => format!("{}/steamapps/common/{}", &root_interpreted, install_dir),
                    Store::Heroic | Store::Legendary | Store::Lutris => full_install_dir
                        .and_then(|x| x.interpret().ok())
                        .unwrap_or_else(|| SKIP.to_string()),
                    Store::Ea
                    | Store::Epic
                    | Store::Gog
                    | Store::GogGalaxy
                    | Store::Microsoft
                    | Store::Origin
                    | Store::Prime
                    | Store::Uplay
                    | Store::OtherHome
                    | Store::OtherWine
                    | Store::OtherWindows
                    | Store::OtherLinux
                    | Store::OtherMac
                    | Store::Other => format!("{}/{}", &root_interpreted, install_dir),
                },
            )
            .replace(HOME, home)
            .replace(STORE_USER_ID, "*")
            .replace(OS_USER_NAME, &crate::prelude::OS_USERNAME)
            .replace(WIN_APP_DATA, check_windows_path(data_dir))
            .replace(WIN_LOCAL_APP_DATA, check_windows_path(data_local_dir))
            .replace(WIN_DOCUMENTS, check_windows_path(CommonPath::Document.get_or_skip()))
            .replace(WIN_PUBLIC, check_windows_path(CommonPath::Public.get_or_skip()))
            .replace(WIN_PROGRAM_DATA, check_windows_path("C:/ProgramData"))
            .replace(WIN_DIR, check_windows_path("C:/Windows"))
            .replace(XDG_DATA, check_nonwindows_path(data_dir))
            .replace(XDG_CONFIG, check_nonwindows_path(config_dir)),
        platform.is_case_sensitive(),
    ));
    if Os::HOST == Os::Windows {
        let (mut virtual_store, case_sensitive) = paths.iter().next().unwrap().clone();
        for virtualized in ["Program Files (x86)", "Program Files", "Windows", "ProgramData"] {
            for separator in ['/', '\\'] {
                virtual_store = virtual_store.replace(
                    &format!("C:{}{}", separator, virtualized),
                    &format!("{}/VirtualStore/{}", &data_local_dir, virtualized),
                );
            }
        }
        paths.insert((virtual_store, case_sensitive));

        if let Some(saved_games_dir) = saved_games_dir.as_ref() {
            paths.insert((
                path.replace('\\', "/")
                    .replace(GAME, install_dir)
                    .replace(STORE_USER_ID, "*")
                    .replace(OS_USER_NAME, &crate::prelude::OS_USERNAME)
                    .replace("<home>/Saved Games/", &format!("{}/", saved_games_dir))
                    .replace(HOME, home),
                platform.is_case_sensitive(),
            ));
        }
    }
    if Os::HOST == Os::Linux {
        // Default XDG paths, in case we're in a Flatpak context.
        paths.insert((
            path.replace(GAME, install_dir)
                .replace(STORE_USER_ID, "*")
                .replace(OS_USER_NAME, &crate::prelude::OS_USERNAME)
                .replace(XDG_DATA, "<home>/.local/share")
                .replace(XDG_CONFIG, "<home>/.config")
                .replace(HOME, home),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::Gog && Os::HOST == Os::Linux {
        paths.insert((
            path.replace(GAME, &format!("{}/game", install_dir))
                .replace(BASE, &format!("{}/{}/game", &root_interpreted, install_dir)),
            platform.is_case_sensitive(),
        ));
    }

    // NOTE.2022-10-26 - Heroic flatpak installation detection
    //
    // flatpak wiki on filesystems
    // (https://github.com/flatpak/flatpak/wiki/Filesystem) as well as
    // https://docs.flatpak.org do not seem to mention an option to relocate
    // per-app data directories.  These are by default located in
    // $HOME/.var/app/$FLATPAK_ID, so we cat detect a flatpak installed heroic
    // by looking at the `root_interpreted` and check for
    // ".var/app/com.heroicgameslauncher.hgl/config/heroic"
    if root.store == Store::Heroic
        && Os::HOST == Os::Linux
        && root_interpreted.ends_with(".var/app/com.heroicgameslauncher.hgl/config/heroic")
    {
        paths.insert((
            path.replace(
                XDG_DATA,
                check_nonwindows_path(&format!("{}/../../data", &root_interpreted)),
            )
            .replace(
                XDG_CONFIG,
                check_nonwindows_path(&format!("{}/../../config", &root_interpreted)),
            )
            .replace(STORE_USER_ID, "*"),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::OtherHome {
        paths.insert((
            path.replace(ROOT, &root_interpreted)
                .replace(GAME, install_dir)
                .replace(BASE, &format!("{}/{}", &root_interpreted, install_dir))
                .replace(STORE_USER_ID, SKIP)
                .replace(OS_USER_NAME, &crate::prelude::OS_USERNAME)
                .replace(WIN_APP_DATA, check_windows_path("<home>/AppData/Roaming"))
                .replace(WIN_LOCAL_APP_DATA, check_windows_path("<home>/AppData/Local"))
                .replace(WIN_DOCUMENTS, check_windows_path("<home>/Documents"))
                .replace(WIN_PUBLIC, check_windows_path(CommonPath::Public.get_or_skip()))
                .replace(WIN_PROGRAM_DATA, check_windows_path("C:/ProgramData"))
                .replace(WIN_DIR, check_windows_path("C:/Windows"))
                .replace(XDG_DATA, check_nonwindows_path("<home>/.local/share"))
                .replace(XDG_CONFIG, check_nonwindows_path("<home>/.config"))
                .replace(HOME, &root_interpreted),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::Steam {
        if let Some(steam_shortcut) = steam_shortcut {
            if let Some(start_dir) = &steam_shortcut.start_dir {
                if let Ok(start_dir) = start_dir.interpret() {
                    paths.insert((path.replace(BASE, &start_dir), platform.is_case_sensitive()));
                }
            }
        }
    }
    if root.store == Store::Steam && Os::HOST == Os::Linux {
        // Check XDG folders inside of Steam installation.
        if root_interpreted.ends_with(".var/app/com.valvesoftware.Steam/.steam/steam") {
            paths.insert((
                path.replace(STORE_USER_ID, "*")
                    .replace(OS_USER_NAME, &crate::prelude::OS_USERNAME)
                    .replace(XDG_DATA, &format!("{}../../.local/share", &root_interpreted))
                    .replace(XDG_CONFIG, &format!("{}../../.config", &root_interpreted)),
                platform.is_case_sensitive(),
            ));
        }

        for id in steam_ids {
            let prefix = format!("{}/steamapps/compatdata/{}/pfx/drive_c", &root_interpreted, id);
            let path2 = path
                .replace(ROOT, &root_interpreted)
                .replace(GAME, install_dir)
                .replace(BASE, &format!("{}/steamapps/common/{}", &root_interpreted, install_dir))
                .replace(HOME, &format!("{}/users/steamuser", prefix))
                .replace(STORE_USER_ID, "*")
                .replace(OS_USER_NAME, "steamuser")
                .replace(WIN_PUBLIC, &format!("{}/users/Public", prefix))
                .replace(WIN_PROGRAM_DATA, &format!("{}/ProgramData", prefix))
                .replace(WIN_DIR, &format!("{}/windows", prefix))
                .replace(XDG_DATA, check_nonwindows_path(data_dir))
                .replace(XDG_CONFIG, check_nonwindows_path(config_dir));
            paths.insert((
                path2
                    .replace(WIN_DOCUMENTS, &format!("{}/users/steamuser/Documents", prefix))
                    .replace(WIN_APP_DATA, &format!("{}/users/steamuser/AppData/Roaming", prefix))
                    .replace(WIN_LOCAL_APP_DATA, &format!("{}/users/steamuser/AppData/Local", prefix)),
                false,
            ));
            paths.insert((
                path2
                    .replace(WIN_DOCUMENTS, &format!("{}/users/steamuser/My Documents", prefix))
                    .replace(WIN_APP_DATA, &format!("{}/users/steamuser/Application Data", prefix))
                    .replace(
                        WIN_LOCAL_APP_DATA,
                        &format!("{}/users/steamuser/Local Settings/Application Data", prefix),
                    ),
                false,
            ));

            if data
                .when
                .as_ref()
                .map(|x| x.iter().any(|x| x.store == Some(Store::Uplay)))
                .unwrap_or_default()
            {
                let ubisoft = format!("{}/Program Files (x86)/Ubisoft/Ubisoft Game Launcher", prefix);
                paths.insert((
                    path.replace(ROOT, &ubisoft)
                        .replace(GAME, install_dir)
                        .replace(BASE, &format!("{}/{}", &ubisoft, install_dir))
                        .replace(STORE_USER_ID, "*")
                        .replace(OS_USER_NAME, "steamuser"),
                    platform.is_case_sensitive(),
                ));
            }
        }
    }
    if root.store == Store::OtherWine {
        let prefix = format!("{}/drive_*", &root_interpreted);
        let path2 = path
            .replace(ROOT, &root_interpreted)
            .replace(GAME, install_dir)
            .replace(BASE, &format!("{}/{}", &root_interpreted, install_dir))
            .replace(HOME, &format!("{}/users/*", prefix))
            .replace(STORE_USER_ID, "*")
            .replace(OS_USER_NAME, "*")
            .replace(WIN_PUBLIC, &format!("{}/users/Public", prefix))
            .replace(WIN_PROGRAM_DATA, &format!("{}/ProgramData", prefix))
            .replace(WIN_DIR, &format!("{}/windows", prefix))
            .replace(XDG_DATA, check_nonwindows_path(data_dir))
            .replace(XDG_CONFIG, check_nonwindows_path(config_dir));
        paths.insert((
            path2
                .replace(WIN_DOCUMENTS, &format!("{}/users/*/Documents", prefix))
                .replace(WIN_APP_DATA, &format!("{}/users/*/AppData/Roaming", prefix))
                .replace(WIN_LOCAL_APP_DATA, &format!("{}/users/*/AppData/Local", prefix)),
            false,
        ));
        paths.insert((
            path2
                .replace(WIN_DOCUMENTS, &format!("{}/users/*/My Documents", prefix))
                .replace(WIN_APP_DATA, &format!("{}/users/*/Application Data", prefix))
                .replace(
                    WIN_LOCAL_APP_DATA,
                    &format!("{}/users/*/Local Settings/Application Data", prefix),
                ),
            false,
        ));
    }

    if root.store == Store::OtherWindows {
        paths.insert((
            path.replace(HOME, &format!("{}/Users/*", &root_interpreted))
                .replace(STORE_USER_ID, "*")
                .replace(OS_USER_NAME, "*")
                .replace(WIN_APP_DATA, &format!("{}/Users/*/AppData/Roaming", &root_interpreted))
                .replace(
                    WIN_LOCAL_APP_DATA,
                    &format!("{}/Users/*/AppData/Local", &root_interpreted),
                )
                .replace(WIN_DOCUMENTS, &format!("{}/Users/*/Documents", &root_interpreted))
                .replace(WIN_PUBLIC, &format!("{}/Users/Public", &root_interpreted))
                .replace(WIN_PROGRAM_DATA, &format!("{}/ProgramData", &root_interpreted))
                .replace(WIN_DIR, &format!("{}/Windows", &root_interpreted)),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::OtherLinux {
        paths.insert((
            path.replace(HOME, &format!("{}/home/*", &root_interpreted))
                .replace(STORE_USER_ID, "*")
                .replace(OS_USER_NAME, "*")
                .replace(XDG_DATA, &format!("{}/home/*/.local/share", &root_interpreted))
                .replace(XDG_CONFIG, &format!("{}/home/*/.config", &root_interpreted)),
            platform.is_case_sensitive(),
        ));
    }
    if root.store == Store::OtherMac {
        paths.insert((
            path.replace(HOME, &format!("{}/Users/*", &root_interpreted))
                .replace(STORE_USER_ID, "*")
                .replace(OS_USER_NAME, "*")
                .replace(XDG_DATA, &format!("{}/Users/*/Library", &root_interpreted))
                .replace(
                    XDG_CONFIG,
                    &format!("{}/Users/*/Library/Preferences", &root_interpreted),
                ),
            platform.is_case_sensitive(),
        ));
    }

    if Os::HOST != Os::Windows {
        if let Some(flatpak_id) = ids.and_then(|x| x.flatpak.as_ref()) {
            paths.insert((
                path.replace(HOME, home)
                    .replace(STORE_USER_ID, "*")
                    .replace(OS_USER_NAME, "*")
                    .replace(XDG_DATA, &format!("{home}/.var/app/{flatpak_id}/data"))
                    .replace(XDG_CONFIG, &format!("{home}/.var/app/{flatpak_id}/config")),
                platform.is_case_sensitive(),
            ));

            if root.store == Store::OtherHome {
                let home = &root_interpreted;
                paths.insert((
                    path.replace(HOME, home)
                        .replace(STORE_USER_ID, "*")
                        .replace(OS_USER_NAME, "*")
                        .replace(XDG_DATA, &format!("{home}/.var/app/{flatpak_id}/data"))
                        .replace(XDG_CONFIG, &format!("{home}/.var/app/{flatpak_id}/config")),
                    platform.is_case_sensitive(),
                ));
            }
        }
    }

    paths
        .iter()
        .map(|(x, y)| (StrictPath::relative(x.to_string(), manifest_dir.interpret().ok()), *y))
        .collect()
}

pub fn scan_game_for_backup(
    game: &Game,
    name: &str,
    roots: &[RootsConfig],
    manifest_dir: &StrictPath,
    launchers: &Launchers,
    filter: &BackupFilter,
    wine_prefix: &Option<StrictPath>,
    ignored_paths: &ToggledPaths,
    #[allow(unused_variables)] ignored_registry: &ToggledRegistry,
    previous: Option<LatestBackup>,
    redirects: &[RedirectConfig],
    steam_shortcuts: &SteamShortcuts,
) -> ScanInfo {
    log::trace!("[{name}] beginning scan for backup");

    let mut found_files = HashSet::new();
    #[allow(unused_mut)]
    let mut found_registry_keys = HashSet::new();

    let mut paths_to_check = HashSet::<(StrictPath, Option<bool>)>::new();

    // Add a dummy root for checking paths without `<root>`.
    let mut roots_to_check: Vec<RootsConfig> = vec![RootsConfig {
        path: StrictPath::new(SKIP.to_string()),
        store: Store::Other,
    }];
    roots_to_check.extend(roots.iter().cloned());

    let manifest_dir_interpreted = manifest_dir.interpret().unwrap();
    let steam_ids = steam_ids(game, steam_shortcuts.get(name));

    // We can add this for Wine prefixes from the CLI because they're
    // typically going to be used for only one or a few games at a time.
    // For other Wine roots, it would trigger for every game.
    if let Some(wp) = wine_prefix {
        log::trace!("[{name}] adding extra Wine prefix: {}", wp.raw());
        scan_game_for_backup_add_prefix(&mut roots_to_check, &mut paths_to_check, wp, game.registry.is_some());
    }

    // handle what was found for heroic
    for root in roots {
        if let Some(wp) = launchers.get_prefix(root, name) {
            let with_pfx = wp.joined("pfx");
            scan_game_for_backup_add_prefix(
                &mut roots_to_check,
                &mut paths_to_check,
                if with_pfx.exists() { &with_pfx } else { wp },
                game.registry.is_some(),
            );
        }
    }

    for root in roots_to_check {
        log::trace!(
            "[{name}] adding candidates from {:?} root: {}",
            root.store,
            root.path.raw()
        );
        if root.path.raw().trim().is_empty() {
            continue;
        }
        let Ok(root_interpreted) = root.path.interpret_unless_skip() else {
            log::error!("Invalid root: {:?}", &root.path);
            continue;
        };

        let platform = launchers.get_platform(&root, name).unwrap_or(Os::HOST);

        if let Some(files) = &game.files {
            let install_dir = launchers.get_install_dir_leaf(&root, name);
            let full_install_dir = launchers.get_install_dir(&root, name);

            for (raw_path, path_data) in files {
                log::trace!("[{name}] parsing candidates from: {}", raw_path);
                if raw_path.trim().is_empty() {
                    continue;
                }
                let candidates = parse_paths(
                    raw_path,
                    path_data,
                    &root,
                    &install_dir,
                    &full_install_dir,
                    &steam_ids,
                    game.id.as_ref(),
                    manifest_dir,
                    steam_shortcuts.get(name),
                    platform,
                );
                for (candidate, case_sensitive) in candidates {
                    log::trace!("[{name}] parsed candidate: {}", candidate.raw());
                    if candidate.raw().contains('<') {
                        // This covers `SKIP` and any other unmatched placeholders.
                        continue;
                    }
                    paths_to_check.insert((candidate, Some(case_sensitive)));
                }
            }
        }
        if root.store == Store::Steam {
            for id in &steam_ids {
                // Cloud saves:
                paths_to_check.insert((
                    StrictPath::relative(
                        format!("{}/userdata/*/{}/remote/", root_interpreted.clone(), id),
                        Some(manifest_dir_interpreted.clone()),
                    ),
                    None,
                ));

                // Screenshots:
                if !filter.exclude_store_screenshots {
                    paths_to_check.insert((
                        StrictPath::relative(
                            format!("{}/userdata/*/760/remote/{}/screenshots/*.*", &root_interpreted, id),
                            Some(manifest_dir_interpreted.clone()),
                        ),
                        None,
                    ));
                }

                // Registry:
                if game.registry.is_some() {
                    let prefix = format!("{}/steamapps/compatdata/{}/pfx", &root_interpreted, id);
                    paths_to_check.insert((
                        StrictPath::relative(format!("{}/*.reg", prefix), Some(manifest_dir_interpreted.clone())),
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
                .map(|x| (x.original_path(), &x.hash))
                .collect()
        })
        .unwrap_or_default();

    for (path, case_sensitive) in paths_to_check {
        log::trace!("[{name}] checking: {}", path.raw());
        if filter.is_path_ignored(&path) {
            log::debug!("[{name}] excluded: {}", path.raw());
            continue;
        }
        let paths = match case_sensitive {
            None => path.glob(),
            Some(cs) => path.glob_case_sensitive(cs),
        };
        for p in paths {
            let p = p.rendered();
            if p.is_file() {
                if filter.is_path_ignored(&p) {
                    log::debug!("[{name}] excluded: {}", p.raw());
                    continue;
                }
                let ignored = ignored_paths.is_ignored(name, &p);
                log::debug!("[{name}] found: {}", p.raw());
                let hash = p.sha1();
                let redirected = game_file_target(&p, redirects, false);
                found_files.insert(ScannedFile {
                    change: ScanChange::evaluate_backup(&hash, previous_files.get(redirected.as_ref().unwrap_or(&p))),
                    size: p.size(),
                    hash,
                    redirected,
                    path: p,
                    original_path: None,
                    ignored,
                    container: None,
                });
            } else if p.is_dir() {
                log::trace!("[{name}] looking for files in: {}", p.raw());
                for child in walkdir::WalkDir::new(p.as_std_path_buf().unwrap())
                    .max_depth(100)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(filter_map_walkdir)
                {
                    #[cfg(not(target_os = "windows"))]
                    if child.path().to_string_lossy().contains('\\') {
                        // TODO: Support names containing a slash.
                        continue;
                    }

                    if child.file_type().is_file() {
                        let child = StrictPath::from(&child).rendered();
                        if filter.is_path_ignored(&child) {
                            log::debug!("[{name}] excluded: {}", child.raw());
                            continue;
                        }
                        let ignored = ignored_paths.is_ignored(name, &child);
                        log::debug!("[{name}] found: {}", child.raw());
                        let hash = child.sha1();
                        let redirected = game_file_target(&child, redirects, false);
                        found_files.insert(ScannedFile {
                            change: ScanChange::evaluate_backup(
                                &hash,
                                previous_files.get(redirected.as_ref().unwrap_or(&child)),
                            ),
                            size: child.size(),
                            hash,
                            redirected,
                            path: child,
                            original_path: None,
                            ignored,
                            container: None,
                        });
                    }
                }
            }
        }
    }

    // Mark removed files.
    let current_files: Vec<_> = found_files
        .iter()
        .map(|x| x.redirected.as_ref().unwrap_or(&x.path).interpret())
        .collect();
    // But if a file is only "removed" because now it has a redirect,
    // then the removal isn't very interesting
    // and would lead to duplicate hash keys during reporting.
    let current_files_with_redirects: Vec<_> = found_files
        .iter()
        .filter(|&x| x.redirected.is_some())
        .map(|x| x.path.interpret())
        .collect();
    for (previous_file, _) in previous_files {
        let previous_file_interpreted = previous_file.interpret();
        if !current_files.contains(&previous_file_interpreted)
            && !current_files_with_redirects.contains(&previous_file_interpreted)
        {
            found_files.insert(ScannedFile {
                change: ScanChange::Removed,
                size: 0,
                hash: "".to_string(),
                redirected: None,
                path: previous_file.to_owned(),
                original_path: None,
                ignored: ignored_paths.is_ignored(name, previous_file),
                container: None,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        let previous_registry = match previous.map(|x| x.registry_content) {
            Some(Some(content)) => registry::Hives::deserialize(&content),
            _ => None,
        };

        if let Some(registry) = &game.registry {
            for key in registry.keys() {
                if key.trim().is_empty() {
                    continue;
                }

                log::trace!("[{name}] computing candidates for registry: {key}");
                let mut candidates = vec![key.clone()];
                let normalized = key.replace('\\', "/").to_lowercase();
                if normalized.starts_with("hkey_local_machine/software/") && !normalized.contains("/wow6432node/") {
                    let tail = &key[28..];
                    candidates.push(format!("HKEY_LOCAL_MACHINE/SOFTWARE/Wow6432Node/{}", tail));
                    candidates.push(format!(
                        "HKEY_CURRENT_USER/Software/Classes/VirtualStore/MACHINE/SOFTWARE/{}",
                        tail
                    ));
                    candidates.push(format!(
                        "HKEY_CURRENT_USER/Software/Classes/VirtualStore/MACHINE/SOFTWARE/Wow6432Node/{}",
                        tail
                    ));
                }

                for candidate in candidates {
                    log::trace!("[{name}] checking registry: {candidate}");
                    for mut scanned in
                        registry::scan_registry(name, &candidate, filter, ignored_registry, &previous_registry)
                            .unwrap_or_default()
                    {
                        log::debug!("[{name}] found registry: {}", scanned.path.raw());

                        // Mark removed registry values.
                        let previous_values = previous_registry
                            .as_ref()
                            .and_then(|x| {
                                x.get_path(&scanned.path)
                                    .map(|y| y.0.keys().cloned().collect::<Vec<_>>())
                            })
                            .unwrap_or_default();
                        for previous_value in previous_values {
                            #[allow(clippy::map_entry)]
                            if !scanned.values.contains_key(&previous_value) {
                                let ignored = ignored_registry.is_ignored(name, &scanned.path, Some(&previous_value));
                                scanned.values.insert(
                                    previous_value,
                                    ScannedRegistryValue {
                                        ignored,
                                        change: ScanChange::Removed,
                                    },
                                );
                            }
                        }

                        found_registry_keys.insert(scanned);
                    }
                }
            }
        }

        // Mark removed registry keys.
        if let Some(previous_registry) = &previous_registry {
            let current_registry_keys: Vec<_> = found_registry_keys.iter().map(|x| x.path.interpret()).collect();
            for (previous_hive, previous_keys) in &previous_registry.0 {
                for previous_key in previous_keys.0.keys() {
                    let path = RegistryItem::from_hive_and_key(previous_hive, previous_key);
                    if !current_registry_keys.contains(&path.interpret()) {
                        let ignored = ignored_registry.is_ignored(name, &path, None);
                        found_registry_keys.insert(ScannedRegistry {
                            change: ScanChange::Removed,
                            path,
                            ignored,
                            values: Default::default(),
                        });
                    }
                }
            }
        }
    }

    log::trace!("[{name}] completed scan for backup");

    ScanInfo {
        game_name: name.to_string(),
        found_files,
        found_registry_keys,
        ..Default::default()
    }
}

fn scan_game_for_backup_add_prefix(
    roots_to_check: &mut Vec<RootsConfig>,
    paths_to_check: &mut HashSet<(StrictPath, Option<bool>)>,
    wp: &StrictPath,
    has_registry: bool,
) {
    roots_to_check.push(RootsConfig {
        path: wp.clone(),
        store: Store::OtherWine,
    });
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
        SortKey::Status => compare_games_by_status(scan_info1, scan_info2),
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

fn compare_games_by_status(scan_info1: &ScanInfo, scan_info2: &ScanInfo) -> std::cmp::Ordering {
    scan_info1
        .overall_change()
        .cmp(&scan_info2.overall_change())
        .then_with(|| compare_games_by_name(&scan_info1.game_name, &scan_info2.game_name))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use velcro::{btree_map, hash_set};

    use super::*;
    #[cfg(target_os = "windows")]
    use crate::resource::config::ToggledRegistryEntry;
    use crate::{
        resource::{config::Config, manifest::Manifest, ResourceFile},
        testing::{repo, s, EMPTY_HASH},
    };

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
            "#,
        )
        .unwrap()
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches() {
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2, "9d891e731f75deae56884d79e9816736b7488080").change_new(),
                    ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );

        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/root2/game2/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game 2".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_deduplicating_symlinks() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/root3", repo())),
            store: Store::Other,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game5"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/root3/game5/data/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game5"],
                "game5",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game5".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_fuzzy_matched_install_dir() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/root3", repo())),
            store: Store::Other,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game 2"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/root3/game_2/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game 2"],
                "game 2",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game 2".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_file_matches_in_custom_home_folder() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/home", repo())),
            store: Store::OtherHome,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/home/data.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/AppData/Roaming/winAppData.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/AppData/Local/winLocalAppData.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/Documents/winDocuments.txt", repo()), 0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game4".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn can_scan_game_for_backup_with_file_matches_in_custom_home_folder() {
        let roots = &[RootsConfig {
            path: StrictPath::new(format!("{}/tests/home", repo())),
            store: Store::OtherHome,
        }];
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/home/data.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/.config/xdgConfig.txt", repo()), 0, EMPTY_HASH).change_new(),
                    ScannedFile::new(format!("{}/tests/home/.local/share/xdgData.txt", repo()), 0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(roots, &manifest(), &["game4".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_file_matches_in_wine_prefix() {
        assert_eq!(
            ScanInfo {
                game_name: s("game4"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/wine-prefix/drive_c/users/anyone/data.txt", repo()), 0, EMPTY_HASH).change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game4"],
                "game4",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game4".to_string()]),
                &BackupFilter::default(),
                &Some(StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    fn can_scan_game_for_backup_with_registry_files_in_wine_prefix() {
        assert_eq!(
            ScanInfo {
                game_name: s("fake-registry"),
                found_files: hash_set! {
                    ScannedFile::new(format!("{}/tests/wine-prefix/user.reg", repo()), 37, "4a5b7e9de7d84ffb4bb3e9f38667f85741d5fbc0",).change_new(),
                },
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["fake-registry"],
                "fake-registry",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["fake-registry".to_string()]),
                &BackupFilter::default(),
                &Some(StrictPath::new(format!("{}/tests/wine-prefix", repo()))),
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
        let found = hash_set! {
            ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
        };

        filter.build_globs();
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: found,
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                &None,
                &ignored,
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
        let found = hash_set! {
            ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2, "9d891e731f75deae56884d79e9816736b7488080").change_new().ignored(),
            ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
        };

        filter.build_globs();
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: found,
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                &None,
                &ignored,
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
        let found = hash_set! {
            ScannedFile::new(format!("{}/tests/root1/game1/subdir/file2.txt", repo()), 2, "9d891e731f75deae56884d79e9816736b7488080").change_new().ignored(),
            ScannedFile::new(format!("{}/tests/root2/game1/file1.txt", repo()), 1, "3a52ce780950d4d969792a2559cd519d7ee8c727").change_new(),
        };

        filter.build_globs();
        assert_eq!(
            ScanInfo {
                game_name: s("game1"),
                found_files: found,
                found_registry_keys: hash_set! {},
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game1"],
                "game1",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                &filter,
                &None,
                &ignored,
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_leaf_key_with_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3"),
                found_files: hash_set! {},
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz")
                },
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3"],
                "game3",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game3".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
            ),
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn can_scan_game_for_backup_with_registry_matches_on_parent_key_without_values() {
        assert_eq!(
            ScanInfo {
                game_name: s("game3-outer"),
                found_files: hash_set! {},
                found_registry_keys: hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change_as(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/invalid").change_as(ScanChange::New)
                        .with_value_new("dword"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").change_as(ScanChange::New),
                },
                ..Default::default()
            },
            scan_game_for_backup(
                &manifest().0["game3-outer"],
                "game3-outer",
                &config().roots,
                &StrictPath::new(repo()),
                &Launchers::scan_dirs(&config().roots, &manifest(), &["game3-outer".to_string()]),
                &BackupFilter::default(),
                &None,
                &ToggledPaths::default(),
                &ToggledRegistry::default(),
                None,
                &[],
                &Default::default(),
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
                hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change_as(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value_new("qword")
                        .with_value_new("sz"),
                },
            ),
            (
                BackupFilter::default(),
                ToggledRegistry::new(btree_map! {
                    s("game3-outer"): btree_map! {
                        RegistryItem::new(s("HKEY_CURRENT_USER\\Software/Ludusavi")): ToggledRegistryEntry::Key(false)
                    }
                }),
                hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").ignored().change_as(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").ignored().change_as(ScanChange::New)
                        .with_value("binary", ScanChange::New, true)
                        .with_value("dword", ScanChange::New, true)
                        .with_value("expandSz", ScanChange::New, true)
                        .with_value("multiSz", ScanChange::New, true)
                        .with_value("qword", ScanChange::New, true)
                        .with_value("sz", ScanChange::New, true),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/invalid").ignored().change_as(ScanChange::New)
                        .with_value("dword", ScanChange::New, true),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").ignored().change_as(ScanChange::New),
                },
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
                hash_set! {
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi").change_as(ScanChange::New),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/game3").change_as(ScanChange::New)
                        .with_value_new("binary")
                        .with_value_new("dword")
                        .with_value_new("expandSz")
                        .with_value_new("multiSz")
                        .with_value("qword", ScanChange::New, true)
                        .with_value_new("sz"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/invalid").change_as(ScanChange::New)
                        .with_value_new("dword"),
                    ScannedRegistry::new("HKEY_CURRENT_USER/Software/Ludusavi/other").ignored().change_as(ScanChange::New),
                },
            ),
        ];

        for (filter, ignored, found) in cases {
            assert_eq!(
                ScanInfo {
                    game_name: s("game3-outer"),
                    found_files: hash_set! {},
                    found_registry_keys: found,
                    ..Default::default()
                },
                scan_game_for_backup(
                    &manifest().0["game3-outer"],
                    "game3-outer",
                    &config().roots,
                    &StrictPath::new(repo()),
                    &Launchers::scan_dirs(&config().roots, &manifest(), &["game1".to_string()]),
                    &filter,
                    &None,
                    &ToggledPaths::default(),
                    &ignored,
                    None,
                    &[],
                    &Default::default(),
                ),
            );
        }
    }
}
