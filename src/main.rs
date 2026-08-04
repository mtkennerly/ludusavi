mod cli;

#[cfg(test)]
mod testing;

use ludusavi::{
    cloud,
    lang::{self, TRANSLATOR},
    metadata, path,
    prelude::{self, CONFIG_DIR, VERSION, app_dir},
    report, resource, scan, sync, wrap,
};

/// The logger handle must be retained until the application closes.
/// https://docs.rs/flexi_logger/0.23.1/flexi_logger/error_info/index.html#write
fn prepare_logging(debug: bool) -> Result<flexi_logger::LoggerHandle, flexi_logger::FlexiLoggerError> {
    if debug {
        flexi_logger::Logger::try_with_str("ludusavi=trace")
            .unwrap()
            .log_to_file(
                flexi_logger::FileSpec::default()
                    .directory(app_dir().as_std_path_buf().unwrap())
                    .basename("ludusavi_debug")
                    .suppress_timestamp(),
            )
            .write_mode(flexi_logger::WriteMode::BufferAndFlush)
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
    } else {
        flexi_logger::Logger::try_with_env_or_str("ludusavi=warn")
            .unwrap()
            .log_to_file(flexi_logger::FileSpec::default().directory(app_dir().as_std_path_buf().unwrap()))
            .write_mode(flexi_logger::WriteMode::BufferAndFlush)
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
}

/// Based on: https://github.com/Traverse-Research/panic-log/blob/874a61b24a8bc8f9b07f9c26dc10b13cbc2622f9/src/lib.rs#L26
/// Modified to flush a provided log handle.
fn prepare_panic_hook(handle: Option<flexi_logger::LoggerHandle>) {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread_name = std::thread::current().name().unwrap_or("<unnamed thread>").to_owned();

        let location = if let Some(panic_location) = info.location() {
            format!(
                "{}:{}:{}",
                panic_location.file(),
                panic_location.line(),
                panic_location.column()
            )
        } else {
            "<unknown location>".to_owned()
        };
        let message = info.payload().downcast_ref::<&str>().unwrap_or(&"");

        let backtrace = std::backtrace::Backtrace::force_capture();

        log::error!("thread '{thread_name}' panicked at {location}:\n{message}\nstack backtrace:\n{backtrace}");

        if let Some(handle) = handle.clone() {
            handle.flush();
        }

        original_hook(info);
    }));
}

fn main() {
    let mut failed = false;
    let args = cli::parse();

    if let Some(config_dir) = args.as_ref().ok().and_then(|args| args.config.as_ref()) {
        *CONFIG_DIR.lock().unwrap() = Some(config_dir.clone());
    }
    let debug = args.as_ref().map(|x| x.debug).unwrap_or_default();

    let logger = prepare_logging(debug);
    #[allow(clippy::useless_asref)]
    prepare_panic_hook(logger.as_ref().map(|x| x.clone()).ok());
    let flush_logger = || {
        if let Ok(logger) = &logger {
            logger.flush();
        }
    };

    log::debug!("Version: {}", *VERSION);
    log::debug!("Invocation: {:?}", std::env::args());

    let args = match args {
        Ok(x) => x,
        Err(e) => {
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {}
                _ => {
                    log::error!("CLI failed to parse: {e}");
                }
            }
            flush_logger();
            debug_on_exit(debug);
            e.exit()
        }
    };

    match args.sub {
        None => {
            use clap::CommandFactory;
            let _ = cli::parse::Cli::command().print_help();
            println!();
        }
        Some(sub) => {
            let gui = sub.gui();
            let force = sub.force();

            if let Err(e) = cli::run(sub, args.no_manifest_update, args.try_manifest_update) {
                failed = true;
                cli::show_error(&[], &e, gui, force);
            }
        }
    };

    flush_logger();
    debug_on_exit(debug);

    if failed {
        std::process::exit(1);
    }
}

fn debug_on_exit(debug: bool) {
    if debug {
        let path = app_dir();
        if let Err(e) = opener::open(path.raw()) {
            eprintln!("{}", TRANSLATOR.unable_to_open_dir(&path));
            log::error!("Unable to open directory: `{:?}` - {:?}", &path, e);
        }
    }
}
