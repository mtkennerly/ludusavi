use iced::{alignment, Length};

use crate::gui::{
    font,
    widget::{text, Text},
};

pub enum Icon {
    Add,
    AddCircle,
    ArrowBack,
    ArrowDownward,
    ArrowForward,
    ArrowUpward,
    CallMerge,
    CallSplit,
    Comment,
    Copy,
    Delete,
    Download,
    Edit,
    Error,
    FastForward,
    Filter,
    FolderOpen,
    Info,
    KeyboardArrowDown,
    KeyboardArrowRight,
    Language,
    Lock,
    LockOpen,
    MoreVert,
    OpenInBrowser,
    OpenInNew,
    PlayCircleOutline,
    Refresh,
    Remove,
    RemoveCircle,
    Search,
    #[allow(unused)]
    Settings,
    SubdirectoryArrowRight,
    Upload,
    VisibilityOff,
}

impl Icon {
    pub const fn as_char(&self) -> char {
        match self {
            Self::Add => '\u{E145}',
            Self::AddCircle => '\u{E147}',
            Self::ArrowBack => '\u{e5c4}',
            Self::ArrowDownward => '\u{E5DB}',
            Self::ArrowForward => '\u{e5c8}',
            Self::ArrowUpward => '\u{E5D8}',
            Self::CallMerge => '\u{e0b3}',
            Self::CallSplit => '\u{e0b6}',
            Self::Comment => '\u{E0B9}',
            Self::Copy => '\u{e14d}',
            Self::Delete => '\u{E872}',
            Self::Download => '\u{f090}',
            Self::Edit => '\u{E150}',
            Self::Error => '\u{e001}',
            Self::FastForward => '\u{E01F}',
            Self::Filter => '\u{ef4f}',
            Self::FolderOpen => '\u{E2C8}',
            Self::Info => '\u{e88f}',
            Self::KeyboardArrowDown => '\u{E313}',
            Self::KeyboardArrowRight => '\u{E315}',
            Self::Language => '\u{E894}',
            Self::Lock => '\u{e899}',
            Self::LockOpen => '\u{e898}',
            Self::MoreVert => '\u{E5D4}',
            Self::OpenInBrowser => '\u{e89d}',
            Self::OpenInNew => '\u{E89E}',
            Self::PlayCircleOutline => '\u{E039}',
            Self::Refresh => '\u{E5D5}',
            Self::Remove => '\u{E15B}',
            Self::RemoveCircle => '\u{E15C}',
            Self::Search => '\u{e8b6}',
            Self::Settings => '\u{E8B8}',
            Self::SubdirectoryArrowRight => '\u{E5DA}',
            Self::Upload => '\u{f09b}',
            Self::VisibilityOff => '\u{e8f5}',
        }
    }

    pub fn text(self) -> Text<'static> {
        text(self.as_char().to_string())
            .font(font::ICONS)
            .size(20)
            .width(60)
            .height(20)
            .align_x(alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }

    pub fn text_narrow(self) -> Text<'static> {
        self.text().width(Length::Shrink)
    }

    pub fn text_small(self) -> Text<'static> {
        text(self.as_char().to_string())
            .font(font::ICONS)
            .size(15)
            .width(15)
            .height(15)
            .align_x(alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }
}
