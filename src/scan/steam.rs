use std::collections::HashMap;

use crate::{prelude::StrictPath, scan::TitleFinder};

#[derive(Clone, Debug, Default)]
pub struct SteamShortcuts(HashMap<String, SteamShortcut>);

#[derive(Clone, Debug, Default)]
pub struct SteamShortcut {
    pub id: u32,
    pub start_dir: Option<StrictPath>,
}

impl SteamShortcuts {
    pub fn scan(title_finder: &TitleFinder) -> Self {
        let mut instance = Self::default();

        let steam = match steamlocate::SteamDir::locate() {
            Ok(x) => x,
            Err(e) => {
                log::info!("Unable to locate Steam directory: {:?}", e);
                return instance;
            }
        };

        log::info!("Inspecting Steam shortcuts from: {:?}", steam.path());

        let Ok(shortcuts) = steam.shortcuts() else {
            log::warn!("Unable to load Steam shortcuts");
            return instance;
        };

        for shortcut in shortcuts.filter_map(|x| x.ok()) {
            let Some(official_title) = title_finder.find_one_by_normalized_name(&shortcut.app_name) else {
                log::debug!("Ignoring unrecognized Steam shortcut: {}", &shortcut.app_name);
                continue;
            };

            log::trace!(
                "Found Steam shortcut: app_name='{}', official_title='{}', id={}, start_dir='{}'",
                &shortcut.app_name,
                &official_title,
                shortcut.app_id,
                &shortcut.start_dir
            );
            let start_dir = std::path::Path::new(shortcut.start_dir.trim_start_matches('"').trim_end_matches('"'));
            instance.0.insert(
                official_title,
                SteamShortcut {
                    id: shortcut.app_id,
                    start_dir: if start_dir.is_absolute() {
                        Some(StrictPath::from(start_dir))
                    } else {
                        None
                    },
                },
            );
        }

        instance
    }

    pub fn get(&self, name: &str) -> Option<&SteamShortcut> {
        self.0.get(name)
    }
}
