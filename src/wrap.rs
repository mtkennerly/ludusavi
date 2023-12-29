use crate::scan::TitleFinder;

pub mod heroic;

/// Returned game information with whatever we could find
#[derive(Default, Debug)]
pub struct WrapGameInfo {
    pub name: Option<String>,
    pub gog_id: Option<u64>,
}

impl WrapGameInfo {
    fn is_empty(&self) -> bool {
        self.name.is_none() && self.gog_id.is_none()
    }
}

pub fn infer_game_from_steam(title_finder: &TitleFinder) -> Option<WrapGameInfo> {
    let app_id = std::env::var("STEAMAPPID").ok()?.parse::<u32>().ok()?;

    log::debug!("Found Steam environment variable: STEAMAPPID={}", app_id,);

    let result = WrapGameInfo {
        name: title_finder.find_one(&[], &Some(app_id), &None, false),
        gog_id: None,
    };

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
