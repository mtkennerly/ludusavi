pub mod heroic;
pub mod ui;

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
