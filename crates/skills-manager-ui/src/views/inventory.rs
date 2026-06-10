//! Library view for browsing, filtering, and managing installed skills and resources.
//!
//! Renders the main inventory table with scope-grouped skill rows, filter
//! and sort controls, a detail inspector panel, and resource list views
//! for plugins and marketplaces.

use std::path::PathBuf;

use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
use skills_manager_core::{InstalledSkill, SkillScope, format_bytes};

use crate::theme::*;
use crate::{
    app::{App, HealthFilter, Message, ScopeFilter, SortKey, SourceFilter},
    components, icons, theme,
};

const TOOL_SECTION_ORDER: [SkillScope; 8] = [
    SkillScope::Codex,
    SkillScope::Zed,
    SkillScope::ClaudeCode,
    SkillScope::Droid,
    SkillScope::OpenCode,
    SkillScope::Global,
    SkillScope::Project,
    SkillScope::Custom,
];

/// Renders the inventory/library view with skill list, filters, and detail inspector.
pub fn view(app: &App) -> Element<'_, Message> {
    let counts = app.counts();
    let visible = app.filtered_skills();
    let selected = app.selected_skill();
    let attention = counts.warning + counts.invalid + counts.shadowed;

    let summary = row![
        components::summary_stat("skills", app.skills.len().to_string(), PRIMARY),
        text("\u{00B7}").size(FONT_BODY).color(TEXT_MUTED),
        components::summary_stat("enabled", counts.enabled.to_string(), SUCCESS),
        text("\u{00B7}").size(FONT_BODY).color(TEXT_MUTED),
        components::summary_stat("need attention", attention.to_string(), WARNING),
        text("\u{00B7}").size(FONT_BODY).color(TEXT_MUTED),
        components::summary_stat("exportable", counts.exportable.to_string(), INFO),
    ]
    .spacing(SPACING_SM)
    .align_y(Alignment::Center);

    let filters = filter_bar(app);

    let list = components::card(
        column![
            filters,
            table_header(app),
            scrollable(skill_list(app, &visible)).height(Length::Fill),
        ]
        .spacing(SPACING_MD),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let selected_visible_scopes = selected
        .map(|skill| app.visible_scopes_for_skill(skill))
        .unwrap_or_default();
    let inspector = skill_inspector(
        selected,
        selected_visible_scopes,
        app.inventory.pending_remove_skill.as_ref(),
        app.busy,
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    column![
        summary,
        row![list, inspector]
            .spacing(SPACING_LG)
            .height(Length::Fill)
    ]
    .spacing(SPACING_MD)
    .height(Length::Fill)
    .into()
}

fn filter_bar(app: &App) -> Element<'_, Message> {
    row![
        text_input("Filter skills...", &app.inventory.skill_search_query)
            .on_input(Message::SkillSearchChanged)
            .padding([SPACING_SM, SPACING_MD])
            .style(theme::input)
            .width(Length::FillPortion(2)),
        container(components::styled_pick_list(
            &ScopeFilter::ALL,
            Some(app.inventory.scope_filter),
            Message::ScopeFilterSelected,
            Length::Fill,
        ))
        .width(Length::FillPortion(1)),
        container(components::styled_pick_list(
            &HealthFilter::ALL,
            Some(app.inventory.health_filter),
            Message::HealthFilterSelected,
            Length::Fill,
        ))
        .width(Length::FillPortion(1)),
        container(components::styled_pick_list(
            &SourceFilter::ALL,
            Some(app.inventory.source_filter),
            Message::SourceFilterSelected,
            Length::Fill,
        ))
        .width(Length::FillPortion(1)),
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center)
    .into()
}

fn skill_list<'a>(app: &'a App, skills: &[&'a InstalledSkill]) -> Element<'a, Message> {
    if skills.is_empty() {
        return components::empty_state("No skills found", "Try adjusting your search or filters.")
            .into();
    }

    components::list_column(tool_scopes(skills), SPACING_LG, |scope| {
        let scoped_skills = skills
            .iter()
            .copied()
            .filter(|skill| skill.scope == scope)
            .collect::<Vec<_>>();
        tool_section(app, scope, scoped_skills)
    })
    .into()
}

fn tool_section<'a>(
    app: &'a App,
    scope: SkillScope,
    skills: Vec<&'a InstalledSkill>,
) -> Element<'a, Message> {
    let count = skills.len();
    let rows = components::list_column(skills, SPACING_SM, |skill| {
        let selected = app
            .inventory
            .selected_skill_id
            .as_ref()
            .is_some_and(|id| id == &skill.id);
        let pending_remove = app
            .inventory
            .pending_remove_skill
            .as_ref()
            .is_some_and(|path| path == &skill.root_dir);
        skill_row(skill, selected, pending_remove, app.busy)
    });

    column![
        row![
            components::scope_chip(scope),
            text(format!("{} skill(s)", count))
                .size(FONT_CAPTION)
                .color(TEXT_MUTED),
        ]
        .spacing(SPACING_SM)
        .align_y(Alignment::Center),
        rows,
    ]
    .spacing(SPACING_SM)
    .into()
}

