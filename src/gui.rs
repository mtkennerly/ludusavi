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
