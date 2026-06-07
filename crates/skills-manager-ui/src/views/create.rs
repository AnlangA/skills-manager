use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, row, scrollable, text},
};
use skills_manager_core::{SkillScaffoldPreview, format_bytes};

use crate::theme::*;
use crate::{
    app::{App, Message, UiScope},
    components, icons,
};

use super::strategy_label;

pub fn view(app: &App) -> Element<'_, Message> {
    let can_preview = !app.busy
        && !app.create.name.trim().is_empty()
        && !app.create.description.trim().is_empty()
        && (app.create.target != UiScope::Custom || !app.create.custom_path.trim().is_empty());
    let can_create = can_preview && app.create.preview.is_some();

    let form = components::card(scrollable(
        column![
            components::section_header("New Skill", "Scaffold a SKILL.md for a target"),
            components::field(
                "Name",
                "Lowercase, numbers, hyphens, or underscores.",
                "my-skill",
                &app.create.name,
                Message::CreateNameChanged,
            ),
            components::field(
                "Description",
                "What the skill does and when to use it.",
                "Use this skill when...",
                &app.create.description,
                Message::CreateDescriptionChanged,
            ),
            target_controls(app),
            advanced_section(app),
            row![
                components::primary_button("Preview", Some(icons::SEARCH))
                    .on_press_maybe(can_preview.then_some(Message::PreviewScaffold)),
                components::primary_button("Create", Some(icons::FILE))
                    .on_press_maybe(can_create.then_some(Message::CreateSkill)),
            ]
            .spacing(SPACING_MD),
        ]
        .spacing(SPACING_LG),
    ))
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let preview = components::card(
        column![
            components::section_header("Preview", preview_meta(app.create.preview.as_ref())),
            scrollable(preview_body(app.create.preview.as_ref())).height(Length::Fill),
        ]
        .spacing(SPACING_MD),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    components::form_preview_layout(form, preview)
}

fn target_controls(app: &App) -> Element<'_, Message> {
    let mut controls = column![
        components::section_label("TARGET"),
        components::styled_pick_list(
            &UiScope::ALL,
            Some(app.create.target),
            Message::CreateTargetSelected,
            Length::Fill,
        ),
    ]
    .spacing(SPACING_XS + 2.0);

    if app.create.target == UiScope::Custom {
        controls = controls.push(components::compact_field(
            "Custom root",
            "/path/to/skills/root",
            &app.create.custom_path,
            Message::CreateCustomPathChanged,
        ));
    }

    if let Some(profile) = app
        .settings
        .target_profiles
        .iter()
        .find(|profile| profile.scope.label() == app.create.target.to_string())
    {
        controls = controls.push(
            text(format!(
                "{} at {}",
                strategy_label(profile.enablement_strategy),
                profile
                    .skills_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "unavailable".to_string())
            ))
            .size(FONT_MICRO)
            .color(TEXT_MUTED)
            .wrapping(text::Wrapping::WordOrGlyph),
        );
    }

    controls.into()
}

fn advanced_section(app: &App) -> Element<'_, Message> {
    column![
        components::section_label("ADVANCED OPTIONS"),
        components::field(
            "Tags",
            "Comma-separated discovery tags.",
            "analysis,docs,automation",
            &app.create.tags,
            Message::CreateTagsChanged,
        ),
        components::field(
            "Allowed tools",
            "Comma-separated tools.",
            "shell,browser",
            &app.create.allowed_tools,
            Message::CreateAllowedToolsChanged,
        ),
        components::field(
            "When to use",
            "Claude Code trigger guidance.",
            "Use when the user asks to...",
            &app.create.when_to_use,
            Message::CreateWhenToUseChanged,
        ),
        row![
            iced::widget::container(components::compact_field(
                "Compatibility",
                "Claude Code, Codex, Zed",
                &app.create.compatibility,
                Message::CreateCompatibilityChanged,
            ))
            .width(Length::FillPortion(1)),
            iced::widget::container(components::compact_field(
                "License",
                "MIT",
                &app.create.license,
                Message::CreateLicenseChanged,
            ))
            .width(Length::FillPortion(1)),
        ]
        .spacing(SPACING_MD),
        row![
            checkbox(app.create.disable_model_invocation)
                .size(18)
                .on_toggle(Message::CreateDisableModelInvocationChanged),
            text("Disable model invocation").size(FONT_BODY).color(TEXT),
        ]
        .spacing(SPACING_SM)
        .align_y(Alignment::Center),
    ]
    .spacing(SPACING_MD)
    .into()
}

fn preview_meta(preview: Option<&SkillScaffoldPreview>) -> String {
    preview.map_or_else(
        || "No scaffold previewed".to_string(),
        |preview| {
            format!(
                "{} [{}] -> {}",
                preview
                    .frontmatter
                    .name
                    .as_deref()
                    .unwrap_or("unnamed skill"),
                preview.scope.label(),
                preview.destination_root.display()
            )
        },
    )
}

fn preview_body(preview: Option<&SkillScaffoldPreview>) -> Element<'_, Message> {
    let Some(preview) = preview else {
        return components::empty_state(
            "No preview yet",
            "Fill in a name and description, then preview.",
        )
        .into();
    };

    column![
        row![
            components::health_dot(preview.health),
            components::scope_chip(preview.scope),
        ]
        .spacing(SPACING_SM)
        .align_y(Alignment::Center),
        components::detail_row("Folder", preview.destination_root.display().to_string()),
        components::detail_row("Skill file", preview.skill_file.display().to_string()),
        components::detail_row("Resources", format!("0 file(s), {}", format_bytes(0))),
        components::diagnostic_lines(&preview.diagnostics, "No diagnostics"),
        components::flat_card(
            text(&preview.content)
                .size(FONT_CAPTION)
                .color(TEXT)
                .wrapping(text::Wrapping::WordOrGlyph)
        ),
    ]
    .spacing(SPACING_MD)
    .into()
}
