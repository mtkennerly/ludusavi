pub mod heroic;

/// Returned game information with whatever we could find
#[derive(Clone, Default, Debug)]
pub struct WrapGameInfo {
    pub name: Option<String>,
    pub steam_id: Option<u32>,
    pub gog_id: Option<u64>,
}

impl WrapGameInfo {
    fn is_empty(&self) -> bool {
        let Self { name, steam_id, gog_id } = self;

        name.is_none() && steam_id.is_none() && gog_id.is_none()
    }
}

pub fn infer_game_from_steam() -> Option<WrapGameInfo> {
    for var in ["SteamAppId", "STEAMAPPID"] {
        let Ok(raw) = std::env::var(var) else { continue };
        let Ok(app_id) = raw.parse::<u32>() else { continue };

        log::debug!("Found Steam environment variable: {}={}", var, app_id);

        let result = WrapGameInfo {
            steam_id: Some(app_id),
            ..Default::default()
        };

        return Some(result);
    }

    None
}

pub mod lutris {
    use super::*;

    use crate::path::StrictPath;

    pub struct Metadata {
        pub title: String,
        pub base: Option<StrictPath>,
        pub prefix: Option<StrictPath>,
    }

    pub fn infer() -> Option<WrapGameInfo> {
        let title = if let Ok(title) = std::env::var("GAME_NAME") {
            log::debug!("Found Lutris environment variable: GAME_NAME={}", &title);
            title
        } else if let Ok(title) = std::env::var("game_name") {
            log::debug!("Found Lutris environment variable: game_name={}", &title);
            title
        } else {
            return None;
        };

        let result = WrapGameInfo {
            name: Some(title),
            ..Default::default()
        };

        Some(result)
    }

    pub fn get_normalized_title() -> Option<String> {
        if let Ok(title) = std::env::var("GAME_NAME") {
            Some(title)
        } else if let Ok(title) = std::env::var("game_name") {
            Some(title)
        } else {
            None
        }
    }

    pub fn save_normalized_title(_title: String) {
        // Intentionally empty - no longer using shared state
    }

    pub fn infer_metadata() -> Option<Metadata> {
        let title = get_normalized_title()?;

        let base = std::env::var("GAME_DIRECTORY").ok();
        let prefix = std::env::var("WINEPREFIX").ok();

        log::debug!(
            "Found Lutris environment variables for inferred game '{}': GAME_DIRECTORY={:?}, WINEPREFIX={:?}",
            &title,
            &base,
            &prefix
        );

        Some(Metadata {
            title,
            base: base.map(StrictPath::new),
            prefix: prefix.map(StrictPath::new),
        })
    }
}
