//! Reusable UI component helpers for cards, buttons, fields, chips, and lists.
//!
//! Provides builder functions for common widget patterns used across all
//! views, including styled cards, form fields, pick lists, action buttons,
//! health/enablement/scope chips, and diagnostic display helpers.

use iced::{
    Alignment, Element, Length,
    widget::{Button, Container, button, column, container, pick_list, row, text, text_input},
};
use skills_manager_core::{SkillDiagnostic, SkillEnablement, SkillHealth, SkillScope};

use crate::theme::*;
use crate::{
    app::{ExpandedEditorTarget, Message},
    icons, theme,
};

/// Creates a styled card container with large padding and the default card theme.
pub fn card<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content).padding(SPACING_LG).style(theme::card)
}

/// Creates a flat card container with medium padding and the flat card theme.
pub fn flat_card<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .padding(SPACING_MD)
        .style(theme::flat_card)
}

/// Renders an uppercase section label in micro font.
pub fn section_label<'a>(label: &'a str) -> Element<'a, Message> {
    text(label.to_uppercase())
        .size(FONT_MICRO)
        .color(TEXT_MUTED)
        .into()
}

/// Renders a section header with a title and metadata label.
pub fn section_header<'a>(
    title: impl Into<String>,
    meta: impl Into<String>,
) -> Element<'a, Message> {
    row![
        text(title.into()).size(FONT_BODY).color(TEXT),
        text(meta.into()).size(FONT_CAPTION).color(TEXT_MUTED),
    ]
    .spacing(SPACING_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Renders a metric display card with a value, label, and accent color.
pub fn metric<'a>(
    label: impl Into<String>,
    value: impl Into<String>,
    color: iced::Color,
) -> Container<'a, Message> {
    container(
        column![
            text(value.into()).size(FONT_DISPLAY).color(color),
            text(label.into()).size(FONT_MICRO).color(TEXT_MUTED),
        ]
        .spacing(SPACING_XS),
    )
    .padding([SPACING_MD, SPACING_LG])
    .width(Length::FillPortion(1))
    .style(theme::metric_card)
}

/// Renders an inline summary stat with a bullet, value, and label.
pub fn summary_stat<'a>(
    label: impl Into<String>,
    value: impl Into<String>,
    color: iced::Color,
) -> Element<'a, Message> {
    row![
        container(text("\u{2022}").size(FONT_BODY).color(color)).width(Length::Fixed(8.0)),
        text(value.into()).size(FONT_BODY).color(TEXT),
        text(label.into()).size(FONT_CAPTION).color(TEXT_MUTED),
    ]
    .spacing(SPACING_XS)
    .align_y(Alignment::Center)
    .into()
}

/// Wraps content in a titled detail section.
pub fn detail_section<'a>(
    title: &'a str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![section_label(title), content.into(),]
        .spacing(SPACING_XS)
        .into()
}

/// Renders a label-value pair in a vertical column layout.
pub fn detail_row<'a>(
    label: &'a str,
    value: impl Into<String>,
) -> iced::widget::Column<'a, Message> {
    column![
        text(label).size(FONT_MICRO).color(TEXT_MUTED),
        text(value.into())
            .size(FONT_BODY)
            .color(TEXT)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(SPACING_XS)
}

/// Renders a form field with label, optional helper text, placeholder, and input callback.
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
    let mut col = column![
        text(label).size(FONT_CAPTION).color(TEXT),
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([SPACING_SM, SPACING_MD])
            .style(theme::input)
            .width(Length::Fill),
    ]
    .spacing(SPACING_XS + 2.0);

    if !helper.is_empty() {
        col = col.push(text(helper).size(FONT_MICRO).color(TEXT_MUTED));
    }

    col.into()
}

/// Renders a form field with an expand button for wide editing.
pub fn expandable_field<'a, F>(
    label: &'a str,
    helper: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: F,
    target: ExpandedEditorTarget,
) -> Element<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    let header = row![
        text(label).size(FONT_CAPTION).color(TEXT),
        column![].width(Length::Fill),
        small_ghost_button("Expand", None).on_press(Message::OpenExpandedEditor(target)),
    ]
    .spacing(SPACING_SM)
    .align_y(Alignment::Center);

    let mut col = column![
        header,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([SPACING_SM, SPACING_MD])
            .style(theme::input)
            .width(Length::Fill),
    ]
    .spacing(SPACING_XS + 2.0);

    if !helper.is_empty() {
        col = col.push(text(helper).size(FONT_MICRO).color(TEXT_MUTED));
    }

    col.into()
}

