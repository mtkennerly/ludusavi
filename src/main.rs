#![allow(clippy::too_many_arguments)]

mod cli;
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
    lang::TRANSLATOR,
    prelude::{app_dir, CONFIG_DIR, VERSION},
};

/// The logger must be assigned to a variable because we're using async logging.
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
        .start()
}

fn main() {
    let args = cli::parse();
    if let Some(config_dir) = args.config.as_deref() {
        *CONFIG_DIR.lock().unwrap() = Some(config_dir.to_path_buf());
    }
    match args.sub {
        None => {
            #[cfg(target_os = "windows")]
            {
                if std::env::var("LUDUSAVI_DEBUG").is_err() {
                    // The purpose of this unsafe block is to detach the process from the console
                    // that it starts with. Otherwise, the GUI would be accompanied by a console
                    // window. Unfortunately, it does not seem to be possible to go the other direction
                    // (setting `#![windows_subsystem = "windows"]` and calling `AllocConsole`),
                    // so there's a brief console icon in the task bar, but no visible console window.
                    let code = unsafe { winapi::um::wincon::FreeConsole() };
                    if code == 0 {
                        eprintln!("Unable to detach the console");
                        std::process::exit(1);
                    }
                }
            }

            // We must do this after detaching the console, or else it will still be present, somehow.
            #[allow(unused)]
            let logger = prepare_logging();

            log::debug!("Version: {}", *VERSION);

            gui::run();
        }
        Some(sub) => {
            #[allow(unused)]
            let logger = prepare_logging();

            log::debug!("Version: {}", *VERSION);

            let api = sub.api();
            if let Err(e) = cli::run(sub) {
                if !api {
                    eprintln!("{}", TRANSLATOR.handle_error(&e));
                }
                std::process::exit(1);
            }
        }
    };
}
