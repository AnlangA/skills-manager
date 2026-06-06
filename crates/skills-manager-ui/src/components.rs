use iced::{
    Alignment, Element, Length,
    widget::{Button, Container, button, column, container, row, text},
};
use skills_manager_core::{SkillEnablement, SkillHealth, SkillScope};

use crate::{app::Message, icons, theme};

pub fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content).padding(14).style(theme::panel)
}

pub fn flat_panel<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content).padding(12).style(theme::flat_panel)
}

pub fn section_header<'a>(
    title: impl Into<String>,
    meta: impl Into<String>,
) -> Element<'a, Message> {
    row![
        text(title.into()).size(16).color(theme::TEXT),
        text(meta.into()).size(12).color(theme::MUTED),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

pub fn metric<'a>(
    label: impl Into<String>,
    value: impl Into<String>,
    color: iced::Color,
) -> Container<'a, Message> {
    container(
        row![
            text(value.into()).size(22).color(color),
            text(label.into()).size(12).color(theme::MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([9, 12])
    .width(Length::FillPortion(1))
    .style(theme::flat_panel)
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
        SkillHealth::Valid => (
            "Valid",
            iced::Color::from_rgb8(22, 101, 52),
            iced::Color::from_rgb8(220, 252, 231),
        ),
        SkillHealth::Warning => (
            "Warning",
            iced::Color::from_rgb8(146, 64, 14),
            iced::Color::from_rgb8(254, 243, 199),
        ),
        SkillHealth::Invalid => (
            "Invalid",
            iced::Color::from_rgb8(153, 27, 27),
            iced::Color::from_rgb8(254, 226, 226),
        ),
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
        SkillEnablement::Enabled => chip(
            "Enabled",
            iced::Color::from_rgb8(21, 128, 61),
            iced::Color::from_rgb8(220, 252, 231),
        ),
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
        SkillScope::User => chip("User", theme::CYAN, iced::Color::from_rgb8(207, 250, 254)),
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
