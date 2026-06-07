use std::path::PathBuf;

use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, pick_list, row, rule, scrollable, text},
};
use skills_manager_core::{InstalledSkill, SkillScope, format_bytes};

use crate::{
    app::{App, DetailTab, HealthFilter, Message, ScopeFilter, SortKey, SourceFilter},
    components, icons, theme,
};

use super::detail_row;

const TOOL_SECTION_ORDER: [SkillScope; 8] = [
    SkillScope::Codex,
    SkillScope::Zed,
    SkillScope::ClaudeCode,
    SkillScope::Droid,
    SkillScope::Pencode,
    SkillScope::Global,
    SkillScope::Project,
    SkillScope::Custom,
];

pub fn view(app: &App) -> Element<'_, Message> {
    let counts = app.counts();
    let visible = app.filtered_skills();
    let selected = app.selected_skill();
    let attention = counts.warning + counts.invalid + counts.shadowed;

    let metrics = row![
        components::compact_metric("Total", app.skills.len().to_string(), theme::PRIMARY),
        components::compact_metric("Usable", counts.exportable.to_string(), theme::CYAN),
        components::compact_metric("Enabled", counts.enabled.to_string(), theme::SUCCESS),
        components::compact_metric("Needs attention", attention.to_string(), theme::WARNING),
    ]
    .spacing(8);

    let filters = column![
        components::compact_field(
            "Search inventory",
            "Search local skills",
            &app.inventory.search_query,
            Message::SearchChanged,
        ),
        row![
            pick_list(
                ScopeFilter::ALL,
                Some(app.inventory.scope_filter),
                Message::ScopeFilterSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(
                HealthFilter::ALL,
                Some(app.inventory.health_filter),
                Message::HealthFilterSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(
                SourceFilter::ALL,
                Some(app.inventory.source_filter),
                Message::SourceFilterSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(
                SortKey::ALL,
                Some(app.inventory.sort_key),
                Message::SortSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        quick_filters(app),
    ]
    .spacing(10);

    let list = components::panel(
        column![
            components::section_header("Library", format!("{} matching", visible.len())),
            filters,
            table_header(app),
            scrollable(skill_list(app, &visible)).height(Length::Fill),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let selected_visible_scopes = selected
        .map(|skill| app.visible_scopes_for_skill(skill))
        .unwrap_or_default();
    let inspector = skill_inspector(
        selected,
        app.inventory.detail_tab,
        selected_visible_scopes,
        app.inventory.pending_remove_skill.as_ref(),
        app.busy,
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    column![
        metrics,
        target_dashboard(app),
        row![list, inspector].spacing(14).height(Length::Fill)
    ]
    .spacing(12)
    .height(Length::Fill)
    .into()
}

fn target_dashboard(app: &App) -> Element<'_, Message> {
    TOOL_SECTION_ORDER
        .into_iter()
        .fold(row![].spacing(8), |cards, scope| {
            cards.push(target_card(app, scope).width(Length::FillPortion(1)))
        })
        .into()
}

fn target_card<'a>(app: &'a App, scope: SkillScope) -> iced::widget::Button<'a, Message> {
    let total = app
        .skills
        .iter()
        .filter(|skill| skill.scope == scope)
        .count();
    let usable = app
        .skills
        .iter()
        .filter(|skill| skill.scope == scope && skill.is_exportable())
        .count();
    let disabled = app
        .skills
        .iter()
        .filter(|skill| skill.scope == scope && !skill.is_enabled())
        .count();
    let invalid = app
        .skills
        .iter()
        .filter(|skill| {
            skill.scope == scope && skill.health == skills_manager_core::SkillHealth::Invalid
        })
        .count();
    let filter = scope_filter_for_scope(scope);

    button(
        column![
            text(scope.label()).size(13).color(theme::TEXT),
            text(format!("{usable} usable / {disabled} disabled"))
                .size(11)
                .color(theme::MUTED),
            text(format!("{invalid} invalid / {total} managed"))
                .size(11)
                .color(theme::SUBTLE),
        ]
        .spacing(3),
    )
    .padding([8, 10])
    .style(theme::subtle_button(app.inventory.scope_filter == filter))
    .on_press(Message::ScopeFilterSelected(filter))
}

fn scope_filter_for_scope(scope: SkillScope) -> ScopeFilter {
    match scope {
        SkillScope::Project => ScopeFilter::Project,
        SkillScope::Global => ScopeFilter::Global,
        SkillScope::ClaudeCode => ScopeFilter::ClaudeCode,
        SkillScope::Droid => ScopeFilter::Droid,
        SkillScope::Pencode => ScopeFilter::Pencode,
        SkillScope::Codex => ScopeFilter::Codex,
        SkillScope::Zed => ScopeFilter::Zed,
        SkillScope::Custom => ScopeFilter::Custom,
    }
}

fn skill_list<'a>(app: &'a App, skills: &[&'a InstalledSkill]) -> Element<'a, Message> {
    if skills.is_empty() {
        return components::empty_state(
            "No skills found",
            "Try a different search, scope, health, or source filter.",
        )
        .into();
    }

    tool_scopes(skills)
        .into_iter()
        .fold(column![].spacing(14), |list, scope| {
            let scoped_skills = skills
                .iter()
                .copied()
                .filter(|skill| skill.scope == scope)
                .collect::<Vec<_>>();
            list.push(tool_section(app, scope, scoped_skills))
        })
        .into()
}

fn tool_section<'a>(
    app: &'a App,
    scope: SkillScope,
    skills: Vec<&'a InstalledSkill>,
) -> Element<'a, Message> {
    let section_meta = tool_section_meta(&skills);
    let rows = skills
        .into_iter()
        .fold(column![].spacing(8), |list, skill| {
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
            let visible_scopes = app.visible_scopes_for_skill(skill);
            list.push(skill_row(
                skill,
                visible_scopes,
                selected,
                pending_remove,
                app.busy,
            ))
        });

    column![
        row![
            components::scope_chip(scope),
            text(section_meta).size(12).color(theme::MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        rows,
    ]
    .spacing(8)
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

fn tool_section_meta(skills: &[&InstalledSkill]) -> String {
    let usable = skills.iter().filter(|skill| skill.is_exportable()).count();
    let disabled = skills.iter().filter(|skill| !skill.is_enabled()).count();
    let attention = skills
        .iter()
        .filter(|skill| {
            matches!(
                skill.health,
                skills_manager_core::SkillHealth::Warning
                    | skills_manager_core::SkillHealth::Invalid
                    | skills_manager_core::SkillHealth::Shadowed
            )
        })
        .count();

    format!(
        "{} usable / {} disabled / {} needs attention / {} managed",
        usable,
        disabled,
        attention,
        skills.len()
    )
}

fn skill_row<'a>(
    skill: &'a InstalledSkill,
    visible_scopes: Vec<SkillScope>,
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

    let details = if selected {
        components::primary_button("Details", Some(icons::EYE)).on_press(Message::SelectSkill(id))
    } else {
        components::secondary_button("Details", Some(icons::EYE)).on_press(Message::SelectSkill(id))
    };
    let remove_label = if pending_remove { "Confirm" } else { "Remove" };
    let remove_message = if pending_remove {
        Message::ConfirmRemoveSkill(skill_root)
    } else {
        Message::RequestRemoveSkill(skill_root)
    };
    let remove = components::danger_button(remove_label, Some(icons::TRASH))
        .on_press_maybe((!busy).then_some(remove_message));

    container(
        row![
            toggle,
            column![
                text(&skill.display_name).size(15).color(theme::TEXT),
                text(skill.destination_name())
                    .size(12)
                    .color(theme::SUBTLE)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(4)
            .width(Length::FillPortion(5)),
            column![
                components::health_chip(skill.health),
                components::enablement_chip(skill.enablement),
            ]
            .spacing(5)
            .width(Length::FillPortion(2)),
            column![
                text(visibility_label(&visible_scopes))
                    .size(11)
                    .color(theme::SUBTLE),
            ]
            .spacing(5)
            .width(Length::FillPortion(2)),
            column![
                text(skill.resource_count.to_string())
                    .size(13)
                    .color(theme::TEXT),
                text(format_bytes(skill.resource_bytes))
                    .size(11)
                    .color(theme::SUBTLE),
            ]
            .spacing(4)
            .width(Length::FillPortion(2)),
            row![details, remove]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Shrink),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([9, 10])
    .style(if selected {
        theme::selected_table_row
    } else {
        theme::table_row
    })
    .into()
}

fn table_header(app: &App) -> Element<'_, Message> {
    container(
        row![
            text("").width(Length::Fixed(22.0)),
            sort_header("Skill", SortKey::Name, app.inventory.sort_key)
                .width(Length::FillPortion(5)),
            sort_header("State", SortKey::Health, app.inventory.sort_key)
                .width(Length::FillPortion(2)),
            sort_header("Visibility", SortKey::Scope, app.inventory.sort_key)
                .width(Length::FillPortion(2)),
            sort_header("Resources", SortKey::Resources, app.inventory.sort_key)
                .width(Length::FillPortion(2)),
            text("Actions")
                .size(11)
                .color(theme::SUBTLE)
                .width(Length::Shrink),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([7, 10])
    .style(theme::table_header)
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
            format!("{label} sorted")
        } else {
            label.to_string()
        })
        .size(11),
    )
    .padding([4, 6])
    .style(theme::subtle_button(active))
    .on_press(Message::SortSelected(key))
}

fn quick_filters(app: &App) -> Element<'_, Message> {
    let counts = app.counts();
    row![
        quick_filter(
            "Needs attention",
            counts.warning + counts.invalid + counts.shadowed,
            HealthFilter::NeedsAttention,
            app.inventory.health_filter,
        ),
        quick_filter(
            "Invalid",
            counts.invalid,
            HealthFilter::Invalid,
            app.inventory.health_filter,
        ),
        quick_filter(
            "Warnings",
            counts.warning,
            HealthFilter::Warning,
            app.inventory.health_filter,
        ),
        quick_filter(
            "Shadowed",
            counts.shadowed,
            HealthFilter::Shadowed,
            app.inventory.health_filter,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn quick_filter<'a>(
    label: &'a str,
    count: usize,
    filter: HealthFilter,
    selected: HealthFilter,
) -> iced::widget::Button<'a, Message> {
    button(text(format!("{label} {count}")).size(12))
        .padding([6, 9])
        .style(theme::subtle_button(filter == selected))
        .on_press(Message::HealthFilterSelected(filter))
}

fn skill_inspector<'a>(
    skill: Option<&'a InstalledSkill>,
    detail_tab: DetailTab,
    visible_scopes: Vec<SkillScope>,
    pending_remove_skill: Option<&'a PathBuf>,
    busy: bool,
) -> iced::widget::Container<'a, Message> {
    match skill {
        Some(skill) => components::panel(
            scrollable(
                column![
                    components::section_header("Inspector", "Selected skill"),
                    row![
                        components::health_chip(skill.health),
                        components::scope_chip(skill.scope),
                        components::enablement_chip(skill.enablement),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text(&skill.display_name).size(22).color(theme::TEXT),
                    detail_tabs(detail_tab),
                    rule::horizontal(1),
                    inspector_tab_content(
                        skill,
                        detail_tab,
                        visible_scopes,
                        pending_remove_skill,
                        busy
                    ),
                ]
                .spacing(10),
            )
            .height(Length::Fill),
        ),
        None => components::panel(
            column![
                components::section_header("Inspector", "No selection"),
                components::empty_state(
                    "No skill selected",
                    "Select a skill to inspect metadata, diagnostics, and paths."
                ),
            ]
            .spacing(12),
        ),
    }
}

fn detail_tabs(current: DetailTab) -> Element<'static, Message> {
    DetailTab::ALL
        .into_iter()
        .fold(row![].spacing(6), |tabs, tab| {
            tabs.push(
                button(text(tab.label()).size(12))
                    .padding([6, 8])
                    .style(theme::subtle_button(tab == current))
                    .on_press(Message::DetailTabSelected(tab)),
            )
        })
        .into()
}

fn inspector_tab_content<'a>(
    skill: &'a InstalledSkill,
    detail_tab: DetailTab,
    visible_scopes: Vec<SkillScope>,
    pending_remove_skill: Option<&'a PathBuf>,
    busy: bool,
) -> Element<'a, Message> {
    match detail_tab {
        DetailTab::Overview => column![
            detail_row(
                "Description",
                skill
                    .description
                    .as_deref()
                    .unwrap_or("No description")
                    .to_string()
            ),
            detail_row("Current target", skill.scope.label().to_string()),
            detail_row(
                "Source",
                skill.source_url.as_deref().unwrap_or("Unknown").to_string()
            ),
            detail_row(
                "Installed",
                skill
                    .installed_at
                    .map(|time| time.to_rfc3339())
                    .unwrap_or_else(|| "Unknown".to_string()),
            ),
            detail_row(
                "License",
                skill
                    .frontmatter
                    .license
                    .as_deref()
                    .unwrap_or("Unknown")
                    .to_string()
            ),
            detail_row(
                "Compatibility",
                skill
                    .frontmatter
                    .compatibility
                    .as_deref()
                    .unwrap_or("Not declared")
                    .to_string(),
            ),
            detail_row(
                "Tags",
                if skill.frontmatter.tags.is_empty() {
                    "None declared".to_string()
                } else {
                    skill.frontmatter.tags.join(", ")
                }
            ),
        ]
        .spacing(10)
        .into(),
        DetailTab::Visibility => column![
            visibility_detail(visible_scopes),
            detail_row("Folder", skill.root_dir.display().to_string()),
            detail_row("Skill file", skill.skill_file.display().to_string()),
            detail_row("Enablement", skill.enablement.label().to_string()),
            detail_row("Health", skill.health.label().to_string()),
        ]
        .spacing(10)
        .into(),
        DetailTab::Diagnostics => column![diagnostics_block(skill), metadata_block(skill)]
            .spacing(10)
            .into(),
        DetailTab::Files => column![
            detail_row(
                "Resources",
                format!(
                    "{} file(s), {}",
                    skill.resource_count,
                    format_bytes(skill.resource_bytes)
                ),
            ),
            detail_row(
                "Allowed tools",
                if skill.frontmatter.allowed_tools.is_empty() {
                    "None declared".to_string()
                } else {
                    skill.frontmatter.allowed_tools.join(", ")
                },
            ),
            detail_row(
                "When to use",
                skill
                    .frontmatter
                    .when_to_use
                    .as_deref()
                    .unwrap_or("Not declared")
                    .to_string(),
            ),
            detail_row(
                "Disable model invocation",
                skill
                    .frontmatter
                    .disable_model_invocation
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "Not declared".to_string()),
            ),
        ]
        .spacing(10)
        .into(),
        DetailTab::Actions => actions_tab(skill, pending_remove_skill, busy),
    }
}

