mod app;
mod badge;
mod button;
mod common;
mod editor;
mod file_tree;
mod font;
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

use iced::Size;

use self::app::App;
pub use self::common::Flags;

pub fn run(flags: Flags) {
    let app = iced::application(move || App::new(flags.clone()), App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .title(App::title)
        .executor::<app::Executor>()
        .settings(iced::Settings {
            default_font: font::TEXT,
            ..Default::default()
        })
        .window(iced::window::Settings {
            min_size: Some(Size::new(800.0, 600.0)),
            exit_on_close_request: false,
            #[cfg(target_os = "linux")]
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: std::env::var(crate::prelude::ENV_LINUX_APP_ID)
                    .unwrap_or_else(|_| crate::prelude::LINUX_APP_ID.to_string()),
                ..Default::default()
            },
            icon: match image::load_from_memory(include_bytes!("../assets/icon.png")) {
                Ok(buffer) => {
                    let buffer = buffer.to_rgba8();
                    let width = buffer.width();
                    let height = buffer.height();
                    let dynamic_image = image::DynamicImage::ImageRgba8(buffer);
                    iced::window::icon::from_rgba(dynamic_image.into_bytes(), width, height).ok()
                }
                Err(_) => None,
            },
            ..Default::default()
        });

    if let Err(e) = app.run() {
        log::error!("Failed to initialize GUI: {e:?}");
        eprintln!("Failed to initialize GUI: {e:?}");

        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_description(e.to_string())
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}
