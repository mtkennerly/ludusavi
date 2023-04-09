use std::collections::HashMap;

use crate::prelude::StrictPath;

#[derive(Clone, Debug, Default)]
pub struct SteamShortcuts(HashMap<String, SteamShortcut>);

#[derive(Clone, Debug, Default)]
pub struct SteamShortcut {
    pub id: u32,
    pub start_dir: Option<StrictPath>,
}

impl SteamShortcuts {
    pub fn scan() -> Self {
        let mut instance = Self::default();

        let mut steam = match steamlocate::SteamDir::locate() {
            Some(x) => x,
            None => return instance,
        };

        for shortcut in steam.shortcuts() {
            log::trace!(
                "Found Steam shortcut: name={}, id={}, start_dir={}",
                &shortcut.app_name,
                shortcut.appid,
                &shortcut.start_dir
            );
            let start_dir = std::path::Path::new(shortcut.start_dir.trim_start_matches('"').trim_end_matches('"'));
            instance.0.insert(
                shortcut.app_name.clone(),
                SteamShortcut {
                    id: shortcut.appid,
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
