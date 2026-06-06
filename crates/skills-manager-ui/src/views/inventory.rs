use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, pick_list, row, rule, scrollable, text},
};
use skills_manager_core::{InstalledSkill, format_bytes};

use crate::{
    app::{App, HealthFilter, Message, ScopeFilter, SortKey, SourceFilter},
    components, icons, theme,
};

use super::{detail_row, source_summary};

pub fn view(app: &App) -> Element<'_, Message> {
    let counts = app.counts();
    let visible = app.filtered_skills();
    let selected = app.selected_skill();

    let metrics = row![
        components::metric("Total", app.skills.len().to_string(), theme::PRIMARY),
        components::metric("Exportable", counts.exportable.to_string(), theme::CYAN),
        components::metric("Enabled", counts.enabled.to_string(), theme::SUCCESS),
        components::metric("Disabled", counts.disabled.to_string(), theme::SUBTLE),
        components::metric("Warning", counts.warning.to_string(), theme::WARNING),
        components::metric("Invalid", counts.invalid.to_string(), theme::DANGER),
    ]
    .spacing(8);

    let coverage = row![
        components::metric("Project", counts.project.to_string(), theme::PRIMARY),
        components::metric("User", counts.user.to_string(), theme::CYAN),
        components::metric(
            "Known Source",
            counts.known_source.to_string(),
            theme::SUCCESS
        ),
        components::metric("Shadowed", counts.shadowed.to_string(), theme::SUBTLE),
    ]
    .spacing(8);

    let filters = column![
        components::field(
            "Search inventory",
            "Matches skill name, description, folder, and allowed tools.",
            "Search local skills",
            &app.search_query,
            Message::SearchChanged,
        ),
        row![
            pick_list(
                ScopeFilter::ALL,
                Some(app.scope_filter),
                Message::ScopeFilterSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(
                HealthFilter::ALL,
                Some(app.health_filter),
                Message::HealthFilterSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(
                SourceFilter::ALL,
                Some(app.source_filter),
                Message::SourceFilterSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(SortKey::ALL, Some(app.sort_key), Message::SortSelected)
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
            components::section_header("Inventory", format!("{} matching", visible.len())),
            filters,
            table_header(app),
            scrollable(skill_list(app, &visible)).height(Length::Fill),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let inspector = skill_inspector(selected, app.busy)
        .width(Length::FillPortion(2))
        .height(Length::Fill);

    column![
        metrics,
        coverage,
        row![list, inspector].spacing(14).height(Length::Fill)
    ]
    .spacing(14)
    .height(Length::Fill)
    .into()
}

fn skill_list<'a>(app: &'a App, skills: &[&'a InstalledSkill]) -> Element<'a, Message> {
    if skills.is_empty() {
        return components::empty_state(
            "No skills found",
            "Try a different search, scope, health, or source filter.",
        )
        .into();
    }

    skills
        .iter()
        .fold(column![].spacing(8), |list, skill| {
            let selected = app
                .selected_skill_id
                .as_ref()
                .is_some_and(|id| id == &skill.id);
            list.push(skill_row(skill, selected, app.busy))
        })
        .into()
}

fn skill_row<'a>(skill: &'a InstalledSkill, selected: bool, busy: bool) -> Element<'a, Message> {
    let skill_file = skill.skill_file.clone();
    let skill_root = skill.root_dir.clone();
    let id = skill.id.clone();
    let enabled = skill.is_enabled();
    let toggle = checkbox(enabled).size(18).on_toggle_maybe(
        (!busy).then_some(move |checked| Message::SetSkillEnabled(skill_file.clone(), checked)),
    );

    let details = components::secondary_button("Details", Some(icons::EYE))
        .on_press(Message::SelectSkill(id));
    let remove = components::danger_button("Remove", Some(icons::TRASH))
        .on_press_maybe((!busy).then_some(Message::RemoveSkill(skill_root)));
    let installed = skill
        .installed_at
        .map(|time| time.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    container(
        row![
            toggle,
            column![
                text(&skill.display_name).size(15).color(theme::TEXT),
                text(skill.description.as_deref().unwrap_or("No description"))
                    .size(12)
                    .color(theme::MUTED)
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
                components::scope_chip(skill.scope),
                text(
                    skill
                        .source_url
                        .as_deref()
                        .map(source_summary)
                        .unwrap_or("unknown source")
                )
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
            text(installed)
                .size(12)
                .color(theme::MUTED)
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
            sort_header("Skill", SortKey::Name, app.sort_key).width(Length::FillPortion(5)),
            sort_header("Health", SortKey::Health, app.sort_key).width(Length::FillPortion(2)),
            sort_header("Scope / Source", SortKey::Scope, app.sort_key)
                .width(Length::FillPortion(2)),
            sort_header("Resources", SortKey::Resources, app.sort_key)
                .width(Length::FillPortion(2)),
            sort_header("Installed", SortKey::Installed, app.sort_key)
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
            app.health_filter,
        ),
        quick_filter(
            "Invalid",
            counts.invalid,
            HealthFilter::Invalid,
            app.health_filter,
        ),
        quick_filter(
            "Warnings",
            counts.warning,
            HealthFilter::Warning,
            app.health_filter,
        ),
        quick_filter(
            "Shadowed",
            counts.shadowed,
            HealthFilter::Shadowed,
            app.health_filter,
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

fn skill_inspector(
    skill: Option<&InstalledSkill>,
    busy: bool,
) -> iced::widget::Container<'_, Message> {
    match skill {
        Some(skill) => {
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
            let toggle_target = !skill.is_enabled();
            let skill_file = skill.skill_file.clone();
            let skill_root = skill.root_dir.clone();
            components::panel(
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
                        text(skill.description.as_deref().unwrap_or("No description"))
                            .size(14)
                            .color(theme::MUTED)
                            .wrapping(text::Wrapping::WordOrGlyph),
                        row![
                            components::primary_button(toggle_label, Some(toggle_icon))
                                .on_press_maybe((!busy).then_some(Message::SetSkillEnabled(
                                    skill_file,
                                    toggle_target
                                ))),
                            components::danger_button("Remove", Some(icons::TRASH)).on_press_maybe(
                                (!busy).then_some(Message::RemoveSkill(skill_root))
                            ),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        rule::horizontal(1),
                        detail_row("Folder", skill.root_dir.display().to_string()),
                        detail_row("Skill file", skill.skill_file.display().to_string()),
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
                        diagnostics_block(skill),
                        metadata_block(skill),
                    ]
                    .spacing(10),
                )
                .height(Length::Fill),
            )
        }
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
