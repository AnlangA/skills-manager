use iced::{
    Background, Border, Color, Shadow, Theme, Vector, theme,
    widget::{button, container, pick_list, text_input},
};

pub const BACKGROUND: Color = Color::from_rgb8(248, 250, 252);
pub const SIDEBAR: Color = Color::from_rgb8(17, 24, 39);
pub const SIDEBAR_HOVER: Color = Color::from_rgb8(31, 41, 55);
pub const SIDEBAR_ACTIVE: Color = Color::from_rgb8(55, 65, 81);
pub const SURFACE: Color = Color::from_rgb8(255, 255, 255);
pub const SURFACE_ALT: Color = Color::from_rgb8(249, 250, 251);
pub const SURFACE_HOVER: Color = Color::from_rgb8(243, 244, 246);
pub const BORDER: Color = Color::from_rgb8(229, 231, 235);
pub const BORDER_LIGHT: Color = Color::from_rgb8(243, 244, 246);
pub const TEXT: Color = Color::from_rgb8(17, 24, 39);
pub const TEXT_SECONDARY: Color = Color::from_rgb8(75, 85, 99);
pub const TEXT_MUTED: Color = Color::from_rgb8(156, 163, 175);
pub const PRIMARY: Color = Color::from_rgb8(59, 130, 246);
pub const PRIMARY_HOVER: Color = Color::from_rgb8(37, 99, 235);
pub const PRIMARY_PRESSED: Color = Color::from_rgb8(29, 78, 216);
pub const PRIMARY_SOFT: Color = Color::from_rgb8(239, 246, 255);
pub const SUCCESS: Color = Color::from_rgb8(34, 197, 94);
pub const SUCCESS_SOFT: Color = Color::from_rgb8(240, 253, 244);
pub const WARNING: Color = Color::from_rgb8(245, 158, 11);
pub const WARNING_SOFT: Color = Color::from_rgb8(255, 251, 235);
pub const DANGER: Color = Color::from_rgb8(239, 68, 68);
pub const DANGER_HOVER: Color = Color::from_rgb8(220, 38, 38);
pub const DANGER_PRESSED: Color = Color::from_rgb8(185, 28, 28);
pub const DANGER_SOFT: Color = Color::from_rgb8(254, 242, 242);
pub const INFO: Color = Color::from_rgb8(6, 182, 212);
pub const INFO_SOFT: Color = Color::from_rgb8(236, 254, 255);

pub const CLAUDE_CODE: Color = Color::from_rgb8(126, 34, 206);
pub const CLAUDE_CODE_SOFT: Color = Color::from_rgb8(243, 232, 255);
pub const DROID: Color = Color::from_rgb8(4, 120, 87);
pub const DROID_SOFT: Color = Color::from_rgb8(209, 250, 229);
pub const OPENCODE: Color = Color::from_rgb8(180, 83, 9);
pub const OPENCODE_SOFT: Color = Color::from_rgb8(254, 243, 199);
pub const CODEX: Color = Color::from_rgb8(15, 118, 110);
pub const CODEX_SOFT: Color = Color::from_rgb8(204, 251, 241);
pub const ZED: Color = Color::from_rgb8(79, 70, 229);
pub const ZED_SOFT: Color = Color::from_rgb8(224, 231, 255);

pub const FONT_DISPLAY: u32 = 24;
pub const FONT_HEADING: u32 = 16;
pub const FONT_BODY: u32 = 14;
pub const FONT_CAPTION: u32 = 12;
pub const FONT_MICRO: u32 = 10;

pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;
pub const SPACING_MD: f32 = 12.0;
pub const SPACING_LG: f32 = 16.0;
pub const SPACING_XL: f32 = 24.0;

pub fn app_theme() -> Theme {
    Theme::custom(
        "Agent Skills Manager",
        theme::Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: PRIMARY,
            success: SUCCESS,
            warning: WARNING,
            danger: DANGER,
        },
    )
}

pub fn app_background(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(BACKGROUND)),
        ..container::Style::default()
    }
}

pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(BORDER),
        background: Some(Background::Color(SIDEBAR)),
        ..container::Style::default()
    }
}

fn card_style(
    bg: Color,
    border_color: Color,
    border_width: f32,
    radius: f32,
    shadow: Option<Shadow>,
) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(bg)),
        border: Border {
            width: border_width,
            radius: radius.into(),
            color: border_color,
        },
        shadow: shadow.unwrap_or_default(),
        ..container::Style::default()
    }
}

pub fn card(_theme: &Theme) -> container::Style {
    card_style(
        SURFACE,
        BORDER,
        1.0,
        12.0,
        Some(Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.04),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 3.0,
        }),
    )
}

