use std::path::PathBuf;

use crate::{
    cloud::WebDavProvider,
    prelude::StrictPath,
    resource::config::{BackupFormat, Sort, SortKey, ZipCompression},
};

macro_rules! possible_values {
    ($t: ty, $options: ident) => {{
        use clap::builder::{PossibleValuesParser, TypedValueParser};
        PossibleValuesParser::new(<$t>::$options).map(|s| s.parse::<$t>().unwrap())
    }};
}

fn parse_strict_path(path: &str) -> Result<StrictPath, std::io::Error> {
    Ok(StrictPath::new(path.to_owned()))
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
    Status,
    StatusReversed,
}

impl CliSort {
    pub const ALL: &'static [&'static str] = &["name", "name-rev", "size", "size-rev", "status", "status-rev"];
}

impl std::str::FromStr for CliSort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "name" => Ok(Self::Name),
            "name-rev" => Ok(Self::NameReversed),
            "size" => Ok(Self::Size),
            "size-rev" => Ok(Self::SizeReversed),
            "status" => Ok(Self::Status),
            "status-rev" => Ok(Self::StatusReversed),
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
            CliSort::Status => Self {
                key: SortKey::Status,
                reversed: false,
            },
            CliSort::StatusReversed => Self {
                key: SortKey::Status,
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
        #[clap(long, value_parser = parse_strict_path)]
        path: Option<StrictPath>,

        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,

        /// DEPRECATED: New backups are now always merged into the target folder.
        /// This option will be removed in a future version.
        #[clap(long)]
        merge: bool,

        /// DEPRECATED: New backups are now always merged into the target folder.
        /// This option will be removed in a future version.
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

        /// Extra Wine/Proton prefix to check for saves. This should be a folder
        /// with an immediate child folder named "drive_c" (or another letter).
        #[clap(long, value_parser = parse_strict_path)]
        wine_prefix: Option<StrictPath>,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[clap(long)]
        api: bool,

        /// Sort the game list by different criteria.
        /// When not specified, this defers to the config file.
        #[clap(long, value_parser = possible_values!(CliSort, ALL))]
        sort: Option<CliSort>,

        /// Format in which to store new backups.
        /// When not specified, this defers to the config file.
        #[clap(long, value_parser = possible_values!(BackupFormat, ALL_NAMES))]
        format: Option<BackupFormat>,

        /// Compression method to use for new zip backups.
        /// When not specified, this defers to the config file.
        #[clap(long, value_parser = possible_values!(ZipCompression, ALL_NAMES))]
        compression: Option<ZipCompression>,

        /// Compression level to use for new zip backups.
        /// When not specified, this defers to the config file.
        /// Valid ranges: 1 to 9 for deflate/bzip2, -7 to 22 for zstd.
        #[clap(long, allow_hyphen_values(true))]
        compression_level: Option<i32>,

        /// Maximum number of full backups to retain per game.
        /// Must be between 1 and 255 (inclusive).
        /// When not specified, this defers to the config file.
        #[clap(long)]
        full_limit: Option<u8>,

        /// Maximum number of differential backups to retain per full backup.
        /// Must be between 0 and 255 (inclusive).
        /// When not specified, this defers to the config file.
        #[clap(long)]
        differential_limit: Option<u8>,

        /// Upload any changes to the cloud when the backup is complete.
        /// If the local and cloud backups are not in sync to begin with,
        /// then nothing will be uploaded.
        /// This has no effect on previews.
        /// When not specified, this defers to the config file.
        #[clap(long)]
        cloud_sync: bool,

        /// Don't perform any cloud checks or synchronization.
        /// When not specified, this defers to the config file.
        #[clap(long, conflicts_with("cloud-sync"))]
        no_cloud_sync: bool,

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
        #[clap(long, value_parser = parse_existing_strict_path)]
        path: Option<StrictPath>,

        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,

        /// Print information to stdout in machine-readable JSON.
        /// This replaces the default, human-readable output.
        #[clap(long)]
        api: bool,

        /// Sort the game list by different criteria.
        /// When not specified, this defers to Ludusavi's config file.
        #[clap(long, value_parser = possible_values!(CliSort, ALL))]
        sort: Option<CliSort>,

        /// Restore a specific backup, using an ID returned by the `backups` command.
        /// This is only valid when restoring a single game.
        #[clap(long)]
        backup: Option<String>,

        /// Warn if the local and cloud backups are out of sync.
        /// The restore will still proceed regardless.
        /// This has no effect on previews.
        /// When not specified, this defers to the config file.
        #[clap(long)]
        cloud_sync: bool,

        /// Don't perform any cloud checks or synchronization.
        /// When not specified, this defers to the config file.
        #[clap(long, conflicts_with("cloud-sync"))]
        no_cloud_sync: bool,

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
        #[clap(long, value_parser = parse_strict_path)]
        path: Option<StrictPath>,

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
        #[clap(long, value_parser = parse_strict_path)]
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

        /// Look up game by a GOG ID.
        #[clap(long)]
        gog_id: Option<u64>,

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
    /// Options for Ludusavi's data set.
    Manifest {
        #[clap(subcommand)]
        sub: Option<ManifestSubcommand>,
    },
    /// Cloud sync.
    Cloud {
        #[clap(subcommand)]
        sub: CloudSubcommand,
    },
}