/// Renders a compact form field without helper text.
pub fn compact_field<'a, F>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: F,
) -> Element<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    field(label, "", placeholder, value, on_input)
}

/// Renders a styled dropdown pick list.
pub fn styled_pick_list<'a, T, F>(
    options: &'a [T],
    selected: Option<T>,
    on_select: F,
    width: Length,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + std::fmt::Display + 'a,
    F: Fn(T) -> Message + 'a,
{
    pick_list(options, selected, on_select)
        .padding([SPACING_SM, SPACING_MD])
        .style(theme::select)
        .width(width)
        .into()
}

fn button_content<'a>(
    label: &'a str,
    icon: Option<&'static str>,
    size: u32,
) -> Element<'a, Message> {
    match icon {
        Some(icon) => row![icons::icon(icon, size - 2), text(label).size(size)]
            .spacing(SPACING_SM)
            .align_y(Alignment::Center)
            .into(),
        None => text(label).size(size).into(),
    }
}

/// Creates a primary action button with optional icon.
pub fn primary_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon, FONT_BODY))
        .padding([SPACING_SM, SPACING_LG])
        .style(theme::primary_button)
}

/// Creates a secondary action button with optional icon.
pub fn secondary_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon, FONT_BODY))
        .padding([SPACING_SM, SPACING_LG])
        .style(theme::secondary_button)
}

/// Creates a small ghost button with optional icon.
pub fn small_ghost_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon, FONT_CAPTION))
        .padding([SPACING_XS, SPACING_SM])
        .style(theme::ghost_button)
}

/// Creates a small danger-styled button with optional icon.
pub fn small_danger_button<'a>(label: &'a str, icon: Option<&'static str>) -> Button<'a, Message> {
    button(button_content(label, icon, FONT_CAPTION))
        .padding([SPACING_XS, SPACING_SM])
        .style(theme::danger_button)
}

/// Creates a two-step confirmation button that switches labels between pending and confirm states.
pub fn confirm_button<'a>(
    pending: bool,
    action_label: &'a str,
    confirm_label: &'a str,
    icon: Option<&'static str>,
    request_msg: Message,
    confirm_msg: Message,
    busy: bool,
) -> Button<'a, Message> {
    let label = if pending { confirm_label } else { action_label };
    let msg = if pending { confirm_msg } else { request_msg };
    small_danger_button(label, icon).on_press_maybe((!busy).then_some(msg))
}

/// Creates a sidebar navigation button with icon and selection state.
pub fn nav_button<'a>(
    label: &'a str,
    icon: &'static str,
    selected: bool,
    message: Message,
) -> Button<'a, Message> {
    button(
        row![icons::icon(icon, 18), text(label).size(FONT_BODY)]
            .spacing(SPACING_MD)
            .align_y(Alignment::Center),
    )
    .padding([SPACING_SM, SPACING_MD])
    .width(Length::Fill)
    .style(theme::nav_button(selected))
    .on_press(message)
}

/// Arranges a form and preview panel side-by-side.
pub fn form_preview_layout<'a>(
    form: impl Into<Element<'a, Message>>,
    preview: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        container(form.into())
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        container(preview.into())
            .width(Length::FillPortion(3))
            .height(Length::Fill),
    ]
    .spacing(SPACING_LG)
    .height(Length::Fill)
    .into()
}

/// Renders a colored dot indicator for skill health status.
pub fn health_dot<'a>(health: SkillHealth) -> Element<'a, Message> {
    let (label, color) = match health {
        SkillHealth::Valid => ("Valid", SUCCESS),
        SkillHealth::Warning => ("Warning", WARNING),
        SkillHealth::Invalid => ("Invalid", DANGER),
        SkillHealth::Shadowed => ("Shadowed", TEXT_MUTED),
    };
    row![
        container(text("\u{2022}").size(FONT_HEADING).color(color)).width(Length::Fixed(10.0)),
        text(label).size(FONT_CAPTION).color(TEXT_SECONDARY),
    ]
    .spacing(SPACING_XS)
    .align_y(Alignment::Center)
    .into()
}

