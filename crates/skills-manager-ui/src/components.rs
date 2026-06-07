use iced::{
    Alignment, Element, Length,
    widget::{Button, Container, button, column, container, row, text, text_input},
};
use skills_manager_core::{SkillEnablement, SkillHealth, SkillScope};

use crate::{app::Message, icons, theme};

pub fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content).padding(14).style(theme::panel)
}

pub fn flat_panel<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content).padding(10).style(theme::flat_panel)
}

pub fn section_header<'a>(
    title: impl Into<String>,
    meta: impl Into<String>,
) -> Element<'a, Message> {
    column![
        text(title.into()).size(16).color(theme::TEXT),
        text(meta.into())
            .size(12)
            .color(theme::MUTED)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(3)
    .into()
}

pub fn compact_metric<'a>(
    label: impl Into<String>,
    value: impl Into<String>,
    color: iced::Color,
) -> Container<'a, Message> {
    container(
        column![
            text(value.into()).size(18).color(color),
            text(label.into()).size(11).color(theme::MUTED),
        ]
        .spacing(2),
    )
    .padding([8, 10])
    .width(Length::FillPortion(1))
    .style(theme::metric_panel)
}

pub fn field<'a, F>(
    label: &'a str,
    helper: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: F,
) -> Element<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    column![
        text(label).size(12).color(theme::TEXT),
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([10, 12])
            .style(theme::input)
            .width(Length::Fill),
        text(helper).size(11).color(theme::SUBTLE),
    ]
    .spacing(5)
    .into()
}

pub fn compact_field<'a, F>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: F,
) -> Element<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    column![
        text(label).size(12).color(theme::TEXT),
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([9, 12])
            .style(theme::input)
            .width(Length::Fill),
    ]
    .spacing(5)
    .into()
}

pub fn primary_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon))
        .padding([8, 12])
        .style(theme::primary_button)
}

pub fn secondary_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon))
        .padding([8, 12])
        .style(theme::secondary_button)
}

pub fn danger_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon))
        .padding([8, 12])
        .style(theme::danger_button)
}

pub fn nav_button<'a>(
    label: &'a str,
    icon: &'static str,
    selected: bool,
    message: Message,
) -> Button<'a, Message> {
    button(
        row![icons::icon(icon, 16), text(label).size(14)]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(theme::nav_button(selected))
    .on_press(message)
}

pub fn health_chip<'a>(health: SkillHealth) -> Container<'a, Message> {
    let (label, foreground, background) = match health {
        SkillHealth::Valid => ("Valid", theme::SUCCESS, theme::SUCCESS_SOFT),
        SkillHealth::Warning => ("Warning", theme::WARNING, theme::WARNING_SOFT),
        SkillHealth::Invalid => ("Invalid", theme::DANGER, theme::DANGER_SOFT),
        SkillHealth::Shadowed => (
            "Shadowed",
            iced::Color::from_rgb8(55, 65, 81),
            iced::Color::from_rgb8(229, 231, 235),
        ),
    };

    chip(label, foreground, background)
}

pub fn enablement_chip<'a>(enablement: SkillEnablement) -> Container<'a, Message> {
    match enablement {
        SkillEnablement::Enabled => chip("Enabled", theme::SUCCESS, theme::SUCCESS_SOFT),
        SkillEnablement::Disabled => chip(
            "Disabled",
            theme::MUTED,
            iced::Color::from_rgb8(226, 232, 240),
        ),
    }
}

pub fn scope_chip<'a>(scope: SkillScope) -> Container<'a, Message> {
    match scope {
        SkillScope::Project => chip(
            "Project",
            theme::PRIMARY,
            iced::Color::from_rgb8(219, 234, 254),
        ),
        SkillScope::Global => chip("Global", theme::CYAN, iced::Color::from_rgb8(207, 250, 254)),
        SkillScope::ClaudeCode => chip(
            "Claude Code",
            iced::Color::from_rgb8(126, 34, 206),
            iced::Color::from_rgb8(243, 232, 255),
        ),
        SkillScope::Droid => chip(
            "Droid",
            iced::Color::from_rgb8(4, 120, 87),
            iced::Color::from_rgb8(209, 250, 229),
        ),
        SkillScope::Pencode => chip(
            "Pencode",
            iced::Color::from_rgb8(180, 83, 9),
            iced::Color::from_rgb8(254, 243, 199),
        ),
        SkillScope::Codex => chip(
            "Codex",
            iced::Color::from_rgb8(15, 118, 110),
            iced::Color::from_rgb8(204, 251, 241),
        ),
        SkillScope::Zed => chip(
            "Zed",
            iced::Color::from_rgb8(79, 70, 229),
            iced::Color::from_rgb8(224, 231, 255),
        ),
        SkillScope::Custom => chip(
            "Custom",
            iced::Color::from_rgb8(126, 34, 206),
            iced::Color::from_rgb8(243, 232, 255),
        ),
    }
}

pub fn status_badge<'a>(status: &'a str, busy: bool) -> Container<'a, Message> {
    let (label, foreground, background) = if busy {
        (
            "WORKING",
            theme::CYAN,
            iced::Color::from_rgb8(207, 250, 254),
        )
    } else {
        (
            "STATUS",
            theme::PRIMARY,
            iced::Color::from_rgb8(219, 234, 254),
        )
    };

    container(
        row![
            text(label).size(11).color(foreground),
            text(status).size(13).color(theme::MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(theme::chip(background, foreground))
}

pub fn empty_state<'a>(title: &'a str, body: &'a str) -> Container<'a, Message> {
    flat_panel(
        column![
            text(title).size(16).color(theme::TEXT),
            text(body).size(13).color(theme::MUTED),
        ]
        .spacing(6),
    )
}

fn chip<'a>(
    label: &'a str,
    foreground: iced::Color,
    background: iced::Color,
) -> Container<'a, Message> {
    container(text(label).size(11).color(foreground))
        .padding([4, 8])
        .style(theme::chip(background, foreground))
}

fn button_content<'a>(label: &'a str, icon: Option<&'static str>) -> Element<'a, Message> {
    match icon {
        Some(icon) => row![icons::icon(icon, 15), text(label).size(13)]
            .spacing(7)
            .align_y(Alignment::Center)
            .into(),
        None => text(label).size(13).into(),
    }
}
