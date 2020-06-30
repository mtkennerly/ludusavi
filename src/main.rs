mod config;
mod gui;
mod lang;
mod manifest;
mod path;
mod prelude;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gui::run_gui();
    Ok(())
}
