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

#[cfg(test)]
mod testing;

use crate::{
    gui::Flags,
    lang::TRANSLATOR,
    prelude::{app_dir, CONFIG_DIR, ENV_DEBUG, ENV_RELAUNCHED, VERSION},
};

/// The logger must be assigned to a variable because we're using async logging.
/// We should also avoid doing this if we're just going to relaunch into detached mode anyway.
/// https://docs.rs/flexi_logger/0.23.1/flexi_logger/error_info/index.html#write
fn prepare_logging() -> Result<flexi_logger::LoggerHandle, flexi_logger::FlexiLoggerError> {
    flexi_logger::Logger::try_with_env_or_str("ludusavi=warn")
        .unwrap()
        .log_to_file(flexi_logger::FileSpec::default().directory(app_dir()))
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

fn has_env(key: &str) -> bool {
    std::env::var(key).is_ok()
}

fn relaunch_detached(args: Vec<String>) -> ! {
    let exe = match std::env::current_exe() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Unable to relaunch in detached mode: {e:?}");
            std::process::exit(1);
        }
    };

    let mut command = std::process::Command::new(exe);
    command.args(args).env(ENV_RELAUNCHED, "1");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
    }

    match command.spawn() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("Unable to relaunch in detached mode: {e:?}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args = cli::parse();
    if let Some(config_dir) = args.config.as_deref() {
        *CONFIG_DIR.lock().unwrap() = Some(config_dir.to_path_buf());
    }
    match args.sub {
        None => {
            if cfg!(target_os = "windows") && !has_env(ENV_DEBUG) && !has_env(ENV_RELAUNCHED) {
                relaunch_detached(args.relaunch_gui_args());
            }

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
