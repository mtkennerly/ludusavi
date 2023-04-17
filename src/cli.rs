mod parse;
mod report;

use clap::CommandFactory;
use indicatif::{ParallelProgressIterator, ProgressBar};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    prelude::IndexedParallelIterator,
};

use crate::{
    cli::{
        parse::{Cli, CompletionShell, ManifestSubcommand, Subcommand},
        report::Reporter,
    },
    cloud::{Rclone, Remote, RemoteChoice},
    lang::TRANSLATOR,
    prelude::{app_dir, get_threads_from_env, initialize_rayon, Error, StrictPath},
    resource::{cache::Cache, config::Config, manifest::Manifest, ResourceFile, SaveableResourceFile},
    scan::{
        heroic::HeroicGames, layout::BackupLayout, prepare_backup_target, scan_game_for_backup, BackupId,
        DuplicateDetector, InstallDirRanking, OperationStepDecision, SteamShortcuts, TitleFinder,
    },
};

#[derive(Clone, Debug, Default)]
struct GameSubjects {
    valid: Vec<String>,
    invalid: Vec<String>,
}

impl GameSubjects {
    pub fn new(known: Vec<String>, requested: Vec<String>, by_steam_id: bool, manifest: &Manifest) -> Self {
        let mut subjects = Self::default();

        if requested.is_empty() {
            subjects.valid = known;
        } else if by_steam_id {
            let steam_ids_to_names = &manifest.map_steam_ids_to_names();
            for game in requested {
                match game.parse::<u32>() {
                    Ok(id) => {
                        if steam_ids_to_names.contains_key(&id) && known.contains(&steam_ids_to_names[&id]) {
                            subjects.valid.push(steam_ids_to_names[&id].clone());
                        } else {
                            subjects.invalid.push(game);
                        }
                    }
                    Err(_) => {
                        subjects.invalid.push(game);
                    }
                }
            }
        } else {
            for game in requested {
                if known.contains(&game) {
                    subjects.valid.push(game);
                } else {
                    subjects.invalid.push(game);
                }
            }
        }

        subjects.valid.sort();
        subjects.invalid.sort();
        subjects
    }
}

fn warn_deprecations(by_steam_id: bool) {
    if by_steam_id {
        eprintln!("WARNING: `--by-steam-id` is deprecated. Use the `find` command instead.");
    }
}

pub fn parse() -> Cli {
    use clap::Parser;
    Cli::from_args()
}

