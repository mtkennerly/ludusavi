use crate::{
    cache::Cache,
    config::{Config, Sort, SortKey},
    heroic::HeroicGames,
    lang::Translator,
    layout::BackupLayout,
    manifest::{Manifest, SteamMetadata},
    prelude::{
        app_dir, back_up_game, prepare_backup_target, scan_game_for_backup, scan_game_for_restoration, BackupId,
        BackupInfo, DuplicateDetector, Error, InstallDirRanking, OperationStatus, OperationStepDecision, ScanChange,
        ScanInfo, StrictPath, TitleFinder,
    },
};
use clap::{CommandFactory, Parser};
use indicatif::ParallelProgressIterator;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    prelude::IndexedParallelIterator,
};

fn parse_strict_path(path: &str) -> StrictPath {
    StrictPath::new(path.to_owned())
}

fn parse_existing_strict_path(path: &str) -> Result<StrictPath, std::io::Error> {
    let sp = StrictPath::new(path.to_owned());
    std::fs::canonicalize(sp.interpret())?;
    Ok(sp)
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum CompletionShell {
    #[clap(about = "Completions for Bash")]
    Bash,
    #[clap(about = "Completions for Fish")]
    Fish,
    #[clap(about = "Completions for Zsh")]
    Zsh,
    #[clap(name = "powershell", about = "Completions for PowerShell")]
    PowerShell,
    #[clap(about = "Completions for Elvish")]
    Elvish,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CliSort {
    #[default]
    Name,
    NameReversed,
    Size,
    SizeReversed,
}

impl CliSort {
    pub const ALL: &'static [&'static str] = &["name", "name-rev", "size", "size-rev"];
}

impl std::str::FromStr for CliSort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "name" => Ok(Self::Name),
            "name-rev" => Ok(Self::NameReversed),
            "size" => Ok(Self::Size),
            "size-rev" => Ok(Self::SizeReversed),
            _ => Err(format!("invalid sort key: {}", s)),
        }
    }
}

