use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, pick_list, row, scrollable, text},
};
use skills_manager_core::{SkillScaffoldPreview, format_bytes};

use crate::{
    app::{App, Message, UiScope},
    components, icons, theme,
};

use super::detail_row;

pub fn view(app: &App) -> Element<'_, Message> {
    let can_preview = !app.busy
        && !app.create.name.trim().is_empty()
        && !app.create.description.trim().is_empty()
        && (app.create.target != UiScope::Custom || !app.create.custom_path.trim().is_empty());
    let can_create = can_preview && app.create.preview.is_some();

    let form = components::panel(scrollable(
        column![
            components::section_header(
                "Create skill",
                "Scaffold a target-aware SKILL.md before installing or sharing it"
            ),
            components::field(
                "Name",
                "Use lowercase letters, numbers, hyphens, or underscores.",
                "my-skill",
                &app.create.name,
                Message::CreateNameChanged,
            ),
            components::field(
                "Description",
                "Explain what the skill does and when the agent should use it.",
                "Use this skill when...",
                &app.create.description,
                Message::CreateDescriptionChanged,
            ),
            target_controls(app),
            components::field(
                "Tags",
                "Comma-separated discovery tags for Codex/Zed-compatible metadata.",
                "analysis,docs,automation",
                &app.create.tags,
                Message::CreateTagsChanged,
            ),
            components::field(
                "Allowed tools",
                "Comma-separated tools for agents that honor allowed-tools.",
                "shell,browser",
                &app.create.allowed_tools,
                Message::CreateAllowedToolsChanged,
            ),
            components::field(
                "When to use",
                "Claude Code specific trigger guidance.",
                "Use when the user asks to...",
                &app.create.when_to_use,
                Message::CreateWhenToUseChanged,
            ),
            components::field(
                "Compatibility",
                "Optional compatibility notes.",
                "Claude Code, Codex, Zed",
                &app.create.compatibility,
                Message::CreateCompatibilityChanged,
            ),
            components::field(
                "License",
                "Optional license string.",
                "MIT",
                &app.create.license,
                Message::CreateLicenseChanged,
            ),
            row![
                checkbox(app.create.disable_model_invocation)
                    .size(18)
                    .on_toggle(Message::CreateDisableModelInvocationChanged),
                text("Disable model invocation").size(13).color(theme::TEXT),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                components::primary_button("Preview", Some(icons::SEARCH))
                    .on_press_maybe(can_preview.then_some(Message::PreviewScaffold)),
                components::primary_button("Create", Some(icons::FILE))
                    .on_press_maybe(can_create.then_some(Message::CreateSkill)),
            ]
            .spacing(8),
        ]
        .spacing(12),
    ))
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let preview = components::panel(
        column![
            components::section_header("Preview", preview_meta(app.create.preview.as_ref())),
            scrollable(preview_body(app.create.preview.as_ref())).height(Length::Fill),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    row![form, preview].spacing(14).height(Length::Fill).into()
}

fn target_controls(app: &App) -> Element<'_, Message> {
    let mut controls = column![
        text("Target").size(12).color(theme::TEXT),
        pick_list(
            UiScope::ALL,
            Some(app.create.target),
            Message::CreateTargetSelected,
        )
        .padding([9, 12])
        .style(theme::select)
        .width(Length::Fill),
    ]
    .spacing(5);

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
                "{} strategy at {}",
                strategy_label(profile.enablement_strategy),
                profile
                    .skills_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "unavailable".to_string())
            ))
            .size(11)
            .color(theme::SUBTLE)
            .wrapping(text::Wrapping::WordOrGlyph),
        );
    }

    controls.into()
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
            "Fill in a name and description, then preview the generated SKILL.md.",
        )
        .into();
    };

    column![
        row![
            components::health_chip(preview.health),
            components::scope_chip(preview.scope),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        detail_row("Folder", preview.destination_root.display().to_string()),
        detail_row("Skill file", preview.skill_file.display().to_string()),
        detail_row("Resources", format!("0 file(s), {}", format_bytes(0)),),
        scaffold_diagnostics(preview),
        components::flat_panel(
            text(&preview.content)
                .size(12)
                .color(theme::TEXT)
                .wrapping(text::Wrapping::WordOrGlyph)
        ),
    ]
    .spacing(10)
    .into()
}

fn scaffold_diagnostics(preview: &SkillScaffoldPreview) -> Element<'_, Message> {
    if preview.diagnostics.is_empty() {
        return text("No diagnostics").size(12).color(theme::SUBTLE).into();
    }

    preview
        .diagnostics
        .iter()
        .fold(
            column![text("Diagnostics").size(11).color(theme::SUBTLE)].spacing(3),
            |list, diagnostic| {
                list.push(
                    text(format!(
                        "- {}: {}",
                        diagnostic.severity.label(),
                        diagnostic.message
                    ))
                    .size(12)
                    .color(theme::MUTED),
                )
            },
        )
        .into()
}

fn strategy_label(strategy: skills_manager_core::EnablementStrategy) -> &'static str {
    match strategy {
        skills_manager_core::EnablementStrategy::ConfigToggle => "Config toggle",
        skills_manager_core::EnablementStrategy::DirectoryMove => "Directory move",
    }
}