impl Subcommand {
    pub fn api(&self) -> bool {
        match self {
            Self::Backup { api, .. } => *api,
            Self::Restore { api, .. } => *api,
            Self::Backups { api, .. } => *api,
            Self::Find { api, .. } => *api,
            Self::Manifest {
                sub: Some(ManifestSubcommand::Show { api }),
            } => *api,
            Self::Manifest { .. } => false,
            Self::Complete { .. } => false,
            Self::Cloud { .. } => false,
        }
    }
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum ManifestSubcommand {
    /// Print the content of the manifest, including any custom entries.
    Show {
        /// Print information to stdout in machine-readable JSON.
        #[clap(long)]
        api: bool,
    },
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum CloudSubcommand {
    /// Configure the cloud system to use.
    Set {
        #[clap(subcommand)]
        sub: CloudSetSubcommand,
    },
    /// Upload your local backups to the cloud, overwriting any existing cloud backups.
    Upload {
        /// Local folder path for backups.
        /// When not specified, this defers to the backup path from the config file.
        #[clap(long, value_parser = parse_strict_path)]
        local: Option<StrictPath>,

        /// Cloud folder path for backups.
        /// When not specified, this defers to the config file.
        #[clap(long)]
        cloud: Option<String>,

        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,

        /// Check what would change, but don't actually apply the changes.
        #[clap(long)]
        preview: bool,

        /// Only sync these specific games.
        #[clap()]
        games: Vec<String>,
    },
    /// Download your cloud backups, overwriting any existing local backups.
    Download {
        /// Local folder path for backups.
        /// When not specified, this defers to the backup path from the config file.
        #[clap(long, value_parser = parse_strict_path)]
        local: Option<StrictPath>,

        /// Cloud folder path for backups.
        /// When not specified, this defers to the config file.
        #[clap(long)]
        cloud: Option<String>,

        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,

        /// Check what would change, but don't actually apply the changes.
        #[clap(long)]
        preview: bool,

        /// Only sync these specific games.
        #[clap()]
        games: Vec<String>,
    },
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum CloudSetSubcommand {
    /// Disable cloud backups.
    None,
    /// Use a pre-existing Rclone remote.
    Custom {
        #[clap(long)]
        id: String,
    },
    /// Use Box.
    Box,
    /// Use Dropbox.
    Dropbox,
    /// Use Google Drive.
    #[clap(name = "google-drive")]
    GoogleDrive,
    /// Use OneDrive.
    #[clap(name = "onedrive")]
    OneDrive,
    /// Use an FTP server.
    Ftp {
        /// Host URL.
        #[clap(long)]
        host: String,
        /// Port number.
        #[clap(long, default_value_t = 21)]
        port: i32,
        /// Username for authentication.
        #[clap(long)]
        username: String,
        /// Password for authentication.
        #[clap(long, default_value = "")]
        password: String,
    },
    /// Use an SMB server.
    Smb {
        /// Host URL.
        #[clap(long)]
        host: String,
        /// Port number.
        #[clap(long, default_value_t = 445)]
        port: i32,
        /// Username for authentication.
        #[clap(long)]
        username: String,
        /// Password for authentication.
        #[clap(long, default_value = "")]
        password: String,
    },
    /// Use a WebDAV server.
    #[clap(name = "webdav")]
    WebDav {
        /// URL.
        #[clap(long)]
        url: String,
        /// Username for authentication.
        #[clap(long)]
        username: String,
        /// Password for authentication.
        #[clap(long, default_value = "")]
        password: String,
        /// Service provider.
        #[clap(long, default_value = WebDavProvider::OTHER, value_parser = possible_values!(WebDavProvider, ALL_CLI))]
        provider: WebDavProvider,
    },
}

/// Back up and restore PC game saves
#[derive(clap::Parser, Clone, Debug, PartialEq, Eq)]
#[clap(name = "ludusavi", version, term_width = 79)]
pub struct Cli {
    /// Use configuration found in DIRECTORY
    #[clap(long, value_name = "DIRECTORY")]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub sub: Option<Subcommand>,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::testing::s;

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
        check_args(
            &["ludusavi"],
            Cli {
                config: None,
                sub: None,
            },
        );
    }

    #[test]
    fn accepts_cli_backup_with_minimal_arguments() {
        check_args(
            &["ludusavi", "backup"],
            Cli {
                config: None,
                sub: Some(Subcommand::Backup {
                    preview: false,
                    path: None,
                    force: false,
                    merge: false,
                    no_merge: false,
                    update: false,
                    try_update: false,
                    wine_prefix: None,
                    api: false,
                    sort: None,
                    format: None,
                    compression: None,
                    compression_level: None,
                    full_limit: None,
                    differential_limit: None,
                    cloud_sync: false,
                    no_cloud_sync: false,
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
                "--wine-prefix",
                "tests/wine-prefix",
                "--api",
                "--sort",
                "name",
                "--format",
                "zip",
                "--compression",
                "bzip2",
                "--compression-level",
                "5",
                "--full-limit",
                "1",
                "--differential-limit",
                "2",
                "--cloud-sync",
                "game1",
                "game2",
            ],
            Cli {
                config: None,
                sub: Some(Subcommand::Backup {
                    preview: true,
                    path: Some(StrictPath::new(s("tests/backup"))),
                    force: true,
                    merge: true,
                    no_merge: false,
                    update: true,
                    try_update: false,
                    wine_prefix: Some(StrictPath::new(s("tests/wine-prefix"))),
                    api: true,
                    sort: Some(CliSort::Name),
                    format: Some(BackupFormat::Zip),
                    compression: Some(ZipCompression::Bzip2),
                    compression_level: Some(5),
                    full_limit: Some(1),
                    differential_limit: Some(2),
                    cloud_sync: true,
                    no_cloud_sync: false,
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
                config: None,
                sub: Some(Subcommand::Backup {
                    preview: false,
                    path: Some(StrictPath::new(s("tests/fake"))),
                    force: false,
                    merge: false,
                    no_merge: false,
                    update: false,
                    try_update: false,
                    wine_prefix: None,
                    api: false,
                    sort: None,
                    format: None,
                    compression: None,
                    compression_level: None,
                    full_limit: None,
                    differential_limit: None,
                    cloud_sync: false,
                    no_cloud_sync: false,
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
                config: None,
                sub: Some(Subcommand::Backup {
                    preview: false,
                    path: None,
                    force: false,
                    merge: false,
                    no_merge: true,
                    update: false,
                    try_update: false,
                    wine_prefix: None,
                    api: false,
                    sort: None,
                    format: None,
                    compression: None,
                    compression_level: None,
                    full_limit: None,
                    differential_limit: None,
                    cloud_sync: false,
                    no_cloud_sync: false,
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
                config: None,
                sub: Some(Subcommand::Backup {
                    preview: false,
                    path: None,
                    force: false,
                    merge: false,
                    no_merge: false,
                    update: false,
                    try_update: true,
                    wine_prefix: None,
                    api: false,
                    sort: None,
                    format: None,
                    compression: None,
                    compression_level: None,
                    full_limit: None,
                    differential_limit: None,
                    cloud_sync: false,
                    no_cloud_sync: false,
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
                    config: None,
                    sub: Some(Subcommand::Backup {
                        preview: false,
                        path: None,
                        force: false,
                        merge: false,
                        no_merge: false,
                        update: false,
                        try_update: false,
                        wine_prefix: None,
                        api: false,
                        sort: Some(sort),
                        format: None,
                        compression: None,
                        compression_level: None,
                        full_limit: None,
                        differential_limit: None,
                        cloud_sync: false,
                        no_cloud_sync: false,
                        games: vec![],
                    }),
                },
            );
        }
    }

    #[test]
    fn accepts_cli_backup_with_negative_compression_level() {
        check_args(
            &["ludusavi", "backup", "--compression-level", "-7"],
            Cli {
                config: None,
                sub: Some(Subcommand::Backup {
                    preview: false,
                    path: None,
                    force: false,
                    merge: false,
                    no_merge: false,
                    update: false,
                    try_update: false,
                    wine_prefix: None,
                    api: false,
                    sort: None,
                    format: None,
                    compression: None,
                    compression_level: Some(-7),
                    full_limit: None,
                    differential_limit: None,
                    cloud_sync: false,
                    no_cloud_sync: false,
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
                config: None,
                sub: Some(Subcommand::Restore {
                    preview: false,
                    path: None,
                    force: false,
                    api: false,
                    sort: None,
                    backup: None,
                    cloud_sync: false,
                    no_cloud_sync: false,
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
                "--api",
                "--sort",
                "name",
                "--backup",
                ".",
                "--cloud-sync",
                "game1",
                "game2",
            ],
            Cli {
                config: None,
                sub: Some(Subcommand::Restore {
                    preview: true,
                    path: Some(StrictPath::new(s("tests/backup"))),
                    force: true,
                    api: true,
                    sort: Some(CliSort::Name),
                    backup: Some(s(".")),
                    cloud_sync: true,
                    no_cloud_sync: false,
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
                    config: None,
                    sub: Some(Subcommand::Restore {
                        preview: false,
                        path: None,
                        force: false,
                        api: false,
                        sort: Some(sort),
                        backup: None,
                        cloud_sync: false,
                        no_cloud_sync: false,
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
                config: None,
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
                config: None,
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
                config: None,
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
                config: None,
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
                config: None,
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
                config: None,
                sub: Some(Subcommand::Backups {
                    path: None,
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
                "--api",
                "game1",
                "game2",
            ],
            Cli {
                config: None,
                sub: Some(Subcommand::Backups {
                    path: Some(StrictPath::new(s("tests/backup"))),
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
                config: None,
                sub: Some(Subcommand::Find {
                    api: false,
                    path: None,
                    backup: false,
                    restore: false,
                    steam_id: None,
                    gog_id: None,
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
                "101",
                "--gog-id",
                "102",
                "--normalized",
                "game1",
                "game2",
            ],
            Cli {
                config: None,
                sub: Some(Subcommand::Find {
                    api: true,
                    path: Some(StrictPath::new(s("tests/backup"))),
                    backup: true,
                    restore: true,
                    steam_id: Some(101),
                    gog_id: Some(102),
                    normalized: true,
                    names: vec![s("game1"), s("game2")],
                }),
            },
        );
    }
}