impl From<CliSort> for Sort {
    fn from(source: CliSort) -> Self {
        match source {
            CliSort::Name => Self {
                key: SortKey::Name,
                reversed: false,
            },
            CliSort::NameReversed => Self {
                key: SortKey::Name,
                reversed: true,
            },
            CliSort::Size => Self {
                key: SortKey::Size,
                reversed: false,
            },
            CliSort::SizeReversed => Self {
                key: SortKey::Size,
                reversed: true,
            },
        }
    }
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Subcommand {
    /// Back up data
    Backup {
        /// List out what would be included, but don't actually perform the operation.
        #[clap(long)]
        preview: bool,

        /// Directory in which to store the backup.
        /// It will be created if it does not already exist.
        /// When not specified, this defers to the config file.
        #[clap(long, parse(from_str = parse_strict_path))]
        path: Option<StrictPath>,

        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,

        /// Merge into existing directory instead of deleting/recreating it.
        /// When not specified, this defers to the config file.
        #[clap(long)]
        merge: bool,

        /// Don't merge; delete and recreate the target directory.
        /// When not specified, this defers to the config file.
        #[clap(long, conflicts_with("merge"))]
        no_merge: bool,

        /// Check for any manifest updates and download if available.
        /// If the check fails, report an error.
        /// Does nothing if the most recent check was within the last 24 hours.
        #[clap(long)]
        update: bool,

        /// Check for any manifest updates and download if available.
        /// If the check fails, continue anyway.
        /// Does nothing if the most recent check was within the last 24 hours.
        #[clap(long, conflicts_with("update"))]
        try_update: bool,

        /// DEPRECATED: Use the `find` command instead.
        /// This option will be removed in a future version.
        ///
        /// When naming specific games to process, this means that you'll
        /// provide the Steam IDs instead of the manifest names, and Ludusavi will
        /// look up those IDs in the manifest to find the corresponding names.
        #[clap(long)]
        by_steam_id: bool,

        /// Extra Wine/Proton prefix to check for saves. This should be a folder
        /// with an immediate child folder named "drive_c" (or another letter).
        #[clap(long, parse(from_str = parse_strict_path))]
        wine_prefix: Option<StrictPath>,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[clap(long)]
        api: bool,

        /// Sort the game list by different criteria.
        /// When not specified, this defers to the config file.
        #[clap(long, possible_values = CliSort::ALL)]
        sort: Option<CliSort>,

        /// Only back up these specific games.
        #[clap()]
        games: Vec<String>,
    },
    /// Restore data
    Restore {
        /// List out what would be included, but don't actually perform the operation.
        #[clap(long)]
        preview: bool,

        /// Directory containing a Ludusavi backup.
        /// When not specified, this defers to the config file.
        #[clap(long, parse(try_from_str = parse_existing_strict_path))]
        path: Option<StrictPath>,

        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,

        /// DEPRECATED: Use the `find` command instead.
        /// This option will be removed in a future version.
        ///
        /// When naming specific games to process, this means that you'll
        /// provide the Steam IDs instead of the manifest names, and Ludusavi will
        /// look up those IDs in the manifest to find the corresponding names.
        #[clap(long)]
        by_steam_id: bool,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[clap(long)]
        api: bool,

        /// Sort the game list by different criteria.
        /// When not specified, this defers to Ludusavi's config file.
        #[clap(long, possible_values = CliSort::ALL)]
        sort: Option<CliSort>,

        /// Restore a specific backup, using an ID returned by the `backups` command.
        /// This is only valid when restoring a single game.
        #[clap(long)]
        backup: Option<String>,

        /// Only restore these specific games.
        #[clap()]
        games: Vec<String>,
    },
    /// Generate shell completion scripts
    Complete {
        #[clap(subcommand)]
        shell: CompletionShell,
    },
    /// Show backups
    Backups {
        /// Directory in which to find backups.
        /// When unset, this defaults to the restore path from the config file.
        #[clap(long, parse(from_str = parse_strict_path))]
        path: Option<StrictPath>,

        /// DEPRECATED: Use the `find` command instead.
        /// This option will be removed in a future version.
        ///
        /// When naming specific games to process, this means that you'll
        /// provide the Steam IDs instead of the manifest names, and Ludusavi will
        /// look up those IDs in the manifest to find the corresponding names.
        #[clap(long)]
        by_steam_id: bool,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[clap(long)]
        api: bool,

        /// Only report these specific games.
        #[clap()]
        games: Vec<String>,
    },
    /// Find game titles
    ///
    /// Precedence: Steam ID -> exact names -> normalized names.
    /// Once a match is found for one of these options,
    /// Ludusavi will stop looking and return that match.
    ///
    /// If there are no matches, Ludusavi will exit with an error.
    /// Depending on the options chosen, there may be multiple matches, but the default is a single match.
    Find {
        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[clap(long)]
        api: bool,

        /// Directory in which to find backups.
        /// When unset, this defaults to the restore path from the config file.
        #[clap(long, parse(from_str = parse_strict_path))]
        path: Option<StrictPath>,

        /// Ensure the game is recognized in a backup context.
        #[clap(long)]
        backup: bool,

        /// Ensure the game is recognized in a restore context.
        #[clap(long)]
        restore: bool,

        /// Look up game by a Steam ID.
        #[clap(long)]
        steam_id: Option<u32>,

        /// Look up game by an approximation of the title.
        /// Ignores capitalization, "edition" suffixes, year suffixes, and some special symbols.
        /// This may find multiple games for a single input.
        #[clap(long)]
        normalized: bool,

        /// Look up game by an exact title.
        /// With multiple values, they will be checked in the order given.
        #[clap()]
        names: Vec<String>,
    },
}

impl Subcommand {
    pub fn api(&self) -> bool {
        match self {
            Self::Backup { api, .. } => *api,
            Self::Restore { api, .. } => *api,
            Self::Backups { api, .. } => *api,
            Self::Find { api, .. } => *api,
            Self::Complete { .. } => false,
        }
    }
}

#[derive(clap::Parser, Clone, Debug, PartialEq, Eq)]
#[clap(
    name = "ludusavi",
    version,
    about = "Back up and restore PC game saves",
    set_term_width = 79
)]
pub struct Cli {
    #[clap(subcommand)]
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
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    ignored: bool,
    change: ScanChange,
    bytes: u64,
    #[serde(rename = "originalPath", skip_serializing_if = "Option::is_none")]
    original_path: Option<String>,
    #[serde(rename = "redirectedPath", skip_serializing_if = "Option::is_none")]
    redirected_path: Option<String>,
    #[serde(
        rename = "duplicatedBy",
        serialize_with = "crate::serialization::ordered_set",
        skip_serializing_if = "crate::serialization::is_empty_set"
    )]
    duplicated_by: std::collections::HashSet<String>,
}

