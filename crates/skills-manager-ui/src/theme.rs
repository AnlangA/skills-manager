use iced::{
    Background, Border, Color, Shadow, Theme, Vector, theme,
    widget::{button, container, pick_list, text_input},
};

pub const BACKGROUND: Color = Color::from_rgb8(242, 245, 249);
pub const SIDEBAR: Color = Color::from_rgb8(18, 24, 38);
pub const SURFACE: Color = Color::from_rgb8(255, 255, 255);
pub const SURFACE_RAISED: Color = Color::from_rgb8(251, 253, 255);
pub const SURFACE_ALT: Color = Color::from_rgb8(237, 242, 248);
pub const BORDER: Color = Color::from_rgb8(213, 222, 234);
pub const TEXT: Color = Color::from_rgb8(24, 32, 46);
pub const MUTED: Color = Color::from_rgb8(73, 86, 109);
pub const SUBTLE: Color = Color::from_rgb8(111, 126, 149);
pub const PRIMARY: Color = Color::from_rgb8(31, 88, 190);
pub const CYAN: Color = Color::from_rgb8(6, 132, 158);
pub const SUCCESS: Color = Color::from_rgb8(19, 132, 73);
pub const WARNING: Color = Color::from_rgb8(184, 92, 9);
pub const DANGER: Color = Color::from_rgb8(200, 36, 52);
pub const PRIMARY_SOFT: Color = Color::from_rgb8(224, 237, 255);
pub const SUCCESS_SOFT: Color = Color::from_rgb8(218, 247, 230);
pub const WARNING_SOFT: Color = Color::from_rgb8(255, 239, 204);
pub const DANGER_SOFT: Color = Color::from_rgb8(255, 226, 231);

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
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(SIDEBAR)),
        ..container::Style::default()
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SURFACE)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        shadow: Shadow {
            color: Color::from_rgba8(18, 24, 38, 0.08),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 14.0,
        },
        ..container::Style::default()
    }
}

pub fn accent_panel(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SURFACE_RAISED)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: Color::from_rgb8(184, 207, 244),
        },
        shadow: Shadow {
            color: Color::from_rgba8(31, 88, 190, 0.08),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

pub fn flat_panel(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SURFACE_ALT)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub fn table_header(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(SUBTLE),
        background: Some(Background::Color(SURFACE_ALT)),
        border: Border {
            width: 1.0,
            radius: 6.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub fn table_row(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SURFACE)),
        border: Border {
            width: 1.0,
            radius: 6.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub fn selected_table_row(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(Color::from_rgb8(236, 245, 255))),
        border: Border {
            width: 1.0,
            radius: 6.0.into(),
            color: PRIMARY,
        },
        ..container::Style::default()
    }
}

pub fn metric_panel(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SURFACE_RAISED)),
        border: Border {
            width: 1.0,
            radius: 6.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub fn subtle_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match (selected, status) {
            (true, button::Status::Hovered | button::Status::Pressed) => {
                Color::from_rgb8(199, 222, 252)
            }
            (true, _) => PRIMARY_SOFT,
            (false, button::Status::Hovered | button::Status::Pressed) => {
                Color::from_rgb8(230, 237, 246)
            }
            (false, _) => SURFACE_RAISED,
        };

        button::Style {
            text_color: if selected { PRIMARY } else { MUTED },
            background: Some(Background::Color(background)),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: if selected { PRIMARY } else { BORDER },
            },
            ..button::Style::default()
        }
    }
}

pub fn chip(background: Color, foreground: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        text_color: Some(foreground),
        background: Some(Background::Color(background)),
        border: Border {
            width: 1.0,
            radius: 6.0.into(),
            color: background,
        },
        ..container::Style::default()
    }
}

pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.background = Background::Color(SURFACE);
    style.border.radius = 6.0.into();
    style.border.color = BORDER;
    style.placeholder = SUBTLE;
    style.value = TEXT;
    style
}

pub fn select(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let mut style = pick_list::default(theme, status);
    style.background = Background::Color(SURFACE);
    style.border.radius = 6.0.into();
    style.border.color = BORDER;
    style.placeholder_color = SUBTLE;
    style.text_color = TEXT;
    style.handle_color = MUTED;
    style
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    rounded_button(button::primary(theme, status))
}

pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    rounded_button(button::secondary(theme, status))
}

pub fn danger_button(theme: &Theme, status: button::Status) -> button::Style {
    rounded_button(button::danger(theme, status))
}

pub fn nav_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match (selected, status) {
            (true, button::Status::Hovered | button::Status::Pressed) => {
                Color::from_rgb8(226, 232, 240)
            }
            (true, _) => Color::WHITE,
            (false, button::Status::Hovered | button::Status::Pressed) => {
                Color::from_rgb8(30, 41, 59)
            }
            (false, _) => SIDEBAR,
        };

        button::Style {
            text_color: if selected { SIDEBAR } else { Color::WHITE },
            background: Some(Background::Color(background)),
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

fn rounded_button(style: button::Style) -> button::Style {
    button::Style {
        border: Border {
            radius: 6.0.into(),
            ..style.border
        },
        ..style
    }
}
