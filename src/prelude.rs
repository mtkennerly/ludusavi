use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, LazyLock, Mutex},
};

use itertools::Itertools;

pub use crate::path::StrictPath;
use crate::{path::CommonPath, resource::manifest::Os};

pub static VERSION: LazyLock<&'static str> =
    LazyLock::new(|| option_env!("LUDUSAVI_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")));
pub static USER_AGENT: LazyLock<String> = LazyLock::new(|| format!("ludusavi/{}", *VERSION));
pub static VARIANT: Option<&'static str> = option_env!("LUDUSAVI_VARIANT");
pub static CANONICAL_VERSION: LazyLock<(u32, u32, u32)> = LazyLock::new(|| {
    let version_parts: Vec<u32> = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|x| x.parse().unwrap_or(0))
        .collect();
    if version_parts.len() != 3 {
        (0, 0, 0)
    } else {
        (version_parts[0], version_parts[1], version_parts[2])
    }
});

pub type AnyError = Box<dyn std::error::Error>;

pub const SKIP: &str = "<skip>";
pub const APP_DIR_NAME: &str = "ludusavi";
#[allow(unused)]
pub const LINUX_APP_ID: &str = "com.mtkennerly.ludusavi";
const PORTABLE_FLAG_FILE_NAME: &str = "ludusavi.portable";
pub const INVALID_FILE_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '\0'];

pub static STEAM_DECK: LazyLock<bool> =
    LazyLock::new(|| Os::HOST == Os::Linux && StrictPath::new("/home/deck".to_string()).exists());
pub static OS_USERNAME: LazyLock<String> = LazyLock::new(whoami::username);

pub static AVAILABLE_PARALELLISM: LazyLock<Option<NonZeroUsize>> =
    LazyLock::new(|| std::thread::available_parallelism().ok());

pub static CONFIG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

#[cfg(feature = "app")]
static HANDLER_SIGINT: Mutex<Option<signal_hook::SigId>> = Mutex::new(None);

pub const ENV_DEBUG: &str = "LUDUSAVI_DEBUG";
const ENV_THREADS: &str = "LUDUSAVI_THREADS";
#[allow(unused)]
pub const ENV_LINUX_APP_ID: &str = "LUDUSAVI_LINUX_APP_ID";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Finality {
    #[default]
    Preview,
    Final,
}

