pub mod app;
pub mod backup_screen;
pub mod badge;
pub mod common;
pub mod custom_games_editor;
pub mod custom_games_screen;
pub mod file_tree;
pub mod game_list;
pub mod icon;
pub mod ignored_items_editor;
pub mod modal;
pub mod notification;
pub mod number_input;
pub mod other_screen;
pub mod popup_menu;
pub mod redirect_editor;
pub mod restore_screen;
pub mod root_editor;
pub mod search;
pub mod shortcuts;
pub mod style;
pub mod undoable;
pub mod widget;

use iced::Application;

pub fn set_app_icon<T>(settings: &mut iced::Settings<T>) {
    settings.window.icon = match image::load_from_memory(include_bytes!("../assets/icon.png")) {
        Ok(buffer) => {
            let buffer = buffer.to_rgba8();
            let width = buffer.width();
            let height = buffer.height();
            let dynamic_image = image::DynamicImage::ImageRgba8(buffer);
            match iced::window::icon::Icon::from_rgba(dynamic_image.into_bytes(), width, height) {
                Ok(icon) => Some(icon),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

pub fn set_app_min_size<T>(settings: &mut iced::Settings<T>) {
    settings.window.min_size = Some((800, 600));
}

pub fn run() {
    let mut settings = iced::Settings::default();
    set_app_icon(&mut settings);
    set_app_min_size(&mut settings);
    let _ = app::App::run(settings);
}
