mod parse;
mod report;
mod ui;

use std::{collections::BTreeSet, process::Command, time::Duration};

use clap::CommandFactory;
use indicatif::{ParallelProgressIterator, ProgressBar};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    prelude::IndexedParallelIterator,
};

use crate::{
    cli::{
        parse::{Cli, CompletionShell, ManifestSubcommand, Subcommand},
        report::{report_cloud_changes, Reporter},
    },
    cloud::{CloudChange, Rclone, Remote},
    lang::TRANSLATOR,
    prelude::{
        app_dir, get_threads_from_env, initialize_rayon, register_sigint, unregister_sigint, Error, Finality,
        StrictPath, SyncDirection,
    },
    resource::{cache::Cache, config::Config, manifest::Manifest, ResourceFile, SaveableResourceFile},
    scan::{
        layout::BackupLayout, prepare_backup_target, scan_game_for_backup, BackupId, DuplicateDetector, Launchers,
        OperationStepDecision, SteamShortcuts, TitleFinder, TitleQuery,
    },
    wrap,
};

const PROGRESS_BAR_REFRESH_INTERVAL: Duration = Duration::from_millis(50);

fn negatable_flag(on: bool, off: bool, default: bool) -> bool {
    if on {
        true
    } else if off {
        false
    } else {
        default
    }
}

fn load_manifest(
    config: &Config,
    cache: &mut Cache,
    no_manifest_update: bool,
    try_manifest_update: bool,
) -> Result<Manifest, Error> {
    if no_manifest_update {
        Ok(Manifest::load().unwrap_or_default().with_extensions(config))
    } else if try_manifest_update {
        if let Err(e) = Manifest::update_mut(config, cache, false) {
            eprintln!("{}", TRANSLATOR.handle_error(&e));
        }
        Ok(Manifest::load().unwrap_or_default().with_extensions(config))
    } else {
        Manifest::update_mut(config, cache, false)?;
        Manifest::load().map(|x| x.with_extensions(config))
    }
}

fn parse_games(games: Vec<String>) -> Vec<String> {
    if !games.is_empty() {
        games
    } else {
        use std::io::IsTerminal;

        let stdin = std::io::stdin();
        if stdin.is_terminal() {
            vec![]
        } else {
            let games = stdin.lines().map_while(Result::ok).collect();
            log::debug!("Games from stdin: {:?}", &games);
            games
        }
    }
}

pub fn evaluate_games(
    default: BTreeSet<String>,
    requested: Vec<String>,
    title_finder: &TitleFinder,
) -> Result<Vec<String>, Vec<String>> {
    if requested.is_empty() {
        return Ok(default.into_iter().collect());
    }

    let mut valid = BTreeSet::new();
    let mut invalid = BTreeSet::new();

    for game in requested {
        match title_finder.find_one_by_name(&game) {
            Some(found) => {
                valid.insert(found);
            }
            None => {
                invalid.insert(game);
            }
        }
    }

    if !invalid.is_empty() {
        return Err(invalid.into_iter().collect());
    }

    Ok(valid.into_iter().collect())
}

pub fn parse() -> Cli {
    use clap::Parser;
    Cli::parse()
}

