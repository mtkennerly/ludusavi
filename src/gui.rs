mod app;
mod badge;
mod button;
mod common;
mod editor;
mod file_tree;
mod game_list;
mod icon;
mod modal;
mod notification;
mod popup_menu;
mod screen;
mod search;
mod shortcuts;
mod style;
mod undoable;
mod widget;

use iced::Application;

pub fn run() {
    let mut settings = iced::Settings::default();

    settings.window.min_size = Some((800, 600));
    settings.exit_on_close_request = false;
    settings.window.icon = match image::load_from_memory(include_bytes!("../assets/icon.png")) {
        Ok(buffer) => {
            let buffer = buffer.to_rgba8();
            let width = buffer.width();
            let height = buffer.height();
            let dynamic_image = image::DynamicImage::ImageRgba8(buffer);
            match iced::window::icon::from_rgba(dynamic_image.into_bytes(), width, height) {
                Ok(icon) => Some(icon),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };

    let _ = app::App::run(settings);
}