fn tool_scopes(skills: &[&InstalledSkill]) -> Vec<SkillScope> {
    let mut scopes = Vec::new();
    for scope in TOOL_SECTION_ORDER {
        if skills.iter().any(|skill| skill.scope == scope) {
            scopes.push(scope);
        }
    }
    for skill in skills {
        if !scopes.contains(&skill.scope) {
            scopes.push(skill.scope);
        }
    }
    scopes
}

fn skill_row<'a>(
    skill: &'a InstalledSkill,
    selected: bool,
    pending_remove: bool,
    busy: bool,
) -> Element<'a, Message> {
    let skill_root = skill.root_dir.clone();
    let toggle_root = skill.root_dir.clone();
    let id = skill.id.clone();
    let enabled = skill.is_enabled();
    let toggle = checkbox(enabled).size(18).on_toggle_maybe(
        (!busy).then_some(move |checked| Message::SetSkillEnabled(toggle_root.clone(), checked)),
    );

    let select =
        components::small_ghost_button("View", Some(icons::EYE)).on_press(Message::SelectSkill(id));
    let remove = components::confirm_button(
        pending_remove,
        "Remove",
        "Confirm",
        Some(icons::TRASH),
        Message::RequestRemoveSkill(skill_root.clone()),
        Message::ConfirmRemoveSkill(skill_root),
        busy,
    );

    container(
        row![
            toggle,
            column![
                text(&skill.display_name).size(FONT_BODY).color(TEXT),
                text(skill.destination_name())
                    .size(FONT_MICRO)
                    .color(TEXT_MUTED)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            components::health_dot(skill.health),
            row![select, remove]
                .spacing(SPACING_SM)
                .align_y(Alignment::Center),
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .padding([SPACING_MD, SPACING_MD + 2.0])
    .style(if selected {
        theme::selected_card
    } else {
        theme::card
    })
    .into()
}

fn table_header(app: &App) -> Element<'_, Message> {
    components::flat_card(
        row![
            text("").width(Length::Fixed(24.0)),
            sort_header("Skill", SortKey::Name, app.inventory.sort_key).width(Length::Fill),
            sort_header("Health", SortKey::Health, app.inventory.sort_key),
            sort_header("Size", SortKey::Resources, app.inventory.sort_key),
            text("Actions").size(FONT_MICRO).color(TEXT_MUTED),
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .into()
}

fn sort_header<'a>(
    label: &'a str,
    key: SortKey,
    current: SortKey,
) -> iced::widget::Button<'a, Message> {
    let active = key == current;
    button(
        text(if active {
            format!("{label} \u{2193}")
        } else {
            label.to_string()
        })
        .size(FONT_MICRO),
    )
    .padding([SPACING_XS, SPACING_SM])
    .style(theme::pill_button(active))
    .on_press(Message::SortSelected(key))
}

fn skill_inspector<'a>(
    skill: Option<&'a InstalledSkill>,
    visible_scopes: Vec<SkillScope>,
    pending_remove_skill: Option<&'a PathBuf>,
    busy: bool,
) -> iced::widget::Container<'a, Message> {
    match skill {
        Some(skill) => components::card(
            scrollable(
                column![
                    row![
                        components::health_dot(skill.health),
                        components::scope_chip(skill.scope),
                        components::enablement_chip(skill.enablement),
                    ]
                    .spacing(SPACING_SM)
                    .align_y(Alignment::Center),
                    text(&skill.display_name).size(FONT_DISPLAY).color(TEXT),
                    inspector_section("OVERVIEW", overview_section(skill)),
                    inspector_section("FILES & PATHS", files_section(skill, visible_scopes)),
                    inspector_section("DIAGNOSTICS", diagnostics_section(skill)),
                    inspector_section(
                        "ACTIONS",
                        actions_section(skill, pending_remove_skill, busy)
                    ),
                ]
                .spacing(SPACING_LG),
            )
            .height(Length::Fill),
        ),
        None => components::card(
            column![
                components::section_header("Inspector", "No selection"),
                components::empty_state(
                    "No skill selected",
                    "Select a skill to view details, diagnostics, and actions."
                ),
            ]
            .spacing(SPACING_MD),
        ),
    }
}