/// Renders a colored chip badge for skill health status.
pub fn health_chip<'a>(health: SkillHealth) -> Container<'a, Message> {
    let (label, foreground, background) = match health {
        SkillHealth::Valid => ("Valid", SUCCESS, SUCCESS_SOFT),
        SkillHealth::Warning => ("Warning", WARNING, WARNING_SOFT),
        SkillHealth::Invalid => ("Invalid", DANGER, DANGER_SOFT),
        SkillHealth::Shadowed => ("Shadowed", TEXT_SECONDARY, SURFACE_ALT),
    };
    chip(label, foreground, background)
}

/// Renders a colored chip badge for skill enablement status.
pub fn enablement_chip<'a>(enablement: SkillEnablement) -> Container<'a, Message> {
    match enablement {
        SkillEnablement::Enabled => chip("Enabled", SUCCESS, SUCCESS_SOFT),
        SkillEnablement::Disabled => chip("Disabled", TEXT_MUTED, SURFACE_ALT),
    }
}

/// Renders a colored chip badge for a skill scope.
pub fn scope_chip<'a>(scope: SkillScope) -> Container<'a, Message> {
    match scope {
        SkillScope::Project => chip("Project", PRIMARY, PRIMARY_SOFT),
        SkillScope::Global => chip("Global", INFO, INFO_SOFT),
        SkillScope::ClaudeCode => chip("Claude Code", CLAUDE_CODE, CLAUDE_CODE_SOFT),
        SkillScope::Droid => chip("Droid", DROID, DROID_SOFT),
        SkillScope::OpenCode => chip("OpenCode", OPENCODE, OPENCODE_SOFT),
        SkillScope::Codex => chip("Codex", CODEX, CODEX_SOFT),
        SkillScope::Zed => chip("Zed", ZED, ZED_SOFT),
        SkillScope::Custom => chip("Custom", TEXT_SECONDARY, SURFACE_ALT),
    }
}

/// Renders an empty state placeholder with title and body text.
pub fn empty_state<'a>(title: &'a str, body: &'a str) -> Container<'a, Message> {
    flat_card(
        column![
            text(title).size(FONT_BODY).color(TEXT),
            text(body).size(FONT_CAPTION).color(TEXT_MUTED),
        ]
        .spacing(SPACING_XS + 2.0),
    )
}

/// Builds a scrollable column from an iterator of items with a custom element mapper.
pub fn list_column<'a, T, F>(
    items: impl IntoIterator<Item = T>,
    spacing: f32,
    mut f: F,
) -> iced::widget::Column<'a, Message>
where
    F: FnMut(T) -> Element<'a, Message>,
{
    items
        .into_iter()
        .fold(column![].spacing(spacing), |list, item| list.push(f(item)))
}

/// Renders bulleted text lines from an iterator, with a fallback empty message.
pub fn bullet_lines<'a, I>(items: I, empty: &'a str) -> Element<'a, Message>
where
    I: IntoIterator<Item = String>,
{
    text_lines(
        items.into_iter().map(|item| format!("- {item}")),
        empty,
        TEXT_SECONDARY,
        FONT_CAPTION,
    )
}

/// Renders diagnostic entries as formatted text lines with severity prefixes.
pub fn diagnostic_lines<'a>(
    diagnostics: &'a [SkillDiagnostic],
    empty: &'a str,
) -> Element<'a, Message> {
    text_lines(
        diagnostics
            .iter()
            .map(|d| format!("- {}: {}", d.severity.label(), d.message)),
        empty,
        TEXT_SECONDARY,
        FONT_CAPTION,
    )
}

/// Renders generic text lines with custom color and font size, with a fallback empty message.
pub fn text_lines<'a, I>(
    lines: I,
    empty: &'a str,
    color: iced::Color,
    size: u32,
) -> Element<'a, Message>
where
    I: IntoIterator<Item = String>,
{
    let lines = lines.into_iter().collect::<Vec<_>>();
    if lines.is_empty() {
        return text(empty).size(size).color(TEXT_MUTED).into();
    }
    list_column(lines, SPACING_XS, move |line| {
        text(line)
            .size(size)
            .color(color)
            .wrapping(text::Wrapping::WordOrGlyph)
            .into()
    })
    .into()
}

fn chip<'a>(
    label: &'a str,
    foreground: iced::Color,
    background: iced::Color,
) -> Container<'a, Message> {
    container(text(label).size(FONT_MICRO).color(foreground))
        .padding([SPACING_XS, SPACING_SM + 2.0])
        .style(theme::chip(background, foreground))
}
