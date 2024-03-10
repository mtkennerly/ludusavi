use iced::{font, Font};

pub const TEXT_DATA: &[u8] = include_bytes!("../../assets/NotoSans-Regular.ttf");
pub const TEXT: Font = Font {
    family: font::Family::Name("Noto Sans"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

pub const ICONS_DATA: &[u8] = include_bytes!("../../assets/MaterialIcons-Regular.ttf");
pub const ICONS: Font = Font {
    family: font::Family::Name("Material Icons"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};