fn inspector_section<'a>(label: &'a str, content: Element<'a, Message>) -> Element<'a, Message> {
    column![components::section_label(label), content,]
        .spacing(SPACING_SM)
        .into()
}

fn overview_section<'a>(skill: &'a InstalledSkill) -> Element<'a, Message> {
    column![
        components::detail_row(
            "Description",
            skill.description.as_deref().unwrap_or("No description"),
        ),
        components::detail_row("Source", skill.source_url.as_deref().unwrap_or("Unknown"),),
        components::detail_row(
            "Installed",
            skill
                .installed_at
                .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
        ),
        components::detail_row(
            "License",
            skill.frontmatter.license.as_deref().unwrap_or("Unknown"),
        ),
        components::detail_row(
            "Compatibility",
            skill
                .frontmatter
                .compatibility
                .as_deref()
                .unwrap_or("Not declared"),
        ),
        components::detail_row(
            "Tags",
            if skill.frontmatter.tags.is_empty() {
                "None".to_string()
            } else {
                skill.frontmatter.tags.join(", ")
            },
        ),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn files_section<'a>(
    skill: &'a InstalledSkill,
    visible_scopes: Vec<SkillScope>,
) -> Element<'a, Message> {
    column![
        components::detail_row(
            "Resources",
            format!(
                "{} file(s), {}",
                skill.resource_count,
                format_bytes(skill.resource_bytes)
            ),
        ),
        components::detail_row("Folder", skill.root_dir.display().to_string()),
        components::detail_row("Skill file", skill.skill_file.display().to_string()),
        components::detail_row("Enablement", skill.enablement.label().to_string()),
        visibility_detail(visible_scopes),
        components::detail_row(
            "Allowed tools",
            if skill.frontmatter.allowed_tools.is_empty() {
                "None".to_string()
            } else {
                skill.frontmatter.allowed_tools.join(", ")
            },
        ),
        components::detail_row(
            "When to use",
            skill
                .frontmatter
                .when_to_use
                .as_deref()
                .unwrap_or("Not declared"),
        ),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn diagnostics_section<'a>(skill: &'a InstalledSkill) -> Element<'a, Message> {
    column![
        components::diagnostic_lines(&skill.diagnostics, "No diagnostics"),
        metadata_block(skill),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn actions_section<'a>(
    skill: &'a InstalledSkill,
    pending_remove_skill: Option<&'a PathBuf>,
    busy: bool,
) -> Element<'a, Message> {
    let toggle_label = if skill.is_enabled() {
        "Disable"
    } else {
        "Enable"
    };
    let toggle_icon = if skill.is_enabled() {
        icons::EYE_OFF
    } else {
        icons::EYE
    };
    let pending_remove = pending_remove_skill.is_some_and(|path| path == &skill.root_dir);
    let remove = components::confirm_button(
        pending_remove,
        "Remove",
        "Confirm remove",
        Some(icons::TRASH),
        Message::RequestRemoveSkill(skill.root_dir.clone()),
        Message::ConfirmRemoveSkill(skill.root_dir.clone()),
        busy,
    );

    row![
        components::primary_button(toggle_label, Some(toggle_icon)).on_press_maybe(
            (!busy).then_some(Message::SetSkillEnabled(
                skill.root_dir.clone(),
                !skill.is_enabled(),
            ))
        ),
        remove,
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center)
    .into()
}

fn visibility_detail<'a>(scopes: Vec<SkillScope>) -> iced::widget::Column<'a, Message> {
    column![
        text("Visible in").size(FONT_MICRO).color(TEXT_MUTED),
        visibility_chips(scopes),
    ]
    .spacing(SPACING_XS + 2.0)
}

fn visibility_chips<'a>(scopes: Vec<SkillScope>) -> Element<'a, Message> {
    if scopes.is_empty() {
        return text("Not visible in any target")
            .size(FONT_CAPTION)
            .color(TEXT_MUTED)
            .into();
    }

    scopes
        .into_iter()
        .fold(
            row![].spacing(SPACING_XS + 2.0).align_y(Alignment::Center),
            |chips, scope| chips.push(components::scope_chip(scope)),
        )
        .into()
}

fn metadata_block(skill: &InstalledSkill) -> Element<'_, Message> {
    let entries: Vec<String> = skill
        .frontmatter
        .metadata
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect();

    if entries.is_empty() {
        return components::detail_row("Metadata", "No extra metadata").into();
    }

    components::detail_section(
        "Metadata",
        components::text_lines(entries, "No extra metadata", TEXT_SECONDARY, FONT_CAPTION),
    )
}