impl Finality {
    pub fn preview(&self) -> bool {
        *self == Self::Preview
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Privacy {
    Public,
    Private,
}

impl Privacy {
    pub fn sensitive(&self) -> bool {
        *self == Self::Private
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncDirection {
    Upload,
    Download,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    ManifestInvalid {
        why: String,
        identifier: Option<String>,
    },
    ManifestCannotBeUpdated {
        identifier: Option<String>,
    },
    ConfigInvalid {
        why: String,
    },
    CliUnrecognizedGames {
        games: Vec<String>,
    },
    CliUnableToRequestConfirmation,
    CliBackupIdWithMultipleGames,
    CliInvalidBackupId,
    NoSaveDataFound,
    GameIsUnrecognized,
    SomeEntriesFailed,
    CannotPrepareBackupTarget {
        path: StrictPath,
    },
    RestorationSourceInvalid {
        path: StrictPath,
    },
    #[cfg_attr(not(target_os = "windows"), allow(unused))]
    RegistryIssue,
    UnableToOpenDir(StrictPath),
    UnableToOpenUrl(String),
    RcloneUnavailable,
    CloudNotConfigured,
    CloudPathInvalid,
    UnableToConfigureCloud(CommandError),
    UnableToSynchronizeCloud(CommandError),
    CloudConflict,
    GameDidNotLaunch {
        why: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    Launched {
        program: String,
        args: Vec<String>,
        raw: String,
    },
    Terminated {
        program: String,
        args: Vec<String>,
    },
    Exited {
        program: String,
        args: Vec<String>,
        code: i32,
        stdout: Option<String>,
        stderr: Option<String>,
    },
}

impl CommandError {
    pub fn command(&self) -> String {
        let (program, args) = match self {
            Self::Launched { program, args, .. } => (program, args),
            Self::Terminated { program, args } => (program, args),
            Self::Exited { program, args, .. } => (program, args),
        };

        let program = quote(program);
        let args = args.iter().map(quote).join(" ");

        format!("{program} {args}")
    }
}

pub fn app_dir() -> StrictPath {
    if let Some(dir) = CONFIG_DIR.lock().unwrap().as_ref() {
        return StrictPath::from(dir.clone());
    }

    if let Ok(mut flag) = std::env::current_exe() {
        flag.pop();
        flag.push(PORTABLE_FLAG_FILE_NAME);
        if flag.exists() {
            flag.pop();
            return StrictPath::from(flag);
        }
    }

    StrictPath::new(format!("{}/{}", CommonPath::Config.get().unwrap(), APP_DIR_NAME))
}

pub fn filter_map_walkdir(context: &str, e: Result<walkdir::DirEntry, walkdir::Error>) -> Option<walkdir::DirEntry> {
    if let Err(e) = &e {
        log::warn!("[{context}] failed to walk: {:?} | {e:?}", e.path());
    }
    e.ok()
}

#[cfg_attr(not(target_os = "windows"), allow(unused))]
pub fn sha1(content: String) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

pub fn get_threads_from_env() -> Option<NonZeroUsize> {
    if let Ok(raw) = std::env::var(ENV_THREADS) {
        if let Ok(threads) = raw.parse::<NonZeroUsize>() {
            log::debug!("Using threads '{}' from {} environment variable", raw, ENV_THREADS);
            Some(threads)
        } else {
            log::warn!(
                "Ignoring invalid threads '{}' from {} environment variable",
                raw,
                ENV_THREADS
            );
            None
        }
    } else {
        None
    }
}

pub fn initialize_rayon(threads: NonZeroUsize) {
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.get())
        .build_global();
}

fn quote(text: impl AsRef<str>) -> String {
    let text = text.as_ref();

    if text.contains(' ') {
        format!("\"{text}\"")
    } else {
        text.to_owned()
    }
}

pub struct CommandOutput {
    #[allow(unused)]
    pub code: i32,
    pub stdout: String,
    #[allow(unused)]
    pub stderr: String,
}

pub fn run_command(
    executable: &str,
    args: &[&str],
    success: &[i32],
    privacy: Privacy,
) -> Result<CommandOutput, CommandError> {
    let mut command = std::process::Command::new(executable);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    command.args(args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    }

    let collect_args = || {
        if privacy.sensitive() {
            vec!["**REDACTED**".to_string()]
        } else {
            args.iter().map(|x| x.to_string()).collect()
        }
    };
    let format_args = || {
        if privacy.sensitive() {
            "**REDACTED**".to_string()
        } else {
            args.iter().map(|x| x.to_string()).join(" ")
        }
    };
    log::debug!("Running command: {} {:?}", executable, collect_args());

    match command.output() {
        Ok(output) => match output.status.code() {
            Some(code) if success.contains(&code) => {
                log::debug!("Command succeeded with {}: {} {}", code, executable, format_args());

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                Ok(CommandOutput { code, stdout, stderr })
            }
            Some(code) => {
                log::error!("Command failed with {}: {} {}", code, executable, format_args());

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                log::error!("Command stdout: {}", stdout);
                log::error!("Command stderr: {}", stderr);

                Err(CommandError::Exited {
                    program: executable.to_string(),
                    args: collect_args(),
                    code,
                    stdout: (!stdout.is_empty()).then_some(stdout),
                    stderr: (!stderr.is_empty()).then_some(stderr),
                })
            }
            None => {
                log::warn!("Command terminated: {} {}", executable, format_args());
                Err(CommandError::Terminated {
                    program: executable.to_string(),
                    args: collect_args(),
                })
            }
        },
        Err(error) => {
            log::warn!("Command did not launch: {} {}", executable, format_args());
            Err(CommandError::Launched {
                program: executable.to_string(),
                args: collect_args(),
                raw: error.to_string(),
            })
        }
    }
}

#[cfg(feature = "app")]
pub fn register_sigint() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));

    let guard = HANDLER_SIGINT.lock();
    if let Ok(mut guard) = guard {
        if let Some(id) = guard.as_ref() {
            signal_hook::low_level::unregister(*id);
            *guard = None;
        }

        let res = signal_hook::flag::register(signal_hook::consts::SIGINT, flag.clone());
        if let Ok(id) = res {
            *guard = Some(id);
        }
    }

    flag
}

#[cfg(feature = "app")]
pub fn unregister_sigint() {
    let guard = HANDLER_SIGINT.lock();
    if let Ok(mut guard) = guard {
        if let Some(id) = guard.as_ref() {
            signal_hook::low_level::unregister(*id);
            *guard = None;
        }

        let res = signal_hook::flag::register_conditional_default(
            signal_hook::consts::SIGINT,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        );
        if let Ok(id) = res {
            *guard = Some(id);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditDirection {
    Up,
    Down,
}

impl EditDirection {
    pub fn shift(&self, index: usize) -> usize {
        match self {
            Self::Up => index - 1,
            Self::Down => index + 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    Add,
    Change(usize, String),
    Remove(usize),
    Move(usize, EditDirection),
}

impl EditAction {
    pub fn move_up(index: usize) -> Self {
        Self::Move(index, EditDirection::Up)
    }

    pub fn move_down(index: usize) -> Self {
        Self::Move(index, EditDirection::Down)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectEditActionField {
    Source,
    Target,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Security {
    /// Enable certificate and hostname validation when performing downloads.
    #[default]
    Safe,
    /// Disable certificate and hostname validation when performing downloads.
    Unsafe,
}

pub fn get_reqwest_client(security: Security) -> reqwest::Client {
    match security {
        Security::Safe => reqwest::Client::new(),
        Security::Unsafe => reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap(),
    }
}

pub fn get_reqwest_blocking_client(security: Security) -> reqwest::blocking::Client {
    match security {
        Security::Safe => reqwest::blocking::Client::new(),
        Security::Unsafe => reqwest::blocking::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap(),
    }
}