pub fn run(sub: Subcommand, no_manifest_update: bool, try_manifest_update: bool) -> Result<(), Error> {
    let mut config = Config::load()?;
    if let Some(threads) = get_threads_from_env().or(config.runtime.threads) {
        initialize_rayon(threads);
    }
    let mut cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
    TRANSLATOR.set_language(config.language);
    let mut failed = false;
    let mut duplicate_detector = DuplicateDetector::default();

    log::debug!("Config on startup: {config:?}");
    log::debug!("Invocation: {sub:?}");

    match sub {
        Subcommand::Backup {
            preview,
            path,
            force,
            wine_prefix,
            api,
            sort,
            format,
            compression,
            compression_level,
            full_limit,
            differential_limit,
            cloud_sync,
            no_cloud_sync,
            games,
        } => {
            let games = parse_games(games);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };

            let manifest = load_manifest(&config, &mut cache, no_manifest_update, try_manifest_update)?;

            let backup_dir = match path {
                None => config.backup.path.clone(),
                Some(p) => p,
            };
            let roots = config.expanded_roots();

            if !preview && !force {
                match dialoguer::Confirm::new()
                    .with_prompt(TRANSLATOR.confirm_backup(&backup_dir, backup_dir.exists(), false))
                    .interact()
                {
                    Ok(true) => (),
                    Ok(false) => return Ok(()),
                    Err(_) => return Err(Error::CliUnableToRequestConfirmation),
                }
            }

            if !preview {
                prepare_backup_target(&backup_dir)?;
            }

            let mut retention = config.backup.retention.clone();
            if let Some(full_limit) = full_limit {
                retention.full = full_limit;
            }
            if let Some(differential_limit) = differential_limit {
                retention.differential = differential_limit;
            }

            let layout = BackupLayout::new(backup_dir.clone(), retention);
            let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());

            let games_specified = !games.is_empty();
            let games = match evaluate_games(manifest.primary_titles(), games, &title_finder) {
                Ok(games) => games,
                Err(games) => {
                    reporter.trip_unknown_games(games.clone());
                    reporter.print_failure();
                    return Err(Error::CliUnrecognizedGames { games });
                }
            };

            let launchers = Launchers::scan(&roots, &manifest, &games, &title_finder, None);
            let filter = config.backup.filter.clone();
            let toggled_paths = config.backup.toggled_paths.clone();
            let toggled_registry = config.backup.toggled_registry.clone();
            let steam_shortcuts = SteamShortcuts::scan();

            let cloud_sync = negatable_flag(
                cloud_sync && !preview,
                no_cloud_sync,
                config.cloud.synchronize
                    && !preview
                    && crate::cloud::validate_cloud_config(&config, &config.cloud.path).is_ok(),
            );
            let mut should_sync_cloud_after = cloud_sync && !preview;
            if cloud_sync {
                let changes = sync_cloud(
                    &config,
                    &backup_dir,
                    &config.cloud.path,
                    SyncDirection::Upload,
                    Finality::Preview,
                    if games_specified { &games } else { &[] },
                );
                match changes {
                    Ok(changes) => {
                        if !changes.is_empty() {
                            should_sync_cloud_after = false;
                            reporter.trip_cloud_conflict();
                        }
                    }
                    Err(_) => {
                        should_sync_cloud_after = false;
                        reporter.trip_cloud_sync_failed();
                    }
                }
            }

            log::info!("beginning backup with {} steps", games.len());

            let mut info: Vec<_> = games
                .par_iter()
                .enumerate()
                .progress_with(scan_progress_bar(games.len() as u64))
                .filter_map(|(i, name)| {
                    log::trace!("step {i} / {}: {name}", games.len());
                    let game = &manifest.0[name];

                    let previous = layout.latest_backup(name, false, &config.redirects, &config.restore.toggled_paths);

                    let scan_info = scan_game_for_backup(
                        game,
                        name,
                        &roots,
                        &app_dir(),
                        &launchers,
                        &filter,
                        &wine_prefix,
                        &toggled_paths,
                        &toggled_registry,
                        previous,
                        &config.redirects,
                        &steam_shortcuts,
                    );
                    let ignored = !&config.is_game_enabled_for_backup(name) && !games_specified;
                    let decision = if ignored {
                        OperationStepDecision::Ignored
                    } else {
                        OperationStepDecision::Processed
                    };
                    let backup_info = if preview || ignored {
                        crate::scan::BackupInfo::default()
                    } else {
                        let mut backup_format = config.backup.format.clone();
                        if let Some(format) = format {
                            backup_format.chosen = format;
                        }
                        if let Some(compression) = compression {
                            backup_format.zip.compression = compression;
                        }
                        if let Some(level) = compression_level {
                            backup_format
                                .compression
                                .set_level(&backup_format.zip.compression, level);
                        }

                        layout
                            .game_layout(name)
                            .back_up(&scan_info, &chrono::Utc::now(), &backup_format)
                    };
                    log::trace!("step {i} completed");
                    if !scan_info.can_report_game() {
                        None
                    } else {
                        let display_title = config.display_name(name);
                        Some((display_title, scan_info, backup_info, decision))
                    }
                })
                .collect();
            log::info!("completed backup");

            if should_sync_cloud_after {
                let changed_games: Vec<_> = info
                    .iter()
                    .filter(|(_, scan_info, _, _)| scan_info.needs_cloud_sync())
                    .map(|(_, scan_info, _, _)| scan_info.game_name.clone())
                    .collect();
                if !changed_games.is_empty() {
                    let sync_result = sync_cloud(
                        &config,
                        &backup_dir,
                        &config.cloud.path,
                        SyncDirection::Upload,
                        Finality::Final,
                        &changed_games,
                    );
                    if sync_result.is_err() {
                        reporter.trip_cloud_sync_failed();
                    }
                }
            }

            for (_, scan_info, _, _) in info.iter() {
                duplicate_detector.add_game(
                    scan_info,
                    config.is_game_enabled_for_operation(&scan_info.game_name, false),
                );
            }

            let sort = sort.map(From::from).unwrap_or_else(|| config.backup.sort.clone());
            info.sort_by(
                |(name1, scan_info1, backup_info1, ..), (name2, scan_info2, backup_info2, ..)| {
                    crate::scan::compare_games(
                        sort.key,
                        name1,
                        scan_info1,
                        Some(backup_info1),
                        name2,
                        scan_info2,
                        Some(backup_info2),
                    )
                },
            );
            if sort.reversed {
                info.reverse();
            }

            for (name, scan_info, backup_info, decision) in info {
                if !reporter.add_game(name, &scan_info, &backup_info, &decision, &duplicate_detector) {
                    failed = true;
                }
            }
            reporter.print(&backup_dir);
        }
        Subcommand::Restore {
            preview,
            path,
            force,
            api,
            sort,
            backup,
            cloud_sync,
            no_cloud_sync,
            games,
        } => {
            let games = parse_games(games);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };

            let restore_dir = match path {
                None => config.restore.path.clone(),
                Some(p) => p,
            };

            if !preview && !force {
                match dialoguer::Confirm::new()
                    .with_prompt(TRANSLATOR.confirm_restore(&restore_dir, false))
                    .interact()
                {
                    Ok(true) => (),
                    Ok(false) => return Ok(()),
                    Err(_) => return Err(Error::CliUnableToRequestConfirmation),
                }
            }

            let layout = BackupLayout::new(restore_dir.clone(), config.backup.retention.clone());

            if backup.is_some() && games.len() != 1 {
                return Err(Error::CliBackupIdWithMultipleGames);
            }
            let backup_id = backup.as_ref().map(|x| BackupId::Named(x.clone()));

            let manifest = load_manifest(&config, &mut cache, true, false).unwrap_or_default();
            let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());

            let games_specified = !games.is_empty();
            let games = match evaluate_games(layout.restorable_game_set(), games, &title_finder) {
                Ok(games) => games,
                Err(games) => {
                    reporter.trip_unknown_games(games.clone());
                    reporter.print_failure();
                    return Err(Error::CliUnrecognizedGames { games });
                }
            };

            let cloud_sync = negatable_flag(
                cloud_sync && !preview,
                no_cloud_sync,
                config.cloud.synchronize
                    && !preview
                    && crate::cloud::validate_cloud_config(&config, &config.cloud.path).is_ok(),
            );
            if cloud_sync {
                let changes = sync_cloud(
                    &config,
                    &restore_dir,
                    &config.cloud.path,
                    SyncDirection::Upload,
                    Finality::Preview,
                    if games_specified { &games } else { &[] },
                );
                match changes {
                    Ok(changes) => {
                        if !changes.is_empty() {
                            reporter.trip_cloud_conflict();
                        }
                    }
                    Err(_) => {
                        reporter.trip_cloud_sync_failed();
                    }
                }
            }

            log::info!("beginning restore with {} steps", games.len());

            let mut info: Vec<_> = games
                .par_iter()
                .enumerate()
                .progress_with(scan_progress_bar(games.len() as u64))
                .filter_map(|(i, name)| {
                    log::trace!("step {i} / {}: {name}", games.len());
                    let mut layout = layout.game_layout(name);
                    let scan_info = layout.scan_for_restoration(
                        name,
                        backup_id.as_ref().unwrap_or(&BackupId::Latest),
                        &config.redirects,
                        &config.restore.toggled_paths,
                        &config.restore.toggled_registry,
                    );
                    let ignored = !&config.is_game_enabled_for_restore(name) && !games_specified;
                    let decision = if ignored {
                        OperationStepDecision::Ignored
                    } else {
                        OperationStepDecision::Processed
                    };

                    if let Some(backup) = &backup {
                        if let Some(BackupId::Named(scanned_backup)) = scan_info.backup.as_ref().map(|x| x.id()) {
                            if backup != &scanned_backup {
                                log::trace!("step {i} completed (backup mismatch)");
                                let display_title = config.display_name(name);
                                return Some((
                                    display_title,
                                    scan_info,
                                    Default::default(),
                                    decision,
                                    Some(Err(Error::CliInvalidBackupId)),
                                ));
                            }
                        }
                    }

                    let restore_info = if scan_info.backup.is_none() || preview || ignored {
                        crate::scan::BackupInfo::default()
                    } else {
                        layout.restore(&scan_info, &config.restore.toggled_registry)
                    };
                    log::trace!("step {i} completed");
                    if !scan_info.can_report_game() {
                        None
                    } else {
                        let display_title = config.display_name(name);
                        Some((display_title, scan_info, restore_info, decision, None))
                    }
                })
                .collect();
            log::info!("completed restore");

            for (_, scan_info, _, _, failure) in info.iter() {
                if let Some(failure) = failure {
                    return failure.clone();
                }
                duplicate_detector.add_game(
                    scan_info,
                    config.is_game_enabled_for_operation(&scan_info.game_name, true),
                );
            }

            let sort = sort.map(From::from).unwrap_or_else(|| config.restore.sort.clone());
            info.sort_by(
                |(name1, scan_info1, backup_info1, ..), (name2, scan_info2, backup_info2, ..)| {
                    crate::scan::compare_games(
                        sort.key,
                        name1,
                        scan_info1,
                        Some(backup_info1),
                        name2,
                        scan_info2,
                        Some(backup_info2),
                    )
                },
            );
            if sort.reversed {
                info.reverse();
            }

            for (name, scan_info, backup_info, decision, _) in info {
                if !reporter.add_game(name, &scan_info, &backup_info, &decision, &duplicate_detector) {
                    failed = true;
                }
            }
            reporter.print(&restore_dir);
        }
        Subcommand::Complete { shell } => {
            let clap_shell = match shell {
                CompletionShell::Bash => clap_complete::Shell::Bash,
                CompletionShell::Fish => clap_complete::Shell::Fish,
                CompletionShell::Zsh => clap_complete::Shell::Zsh,
                CompletionShell::PowerShell => clap_complete::Shell::PowerShell,
                CompletionShell::Elvish => clap_complete::Shell::Elvish,
            };
            clap_complete::generate(
                clap_shell,
                &mut Cli::command(),
                env!("CARGO_PKG_NAME"),
                &mut std::io::stdout(),
            )
        }
        Subcommand::Backups { path, api, games } => {
            let games = parse_games(games);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };
            reporter.suppress_overall();

            let restore_dir = match path {
                None => config.restore.path.clone(),
                Some(p) => p,
            };

            let layout = BackupLayout::new(restore_dir.clone(), config.backup.retention.clone());
            let manifest = load_manifest(&config, &mut cache, true, false).unwrap_or_default();
            let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());

            let games = match evaluate_games(layout.restorable_game_set(), games, &title_finder) {
                Ok(games) => games,
                Err(games) => {
                    reporter.trip_unknown_games(games.clone());
                    reporter.print_failure();
                    return Err(Error::CliUnrecognizedGames { games });
                }
            };

            let info: Vec<_> = games
                .par_iter()
                .progress_count(games.len() as u64)
                .map(|name| {
                    let mut layout = layout.game_layout(name);
                    let backups = layout.get_backups();
                    let display_title = config.display_name(name);
                    (name, display_title, backups)
                })
                .collect();

            for (name, display_title, backups) in info {
                reporter.add_backups(name, display_title, &backups);
            }
            reporter.print(&restore_dir);
        }
        Subcommand::Find {
            api,
            path,
            backup,
            restore,
            steam_id,
            gog_id,
            normalized,
            disabled,
            partial,
            names,
        } => {
            let names = parse_games(names);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };
            reporter.suppress_overall();

            let manifest = load_manifest(&config, &mut cache, no_manifest_update, try_manifest_update)?;

            let restore_dir = match path {
                None => config.restore.path.clone(),
                Some(p) => p,
            };
            let layout = BackupLayout::new(restore_dir.clone(), config.backup.retention.clone());

            let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());
            let found = title_finder.find(TitleQuery {
                names: names.clone(),
                steam_id,
                gog_id,
                normalized,
                backup,
                restore,
                disabled,
                partial,
            });
            reporter.add_found_titles(&found);

            if found.is_empty() {
                let mut invalid = names;
                if let Some(steam_id) = steam_id {
                    invalid.push(steam_id.to_string());
                }
                if let Some(gog_id) = gog_id {
                    invalid.push(gog_id.to_string());
                }
                reporter.trip_unknown_games(invalid.clone());
                reporter.print_failure();
                return Err(Error::CliUnrecognizedGames { games: invalid });
            }

            reporter.print(&restore_dir);
        }
        Subcommand::Manifest { sub: manifest_sub } => match manifest_sub {
            ManifestSubcommand::Show { api } => {
                let manifest = load_manifest(&config, &mut cache, true, false).unwrap_or_default();

                if api {
                    println!("{}", serde_json::to_string(&manifest).unwrap());
                } else {
                    println!("{}", serde_yaml::to_string(&manifest).unwrap());
                }
            }
            ManifestSubcommand::Update { force } => {
                Manifest::update_mut(&config, &mut cache, force)?;
            }
        },
        Subcommand::Cloud { sub: cloud_sub } => match cloud_sub {
            parse::CloudSubcommand::Set { sub } => match sub {
                parse::CloudSetSubcommand::None => {
                    config.cloud.remote = None;
                    config.save();
                }
                parse::CloudSetSubcommand::Custom { id } => {
                    configure_cloud(&mut config, Remote::Custom { id })?;
                }
                parse::CloudSetSubcommand::Box => {
                    configure_cloud(
                        &mut config,
                        Remote::Box {
                            id: Remote::generate_id(),
                        },
                    )?;
                }
                parse::CloudSetSubcommand::Dropbox => {
                    configure_cloud(
                        &mut config,
                        Remote::Dropbox {
                            id: Remote::generate_id(),
                        },
                    )?;
                }
                parse::CloudSetSubcommand::Ftp {
                    host,
                    port,
                    username,
                    password,
                } => {
                    configure_cloud(
                        &mut config,
                        Remote::Ftp {
                            id: Remote::generate_id(),
                            host,
                            port,
                            username,
                            password,
                        },
                    )?;
                }
                parse::CloudSetSubcommand::GoogleDrive => {
                    configure_cloud(
                        &mut config,
                        Remote::GoogleDrive {
                            id: Remote::generate_id(),
                        },
                    )?;
                }
                parse::CloudSetSubcommand::OneDrive => {
                    configure_cloud(
                        &mut config,
                        Remote::OneDrive {
                            id: Remote::generate_id(),
                        },
                    )?;
                }
                parse::CloudSetSubcommand::Smb {
                    host,
                    port,
                    username,
                    password,
                } => {
                    configure_cloud(
                        &mut config,
                        Remote::Smb {
                            id: Remote::generate_id(),
                            host,
                            port,
                            username,
                            password,
                        },
                    )?;
                }
                parse::CloudSetSubcommand::WebDav {
                    url,
                    username,
                    password,
                    provider,
                } => {
                    configure_cloud(
                        &mut config,
                        Remote::WebDav {
                            id: Remote::generate_id(),
                            url,
                            username,
                            password,
                            provider,
                        },
                    )?;
                }
            },
            parse::CloudSubcommand::Upload {
                local,
                cloud,
                force,
                preview,
                api,
                games,
            } => {
                let games = parse_games(games);

                let local = local.unwrap_or(config.backup.path.clone());
                let cloud = cloud.unwrap_or(config.cloud.path.clone());

                let finality = if preview { Finality::Preview } else { Finality::Final };
                let direction = SyncDirection::Upload;

                let layout = BackupLayout::new(config.restore.path.clone(), config.backup.retention.clone());
                let manifest = load_manifest(&config, &mut cache, true, false).unwrap_or_default();
                let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());

                let games = match evaluate_games(layout.restorable_game_set(), games, &title_finder) {
                    Ok(games) => games,
                    Err(games) => {
                        let mut reporter = if api { Reporter::json() } else { Reporter::standard() };
                        reporter.trip_unknown_games(games.clone());
                        reporter.print_failure();
                        return Err(Error::CliUnrecognizedGames { games });
                    }
                };

                if !ask(
                    TRANSLATOR.confirm_cloud_upload(&local.render(), &cloud),
                    finality,
                    force,
                )? {
                    return Ok(());
                }

                let changes = sync_cloud(&config, &local, &cloud, direction, finality, &games)?;
                report_cloud_changes(&changes, api);
            }
            parse::CloudSubcommand::Download {
                local,
                cloud,
                force,
                preview,
                api,
                games,
            } => {
                let games = parse_games(games);

                let local = local.unwrap_or(config.backup.path.clone());
                let cloud = cloud.unwrap_or(config.cloud.path.clone());

                let finality = if preview { Finality::Preview } else { Finality::Final };
                let direction = SyncDirection::Download;

                let layout = BackupLayout::new(config.restore.path.clone(), config.backup.retention.clone());
                let manifest = load_manifest(&config, &mut cache, true, false).unwrap_or_default();
                let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());

                let games = match evaluate_games(layout.restorable_game_set(), games, &title_finder) {
                    Ok(games) => games,
                    Err(games) => {
                        let mut reporter = if api { Reporter::json() } else { Reporter::standard() };
                        reporter.trip_unknown_games(games.clone());
                        reporter.print_failure();
                        return Err(Error::CliUnrecognizedGames { games });
                    }
                };

                if !ask(
                    TRANSLATOR.confirm_cloud_download(&local.render(), &cloud),
                    finality,
                    force,
                )? {
                    return Ok(());
                }

                let changes = sync_cloud(&config, &local, &cloud, direction, finality, &games)?;
                report_cloud_changes(&changes, api);
            }
        },
        Subcommand::Wrap {
            name_source,
            force,
            gui,
            commands,
        } => {
            let manifest = load_manifest(&config, &mut cache, no_manifest_update, try_manifest_update)?;
            let layout = BackupLayout::new(config.restore.path.clone(), config.backup.retention.clone());
            let title_finder = TitleFinder::new(&config, &manifest, layout.restorable_game_set());

            // Determine raw game identifiers
            let wrap_game_info = if let Some(name) = name_source.name.as_ref() {
                Some(wrap::WrapGameInfo {
                    name: Some(name.clone()),
                    ..Default::default()
                })
            } else if let Some(infer) = name_source.infer {
                let roots = config.expanded_roots();
                match infer {
                    parse::Launcher::Heroic => wrap::heroic::infer_game_from_heroic(&roots),
                    parse::Launcher::Lutris => wrap::lutris::infer(),
                    parse::Launcher::Steam => wrap::infer_game_from_steam(),
                }
            } else {
                unreachable!();
            };
            log::debug!("Wrap game info: {:?}", &wrap_game_info);

            // Check game identifiers against the manifest
            //
            // e.g. "Slain: Back From Hell" from legendary to "Slain: Back from
            // Hell" as known to ludusavi
            let game_name = wrap_game_info.clone().and_then(|info| {
                let names = info.name.map(|x| vec![x]).unwrap_or_default();
                title_finder.find_one(TitleQuery {
                    names,
                    steam_id: info.steam_id,
                    gog_id: info.gog_id,
                    normalized: true,
                    ..Default::default()
                })
            });
            log::debug!("Title finder result: {:?}", &game_name);

            match game_name.as_ref() {
                Some(game_name) => {
                    wrap::lutris::save_normalized_title(game_name.clone());
                }
                None => {
                    if !ui::confirm_with_question(
                        gui,
                        force.then_some(true),
                        &TRANSLATOR.game_is_unrecognized(),
                        &TRANSLATOR.launch_game_after_error(),
                    )? {
                        return Ok(());
                    }
                }
            }

            // Restore
            //
            // TODO.2023-07-12 detect if there are differences between backed up
            // and actual saves and skip the question if there is none
            'restore: {
                let Some(game_name) = game_name.as_ref() else {
                    break 'restore;
                };

                let game_layout = layout.game_layout(game_name);
                if !game_layout.has_backups() {
                    if ui::confirm_with_question(
                        gui,
                        force.then_some(true),
                        &TRANSLATOR.game_has_nothing_to_restore(),
                        &TRANSLATOR.launch_game_after_error(),
                    )? {
                        break 'restore;
                    } else {
                        return Ok(());
                    }
                }

                if !ui::confirm(
                    gui,
                    force.then_some(true),
                    &TRANSLATOR.restore_one_game_confirm(game_name),
                )? {
                    break 'restore;
                }

                if let Err(err) = run(
                    Subcommand::Restore {
                        games: vec![game_name.clone()],
                        force: true,
                        preview: Default::default(),
                        path: Default::default(),
                        api: Default::default(),
                        sort: Default::default(),
                        backup: Default::default(),
                        cloud_sync: Default::default(),
                        no_cloud_sync: Default::default(),
                    },
                    no_manifest_update,
                    try_manifest_update,
                ) {
                    log::error!("WRAP::restore: failed for game {:?} with: {:?}", wrap_game_info, err);
                    ui::alert_with_error(gui, &TRANSLATOR.restore_one_game_failed(game_name), &err)?;
                    return Err(err);
                }
            }

            // Launch game
            //
            // TODO.2023-07-12 legendary returns immediately, handle this!
            let result = Command::new(&commands[0]).args(&commands[1..]).status();
            match result {
                Ok(status) => {
                    // TODO.2023-07-14 handle return status which indicate an error condition, e.g. != 0
                    log::debug!("WRAP::execute: Game command executed, returning status: {:#?}", status);
                }
                Err(err) => {
                    log::error!("WRAP::execute: Game command execution failed with: {:#?}", err);
                    ui::alert_with_raw_error(gui, &TRANSLATOR.game_did_not_launch(), &err.to_string())?;
                    return Err(Error::GameDidNotLaunch { why: err.to_string() });
                }
            }

            // Backup
            'backup: {
                let Some(game_name) = game_name.as_ref() else {
                    break 'backup;
                };

                if !ui::confirm(
                    gui,
                    force.then_some(true),
                    &TRANSLATOR.back_up_one_game_confirm(game_name),
                )? {
                    break 'backup;
                }

                if let Err(err) = run(
                    Subcommand::Backup {
                        games: vec![game_name.clone()],
                        force: true,
                        preview: Default::default(),
                        path: Default::default(),
                        wine_prefix: Default::default(),
                        api: Default::default(),
                        sort: Default::default(),
                        format: Default::default(),
                        compression: Default::default(),
                        compression_level: Default::default(),
                        full_limit: Default::default(),
                        differential_limit: Default::default(),
                        cloud_sync: Default::default(),
                        no_cloud_sync: Default::default(),
                    },
                    no_manifest_update,
                    try_manifest_update,
                ) {
                    log::error!("WRAP::backup: failed with: {:#?}", err);
                    ui::alert_with_error(gui, &TRANSLATOR.back_up_one_game_failed(game_name), &err)?;
                    return Err(err);
                }
            }
        }
    }
    if failed {
        Err(Error::SomeEntriesFailed)
    } else {
        Ok(())
    }
}

