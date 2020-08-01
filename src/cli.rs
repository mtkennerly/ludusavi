use crate::{
    config::{Config, RedirectConfig},
    lang::Translator,
    layout::BackupLayout,
    manifest::{Game, Manifest, SteamMetadata},
    prelude::{
        app_dir, back_up_game, game_file_restoration_target, prepare_backup_target, restore_game, scan_game_for_backup,
        scan_game_for_restoration, BackupInfo, Error, OperationStatus, OperationStepDecision, ScanInfo, StrictPath,
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

        /// Merge into existing directory instead of deleting/recreating it.
        /// Within the target directory, the subdirectories for individual
        /// games will still be cleared out first, though.
        /// When not specified, this defers to Ludusavi's config file.
        #[structopt(long)]
        merge: bool,

        /// Don't merge; delete and recreate the target directory.
        /// When not specified, this defers to Ludusavi's config file.
        #[structopt(long, conflicts_with("merge"))]
        no_merge: bool,

        /// Download the latest copy of the manifest.
        #[structopt(long)]
        update: bool,

        /// When naming specific games to process, this means that you'll
        /// provide the Steam IDs instead of the manifest names, and Ludusavi will
        /// look up those IDs in the manifest to find the corresponding names.
        #[structopt(long)]
        by_steam_id: bool,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[structopt(long)]
        api: bool,

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

        /// When naming specific games to process, this means that you'll
        /// provide the Steam IDs instead of the manifest names, and Ludusavi will
        /// look up those IDs in the manifest to find the corresponding names.
        #[structopt(long)]
        by_steam_id: bool,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[structopt(long)]
        api: bool,

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

#[derive(Debug, Default, serde::Serialize)]
struct ApiErrors {
    #[serde(rename = "someGamesFailed", skip_serializing_if = "Option::is_none")]
    some_games_failed: Option<bool>,
    #[serde(rename = "unknownGames", skip_serializing_if = "Option::is_none")]
    unknown_games: Option<Vec<String>>,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiFile {
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    failed: bool,
    bytes: u64,
    #[serde(rename = "originalPath", skip_serializing_if = "Option::is_none")]
    original_path: Option<String>,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiRegistry {
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    failed: bool,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiGame {
    decision: OperationStepDecision,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    files: std::collections::HashMap<String, ApiFile>,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    registry: std::collections::HashMap<String, ApiRegistry>,
}

#[derive(Debug, Default, serde::Serialize)]
struct JsonOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<ApiErrors>,
    overall: OperationStatus,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    games: std::collections::HashMap<String, ApiGame>,
}

#[derive(Debug)]
enum Reporter {
    Standard {
        translator: Translator,
        parts: Vec<String>,
        status: OperationStatus,
    },
    Json {
        output: JsonOutput,
    },
}

impl Reporter {
    fn standard(translator: Translator) -> Self {
        Self::Standard {
            translator,
            parts: vec![],
            status: Default::default(),
        }
    }

    fn json() -> Self {
        Self::Json {
            output: Default::default(),
        }
    }

    fn trip_some_games_failed(&mut self) {
        if let Reporter::Json { output, .. } = self {
            if let Some(errors) = &mut output.errors {
                errors.some_games_failed = Some(true);
            } else {
                output.errors = Some(ApiErrors {
                    some_games_failed: Some(true),
                    ..Default::default()
                });
            }
        }
    }

    fn trip_unknown_games(&mut self, games: Vec<String>) {
        if let Reporter::Json { output, .. } = self {
            if let Some(errors) = &mut output.errors {
                errors.unknown_games = Some(games);
            } else {
                output.errors = Some(ApiErrors {
                    unknown_games: Some(games),
                    ..Default::default()
                });
            }
        }
    }

    fn add_game(
        &mut self,
        name: &str,
        scan_info: &ScanInfo,
        backup_info: &BackupInfo,
        decision: &OperationStepDecision,
        redirects: &[RedirectConfig],
    ) -> bool {
        let mut successful = true;

        match self {
            Self::Standard {
                parts,
                status,
                translator,
            } => {
                if !scan_info.found_anything() {
                    return true;
                }

                parts.push(translator.cli_game_header(
                    &name,
                    scan_info.sum_bytes(&Some(backup_info.to_owned())),
                    &decision,
                ));
                for entry in itertools::sorted(&scan_info.found_files) {
                    let mut redirected_from = None;
                    let readable = if let Some(original_path) = &entry.original_path {
                        let (target, original_target) = game_file_restoration_target(&original_path, &redirects);
                        redirected_from = original_target;
                        target
                    } else {
                        entry.path.to_owned()
                    };

                    if backup_info.failed_files.contains(entry) {
                        successful = false;
                        parts.push(translator.cli_game_line_item_failed(&readable.render()));
                    } else {
                        parts.push(translator.cli_game_line_item_successful(&readable.render()));
                    }

                    if let Some(redirected_from) = redirected_from {
                        parts.push(translator.cli_game_line_item_redirected(&redirected_from.render()));
                    }
                }
                for entry in itertools::sorted(&scan_info.found_registry_keys) {
                    if backup_info.failed_registry.contains(entry) {
                        successful = false;
                        parts.push(translator.cli_game_line_item_failed(entry));
                    } else {
                        parts.push(translator.cli_game_line_item_successful(entry));
                    }
                }

                status.add_game(
                    &scan_info,
                    &Some(backup_info.clone()),
                    decision == &OperationStepDecision::Processed,
                );
            }
            Self::Json { output } => {
                if !scan_info.found_anything() {
                    return true;
                }

                let mut api_game = ApiGame::default();
                api_game.decision = decision.clone();

                for entry in itertools::sorted(&scan_info.found_files) {
                    let mut api_file = ApiFile::default();
                    api_file.bytes = entry.size;
                    api_file.failed = backup_info.failed_files.contains(entry);
                    let readable = if let Some(original_path) = &entry.original_path {
                        let (target, original_target) = game_file_restoration_target(&original_path, &redirects);
                        api_file.original_path = original_target.map(|x| x.render());
                        target
                    } else {
                        entry.path.to_owned()
                    };
                    if api_file.failed {
                        successful = false;
                    }
                    api_game.files.insert(readable.render(), api_file);
                }
                for entry in itertools::sorted(&scan_info.found_registry_keys) {
                    let mut api_registry = ApiRegistry::default();
                    if backup_info.failed_registry.contains(entry) {
                        api_registry.failed = true;
                    }
                    if api_registry.failed {
                        successful = false;
                    }
                    api_game.registry.insert(entry.to_string(), api_registry);
                }

                output.games.insert(name.to_string(), api_game);
                output.overall.add_game(
                    &scan_info,
                    &Some(backup_info.clone()),
                    decision == &OperationStepDecision::Processed,
                );
            }
        }

        if !successful {
            self.trip_some_games_failed();
        }
        successful
    }

    fn render(&self, path: &StrictPath) -> String {
        match self {
            Self::Standard {
                parts,
                status,
                translator,
            } => parts.join("\n") + "\n" + &translator.cli_summary(&status, &path),
            Self::Json { output } => serde_json::to_string_pretty(&output).unwrap(),
        }
    }

    fn print_failure(&self) {
        // The standard reporter doesn't need to print on failure because
        // that's handled generically in main.
        if let Self::Json { .. } = self {
            self.print(&StrictPath::new("".to_string()));
        }
    }

    fn print(&self, path: &StrictPath) {
        println!("{}", self.render(&path));
    }
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
            merge,
            no_merge,
            update,
            by_steam_id,
            api,
            games,
        } => {
            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };

            let manifest = Manifest::load(&mut config, update)?;

            let backup_dir = match path {
                None => config.backup.path.clone(),
                Some(p) => p,
            };
            let roots = &config.roots;

            if !preview {
                if !force && !merge && backup_dir.exists() {
                    return Err(crate::prelude::Error::CliBackupTargetExists { path: backup_dir });
                } else if let Err(e) = prepare_backup_target(
                    &backup_dir,
                    if merge {
                        true
                    } else if no_merge {
                        false
                    } else {
                        config.backup.merge
                    },
                ) {
                    return Err(e);
                }
            }

            let steam_ids_to_names = &manifest.map_steam_ids_to_names();
            let mut all_games = manifest.0;
            for custom_game in &config.custom_games {
                all_games.insert(custom_game.name.clone(), Game::from(custom_game.to_owned()));
            }

            let games_specified = !games.is_empty();
            let mut invalid_games: Vec<_> = games
                .iter()
                .filter_map(|game| {
                    if by_steam_id {
                        match game.parse::<u32>() {
                            Ok(id) => {
                                if !steam_ids_to_names.contains_key(&id) {
                                    Some(game.to_owned())
                                } else {
                                    None
                                }
                            }
                            Err(_) => Some(game.to_owned()),
                        }
                    } else if !all_games.contains_key(game) {
                        Some(game.to_owned())
                    } else {
                        None
                    }
                })
                .collect();
            if !invalid_games.is_empty() {
                invalid_games.sort();
                reporter.trip_unknown_games(invalid_games.clone());
                reporter.print_failure();
                return Err(crate::prelude::Error::CliUnrecognizedGames { games: invalid_games });
            }

            let mut subjects: Vec<_> = if !&games.is_empty() {
                if by_steam_id {
                    games
                        .iter()
                        .map(|game| &steam_ids_to_names[&game.parse::<u32>().unwrap()])
                        .cloned()
                        .collect()
                } else {
                    games
                }
            } else {
                all_games.keys().cloned().collect()
            };
            subjects.sort();

            let layout = BackupLayout::new(backup_dir.clone());
            let filter = config.backup.filter.clone();

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
                        &filter,
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
                        back_up_game(&scan_info, &name, &layout)
                    };
                    (name, scan_info, backup_info, decision)
                })
                .collect();

            for (name, scan_info, backup_info, decision) in info {
                if !reporter.add_game(&name, &scan_info, &backup_info, &decision, &[]) {
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
            games,
        } => {
            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };

            let manifest = Manifest::load(&mut config, false)?;

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

            let layout = BackupLayout::new(restore_dir.clone());

            let steam_ids_to_names = &manifest.map_steam_ids_to_names();
            let restorable_names: Vec<_> = layout.mapping.games.keys().collect();

            let games_specified = !games.is_empty();
            let mut invalid_games: Vec<_> = games
                .iter()
                .filter_map(|game| {
                    if by_steam_id {
                        match game.parse::<u32>() {
                            Ok(id) => {
                                if !steam_ids_to_names.contains_key(&id)
                                    || !restorable_names.contains(&&steam_ids_to_names[&id])
                                {
                                    Some(game.to_owned())
                                } else {
                                    None
                                }
                            }
                            Err(_) => Some(game.to_owned()),
                        }
                    } else if !restorable_names.contains(&game) {
                        Some(game.to_owned())
                    } else {
                        None
                    }
                })
                .collect();
            if !invalid_games.is_empty() {
                invalid_games.sort();
                reporter.trip_unknown_games(invalid_games.clone());
                reporter.print_failure();
                return Err(crate::prelude::Error::CliUnrecognizedGames { games: invalid_games });
            }

            let mut subjects: Vec<_> = if !&games.is_empty() {
                restorable_names
                    .iter()
                    .filter_map(|x| {
                        if (by_steam_id && steam_ids_to_names.values().cloned().any(|y| &y == *x))
                            || (games.contains(&x))
                        {
                            Some(x.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                restorable_names
            };
            subjects.sort();

            let info: Vec<_> = subjects
                .par_iter()
                .progress_count(subjects.len() as u64)
                .map(|name| {
                    let scan_info = scan_game_for_restoration(&name, &layout);
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

            for (name, scan_info, backup_info, decision) in info {
                if !reporter.add_game(&name, &scan_info, &backup_info, &decision, &config.get_redirects()) {
                    failed = true;
                }
            }
            reporter.print(&restore_dir);
        }
    }

    if failed {
        Err(crate::prelude::Error::SomeEntriesFailed)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> String {
        text.to_string()
    }

    mod parser {
        use super::*;

        fn check_args(args: &[&str], expected: Cli) {
            assert_eq!(expected, Cli::from_clap(&Cli::clap().get_matches_from(args)));
        }

        fn check_args_err(args: &[&str], error: structopt::clap::ErrorKind) {
            let result = Cli::clap().get_matches_from_safe(args);
            assert!(result.is_err());
            assert_eq!(error, result.unwrap_err().kind);
        }

        #[test]
        fn accepts_cli_without_arguments() {
            check_args(&["ludusavi"], Cli { sub: None });
        }

        #[test]
        fn accepts_cli_backup_with_minimal_arguments() {
            check_args(
                &["ludusavi", "backup"],
                Cli {
                    sub: Some(Subcommand::Backup {
                        preview: false,
                        path: None,
                        force: false,
                        merge: false,
                        no_merge: false,
                        update: false,
                        by_steam_id: false,
                        api: false,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_backup_with_all_arguments() {
            check_args(
                &[
                    "ludusavi",
                    "backup",
                    "--preview",
                    "--path",
                    "tests/backup",
                    "--force",
                    "--merge",
                    "--update",
                    "--by-steam-id",
                    "--api",
                    "game1",
                    "game2",
                ],
                Cli {
                    sub: Some(Subcommand::Backup {
                        preview: true,
                        path: Some(StrictPath::new(s("tests/backup"))),
                        force: true,
                        merge: true,
                        no_merge: false,
                        update: true,
                        by_steam_id: true,
                        api: true,
                        games: vec![s("game1"), s("game2")],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_backup_with_nonexistent_path() {
            check_args(
                &["ludusavi", "backup", "--path", "tests/fake"],
                Cli {
                    sub: Some(Subcommand::Backup {
                        preview: false,
                        path: Some(StrictPath::new(s("tests/fake"))),
                        force: false,
                        merge: false,
                        no_merge: false,
                        update: false,
                        by_steam_id: false,
                        api: false,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_backup_with_no_merge() {
            check_args(
                &["ludusavi", "backup", "--no-merge"],
                Cli {
                    sub: Some(Subcommand::Backup {
                        preview: false,
                        path: None,
                        force: false,
                        merge: false,
                        no_merge: true,
                        update: false,
                        by_steam_id: false,
                        api: false,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_restore_with_minimal_arguments() {
            check_args(
                &["ludusavi", "restore"],
                Cli {
                    sub: Some(Subcommand::Restore {
                        preview: false,
                        path: None,
                        force: false,
                        by_steam_id: false,
                        api: false,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_restore_with_all_arguments() {
            check_args(
                &[
                    "ludusavi",
                    "restore",
                    "--preview",
                    "--path",
                    "tests/backup",
                    "--force",
                    "--by-steam-id",
                    "--api",
                    "game1",
                    "game2",
                ],
                Cli {
                    sub: Some(Subcommand::Restore {
                        preview: true,
                        path: Some(StrictPath::new(s("tests/backup"))),
                        force: true,
                        by_steam_id: true,
                        api: true,
                        games: vec![s("game1"), s("game2")],
                    }),
                },
            );
        }

        #[test]
        fn rejects_cli_restore_with_nonexistent_path() {
            check_args_err(
                &["ludusavi", "restore", "--path", "tests/fake"],
                structopt::clap::ErrorKind::ValueValidation,
            );
        }
    }

    mod reporter {
        use super::*;
        use crate::prelude::ScannedFile;
        use maplit::hashset;
        use pretty_assertions::assert_eq;

        fn drive() -> String {
            if cfg!(target_os = "windows") {
                StrictPath::new(s("foo")).render()[..2].to_string()
            } else {
                s("")
            }
        }

        #[test]
        fn can_render_in_standard_mode_with_minimal_input() {
            let mut reporter = Reporter::standard(Translator::default());
            reporter.add_game(
                "foo",
                &ScanInfo::default(),
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &[],
            );
            assert_eq!(
                format!(
                    r#"

Overall:
  Games: 0
  Size: 0 B
  Location: {}/dev/null
                "#,
                    &drive()
                )
                .trim_end(),
                reporter.render(&StrictPath::new(s("/dev/null")))
            )
        }

        #[test]
        fn can_render_in_standard_mode_with_one_game_in_backup_mode() {
            let mut reporter = Reporter::standard(Translator::default());

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(s("/file1")),
                            size: 102_400,
                            original_path: None,
                        },
                        ScannedFile {
                            path: StrictPath::new(s("/file2")),
                            size: 51_200,
                            original_path: None,
                        },
                    },
                    found_registry_keys: hashset! {
                        s("HKEY_CURRENT_USER/Key1"),
                        s("HKEY_CURRENT_USER/Key2")
                    },
                    registry_file: None,
                },
                &BackupInfo {
                    failed_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(s("/file2")),
                            size: 51_200,
                            original_path: None,
                        },
                    },
                    failed_registry: hashset! {
                        s("HKEY_CURRENT_USER/Key1")
                    },
                },
                &OperationStepDecision::Processed,
                &[],
            );
            assert_eq!(
                r#"
foo [100.00 KiB]:
  - <drive>/file1
  - [FAILED] <drive>/file2
  - [FAILED] HKEY_CURRENT_USER/Key1
  - HKEY_CURRENT_USER/Key2

Overall:
  Games: 1 of 1
  Size: 100.00 of 150.00 KiB
  Location: <drive>/dev/null
                "#
                .trim()
                .replace("<drive>", &drive()),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }

        #[test]
        fn can_render_in_standard_mode_with_one_game_in_restore_mode() {
            let mut reporter = Reporter::standard(Translator::default());

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(format!("{}/backup/file1", drive())),
                            size: 102_400,
                            original_path: Some(StrictPath::new(format!("{}/original/file1", drive()))),
                        },
                        ScannedFile {
                            path: StrictPath::new(format!("{}/backup/file2", drive())),
                            size: 51_200,
                            original_path: Some(StrictPath::new(format!("{}/original/file2", drive()))),
                        },
                    },
                    found_registry_keys: hashset! {},
                    registry_file: None,
                },
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &[],
            );
            assert_eq!(
                r#"
foo [150.00 KiB]:
  - <drive>/original/file1
  - <drive>/original/file2

Overall:
  Games: 1
  Size: 150.00 KiB
  Location: <drive>/dev/null
                "#
                .trim()
                .replace("<drive>", &drive()),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }

        #[test]
        fn can_render_in_json_mode_with_minimal_input() {
            let mut reporter = Reporter::json();

            reporter.add_game(
                "foo",
                &ScanInfo::default(),
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &[],
            );
            assert_eq!(
                r#"
{
  "overall": {
    "totalGames": 0,
    "totalBytes": 0,
    "processedGames": 0,
    "processedBytes": 0
  },
  "games": {}
}
                "#
                .trim(),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }

        #[test]
        fn can_render_in_json_mode_with_one_game_in_backup_mode() {
            let mut reporter = Reporter::json();

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(s("/file1")),
                            size: 100,
                            original_path: None,
                        },
                        ScannedFile {
                            path: StrictPath::new(s("/file2")),
                            size: 50,
                            original_path: None,
                        },
                    },
                    found_registry_keys: hashset! {
                        s("HKEY_CURRENT_USER/Key1"),
                        s("HKEY_CURRENT_USER/Key2")
                    },
                    registry_file: None,
                },
                &BackupInfo {
                    failed_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(s("/file2")),
                            size: 50,
                            original_path: None,
                        },
                    },
                    failed_registry: hashset! {
                        s("HKEY_CURRENT_USER/Key1")
                    },
                },
                &OperationStepDecision::Processed,
                &[],
            );
            assert_eq!(
                r#"
{
  "errors": {
    "someGamesFailed": true
  },
  "overall": {
    "totalGames": 1,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 100
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "files": {
        "<drive>/file1": {
          "bytes": 100
        },
        "<drive>/file2": {
          "failed": true,
          "bytes": 50
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Key1": {
          "failed": true
        },
        "HKEY_CURRENT_USER/Key2": {}
      }
    }
  }
}
                "#
                .trim()
                .replace("<drive>", &drive()),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }

        #[test]
        fn can_render_in_json_mode_with_one_game_in_restore_mode() {
            let mut reporter = Reporter::json();

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(format!("{}/backup/file1", drive())),
                            size: 100,
                            original_path: Some(StrictPath::new(format!("{}/original/file1", drive()))),
                        },
                        ScannedFile {
                            path: StrictPath::new(format!("{}/backup/file2", drive())),
                            size: 50,
                            original_path: Some(StrictPath::new(format!("{}/original/file2", drive()))),
                        },
                    },
                    found_registry_keys: hashset! {},
                    registry_file: None,
                },
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &[],
            );
            assert_eq!(
                r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 150
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "files": {
        "<drive>/original/file1": {
          "bytes": 100
        },
        "<drive>/original/file2": {
          "bytes": 50
        }
      },
      "registry": {}
    }
  }
}
                "#
                .trim()
                .replace("<drive>", &drive()),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }
    }
}
