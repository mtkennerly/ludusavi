use std::fmt::Display;

pub mod heroic;
pub mod ui;

/// Returned game information with whatever we could find
#[derive(Default, Debug)]
pub struct WrapGameInfo {
    pub name: Option<String>,
    pub gog_id: Option<u64>,
}

impl Display for WrapGameInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result: String = "".to_string();

        if self.name.is_some() {
            result += self.name.as_ref().unwrap().as_str();
        }
        if self.gog_id.is_some() {
            if !result.is_empty() {
                result += ", ";
            }
            result += &format!("GOG Id: {}", self.name.as_ref().unwrap().as_str());
        }
        write!(f, "{}", result)
    }
}

impl WrapGameInfo {
    fn is_empty(&self) -> bool {
        self.name.is_none() && self.gog_id.is_none()
    }
}
