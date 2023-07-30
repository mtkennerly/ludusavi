use iced::{alignment, font, Font};

use crate::gui::widget::Text;

pub const ICONS_DATA: &[u8] = include_bytes!("../../assets/MaterialIcons-Regular.ttf");
// pub const ICONS: Font = Font::External {
//     name: "Material Icons",
//     bytes: include_bytes!("../../assets/MaterialIcons-Regular.ttf"),
// };
pub const ICONS: Font = Font {
    // name: "Material Icons",
    // bytes: include_bytes!("../../assets/MaterialIcons-Regular.ttf"),
    family: font::Family::Name("Material Icons"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    monospaced: false,
};

pub enum Icon {
    Add,
    AddCircle,
    ArrowBack,
    ArrowDownward,
    ArrowForward,
    ArrowUpward,
    Close,
    Comment,
    Delete,
    Download,
    Edit,
    Error,
    FastForward,
    Filter,
    FolderOpen,
    KeyboardArrowDown,
    KeyboardArrowRight,
    Language,
    Lock,
    LockOpen,
    MoreVert,
    OpenInNew,
    PlayCircleOutline,
    Refresh,
    Remove,
    RemoveCircle,
    Search,
    Settings,
    SubdirectoryArrowRight,
    Upload,
}

impl Icon {
    pub fn as_char(&self) -> char {
        match self {
            Self::Add => '\u{E145}',
            Self::AddCircle => '\u{E147}',
            Self::ArrowBack => '\u{e5c4}',
            Self::ArrowDownward => '\u{E5DB}',
            Self::ArrowForward => '\u{e5c8}',
            Self::ArrowUpward => '\u{E5D8}',
            Self::Close => '\u{E5CD}',
            Self::Comment => '\u{E0B9}',
            Self::Delete => '\u{E872}',
            Self::Download => '\u{f090}',
            Self::Edit => '\u{E150}',
            Self::Error => '\u{e000}',
            Self::FastForward => '\u{E01F}',
            Self::Filter => '\u{ef4f}',
            Self::FolderOpen => '\u{E2C8}',
            Self::KeyboardArrowDown => '\u{E313}',
            Self::KeyboardArrowRight => '\u{E315}',
            Self::Language => '\u{E894}',
            Self::Lock => '\u{e897}',
            Self::LockOpen => '\u{e898}',
            Self::MoreVert => '\u{E5D4}',
            Self::OpenInNew => '\u{E89E}',
            Self::PlayCircleOutline => '\u{E039}',
            Self::Refresh => '\u{E5D5}',
            Self::Remove => '\u{E15B}',
            Self::RemoveCircle => '\u{E15C}',
            Self::Search => '\u{e8b6}',
            Self::Settings => '\u{E8B8}',
            Self::SubdirectoryArrowRight => '\u{E5DA}',
            Self::Upload => '\u{f09b}',
        }
    }

    pub fn text(self) -> Text<'static> {
        Text::new(self.as_char().to_string())
            .font(ICONS)
            .size(20)
            .width(60)
            .height(20)
            .horizontal_alignment(alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }

    pub fn text_small(self) -> Text<'static> {
        Text::new(self.as_char().to_string())
            .font(ICONS)
            .size(15)
            .width(15)
            .height(15)
            .horizontal_alignment(alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }
}