#[derive(Debug, Default, serde::Serialize)]
struct ApiRegistry {
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    failed: bool,
    #[serde(skip_serializing_if = "crate::serialization::is_false")]
    ignored: bool,
    change: ScanChange,
    #[serde(
        rename = "duplicatedBy",
        serialize_with = "crate::serialization::ordered_set",
        skip_serializing_if = "crate::serialization::is_empty_set"
    )]
    duplicated_by: std::collections::HashSet<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
enum ApiGame {
    Operative {
        decision: OperationStepDecision,
        change: ScanChange,
        #[serde(serialize_with = "crate::serialization::ordered_map")]
        files: std::collections::HashMap<String, ApiFile>,
        #[serde(serialize_with = "crate::serialization::ordered_map")]
        registry: std::collections::HashMap<String, ApiRegistry>,
    },
    Stored {
        backups: Vec<ApiBackup>,
    },
    Found {},
}

#[derive(Debug, serde::Serialize)]
struct ApiBackup {
    name: String,
    when: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default, serde::Serialize)]
struct JsonOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<ApiErrors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overall: Option<OperationStatus>,
    #[serde(serialize_with = "crate::serialization::ordered_map")]
    games: std::collections::HashMap<String, ApiGame>,
}

#[derive(Debug)]
enum Reporter {
    Standard {
        translator: Translator,
        parts: Vec<String>,
        status: Option<OperationStatus>,
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
            status: Some(Default::default()),
        }
    }

    fn json() -> Self {
        Self::Json {
            output: JsonOutput {
                errors: Default::default(),
                overall: Some(Default::default()),
                games: Default::default(),
            },
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

    fn suppress_overall(&mut self) {
        match self {
            Self::Standard { status, .. } => {
                *status = None;
            }
            Self::Json { output, .. } => {
                output.overall = None;
            }
        }
    }

    fn add_game(
        &mut self,
        name: &str,
        scan_info: &ScanInfo,
        backup_info: &BackupInfo,
        decision: &OperationStepDecision,
        duplicate_detector: &DuplicateDetector,
    ) -> bool {
        let mut successful = true;
        let restoring = scan_info.restoring();

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
                    name,
                    scan_info.sum_bytes(&Some(backup_info.to_owned())),
                    decision,
                    duplicate_detector.is_game_duplicated(scan_info),
                    &scan_info.count_changes(),
                ));
                for entry in itertools::sorted(&scan_info.found_files) {
                    let entry_successful = !backup_info.failed_files.contains(entry);
                    if !entry_successful {
                        successful = false;
                    }
                    parts.push(translator.cli_game_line_item(
                        &entry.readable(restoring),
                        entry_successful,
                        entry.ignored,
                        duplicate_detector.is_file_duplicated(entry),
                        entry.change,
                    ));

                    if let Some(alt) = entry.alt_readable(restoring) {
                        if restoring {
                            parts.push(translator.cli_game_line_item_redirected(&alt));
                        } else {
                            parts.push(translator.cli_game_line_item_redirecting(&alt));
                        }
                    }
                }
                for entry in itertools::sorted(&scan_info.found_registry_keys) {
                    let entry_successful = !backup_info.failed_registry.contains(&entry.path);
                    if !entry_successful {
                        successful = false;
                    }
                    parts.push(translator.cli_game_line_item(
                        &entry.path.render(),
                        entry_successful,
                        entry.ignored,
                        duplicate_detector.is_registry_duplicated(&entry.path),
                        entry.change,
                    ));
                }

                // Blank line between games.
                parts.push("".to_string());

                if let Some(status) = status.as_mut() {
                    status.add_game(
                        scan_info,
                        &Some(backup_info.clone()),
                        decision == &OperationStepDecision::Processed,
                    );
                }
            }
            Self::Json { output } => {
                if !scan_info.found_anything() {
                    return true;
                }

                let decision = decision.clone();
                let mut files = std::collections::HashMap::new();
                let mut registry = std::collections::HashMap::new();

                for entry in itertools::sorted(&scan_info.found_files) {
                    let mut api_file = ApiFile {
                        bytes: entry.size,
                        failed: backup_info.failed_files.contains(entry),
                        ignored: entry.ignored,
                        change: entry.change,
                        ..Default::default()
                    };
                    if duplicate_detector.is_file_duplicated(entry) {
                        let mut duplicated_by = duplicate_detector.file(entry);
                        duplicated_by.remove(&scan_info.game_name);
                        api_file.duplicated_by = duplicated_by;
                    }

                    if let Some(alt) = entry.alt_readable(restoring) {
                        if restoring {
                            api_file.original_path = Some(alt);
                        } else {
                            api_file.redirected_path = Some(alt);
                        }
                    }
                    if api_file.failed {
                        successful = false;
                    }

                    files.insert(entry.readable(restoring), api_file);
                }
                for entry in itertools::sorted(&scan_info.found_registry_keys) {
                    let mut api_registry = ApiRegistry {
                        failed: backup_info.failed_registry.contains(&entry.path),
                        ignored: entry.ignored,
                        change: entry.change,
                        ..Default::default()
                    };
                    if duplicate_detector.is_registry_duplicated(&entry.path) {
                        let mut duplicated_by = duplicate_detector.registry(&entry.path);
                        duplicated_by.remove(&scan_info.game_name);
                        api_registry.duplicated_by = duplicated_by;
                    }

                    if api_registry.failed {
                        successful = false;
                    }

                    registry.insert(entry.path.render(), api_registry);
                }

                if let Some(overall) = output.overall.as_mut() {
                    overall.add_game(
                        scan_info,
                        &Some(backup_info.clone()),
                        decision == OperationStepDecision::Processed,
                    );
                }
                output.games.insert(
                    name.to_string(),
                    ApiGame::Operative {
                        decision,
                        change: scan_info.count_changes().overall(),
                        files,
                        registry,
                    },
                );
            }
        }

        if !successful {
            self.trip_some_games_failed();
        }
        successful
    }

    fn add_backup(&mut self, name: &str, scan_info: &ScanInfo) {
        match self {
            Self::Standard { parts, .. } => {
                if scan_info.available_backups.is_empty() {
                    return;
                }

                parts.push(format!("{}:", name));
                for backup in &scan_info.available_backups {
                    parts.push(format!(
                        "  - {} ({})",
                        backup.name(),
                        backup.when_local().format("%Y-%m-%dT%H:%M:%S")
                    ));
                }

                // Blank line between games.
                parts.push("".to_string());
            }
            Self::Json { output } => {
                if scan_info.available_backups.is_empty() {
                    return;
                }

                let mut backups = vec![];
                for backup in &scan_info.available_backups {
                    backups.push(ApiBackup {
                        name: backup.name().to_string(),
                        when: *backup.when(),
                    });
                }

                output.games.insert(name.to_string(), ApiGame::Stored { backups });
            }
        }
    }

    fn add_found_titles(&mut self, names: &std::collections::HashSet<String>) {
        match self {
            Self::Standard { parts, .. } => {
                for name in names {
                    parts.push(name.to_owned());
                }
            }
            Self::Json { output } => {
                for name in names {
                    output.games.insert(name.to_owned(), ApiGame::Found {});
                }
            }
        }
    }

    fn render(&self, path: &StrictPath) -> String {
        match self {
            Self::Standard {
                parts,
                status,
                translator,
            } => match status {
                Some(status) => parts.join("\n") + "\n" + &translator.cli_summary(status, path),
                None => parts.join("\n"),
            },
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
        println!("{}", self.render(path));
    }
}

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

