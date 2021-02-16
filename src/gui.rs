pub mod app;
pub mod backup_screen;
pub mod badge;
pub mod common;
pub mod custom_games_editor;
pub mod custom_games_screen;
pub mod disappearing_progress;
pub mod file_tree;
pub mod game_list;
pub mod icon;
pub mod modal;
pub mod other_screen;
pub mod redirect_editor;
pub mod restore_screen;
pub mod root_editor;
pub mod search;
pub mod style;

use iced::Application;

pub fn set_app_icon<T>(settings: &mut iced::Settings<T>) {
    settings.window.icon = match image::load_from_memory(include_bytes!("../assets/icon.png")) {
        Ok(buffer) => {
            let buffer = buffer.to_rgba8();
            let width = buffer.width();
            let height = buffer.height();
            let dynamic_image = image::DynamicImage::ImageRgba8(buffer);
            match iced::window::icon::Icon::from_rgba(dynamic_image.to_bytes(), width, height) {
                Ok(icon) => Some(icon),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

pub fn set_app_min_size<T>(settings: &mut iced::Settings<T>) {
    settings.window.min_size = Some((640, 480));
}

pub fn run_gui() {
    let mut settings = iced::Settings::default();
    set_app_icon(&mut settings);
    set_app_min_size(&mut settings);
    let _ = app::App::run(settings);
}