pub fn run(sub: Subcommand) -> Result<(), Error> {
    let mut config = Config::load()?;
    if let Some(threads) = get_threads_from_env().or(config.runtime.threads) {
        initialize_rayon(threads);
    }
    TRANSLATOR.set_language(config.language);
    let mut cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
    let mut failed = false;
    let mut duplicate_detector = DuplicateDetector::default();

    log::debug!("Config on startup: {config:?}");
    log::debug!("Invocation: {sub:?}");

    match sub {
        Subcommand::Backup {
            preview,
            path,
            force,
            merge,
            no_merge,
            update,
            try_update,
            by_steam_id,
            wine_prefix,
            api,
            sort,
            format,
            compression,
            compression_level,
            full_limit,
            differential_limit,
            games,
        } => {
            warn_deprecations(by_steam_id);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };

            let manifest = if try_update {
                if let Err(e) = Manifest::update_mut(&config, &mut cache, true) {
                    eprintln!("{}", TRANSLATOR.handle_error(&e));
                }
                Manifest::load().unwrap_or_default()
            } else {
                Manifest::update_mut(&config, &mut cache, update)?;
                Manifest::load()?
            };

            let backup_dir = match path {
                None => config.backup.path.clone(),
                Some(p) => p,
            };
            let roots = config.expanded_roots();

            let merge = if merge {
                true
            } else if no_merge {
                false
            } else {
                config.backup.merge
            };

            if !preview && !force {
                match dialoguer::Confirm::new()
                    .with_prompt(TRANSLATOR.confirm_backup(&backup_dir, backup_dir.exists(), merge, false))
                    .interact()
                {
                    Ok(true) => (),
                    Ok(false) => return Ok(()),
                    Err(_) => return Err(Error::CliUnableToRequestConfirmation),
                }
            }

            if !preview {
                prepare_backup_target(&backup_dir, merge)?;
            }

            let mut all_games = manifest;
            all_games.incorporate_extensions(&config.roots, &config.custom_games);

            let games_specified = !games.is_empty();
            let subjects = GameSubjects::new(all_games.0.keys().cloned().collect(), games, by_steam_id, &all_games);
            if !subjects.invalid.is_empty() {
                reporter.trip_unknown_games(subjects.invalid.clone());
                reporter.print_failure();
                return Err(Error::CliUnrecognizedGames {
                    games: subjects.invalid,
                });
            }

            log::info!("beginning backup with {} steps", subjects.valid.len());

            let mut retention = config.backup.retention.clone();
            if let Some(full_limit) = full_limit {
                retention.full = full_limit;
            }
            if let Some(differential_limit) = differential_limit {
                retention.differential = differential_limit;
            }

            let layout = BackupLayout::new(backup_dir.clone(), retention);
            let title_finder = TitleFinder::new(&all_games, &layout);
            let heroic_games = HeroicGames::scan(&roots, &title_finder, None);
            let filter = config.backup.filter.clone();
            let ranking = InstallDirRanking::scan(&roots, &all_games, &subjects.valid);
            let toggled_paths = config.backup.toggled_paths.clone();
            let toggled_registry = config.backup.toggled_registry.clone();
            let steam_shortcuts = SteamShortcuts::scan();

            let mut info: Vec<_> = subjects
                .valid
                .par_iter()
                .enumerate()
                .progress_count(subjects.valid.len() as u64)
                .map(|(i, name)| {
                    log::trace!("step {i} / {}: {name}", subjects.valid.len());
                    let game = &all_games.0[name];

                    let previous = layout.latest_backup(name, false, &config.redirects);

                    let scan_info = scan_game_for_backup(
                        game,
                        name,
                        &roots,
                        &StrictPath::from_std_path_buf(&app_dir()),
                        &heroic_games,
                        &filter,
                        &wine_prefix,
                        &ranking,
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
                            .back_up(&scan_info, merge, &chrono::Utc::now(), &backup_format)
                    };
                    log::trace!("step {i} completed");
                    (name, scan_info, backup_info, decision)
                })
                .collect();
            log::info!("completed backup");

            for (_, scan_info, _, _) in info.iter() {
                if !scan_info.can_report_game() {
                    continue;
                }
                duplicate_detector.add_game(
                    scan_info,
                    config.is_game_enabled_for_operation(&scan_info.game_name, false),
                );
            }

            let sort = sort.map(From::from).unwrap_or_else(|| config.backup.sort.clone());
            info.sort_by(|(_, scan_info1, backup_info1, ..), (_, scan_info2, backup_info2, ..)| {
                crate::scan::compare_games(sort.key, scan_info1, Some(backup_info1), scan_info2, Some(backup_info2))
            });
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
            by_steam_id,
            api,
            sort,
            backup,
            games,
        } => {
            warn_deprecations(by_steam_id);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };

            if !Manifest::path().exists() {
                Manifest::update_mut(&config, &mut cache, true)?;
            }
            let manifest = Manifest::load()?;

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

            let restorable_names = layout.restorable_games();

            if backup.is_some() && games.len() != 1 {
                return Err(Error::CliBackupIdWithMultipleGames);
            }
            let backup_id = backup.as_ref().map(|x| BackupId::Named(x.clone()));

            let games_specified = !games.is_empty();
            let subjects = GameSubjects::new(restorable_names, games, by_steam_id, &manifest);
            if !subjects.invalid.is_empty() {
                reporter.trip_unknown_games(subjects.invalid.clone());
                reporter.print_failure();
                return Err(Error::CliUnrecognizedGames {
                    games: subjects.invalid,
                });
            }

            log::info!("beginning restore with {} steps", subjects.valid.len());

            let mut info: Vec<_> = subjects
                .valid
                .par_iter()
                .enumerate()
                .progress_count(subjects.valid.len() as u64)
                .map(|(i, name)| {
                    log::trace!("step {i} / {}: {name}", subjects.valid.len());
                    let mut layout = layout.game_layout(name);
                    let scan_info = layout.scan_for_restoration(
                        name,
                        backup_id.as_ref().unwrap_or(&BackupId::Latest),
                        &config.redirects,
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
                                return (
                                    name,
                                    scan_info,
                                    Default::default(),
                                    decision,
                                    Some(Err(Error::CliInvalidBackupId)),
                                );
                            }
                        }
                    }

                    let restore_info = if scan_info.backup.is_none() || preview || ignored {
                        crate::scan::BackupInfo::default()
                    } else {
                        layout.restore(&scan_info)
                    };
                    log::trace!("step {i} completed");
                    (name, scan_info, restore_info, decision, None)
                })
                .collect();
            log::info!("completed restore");

            for (_, scan_info, _, _, failure) in info.iter() {
                if !scan_info.can_report_game() {
                    continue;
                }
                if let Some(failure) = failure {
                    return failure.clone();
                }
                duplicate_detector.add_game(
                    scan_info,
                    config.is_game_enabled_for_operation(&scan_info.game_name, true),
                );
            }

            let sort = sort.map(From::from).unwrap_or_else(|| config.restore.sort.clone());
            info.sort_by(|(_, scan_info1, backup_info1, ..), (_, scan_info2, backup_info2, ..)| {
                crate::scan::compare_games(sort.key, scan_info1, Some(backup_info1), scan_info2, Some(backup_info2))
            });
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
                &mut Cli::into_app(),
                env!("CARGO_PKG_NAME"),
                &mut std::io::stdout(),
            )
        }
        Subcommand::Backups {
            path,
            by_steam_id,
            api,
            games,
        } => {
            warn_deprecations(by_steam_id);

            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };
            reporter.suppress_overall();

            if !Manifest::path().exists() {
                Manifest::update_mut(&config, &mut cache, true)?;
            }
            let manifest = Manifest::load()?;

            let restore_dir = match path {
                None => config.restore.path.clone(),
                Some(p) => p,
            };

            let layout = BackupLayout::new(restore_dir.clone(), config.backup.retention.clone());

            let restorable_names = layout.restorable_games();

            let subjects = GameSubjects::new(restorable_names, games, by_steam_id, &manifest);
            if !subjects.invalid.is_empty() {
                reporter.trip_unknown_games(subjects.invalid.clone());
                reporter.print_failure();
                return Err(Error::CliUnrecognizedGames {
                    games: subjects.invalid,
                });
            }

            let info: Vec<_> = subjects
                .valid
                .par_iter()
                .progress_count(subjects.valid.len() as u64)
                .map(|name| {
                    let mut layout = layout.game_layout(name);
                    let backups = layout.get_backups();
                    (name, backups)
                })
                .collect();

            for (name, backups) in info {
                reporter.add_backups(name, &backups);
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
            names,
        } => {
            let mut reporter = if api { Reporter::json() } else { Reporter::standard() };
            reporter.suppress_overall();

            if let Err(e) = Manifest::update_mut(&config, &mut cache, false) {
                eprintln!("{}", TRANSLATOR.handle_error(&e));
            }
            let mut manifest = Manifest::load().unwrap_or_default();
            manifest.incorporate_extensions(&config.roots, &config.custom_games);

            let restore_dir = match path {
                None => config.restore.path.clone(),
                Some(p) => p,
            };
            let layout = BackupLayout::new(restore_dir.clone(), config.backup.retention.clone());

            let title_finder = TitleFinder::new(&manifest, &layout);
            let found = title_finder.find(&names, &steam_id, &gog_id, normalized, backup, restore);
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
        Subcommand::Manifest { sub: manifest_sub } => {
            if let Some(ManifestSubcommand::Show { api }) = manifest_sub {
                let mut manifest = Manifest::load().unwrap_or_default();
                manifest.incorporate_extensions(&config.roots, &config.custom_games);

                if api {
                    println!("{}", serde_json::to_string(&manifest).unwrap());
                } else {
                    println!("{}", serde_yaml::to_string(&manifest).unwrap());
                }
            }
        }
        Subcommand::Cloud { sub: cloud_sub } => match cloud_sub {
            parse::CloudSubcommand::Set { remote, name } => {
                let remote = match remote {
                    RemoteChoice::None => {
                        config.cloud.remote = None;
                        config.save();
                        return Ok(());
                    }
                    RemoteChoice::Custom => Remote::Custom {
                        name: name.unwrap_or_else(|| "ludusavi".to_string()),
                    },
                    RemoteChoice::Box => Remote::Box,
                    RemoteChoice::Dropbox => Remote::Dropbox,
                    RemoteChoice::GoogleDrive => Remote::GoogleDrive,
                    RemoteChoice::OneDrive => Remote::OneDrive,
                };
                if remote.needs_configuration() {
                    let rclone = Rclone::new(config.apps.rclone.clone(), remote.clone());
                    if let Err(e) = rclone.configure_remote() {
                        return Err(Error::UnableToConfigureCloud(e));
                    }
                }
                config.cloud.remote = Some(remote);
                config.save();
            }
            parse::CloudSubcommand::Upload { local, cloud, force } => {
                sync_cloud(&config, local, cloud, force, CloudSync::Upload)?;
            }
            parse::CloudSubcommand::Download { local, cloud, force } => {
                sync_cloud(&config, local, cloud, force, CloudSync::Download)?;
            }
        },
    }

    if failed {
        Err(Error::SomeEntriesFailed)
    } else {
        Ok(())
    }
}

