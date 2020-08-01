mod cli;
mod config;
mod gui;
mod lang;
mod layout;
mod manifest;
mod path;
mod prelude;
mod serialization;
mod shortcuts;

#[cfg(target_os = "windows")]
mod registry;

fn main() {
    let args = cli::parse_cli();
    match args.sub {
        None => {
            #[cfg(target_os = "windows")]
            {
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
            gui::run_gui();
        }
        Some(sub) => {
            if let Err(e) = cli::run_cli(sub) {
                let translator = crate::lang::Translator::default();
                eprintln!("\n{}", translator.handle_error(&e));
                std::process::exit(1);
            }
        }
    };
}
