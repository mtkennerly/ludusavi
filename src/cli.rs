mod parse;
mod report;

use parse::{Cli, CompletionShell, Subcommand};
use report::Reporter;

use crate::{
    cache::Cache,
    cli::parse::ManifestSubcommand,
    config::{Config, SortKey},
    heroic::HeroicGames,
    lang::Translator,
    layout::BackupLayout,
    manifest::Manifest,
    prelude::{app_dir, Error, StrictPath},
    resource::ResourceFile,
    scan::{
        back_up_game, prepare_backup_target, scan_game_for_backup, scan_game_for_restoration, BackupId,
        DuplicateDetector, InstallDirRanking, OperationStepDecision, SteamShortcuts, TitleFinder,
    },
};
use clap::CommandFactory;
use indicatif::ParallelProgressIterator;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    prelude::IndexedParallelIterator,
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
    let translator = Translator::default();
    let mut config = Config::load()?;
    translator.set_language(config.language);
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

            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };

            let manifest = if try_update {
                if let Err(e) = Manifest::update_mut(&config, &mut cache, true) {
                    eprintln!("{}", translator.handle_error(&e));
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
                    .with_prompt(translator.confirm_backup(&backup_dir, backup_dir.exists(), merge, false))
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
            for custom_game in &config.custom_games {
                if custom_game.ignore {
                    continue;
                }
                all_games.add_custom_game(custom_game.clone());
            }

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
                    let steam_id = game.steam.as_ref().and_then(|x| x.id);

                    let previous = layout.latest_backup(name, false, &config.redirects);

                    let scan_info = scan_game_for_backup(
                        game,
                        name,
                        &roots,
                        &StrictPath::from_std_path_buf(&app_dir()),
                        &heroic_games,
                        &steam_id,
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

                        back_up_game(
                            &scan_info,
                            layout.game_layout(name),
                            merge,
                            &chrono::Utc::now(),
                            &backup_format,
                        )
                    };
                    log::trace!("step {i} completed");
                    (name, scan_info, backup_info, decision)
                })
                .collect();
            log::info!("completed backup");

            for (_, scan_info, _, _) in info.iter() {
                if !scan_info.found_anything() {
                    continue;
                }
                duplicate_detector.add_game(scan_info);
            }

            let sort = sort.map(From::from).unwrap_or_else(|| config.backup.sort.clone());
            match sort.key {
                SortKey::Name => {
                    info.sort_by(|(name1, ..), (name2, ..)| crate::scan::compare_games_by_name(name1, name2))
                }
                SortKey::Size => {
                    info.sort_by(|(_, scan_info1, backup_info1, ..), (_, scan_info2, backup_info2, ..)| {
                        crate::scan::compare_games_by_size(
                            scan_info1,
                            &Some(backup_info1.clone()),
                            scan_info2,
                            &Some(backup_info2.clone()),
                        )
                    })
                }
            }
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

            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };

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
                    .with_prompt(translator.confirm_restore(&restore_dir, false))
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
                    let scan_info = scan_game_for_restoration(
                        name,
                        backup_id.as_ref().unwrap_or(&BackupId::Latest),
                        &mut layout,
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
                if !scan_info.found_anything() {
                    continue;
                }
                if let Some(failure) = failure {
                    return failure.clone();
                }
                duplicate_detector.add_game(scan_info);
            }

            let sort = sort.map(From::from).unwrap_or_else(|| config.restore.sort.clone());
            match sort.key {
                SortKey::Name => {
                    info.sort_by(|(name1, ..), (name2, ..)| crate::scan::compare_games_by_name(name1, name2))
                }
                SortKey::Size => {
                    info.sort_by(|(_, scan_info1, backup_info1, ..), (_, scan_info2, backup_info2, ..)| {
                        crate::scan::compare_games_by_size(
                            scan_info1,
                            &Some(backup_info1.clone()),
                            scan_info2,
                            &Some(backup_info2.clone()),
                        )
                    })
                }
            }
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

            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };
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
                    let scan_info = scan_game_for_restoration(name, &BackupId::Latest, &mut layout, &config.redirects);
                    (name, scan_info)
                })
                .collect();

            for (name, scan_info) in info {
                reporter.add_backup(name, &scan_info);
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
            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };
            reporter.suppress_overall();

            if let Err(e) = Manifest::update_mut(&config, &mut cache, false) {
                eprintln!("{}", translator.handle_error(&e));
            }
            let mut manifest = Manifest::load().unwrap_or_default();

            for custom_game in &config.custom_games {
                if custom_game.ignore {
                    continue;
                }
                manifest.add_custom_game(custom_game.clone());
            }

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
                manifest.load_custom_games(&config);

                if api {
                    println!("{}", serde_json::to_string(&manifest).unwrap());
                } else {
                    println!("{}", serde_yaml::to_string(&manifest).unwrap());
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
