#![allow(clippy::too_many_arguments)]

mod cli;
mod cloud;
mod gui;
mod lang;
mod path;
mod prelude;
mod resource;
mod scan;
mod serialization;
mod wrap;

#[cfg(test)]
mod testing;

use crate::{
    gui::Flags,
    lang::TRANSLATOR,
    prelude::{app_dir, CONFIG_DIR, VERSION},
};

/// The logger must be assigned to a variable because we're using async logging.
/// We should also avoid doing this if we're just going to relaunch into detached mode anyway.
/// https://docs.rs/flexi_logger/0.23.1/flexi_logger/error_info/index.html#write
fn prepare_logging() -> Result<flexi_logger::LoggerHandle, flexi_logger::FlexiLoggerError> {
    flexi_logger::Logger::try_with_env_or_str("ludusavi=warn")
        .unwrap()
        .log_to_file(flexi_logger::FileSpec::default().directory(app_dir().as_std_path_buf().unwrap()))
        .write_mode(flexi_logger::WriteMode::Async)
        .rotate(
            flexi_logger::Criterion::Size(1024 * 1024 * 10),
            flexi_logger::Naming::Timestamps,
            flexi_logger::Cleanup::KeepLogFiles(4),
        )
        .use_utc()
        .format_for_files(|w, now, record| {
            write!(
                w,
                "[{}] {} [{}] {}",
                now.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args(),
            )
        })
        .start()
}

/// Detach the current process from its console on Windows.
///
/// ## Testing
/// This has several edge cases and has been the source of multiple bugs.
/// If you change this, be careful and make sure to test this matrix:
///
/// * Arguments:
///   * None (double click in Windows Explorer)
///   * None (from console)
///   * `--help` (has output, but before this function is called)
///   * `backup --preview` (has output, after this function is called)
/// * Consoles:
///   * Command Prompt
///   * PowerShell
///   * Git Bash
/// * Console host for double clicking in Windows Explorer:
///   * Windows Console Host
///   * Windows Terminal
///
/// ## Alternatives
/// We have tried `#![windows_subsystem = "windows"]` plus `AttachConsole`/`AllocConsole`,
/// but that messes up the console output in Command Prompt and PowerShell
/// (a new prompt line is shown, and then the output bleeds into that line).
///
/// We have tried relaunching the program with a special environment variable,
/// but that eventually raised a false positive from Windows Defender (`Win32/Wacapew.C!ml`).
///
/// We may eventually want to try using a manifest to set `<consoleAllocationPolicy>`,
/// but that is not yet widely available:
/// https://github.com/microsoft/terminal/blob/5383cb3a1bb8095e214f7d4da085ea4646db8868/doc/specs/%237335%20-%20Console%20Allocation%20Policy.md
///
/// ## Considerations
/// The current approach is to let the console appear and then immediately `FreeConsole`.
/// Previously, Windows Terminal wouldn't remove the console in that case,
/// but that has been fixed: https://github.com/microsoft/terminal/issues/16174
///
/// There was also an issue where asynchronous Rclone commands would fail to spawn
/// ("The request is not supported (os error 50)"),
/// but that has been solved by resetting the standard device handles:
/// https://github.com/rust-lang/rust/issues/113277
#[cfg(target_os = "windows")]
unsafe fn detach_console() {
    use winapi::um::{
        processenv::SetStdHandle,
        winbase::{STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE},
        wincon::FreeConsole,
    };

    if FreeConsole() == 0 {
        eprintln!("Unable to detach the console");
        std::process::exit(1);
    }
    if SetStdHandle(STD_INPUT_HANDLE, std::ptr::null_mut()) == 0 {
        eprintln!("Unable to reset stdin handle");
        std::process::exit(1);
    }
    if SetStdHandle(STD_OUTPUT_HANDLE, std::ptr::null_mut()) == 0 {
        eprintln!("Unable to reset stdout handle");
        std::process::exit(1);
    }
    if SetStdHandle(STD_ERROR_HANDLE, std::ptr::null_mut()) == 0 {
        eprintln!("Unable to reset stderr handle");
        std::process::exit(1);
    }
}

fn main() {
    let args = cli::parse();
    if let Some(config_dir) = args.config.as_deref() {
        *CONFIG_DIR.lock().unwrap() = Some(config_dir.to_path_buf());
    }
    match args.sub {
        None => {
            #[cfg(target_os = "windows")]
            if std::env::var(crate::prelude::ENV_DEBUG).is_err() {
                unsafe {
                    detach_console();
                }
            }

            // We must do this after detaching the console, or else it will still be present, somehow.
            #[allow(unused)]
            let logger = prepare_logging();

            log::debug!("Version: {}", *VERSION);

            let flags = Flags {
                update_manifest: !args.no_manifest_update,
            };
            gui::run(flags);
        }
        Some(sub) => {
            #[allow(unused)]
            let logger = prepare_logging();

            log::debug!("Version: {}", *VERSION);

            if let Err(e) = cli::run(sub, args.no_manifest_update, args.try_manifest_update) {
                eprintln!("{}", TRANSLATOR.handle_error(&e));
                std::process::exit(1);
            }
        }
    };
}
