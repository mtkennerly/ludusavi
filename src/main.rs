mod config;
mod gui;
mod lang;
mod manifest;
mod path;
mod prelude;

#[cfg(target_os = "windows")]
mod registry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gui::run_gui();
    Ok(())
}