fn actions_tab<'a>(
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
    let remove_label = if pending_remove {
        "Confirm remove"
    } else {
        "Remove"
    };
    let remove_message = if pending_remove {
        Message::ConfirmRemoveSkill(skill.root_dir.clone())
    } else {
        Message::RequestRemoveSkill(skill.root_dir.clone())
    };

    column![
        text("Enable/disable applies to this target. Directory-scanned targets move disabled skills outside the scanned root.")
            .size(12)
            .color(theme::MUTED)
            .wrapping(text::Wrapping::WordOrGlyph),
        row![
            components::primary_button(toggle_label, Some(toggle_icon)).on_press_maybe(
                (!busy).then_some(Message::SetSkillEnabled(
                    skill.root_dir.clone(),
                    !skill.is_enabled(),
                ))
            ),
            components::danger_button(remove_label, Some(icons::TRASH))
                .on_press_maybe((!busy).then_some(remove_message)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .into()
}

fn visibility_label(scopes: &[SkillScope]) -> String {
    if scopes.is_empty() {
        return "Not visible in any target".to_string();
    }

    format!(
        "Visible in {}",
        scopes
            .iter()
            .map(|scope| scope.label())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn visibility_detail<'a>(scopes: Vec<SkillScope>) -> iced::widget::Column<'a, Message> {
    column![
        text("Visible in").size(11).color(theme::SUBTLE),
        visibility_chips(scopes),
    ]
    .spacing(5)
}

fn visibility_chips<'a>(scopes: Vec<SkillScope>) -> Element<'a, Message> {
    if scopes.is_empty() {
        return text("Not visible in any target")
            .size(13)
            .color(theme::MUTED)
            .into();
    }

    scopes
        .into_iter()
        .fold(
            row![].spacing(5).align_y(Alignment::Center),
            |chips, scope| chips.push(components::scope_chip(scope)),
        )
        .into()
}

fn diagnostics_block(skill: &InstalledSkill) -> Element<'_, Message> {
    if skill.diagnostics.is_empty() {
        return detail_row("Diagnostics", "No diagnostics".to_string()).into();
    }

    skill
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

fn metadata_block(skill: &InstalledSkill) -> Element<'_, Message> {
    if skill.frontmatter.metadata.is_empty() {
        return detail_row("Metadata", "No extra metadata".to_string()).into();
    }

    skill
        .frontmatter
        .metadata
        .iter()
        .fold(
            column![text("Metadata").size(11).color(theme::SUBTLE)].spacing(3),
            |list, (key, value)| {
                list.push(text(format!("{key}: {value}")).size(12).color(theme::MUTED))
            },
        )
        .into()
}
