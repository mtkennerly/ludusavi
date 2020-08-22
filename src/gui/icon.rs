use iced::{Font, HorizontalAlignment, Length, Text};

const ICONS: Font = Font::External {
    name: "Material Icons",
    bytes: include_bytes!("../../assets/MaterialIcons-Regular.ttf"),
};

pub enum Icon {
    AddCircle,
    RemoveCircle,
    FolderOpen,
    Edit,
    Search,
    Language,
}

impl Icon {
    pub fn as_text(&self) -> Text {
        let character = match self {
            Self::AddCircle => '\u{E147}',
            Self::RemoveCircle => '\u{E15C}',
            Self::FolderOpen => '\u{E2C8}',
            Self::Edit => '\u{E150}',
            Self::Search => '\u{E8B6}',
            Self::Language => '\u{E894}',
        };
        Text::new(&character.to_string())
            .font(ICONS)
            .width(Length::Units(60))
            .horizontal_alignment(HorizontalAlignment::Center)
    }
}