pub fn flat_card(_theme: &Theme) -> container::Style {
    card_style(SURFACE_ALT, BORDER_LIGHT, 1.0, 8.0, None)
}

pub fn selected_card(_theme: &Theme) -> container::Style {
    card_style(PRIMARY_SOFT, PRIMARY, 1.5, 12.0, None)
}

pub fn metric_card(_theme: &Theme) -> container::Style {
    card_style(SURFACE, BORDER, 1.0, 10.0, None)
}

pub fn chip(background: Color, foreground: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        text_color: Some(foreground),
        background: Some(Background::Color(background)),
        border: Border {
            width: 0.0,
            radius: 6.0.into(),
            color: background,
        },
        ..container::Style::default()
    }
}

pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.background = Background::Color(SURFACE);
    let focused = matches!(status, text_input::Status::Active);
    style.border = Border {
        width: 1.0,
        radius: 8.0.into(),
        color: if focused { PRIMARY } else { BORDER },
    };
    style.placeholder = TEXT_MUTED;
    style.value = TEXT;
    style
}

pub fn select(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let mut style = pick_list::default(theme, status);
    style.background = Background::Color(SURFACE);
    let focused = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );
    style.border = Border {
        width: 1.0,
        radius: 8.0.into(),
        color: if focused { PRIMARY } else { BORDER },
    };
    style.placeholder_color = TEXT_MUTED;
    style.text_color = TEXT;
    style.handle_color = TEXT_SECONDARY;
    style
}

fn solid_button(
    base: Color,
    hover: Color,
    pressed: Color,
    shadow_color: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => hover,
            button::Status::Pressed => pressed,
            _ => base,
        };
        button::Style {
            text_color: Color::WHITE,
            background: Some(Background::Color(background)),
            border: Border {
                radius: 8.0.into(),
                ..Border::default()
            },
            shadow: Shadow {
                color: shadow_color,
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..button::Style::default()
        }
    }
}

fn subtle_button(
    base_bg: Color,
    has_border: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => SURFACE_HOVER,
            button::Status::Pressed => BORDER,
            _ => base_bg,
        };
        let border = if has_border {
            Border {
                width: 1.0,
                radius: 8.0.into(),
                color: BORDER,
            }
        } else {
            Border {
                radius: 8.0.into(),
                ..Border::default()
            }
        };
        button::Style {
            text_color: TEXT_SECONDARY,
            background: Some(Background::Color(background)),
            border,
            ..button::Style::default()
        }
    }
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    solid_button(
        PRIMARY,
        PRIMARY_HOVER,
        PRIMARY_PRESSED,
        Color::from_rgba8(59, 130, 246, 0.2),
    )(theme, status)
}

pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    subtle_button(SURFACE, true)(theme, status)
}

pub fn danger_button(theme: &Theme, status: button::Status) -> button::Style {
    solid_button(
        DANGER,
        DANGER_HOVER,
        DANGER_PRESSED,
        Color::from_rgba8(239, 68, 68, 0.2),
    )(theme, status)
}

pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    subtle_button(Color::TRANSPARENT, false)(theme, status)
}

pub fn nav_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match (selected, status) {
            (true, button::Status::Hovered | button::Status::Pressed) => SIDEBAR_ACTIVE,
            (true, _) => SIDEBAR_HOVER,
            (false, button::Status::Hovered | button::Status::Pressed) => SIDEBAR_HOVER,
            (false, _) => Color::TRANSPARENT,
        };
        let border = if selected {
            Border {
                width: 3.0,
                radius: 8.0.into(),
                color: PRIMARY,
            }
        } else {
            Border {
                radius: 8.0.into(),
                ..Border::default()
            }
        };
        button::Style {
            text_color: if selected { Color::WHITE } else { TEXT_MUTED },
            background: Some(Background::Color(background)),
            border,
            ..button::Style::default()
        }
    }
}

pub fn pill_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (background, text_color, border_color) = match (selected, status) {
            (true, button::Status::Hovered | button::Status::Pressed) => {
                (PRIMARY_HOVER, Color::WHITE, PRIMARY_HOVER)
            }
            (true, _) => (PRIMARY, Color::WHITE, PRIMARY),
            (false, button::Status::Hovered | button::Status::Pressed) => {
                (SURFACE_HOVER, TEXT_SECONDARY, BORDER)
            }
            (false, _) => (SURFACE, TEXT_SECONDARY, BORDER),
        };
        button::Style {
            text_color,
            background: Some(Background::Color(background)),
            border: Border {
                width: 1.0,
                radius: 20.0.into(),
                color: border_color,
            },
            ..button::Style::default()
        }
    }
}
