//! Lucide icon codepoint constants and rendering helper.
//!
//! Provides named constants for commonly used Lucide icons and an
//! [`icon`] function that renders a codepoint as a styled `Text` widget
//! using the embedded `lucide` font.

use iced::{Font, widget::Text};

/// Download icon codepoint.
pub const DOWNLOAD: &str = "\u{E0B2}";
/// Search icon codepoint.
pub const SEARCH: &str = "\u{E151}";
/// Refresh icon codepoint.
pub const REFRESH: &str = "\u{E145}";
/// Trash/delete icon codepoint.
pub const TRASH: &str = "\u{E18E}";
/// Copy icon codepoint.
pub const COPY: &str = "\u{E09E}";
/// Settings/gear icon codepoint.
pub const SETTINGS: &str = "\u{E154}";
/// List icon codepoint.
pub const LIST: &str = "\u{E1D0}";
/// Sparkles icon codepoint.
pub const SPARKLES: &str = "\u{E412}";
/// Database icon codepoint.
pub const DATABASE: &str = "\u{E0AD}";
/// File icon codepoint.
pub const FILE: &str = "\u{E0CC}";
/// Eye/visible icon codepoint.
pub const EYE: &str = "\u{E0BA}";
/// Eye-off/hidden icon codepoint.
pub const EYE_OFF: &str = "\u{E0BB}";

/// Renders a Lucide icon codepoint as a styled `Text` widget at the given size.
pub fn icon(codepoint: &'static str, size: u32) -> Text<'static> {
    Text::new(codepoint)
        .font(Font::with_name("lucide"))
        .size(size)
}