fn configure_cloud(config: &mut Config, remote: Remote) -> Result<(), Error> {
    if let Some(old_remote) = config.cloud.remote.as_ref() {
        _ = Rclone::new(config.apps.rclone.clone(), old_remote.clone()).unconfigure_remote();
    }

    Rclone::new(config.apps.rclone.clone(), remote.clone())
        .configure_remote()
        .map_err(Error::UnableToConfigureCloud)?;

    config.cloud.remote = Some(remote);
    config.save();
    Ok(())
}

fn ask(question: String, finality: Finality, force: bool) -> Result<bool, Error> {
    if finality.preview() || force {
        Ok(true)
    } else {
        dialoguer::Confirm::new()
            .with_prompt(question)
            .interact()
            .map_err(|_| Error::CliUnableToRequestConfirmation)
    }
}

fn scan_progress_bar(length: u64) -> ProgressBar {
    let template = format!(
        "{} ({{elapsed_precise}}) {{wide_bar}} {}: {{pos}} / {{len}}",
        TRANSLATOR.scan_label(),
        TRANSLATOR.total_games()
    );
    let style = indicatif::ProgressStyle::default_bar()
        .template(&template)
        .expect("progress bar");
    let bar = ProgressBar::new(length).with_style(style);
    bar.enable_steady_tick(PROGRESS_BAR_REFRESH_INTERVAL);
    bar
}