enum CloudSync {
    Upload,
    Download,
}

fn sync_cloud(
    config: &Config,
    local: Option<StrictPath>,
    cloud: Option<String>,
    force: bool,
    sync: CloudSync,
) -> Result<(), Error> {
    let local = local.unwrap_or(config.backup.path.clone());
    let cloud = cloud.unwrap_or(config.cloud.path.clone());
    if !config.apps.rclone.is_valid() {
        return Err(Error::RcloneUnavailable);
    }
    let Some(remote) = config.cloud.remote.clone() else { return Err(Error::CloudNotConfigured) };
    crate::cloud::validate_cloud_path(&cloud)?;

    if !force {
        match dialoguer::Confirm::new()
            .with_prompt(match sync {
                CloudSync::Upload => TRANSLATOR.confirm_cloud_upload(&local.render(), &cloud),
                CloudSync::Download => TRANSLATOR.confirm_cloud_download(&local.render(), &cloud),
            })
            .interact()
        {
            Ok(true) => (),
            Ok(false) => return Ok(()),
            Err(_) => return Err(Error::CliUnableToRequestConfirmation),
        }
    }

    let rclone = Rclone::new(config.apps.rclone.clone(), remote);
    let process = match sync {
        CloudSync::Upload => rclone.sync_from_local_to_remote(&local, &cloud),
        CloudSync::Download => rclone.sync_from_remote_to_local(&local, &cloud),
    };
    let mut process = match process {
        Ok(p) => p,
        Err(e) => return Err(Error::UnableToSynchronizeCloud(e)),
    };

    let progress_bar = ProgressBar::new(100);
    loop {
        match process.succeeded() {
            Some(Ok(_)) => return Ok(()),
            Some(Err(e)) => {
                progress_bar.finish_and_clear();
                return Err(Error::UnableToSynchronizeCloud(e));
            }
            None => (),
        }
        if let Some((current, max)) = process.progress() {
            progress_bar.set_length(max as u64);
            progress_bar.set_position(current as u64);
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}
