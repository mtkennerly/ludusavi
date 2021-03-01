use iced::{Font, HorizontalAlignment, Length, Text};

const ICONS: Font = Font::External {
    name: "Material Icons",
    bytes: include_bytes!("../../assets/MaterialIcons-Regular.ttf"),
};

pub enum Icon {
    AddCircle,
    Edit,
    FolderOpen,
    KeyboardArrowRight,
    KeyboardArrowDown,
    Language,
    OpenInNew,
    RemoveCircle,
    Search,
    SubdirectoryArrowRight,
    Delete,
}

impl Icon {
    pub fn as_text(&self) -> Text {
        let character = match self {
            Self::AddCircle => '\u{E147}',
            Self::Edit => '\u{E150}',
            Self::FolderOpen => '\u{E2C8}',
            Self::KeyboardArrowRight => '\u{E315}',
            Self::KeyboardArrowDown => '\u{E313}',
            Self::Language => '\u{E894}',
            Self::OpenInNew => '\u{E89E}',
            Self::RemoveCircle => '\u{E15C}',
            Self::Search => '\u{E8B6}',
            Self::SubdirectoryArrowRight => '\u{E5DA}',
            Self::Delete => '\u{E872}',
        };
        Text::new(&character.to_string())
            .font(ICONS)
            .width(Length::Units(60))
            .horizontal_alignment(HorizontalAlignment::Center)
    }
}
