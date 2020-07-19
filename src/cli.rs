use crate::{
    config::Config,
    lang::Translator,
    manifest::{Manifest, SteamMetadata},
    prelude::{
        app_dir, back_up_game, game_file_restoration_target, prepare_backup_target, restore_game,
        scan_dir_for_restorable_games, scan_dir_for_restoration, scan_game_for_backup, BackupInfo, Error, ScanInfo,
    },
};
use indicatif::ParallelProgressIterator;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use structopt::StructOpt;

fn parse_canonical_path(path: &str) -> Result<String, std::io::Error> {
    Ok(std::fs::canonicalize(path)?.to_string_lossy().to_string())
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
        #[structopt(long, parse(try_from_str = parse_canonical_path))]
        path: Option<String>,

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
        #[structopt(long, parse(try_from_str = parse_canonical_path))]
        path: Option<String>,

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
) -> Option<bool> {
    if scan_info.found_files.is_empty() && scan_info.found_registry_keys.is_empty() {
        return None;
    }

    let mut successful = true;
    println!(
        "{} [{}]:",
        &name,
        translator.mib(scan_info.found_files.iter().map(|x| x.size).sum::<u64>(), false)
    );
    for entry in itertools::sorted(&scan_info.found_files) {
        let readable = if restoring {
            game_file_restoration_target(&entry.path).unwrap()
        } else {
            entry.path.to_owned()
        };
        if backup_info.failed_files.contains(entry) {
            successful = false;
            println!("  - {} {}", translator.cli_label_failed(), readable);
        } else {
            println!("  - {}", readable);
        }
    }
    for entry in itertools::sorted(&scan_info.found_registry_keys) {
        if backup_info.failed_registry.contains(entry) {
            successful = false;
            println!("  - {} {}", translator.cli_label_failed(), entry);
        } else {
            println!("  - {}", entry);
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

            let backup_dir = crate::path::normalize(&path.unwrap_or_else(|| config.backup.path.to_owned()));
            let roots = &config.roots;

            if !preview {
                if !force && crate::path::exists(&backup_dir) {
                    return Err(crate::prelude::Error::CliBackupTargetExists { path: backup_dir });
                } else if let Err(e) = prepare_backup_target(&backup_dir) {
                    return Err(e);
                }
            }

            let mut invalid_games: Vec<_> = games
                .iter()
                .filter_map(|game| {
                    if !manifest.0.contains_key(game) {
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
                manifest.0.keys().cloned().collect()
            };
            subjects.sort();

            let info: Vec<_> = subjects
                .par_iter()
                .progress_count(subjects.len() as u64)
                .map(|name| {
                    let game = &manifest.0[name];
                    let steam_id = &game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;

                    let scan_info = scan_game_for_backup(&game, &name, &roots, &app_dir().to_string_lossy(), &steam_id);
                    let backup_info = if preview {
                        crate::prelude::BackupInfo::default()
                    } else {
                        back_up_game(&scan_info, &backup_dir, &name)
                    };
                    (name, scan_info, backup_info)
                })
                .collect();

            let mut total_games = 0;
            let mut total_bytes = 0;
            for (name, scan_info, backup_info) in info {
                if let Some(successful) = show_outcome(&translator, &name, &scan_info, &backup_info, false) {
                    total_games += 1;
                    total_bytes += scan_info.found_files.iter().map(|x| x.size).sum::<u64>();
                    if !successful {
                        failed = true;
                    }
                };
            }
            eprintln!("{}", translator.cli_summary(total_games, total_bytes, &backup_dir));
        }
        Subcommand::Restore {
            preview,
            path,
            force,
            games,
        } => {
            let restore_dir = crate::path::normalize(&path.unwrap_or_else(|| config.restore.path.to_owned()));

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
                    let restore_info = if preview {
                        crate::prelude::BackupInfo::default()
                    } else {
                        restore_game(&scan_info)
                    };
                    (name, scan_info, restore_info)
                })
                .collect();

            let mut total_games = 0;
            let mut total_bytes = 0;
            for (name, scan_info, backup_info) in info {
                if let Some(successful) = show_outcome(&translator, &name, &scan_info, &backup_info, true) {
                    total_games += 1;
                    total_bytes += scan_info.found_files.iter().map(|x| x.size).sum::<u64>();
                    if !successful {
                        failed = true;
                    }
                };
            }
            eprintln!("{}", translator.cli_summary(total_games, total_bytes, &restore_dir));
        }
    }

    if failed {
        Err(crate::prelude::Error::CliSomeEntriesFailed)
    } else {
        Ok(())
    }
}