fn cloud_progress_bar() -> ProgressBar {
    let template = format!(
        "{} ({{elapsed_precise}}) {{wide_bar}} {{msg}}",
        TRANSLATOR.cloud_label()
    );
    let style = indicatif::ProgressStyle::default_bar()
        .template(&template)
        .expect("progress bar");
    let bar = ProgressBar::new(100).with_style(style);
    bar.enable_steady_tick(PROGRESS_BAR_REFRESH_INTERVAL);
    bar
}

fn sync_cloud(
    config: &Config,
    local: &StrictPath,
    cloud: &str,
    sync: SyncDirection,
    finality: Finality,
    games: &[String],
) -> Result<Vec<CloudChange>, Error> {
    match finality {
        Finality::Preview => log::info!("checking cloud sync"),
        Finality::Final => log::info!("performing cloud sync"),
    }

    let remote = crate::cloud::validate_cloud_config(config, cloud)?;

    let games = if !games.is_empty() {
        let layout = BackupLayout::new(local.clone(), config.backup.retention.clone());
        let games: Vec<_> = games.iter().filter_map(|x| layout.game_folder(x).leaf()).collect();
        games
    } else {
        vec![]
    };

    let rclone = Rclone::new(config.apps.rclone.clone(), remote);
    let mut process = match rclone.sync(local, cloud, sync, finality, &games) {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let interrupted = register_sigint();

    let progress_bar = cloud_progress_bar();
    let mut changes = vec![];
    loop {
        if interrupted.load(std::sync::atomic::Ordering::Relaxed) {
            if let Err(e) = process.kill() {
                eprintln!("Unable to stop Rclone: {e:?}");
            }
            std::process::exit(1);
        }

        let events = process.events();
        for event in events {
            match event {
                crate::cloud::RcloneProcessEvent::Progress { current, max } => {
                    progress_bar.set_length(max as u64);
                    progress_bar.set_position(current as u64);
                    progress_bar.set_message(TRANSLATOR.cloud_progress(current as u64, max as u64))
                }
                crate::cloud::RcloneProcessEvent::Change(change) => {
                    changes.push(change);
                }
            }
        }
        match process.succeeded() {
            Some(Ok(_)) => {
                unregister_sigint();
                return Ok(changes);
            }
            Some(Err(e)) => {
                unregister_sigint();
                progress_bar.finish_and_clear();
                return Err(Error::UnableToSynchronizeCloud(e));
            }
            None => (),
        }
    }
}
