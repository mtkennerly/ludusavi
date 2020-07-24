use crate::{
    config::{Config, RedirectConfig},
    lang::Translator,
    manifest::{Game, Manifest, SteamMetadata},
    prelude::{
        app_dir, back_up_game, game_file_restoration_target, prepare_backup_target, restore_game,
        scan_dir_for_restorable_games, scan_dir_for_restoration, scan_game_for_backup, BackupInfo, Error,
        OperationStatus, OperationStepDecision, ScanInfo, StrictPath,
    },
};
use indicatif::ParallelProgressIterator;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use structopt::StructOpt;

fn parse_strict_path(path: &str) -> StrictPath {
    StrictPath::new(path.to_owned())
}

fn parse_existing_strict_path(path: &str) -> Result<StrictPath, std::io::Error> {
    let sp = StrictPath::new(path.to_owned());
    std::fs::canonicalize(sp.interpret())?;
    Ok(sp)
}

#[derive(structopt::StructOpt, Clone, Debug, PartialEq)]
pub enum Subcommand {
    #[structopt(about = "Back up data")]
    Backup {
        /// List out what would be included, but don't actually perform the operation.
        #[structopt(long)]
        preview: bool,

        /// Directory in which to create the backup. The directory must not
        /// already exist (unless you use --force), but it will be created if necessary.
        /// When unset, this defaults to the value from Ludusavi's config file.
        #[structopt(long, parse(from_str = parse_strict_path))]
        path: Option<StrictPath>,

        /// Delete the target directory if it already exists.
        #[structopt(long)]
        force: bool,

        /// Download the latest copy of the manifest.
        #[structopt(long)]
        update: bool,

        /// Only back up these specific games.
        #[structopt()]
        games: Vec<String>,
    },
    #[structopt(about = "Restore data")]
    Restore {
        /// List out what would be included, but don't actually perform the operation.
        #[structopt(long)]
        preview: bool,

        /// Directory containing a Ludusavi backup. When unset, this
        /// defaults to the value from Ludusavi's config file.
        #[structopt(long, parse(try_from_str = parse_existing_strict_path))]
        path: Option<StrictPath>,

        /// Don't ask for confirmation.
        #[structopt(long)]
        force: bool,

        /// Only restore these specific games.
        #[structopt()]
        games: Vec<String>,
    },
}

#[derive(structopt::StructOpt, Clone, Debug, PartialEq)]
#[structopt(name = "ludusavi", about = "Back up and restore PC game saves", set_term_width = 79)]
pub struct Cli {
    #[structopt(subcommand)]
    pub sub: Option<Subcommand>,
}

pub fn parse_cli() -> Cli {
    Cli::from_args()
}

fn show_outcome(
    translator: &Translator,
    name: &str,
    scan_info: &ScanInfo,
    backup_info: &BackupInfo,
    restoring: bool,
    decision: &OperationStepDecision,
    redirects: &[RedirectConfig],
) -> Option<bool> {
    if !scan_info.found_anything() {
        return None;
    }

    let mut successful = true;
    println!(
        "{}",
        translator.cli_game_header(&name, scan_info.sum_bytes(&Some(backup_info.to_owned())), &decision)
    );
    for entry in itertools::sorted(&scan_info.found_files) {
        let mut redirected_from = None;
        let readable = if restoring {
            let (original_target, redirected_target) = game_file_restoration_target(&entry.path, &redirects).unwrap();
            if original_target != redirected_target {
                redirected_from = Some(original_target);
            }
            redirected_target
        } else {
            entry.path.to_owned()
        };
        if backup_info.failed_files.contains(entry) {
            successful = false;
            println!("{}", translator.cli_game_line_item_failed(&readable.render()));
        } else {
            println!("{}", translator.cli_game_line_item_successful(&readable.render()));
        }
        if let Some(redirected_from) = redirected_from {
            println!(
                "{}",
                translator.cli_game_line_item_redirected(&redirected_from.render())
            );
        }
    }
    for entry in itertools::sorted(&scan_info.found_registry_keys) {
        if backup_info.failed_registry.contains(entry) {
            successful = false;
            println!("{}", translator.cli_game_line_item_failed(entry));
        } else {
            println!("{}", translator.cli_game_line_item_successful(entry));
        }
    }
    Some(successful)
}

