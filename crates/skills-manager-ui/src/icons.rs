use iced::{Font, widget::Text};

pub const DOWNLOAD: &str = "\u{E0B2}";
pub const SEARCH: &str = "\u{E151}";
pub const REFRESH: &str = "\u{E145}";
pub const TRASH: &str = "\u{E18E}";
pub const COPY: &str = "\u{E09E}";
pub const SETTINGS: &str = "\u{E154}";
pub const LIST: &str = "\u{E1D0}";
pub const FOLDER: &str = "\u{E0D7}";
pub const GLOBE: &str = "\u{E0E8}";
pub const SHIELD: &str = "\u{E1FF}";
pub const SPARKLES: &str = "\u{E412}";
pub const DATABASE: &str = "\u{E0AD}";
pub const FILE: &str = "\u{E0CC}";
pub const EYE: &str = "\u{E0BA}";
pub const EYE_OFF: &str = "\u{E0BB}";

pub fn icon(codepoint: &'static str, size: u32) -> Text<'static> {
    Text::new(codepoint)
        .font(Font::with_name("lucide"))
        .size(size)
}
