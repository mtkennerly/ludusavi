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
            Ok(x) => x,
            Err(e) => {
                log::warn!("Unable to locate Steam directory: {:?}", e);
                return instance;
            }
        };

        let Ok(shortcuts) = steam.shortcuts() else {
            log::warn!("Unable to load Steam shortcuts");
            return instance;
        };

        for shortcut in shortcuts.filter_map(|x| x.ok()) {
            log::trace!(
                "Found Steam shortcut: name={}, id={}, start_dir={}",
                &shortcut.app_name,
                shortcut.app_id,
                &shortcut.start_dir
            );
            let start_dir = std::path::Path::new(shortcut.start_dir.trim_start_matches('"').trim_end_matches('"'));
            instance.0.insert(
                shortcut.app_name.clone(),
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