pub fn run_cli(sub: Subcommand) -> Result<(), Error> {
    let translator = Translator::default();
    let mut config = Config::load()?;
    let mut failed = false;

    match sub {
        Subcommand::Backup {
            preview,
            path,
            force,
            update,
            games,
        } => {
            let manifest = Manifest::load(&mut config, update)?;

            let backup_dir = match path {
                None => config.backup.path.clone(),
                Some(p) => p,
            };
            let roots = &config.roots;

            if !preview {
                if !force && backup_dir.exists() {
                    return Err(crate::prelude::Error::CliBackupTargetExists { path: backup_dir });
                } else if let Err(e) = prepare_backup_target(&backup_dir) {
                    return Err(e);
                }
            }

            let mut all_games = manifest.0;
            for custom_game in &config.custom_games {
                all_games.insert(custom_game.name.clone(), Game::from(custom_game.to_owned()));
            }

            let games_specified = !games.is_empty();
            let mut invalid_games: Vec<_> = games
                .iter()
                .filter_map(|game| {
                    if !all_games.contains_key(game) {
                        Some(game.to_owned())
                    } else {
                        None
                    }
                })
                .collect();
            if !invalid_games.is_empty() {
                invalid_games.sort();
                return Err(crate::prelude::Error::CliUnrecognizedGames { games: invalid_games });
            }

            let mut subjects: Vec<_> = if !&games.is_empty() {
                games
            } else {
                all_games.keys().cloned().collect()
            };
            subjects.sort();

            let info: Vec<_> = subjects
                .par_iter()
                .progress_count(subjects.len() as u64)
                .map(|name| {
                    let game = &all_games[name];
                    let steam_id = &game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;

                    let scan_info = scan_game_for_backup(
                        &game,
                        &name,
                        &roots,
                        &StrictPath::from_std_path_buf(&app_dir()),
                        &steam_id,
                    );
                    let ignored = !&config.is_game_enabled_for_backup(&name) && !games_specified;
                    let decision = if ignored {
                        OperationStepDecision::Ignored
                    } else {
                        OperationStepDecision::Processed
                    };
                    let backup_info = if preview || ignored {
                        crate::prelude::BackupInfo::default()
                    } else {
                        back_up_game(&scan_info, &backup_dir, &name)
                    };
                    (name, scan_info, backup_info, decision)
                })
                .collect();

            let mut status = OperationStatus::default();
            for (name, scan_info, backup_info, decision) in info {
                if let Some(successful) =
                    show_outcome(&translator, &name, &scan_info, &backup_info, false, &decision, &[])
                {
                    status.add_game(
                        &scan_info,
                        &Some(backup_info),
                        decision == OperationStepDecision::Processed,
                    );
                    if !successful {
                        failed = true;
                    }
                };
            }
            println!("{}", translator.cli_summary(&status, &backup_dir));
        }
        Subcommand::Restore {
            preview,
            path,
            force,
            games,
        } => {
            let restore_dir = match path {
                None => config.restore.path.clone(),
                Some(p) => p,
            };

            if !preview && !force {
                match dialoguer::Confirm::new()
                    .with_prompt(translator.cli_confirm_restoration(&restore_dir))
                    .interact()
                {
                    Ok(true) => (),
                    Ok(false) => return Ok(()),
                    Err(_) => return Err(Error::CliUnableToRequestConfirmation),
                }
            }

            let restorables = scan_dir_for_restorable_games(&restore_dir);
            let restorable_names: Vec<_> = restorables.iter().map(|(name, _)| name.to_owned()).collect();

            let games_specified = !games.is_empty();
            let mut invalid_games: Vec<_> = games
                .iter()
                .filter_map(|game| {
                    if !restorable_names.contains(game) {
                        Some(game.to_owned())
                    } else {
                        None
                    }
                })
                .collect();
            if !invalid_games.is_empty() {
                invalid_games.sort();
                return Err(crate::prelude::Error::CliUnrecognizedGames { games: invalid_games });
            }

            let mut subjects: Vec<_> = if !&games.is_empty() {
                restorables
                    .iter()
                    .filter_map(|x| if games.contains(&x.0) { Some(x.to_owned()) } else { None })
                    .collect()
            } else {
                restorables
            };
            subjects.sort();

            let info: Vec<_> = subjects
                .par_iter()
                .progress_count(subjects.len() as u64)
                .map(|(name, path)| {
                    let scan_info = scan_dir_for_restoration(&path);
                    let ignored = !&config.is_game_enabled_for_restore(&name) && !games_specified;
                    let decision = if ignored {
                        OperationStepDecision::Ignored
                    } else {
                        OperationStepDecision::Processed
                    };
                    let restore_info = if preview || ignored {
                        crate::prelude::BackupInfo::default()
                    } else {
                        restore_game(&scan_info, &config.get_redirects())
                    };
                    (name, scan_info, restore_info, decision)
                })
                .collect();

            let mut status = OperationStatus::default();
            for (name, scan_info, backup_info, decision) in info {
                if let Some(successful) = show_outcome(
                    &translator,
                    &name,
                    &scan_info,
                    &backup_info,
                    true,
                    &decision,
                    &config.get_redirects(),
                ) {
                    status.add_game(
                        &scan_info,
                        &Some(backup_info),
                        decision == OperationStepDecision::Processed,
                    );
                    if !successful {
                        failed = true;
                    }
                };
            }
            println!("{}", translator.cli_summary(&status, &restore_dir));
        }
    }

    if failed {
        Err(crate::prelude::Error::SomeEntriesFailed)
    } else {
        Ok(())
    }
}