pub fn run_cli(sub: Subcommand) -> Result<(), Error> {
    let translator = Translator::default();
    let mut config = Config::load()?;
    translator.set_language(config.language);
    let mut cache = Cache::load().migrated(&mut config);
    let mut failed = false;
    let mut duplicate_detector = DuplicateDetector::default();

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
            games,
        } => {
            warn_deprecations(by_steam_id);

            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };

            let manifest = if try_update {
                match Manifest::load(&config, &mut cache, true) {
                    Ok(x) => x,
                    Err(e) => {
                        eprintln!("{}", translator.handle_error(&e));
                        match Manifest::load(&config, &mut cache, false) {
                            Ok(y) => y,
                            Err(_) => Manifest::default(),
                        }
                    }
                }
            } else {
                Manifest::load(&config, &mut cache, update)?
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
                return Err(crate::prelude::Error::CliUnrecognizedGames {
                    games: subjects.invalid,
                });
            }

            log::info!("beginning backup with {} steps", subjects.valid.len());

            let heroic_games = HeroicGames::scan(&roots, &all_games, None);
            let layout = BackupLayout::new(backup_dir.clone(), config.backup.retention.clone());
            let filter = config.backup.filter.clone();
            let ranking = InstallDirRanking::scan(&roots, &all_games, &subjects.valid);
            let toggled_paths = config.backup.toggled_paths.clone();
            let toggled_registry = config.backup.toggled_registry.clone();

            let mut info: Vec<_> = subjects
                .valid
                .par_iter()
                .enumerate()
                .progress_count(subjects.valid.len() as u64)
                .map(|(i, name)| {
                    log::trace!("step {i} / {}: {name}", subjects.valid.len());
                    let game = &all_games.0[name];
                    let steam_id = &game.steam.clone().unwrap_or(SteamMetadata { id: None }).id;

                    let previous = layout.latest_backup(name, false, &config.redirects);

                    let scan_info = scan_game_for_backup(
                        game,
                        name,
                        &roots,
                        &StrictPath::from_std_path_buf(&app_dir()),
                        &heroic_games,
                        steam_id,
                        &filter,
                        &wine_prefix,
                        &ranking,
                        &toggled_paths,
                        &toggled_registry,
                        previous,
                        &config.redirects,
                    );
                    let ignored = !&config.is_game_enabled_for_backup(name) && !games_specified;
                    let decision = if ignored {
                        OperationStepDecision::Ignored
                    } else {
                        OperationStepDecision::Processed
                    };
                    let backup_info = if preview || ignored {
                        crate::prelude::BackupInfo::default()
                    } else {
                        back_up_game(
                            &scan_info,
                            layout.game_layout(name),
                            config.backup.merge,
                            &chrono::Utc::now(),
                            &config.backup.format,
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
                SortKey::Name => info.sort_by_key(|(name, _, _, _)| name.to_string()),
                SortKey::Size => info.sort_by_key(|(name, scan_info, backup_info, _)| {
                    (scan_info.sum_bytes(&Some(backup_info.clone())), name.to_string())
                }),
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

            let manifest = Manifest::load(&config, &mut cache, false)?;

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
                return Err(crate::prelude::Error::CliUnrecognizedGames {
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
                        crate::prelude::BackupInfo::default()
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
                SortKey::Name => info.sort_by_key(|(name, _, _, _, _)| name.to_string()),
                SortKey::Size => info.sort_by_key(|(name, scan_info, backup_info, _, _)| {
                    (scan_info.sum_bytes(&Some(backup_info.clone())), name.to_string())
                }),
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

            let manifest = Manifest::load(&config, &mut cache, false)?;

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
                return Err(crate::prelude::Error::CliUnrecognizedGames {
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
            normalized,
            names,
        } => {
            let mut reporter = if api {
                Reporter::json()
            } else {
                Reporter::standard(translator)
            };
            reporter.suppress_overall();

            let mut manifest = match Manifest::load(&config, &mut cache, true) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{}", translator.handle_error(&e));
                    match Manifest::load(&config, &mut cache, false) {
                        Ok(y) => y,
                        Err(_) => Manifest::default(),
                    }
                }
            };
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
            let found = title_finder.find(&names, &steam_id, normalized, backup, restore);
            reporter.add_found_titles(&found);

            if found.is_empty() {
                let mut invalid = names;
                if let Some(steam_id) = steam_id {
                    invalid.push(steam_id.to_string());
                }
                reporter.trip_unknown_games(invalid.clone());
                reporter.print_failure();
                return Err(crate::prelude::Error::CliUnrecognizedGames { games: invalid });
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

        fn check_args_err(args: &[&str], error: clap::ErrorKind) {
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
                        try_update: false,
                        by_steam_id: false,
                        wine_prefix: None,
                        api: false,
                        sort: None,
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
                    "--wine-prefix",
                    "tests/wine-prefix",
                    "--api",
                    "--sort",
                    "name",
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
                        try_update: false,
                        by_steam_id: true,
                        wine_prefix: Some(StrictPath::new(s("tests/wine-prefix"))),
                        api: true,
                        sort: Some(CliSort::Name),
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
                        try_update: false,
                        by_steam_id: false,
                        wine_prefix: None,
                        api: false,
                        sort: None,
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
                        try_update: false,
                        by_steam_id: false,
                        wine_prefix: None,
                        api: false,
                        sort: None,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_backup_with_try_update() {
            check_args(
                &["ludusavi", "backup", "--try-update"],
                Cli {
                    sub: Some(Subcommand::Backup {
                        preview: false,
                        path: None,
                        force: false,
                        merge: false,
                        no_merge: false,
                        update: false,
                        try_update: true,
                        by_steam_id: false,
                        wine_prefix: None,
                        api: false,
                        sort: None,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn rejects_cli_backup_with_update_and_try_update() {
            check_args_err(
                &["ludusavi", "backup", "--update", "--try-update"],
                clap::ErrorKind::ArgumentConflict,
            );
        }

        #[test]
        fn accepts_cli_backup_with_sort_variants() {
            let cases = [
                ("name", CliSort::Name),
                ("name-rev", CliSort::NameReversed),
                ("size", CliSort::Size),
                ("size-rev", CliSort::SizeReversed),
            ];

            for (value, sort) in cases {
                check_args(
                    &["ludusavi", "backup", "--sort", value],
                    Cli {
                        sub: Some(Subcommand::Backup {
                            preview: false,
                            path: None,
                            force: false,
                            merge: false,
                            no_merge: false,
                            update: false,
                            try_update: false,
                            by_steam_id: false,
                            wine_prefix: None,
                            api: false,
                            sort: Some(sort),
                            games: vec![],
                        }),
                    },
                );
            }
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
                        sort: None,
                        backup: None,
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
                    "--sort",
                    "name",
                    "--backup",
                    ".",
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
                        sort: Some(CliSort::Name),
                        backup: Some(s(".")),
                        games: vec![s("game1"), s("game2")],
                    }),
                },
            );
        }

        #[test]
        fn rejects_cli_restore_with_nonexistent_path() {
            check_args_err(
                &["ludusavi", "restore", "--path", "tests/fake"],
                clap::ErrorKind::ValueValidation,
            );
        }

        #[test]
        fn accepts_cli_restore_with_sort_variants() {
            let cases = [
                ("name", CliSort::Name),
                ("name-rev", CliSort::NameReversed),
                ("size", CliSort::Size),
                ("size-rev", CliSort::SizeReversed),
            ];

            for (value, sort) in cases {
                check_args(
                    &["ludusavi", "restore", "--sort", value],
                    Cli {
                        sub: Some(Subcommand::Restore {
                            preview: false,
                            path: None,
                            force: false,
                            by_steam_id: false,
                            api: false,
                            sort: Some(sort),
                            backup: None,
                            games: vec![],
                        }),
                    },
                );
            }
        }

        #[test]
        fn accepts_cli_complete_for_bash() {
            check_args(
                &["ludusavi", "complete", "bash"],
                Cli {
                    sub: Some(Subcommand::Complete {
                        shell: CompletionShell::Bash,
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_complete_for_fish() {
            check_args(
                &["ludusavi", "complete", "fish"],
                Cli {
                    sub: Some(Subcommand::Complete {
                        shell: CompletionShell::Fish,
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_complete_for_zsh() {
            check_args(
                &["ludusavi", "complete", "zsh"],
                Cli {
                    sub: Some(Subcommand::Complete {
                        shell: CompletionShell::Zsh,
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_complete_for_powershell() {
            check_args(
                &["ludusavi", "complete", "powershell"],
                Cli {
                    sub: Some(Subcommand::Complete {
                        shell: CompletionShell::PowerShell,
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_complete_for_elvish() {
            check_args(
                &["ludusavi", "complete", "elvish"],
                Cli {
                    sub: Some(Subcommand::Complete {
                        shell: CompletionShell::Elvish,
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_backups_with_minimal_arguments() {
            check_args(
                &["ludusavi", "backups"],
                Cli {
                    sub: Some(Subcommand::Backups {
                        path: None,
                        by_steam_id: false,
                        api: false,
                        games: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_backups_with_all_arguments() {
            check_args(
                &[
                    "ludusavi",
                    "backups",
                    "--path",
                    "tests/backup",
                    "--by-steam-id",
                    "--api",
                    "game1",
                    "game2",
                ],
                Cli {
                    sub: Some(Subcommand::Backups {
                        path: Some(StrictPath::new(s("tests/backup"))),
                        by_steam_id: true,
                        api: true,
                        games: vec![s("game1"), s("game2")],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_find_with_minimal_arguments() {
            check_args(
                &["ludusavi", "find"],
                Cli {
                    sub: Some(Subcommand::Find {
                        api: false,
                        path: None,
                        backup: false,
                        restore: false,
                        steam_id: None,
                        normalized: false,
                        names: vec![],
                    }),
                },
            );
        }

        #[test]
        fn accepts_cli_find_with_all_arguments() {
            check_args(
                &[
                    "ludusavi",
                    "find",
                    "--api",
                    "--path",
                    "tests/backup",
                    "--backup",
                    "--restore",
                    "--steam-id",
                    "123",
                    "--normalized",
                    "game1",
                    "game2",
                ],
                Cli {
                    sub: Some(Subcommand::Find {
                        api: true,
                        path: Some(StrictPath::new(s("tests/backup"))),
                        backup: true,
                        restore: true,
                        steam_id: Some(123),
                        normalized: true,
                        names: vec![s("game1"), s("game2")],
                    }),
                },
            );
        }
    }

    mod reporter {
        use super::*;
        use crate::prelude::{RegistryItem, ScannedFile, ScannedRegistry};
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
                &DuplicateDetector::default(),
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
                            hash: "1".to_string(),
                            original_path: None,
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                        ScannedFile {
                            path: StrictPath::new(s("/file2")),
                            size: 51_200,
                            hash: "2".to_string(),
                            original_path: None,
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                    },
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key2"),
                    },
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {
                        ScannedFile::new("/file2", 51_200, "2"),
                    },
                    failed_registry: hashset! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Key1"))
                    },
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            assert_eq!(
                r#"
foo [100.00 KiB]:
  - <drive>/file1
  - [FAILED] <drive>/file2
  - [FAILED] HKEY_CURRENT_USER/Key1
  - HKEY_CURRENT_USER/Key2

Overall:
  Games: 1
  Size: 100.00 KiB / 150.00 KiB
  Location: <drive>/dev/null
                "#
                .trim()
                .replace("<drive>", &drive()),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }

        #[test]
        fn can_render_in_standard_mode_with_multiple_games_in_backup_mode() {
            let mut reporter = Reporter::standard(Translator::default());

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(s("/file1")),
                            size: 1,
                            hash: "1".to_string(),
                            original_path: None,
                            ignored: false,
                            change: ScanChange::Same,
                            container: None,
                            redirected: None,
                        },
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {},
                    failed_registry: hashset! {},
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            reporter.add_game(
                "bar",
                &ScanInfo {
                    game_name: s("bar"),
                    found_files: hashset! {
                        ScannedFile {
                            path: StrictPath::new(s("/file2")),
                            size: 3,
                            hash: "2".to_string(),
                            original_path: None,
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {},
                    failed_registry: hashset! {},
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            assert_eq!(
                r#"
foo [1 B]:
  - <drive>/file1

bar [3 B]:
  - <drive>/file2

Overall:
  Games: 2
  Size: 4 B
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
                            hash: "1".to_string(),
                            original_path: Some(StrictPath::new(format!("{}/original/file1", drive()))),
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                        ScannedFile {
                            path: StrictPath::new(format!("{}/backup/file2", drive())),
                            size: 51_200,
                            hash: "2".to_string(),
                            original_path: Some(StrictPath::new(format!("{}/original/file2", drive()))),
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
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
        fn can_render_in_standard_mode_with_duplicated_entries() {
            let mut reporter = Reporter::standard(Translator::default());

            let mut duplicate_detector = DuplicateDetector::default();
            for name in &["foo", "bar"] {
                duplicate_detector.add_game(&ScanInfo {
                    game_name: s(name),
                    found_files: hashset! {
                        ScannedFile::new("/file1", 102_400, "1"),
                    },
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                    },
                    ..Default::default()
                });
            }

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile::new("/file1", 102_400, "1"),
                    },
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                    },
                    ..Default::default()
                },
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &duplicate_detector,
            );
            assert_eq!(
                r#"
foo [100.00 KiB] [DUPLICATES]:
  - [DUPLICATED] <drive>/file1
  - [DUPLICATED] HKEY_CURRENT_USER/Key1

Overall:
  Games: 1
  Size: 100.00 KiB
  Location: <drive>/dev/null
                "#
                .trim()
                .replace("<drive>", &drive()),
                reporter.render(&StrictPath::new(s("/dev/null")))
            );
        }

        #[test]
        fn can_render_in_standard_mode_with_different_file_changes() {
            let mut reporter = Reporter::standard(Translator::default());

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile::new(s("/new"), 1, "1".to_string()).change(ScanChange::New),
                        ScannedFile::new(s("/different"), 1, "1".to_string()).change(ScanChange::Different),
                        ScannedFile::new(s("/same"), 1, "1".to_string()).change(ScanChange::Same),
                        ScannedFile::new(s("/unknown"), 1, "1".to_string()).change(ScanChange::Unknown),
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {},
                    failed_registry: hashset! {},
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            reporter.add_game(
                "bar",
                &ScanInfo {
                    game_name: s("bar"),
                    found_files: hashset! {
                        ScannedFile::new(s("/brand-new"), 1, "1".to_string()).change(ScanChange::New),
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {},
                    failed_registry: hashset! {},
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            assert_eq!(
                r#"
foo [4 B] [Δ]:
  - [Δ] <drive>/different
  - [+] <drive>/new
  - <drive>/same
  - <drive>/unknown

bar [1 B] [+]:
  - [+] <drive>/brand-new

Overall:
  Games: 2 [+1] [Δ1]
  Size: 5 B
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
                &DuplicateDetector::default(),
            );
            assert_eq!(
                r#"
{
  "overall": {
    "totalGames": 0,
    "totalBytes": 0,
    "processedGames": 0,
    "processedBytes": 0,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 0
    }
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
                        ScannedFile::new("/file1", 100, "1"),
                        ScannedFile::new("/file2", 50, "2"),
                    },
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key2")
                    },
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {
                        ScannedFile::new("/file2", 50, "2"),
                    },
                    failed_registry: hashset! {
                        RegistryItem::new(s("HKEY_CURRENT_USER/Key1"))
                    },
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
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
    "processedBytes": 100,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {
        "<drive>/file1": {
          "change": "Unknown",
          "bytes": 100
        },
        "<drive>/file2": {
          "failed": true,
          "change": "Unknown",
          "bytes": 50
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Key1": {
          "failed": true,
          "change": "Unknown"
        },
        "HKEY_CURRENT_USER/Key2": {
          "change": "Unknown"
        }
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
                            hash: "1".to_string(),
                            original_path: Some(StrictPath::new(format!("{}/original/file1", drive()))),
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                        ScannedFile {
                            path: StrictPath::new(format!("{}/backup/file2", drive())),
                            size: 50,
                            hash: "2".to_string(),
                            original_path: Some(StrictPath::new(format!("{}/original/file2", drive()))),
                            ignored: false,
                            change: Default::default(),
                            container: None,
                            redirected: None,
                        },
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            assert_eq!(
                r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 150,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {
        "<drive>/original/file1": {
          "change": "Unknown",
          "bytes": 100
        },
        "<drive>/original/file2": {
          "change": "Unknown",
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

        #[test]
        fn can_render_in_json_mode_with_duplicated_entries() {
            let mut reporter = Reporter::json();

            let mut duplicate_detector = DuplicateDetector::default();
            for name in &["foo", "bar"] {
                duplicate_detector.add_game(&ScanInfo {
                    game_name: s(name),
                    found_files: hashset! {
                        ScannedFile::new("/file1", 102_400, "1"),
                    },
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                    },
                    ..Default::default()
                });
            }

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile::new("/file1", 100, "2"),
                    },
                    found_registry_keys: hashset! {
                        ScannedRegistry::new("HKEY_CURRENT_USER/Key1"),
                    },
                    ..Default::default()
                },
                &BackupInfo::default(),
                &OperationStepDecision::Processed,
                &duplicate_detector,
            );
            assert_eq!(
                r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 100,
    "processedGames": 1,
    "processedBytes": 100,
    "changedGames": {
      "new": 0,
      "different": 0,
      "same": 1
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Same",
      "files": {
        "<drive>/file1": {
          "change": "Unknown",
          "bytes": 100,
          "duplicatedBy": [
            "bar"
          ]
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Key1": {
          "change": "Unknown",
          "duplicatedBy": [
            "bar"
          ]
        }
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
        fn can_render_in_json_mode_with_different_file_changes() {
            let mut reporter = Reporter::json();

            reporter.add_game(
                "foo",
                &ScanInfo {
                    game_name: s("foo"),
                    found_files: hashset! {
                        ScannedFile::new("/new", 1, "1").change(ScanChange::New),
                        ScannedFile::new("/different", 1, "2").change(ScanChange::Different),
                        ScannedFile::new("/same", 1, "2").change(ScanChange::Same),
                        ScannedFile::new("/unknown", 1, "2").change(ScanChange::Unknown),
                    },
                    found_registry_keys: hashset! {},
                    ..Default::default()
                },
                &BackupInfo {
                    failed_files: hashset! {},
                    failed_registry: hashset! {},
                },
                &OperationStepDecision::Processed,
                &DuplicateDetector::default(),
            );
            assert_eq!(
                r#"
{
  "overall": {
    "totalGames": 1,
    "totalBytes": 4,
    "processedGames": 1,
    "processedBytes": 4,
    "changedGames": {
      "new": 0,
      "different": 1,
      "same": 0
    }
  },
  "games": {
    "foo": {
      "decision": "Processed",
      "change": "Different",
      "files": {
        "<drive>/different": {
          "change": "Different",
          "bytes": 1
        },
        "<drive>/new": {
          "change": "New",
          "bytes": 1
        },
        "<drive>/same": {
          "change": "Same",
          "bytes": 1
        },
        "<drive>/unknown": {
          "change": "Unknown",
          "bytes": 1
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
