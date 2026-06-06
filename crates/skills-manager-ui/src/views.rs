use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, container, pick_list, row, rule, scrollable, text, text_input},
};
use skills_manager_core::{InstalledSkill, SkillHealth, format_bytes};

use crate::{
    app::{
        ActiveView, App, CatalogEntryState, HealthFilter, InstallSource, Message,
        PreviewCandidateState, ScopeFilter, UiCatalogFormat, UiConflictPolicy, UiScope,
    },
    components, icons, theme,
};

pub fn view(app: &App) -> Element<'_, Message> {
    let content = row![sidebar(app), main_content(app)]
        .height(Length::Fill)
        .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::app_background)
        .into()
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let nav = ActiveView::ALL.into_iter().fold(
        column![
            row![
                icons::icon(icons::SPARKLES, 19),
                text("Agent Skills").size(18)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text("Open local skill library")
                .size(12)
                .color(iced::Color::from_rgb8(203, 213, 225)),
            rule::horizontal(1),
        ]
        .spacing(10),
        |nav, view| {
            nav.push(components::nav_button(
                view.label(),
                nav_icon(view),
                app.active_view == view,
                Message::ActiveViewSelected(view),
            ))
        },
    );

    container(
        column![
            nav.width(Length::Fill),
            container(column![
                text("Open convention").size(12).color(iced::Color::WHITE),
                text("~/.agents/skills\n<project>/.agents/skills")
                    .size(11)
                    .color(iced::Color::from_rgb8(203, 213, 225)),
            ])
            .padding(10)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb8(31, 41, 55))),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..iced::Border::default()
                },
                ..iced::widget::container::Style::default()
            }),
        ]
        .spacing(16)
        .height(Length::Fill),
    )
    .width(Length::Fixed(220.0))
    .height(Length::Fill)
    .padding(16)
    .style(theme::sidebar)
    .into()
}

fn main_content(app: &App) -> Element<'_, Message> {
    let refresh = components::secondary_button("Refresh", Some(icons::REFRESH))
        .on_press_maybe((!app.busy).then_some(Message::Refresh));
    let header = row![
        column![
            text(app.active_view.label()).size(26).color(theme::TEXT),
            text(header_subtitle(app.active_view))
                .size(13)
                .color(theme::MUTED),
        ]
        .spacing(3)
        .width(Length::Fill),
        components::status_badge(&app.status, app.busy).max_width(460),
        refresh,
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    let body = match app.active_view {
        ActiveView::Inventory => inventory_view(app),
        ActiveView::Install => install_view(app),
        ActiveView::Catalog => catalog_view(app),
        ActiveView::Settings => settings_view(app),
    };

    container(column![header, body].padding(18).spacing(14))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn inventory_view(app: &App) -> Element<'_, Message> {
    let counts = app.counts();
    let visible = app.filtered_skills();
    let selected = app.selected_skill();

    let metrics = row![
        components::metric("Total", app.skills.len().to_string(), theme::PRIMARY),
        components::metric("Enabled", counts.enabled.to_string(), theme::SUCCESS),
        components::metric("Warning", counts.warning.to_string(), theme::WARNING),
        components::metric("Invalid", counts.invalid.to_string(), theme::DANGER),
        components::metric("Shadowed", counts.shadowed.to_string(), theme::SUBTLE),
    ]
    .spacing(8);

    let filters = row![
        text_input(
            "Search name, description, folder, or allowed tools",
            &app.search_query
        )
        .on_input(Message::SearchChanged)
        .padding([10, 12])
        .style(theme::input)
        .width(Length::Fill),
        pick_list(
            ScopeFilter::ALL,
            Some(app.scope_filter),
            Message::ScopeFilterSelected
        )
        .padding([9, 12])
        .style(theme::select)
        .width(Length::Fixed(150.0)),
        pick_list(
            HealthFilter::ALL,
            Some(app.health_filter),
            Message::HealthFilterSelected
        )
        .padding([9, 12])
        .style(theme::select)
        .width(Length::Fixed(150.0)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let list = components::panel(
        column![
            components::section_header("Inventory", format!("{} matching", visible.len())),
            filters,
            scrollable(skill_list(app, &visible)).height(Length::Fill),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let inspector = skill_inspector(selected)
        .width(Length::FillPortion(2))
        .height(Length::Fill);

    column![
        metrics,
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
            "Try a different search, scope, or health filter.",
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

    container(
        row![
            toggle,
            column![
                row![
                    text(&skill.display_name).size(16).color(theme::TEXT),
                    components::health_chip(skill.health),
                    components::scope_chip(skill.scope),
                    components::enablement_chip(skill.enablement),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
                text(skill.description.as_deref().unwrap_or("No description"))
                    .size(13)
                    .color(theme::MUTED)
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(format!(
                    "{} resource(s), {}",
                    skill.resource_count,
                    format_bytes(skill.resource_bytes)
                ))
                .size(12)
                .color(theme::SUBTLE),
            ]
            .spacing(4)
            .width(Length::Fill),
            row![details, remove].spacing(8).align_y(Alignment::Center),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(12)
    .style(if selected {
        theme::selected_row
    } else {
        theme::row
    })
    .into()
}

fn skill_inspector(skill: Option<&InstalledSkill>) -> iced::widget::Container<'_, Message> {
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
                    text(skill.description.as_deref().unwrap_or("No description"))
                        .size(14)
                        .color(theme::MUTED)
                        .wrapping(text::Wrapping::WordOrGlyph),
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

fn install_view(app: &App) -> Element<'_, Message> {
    let source_controls = match app.install_source {
        InstallSource::Url => column![
            text_input("github.com/owner/repo or GitHub tree URL", &app.source_url)
                .on_input(Message::SourceUrlChanged)
                .padding([10, 12])
                .style(theme::input),
        ],
        InstallSource::Local => column![
            text_input(
                "/path/to/folder/containing/SKILL.md",
                &app.local_source_path
            )
            .on_input(Message::LocalSourcePathChanged)
            .padding([10, 12])
            .style(theme::input),
        ],
        InstallSource::Catalog => column![
            text_input(
                "GitHub URL containing skills.json, catalog.json, or marketplace.json",
                &app.catalog_url
            )
            .on_input(Message::CatalogUrlChanged)
            .padding([10, 12])
            .style(theme::input),
            components::secondary_button("Load Catalog", Some(icons::DATABASE)).on_press_maybe(
                (!app.busy && !app.catalog_url.trim().is_empty()).then_some(Message::LoadCatalog)
            ),
            catalog_entries(app),
        ]
        .spacing(10),
    };

    let can_preview = !app.busy
        && match app.install_source {
            InstallSource::Url => !app.source_url.trim().is_empty(),
            InstallSource::Local => !app.local_source_path.trim().is_empty(),
            InstallSource::Catalog => !app.catalog_url.trim().is_empty(),
        };
    let can_install =
        !app.busy && app.preview.is_some() && app.install_source != InstallSource::Catalog;

    let source_panel = components::panel(
        column![
            components::section_header("Source", "Preview before installing"),
            row![
                pick_list(
                    InstallSource::ALL,
                    Some(app.install_source),
                    Message::InstallSourceSelected
                )
                .padding([9, 12])
                .style(theme::select)
                .width(Length::Fixed(170.0)),
                pick_list(
                    UiScope::ALL,
                    Some(app.install_scope),
                    Message::InstallScopeSelected
                )
                .padding([9, 12])
                .style(theme::select)
                .width(Length::Fixed(120.0)),
                pick_list(
                    UiConflictPolicy::ALL,
                    Some(app.conflict_policy),
                    Message::ConflictSelected
                )
                .padding([9, 12])
                .style(theme::select)
                .width(Length::Fixed(190.0)),
            ]
            .spacing(10),
            source_controls,
            row![
                components::primary_button("Preview", Some(icons::SEARCH))
                    .on_press_maybe(can_preview.then_some(Message::PreviewInstall)),
                components::primary_button("Install", Some(icons::DOWNLOAD))
                    .on_press_maybe(can_install.then_some(Message::InstallPreview)),
            ]
            .spacing(8),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(2));

    let preview_panel = components::panel(
        column![
            components::section_header("Preview", preview_meta(app)),
            scrollable(preview_list(app)).height(Length::Fill),
        ]
        .spacing(12),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    row![source_panel, preview_panel]
        .spacing(14)
        .height(Length::Fill)
        .into()
}

fn preview_list(app: &App) -> Element<'_, Message> {
    let Some(preview) = &app.preview else {
        return components::empty_state(
            "No preview yet",
            "Preview a GitHub URL, local folder, or catalog before installing.",
        )
        .into();
    };

    preview
        .candidates
        .iter()
        .fold(column![].spacing(8), |list, candidate| {
            list.push(preview_candidate(candidate))
        })
        .into()
}

fn preview_candidate(candidate: &PreviewCandidateState) -> Element<'_, Message> {
    components::flat_panel(
        column![
            row![
                text(&candidate.name).size(16).color(theme::TEXT),
                components::health_chip(candidate.health),
                if candidate.conflict {
                    components::health_chip(SkillHealth::Warning)
                } else {
                    components::health_chip(SkillHealth::Valid)
                },
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(&candidate.description)
                .size(13)
                .color(theme::MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            detail_row("Source", candidate.source_root.display().to_string()),
            detail_row(
                "Destination",
                candidate.destination_root.display().to_string()
            ),
            detail_row(
                "Resources",
                format!(
                    "{} file(s), {}",
                    candidate.resource_count,
                    format_bytes(candidate.resource_bytes)
                ),
            ),
            diagnostics_text(&candidate.diagnostics),
        ]
        .spacing(6),
    )
    .into()
}

fn catalog_view(app: &App) -> Element<'_, Message> {
    let exportable = app
        .skills
        .iter()
        .filter(|skill| skill.is_exportable())
        .count();
    let output: Element<'_, Message> = if app.catalog_output.is_empty() {
        components::empty_state(
            "No export generated",
            "Generate a catalog to preview JSON, XML, or Markdown output.",
        )
        .into()
    } else {
        container(
            scrollable(
                text(&app.catalog_output)
                    .size(12)
                    .color(theme::TEXT)
                    .wrapping(text::Wrapping::WordOrGlyph),
            )
            .height(Length::Fill),
        )
        .padding(12)
        .style(theme::flat_panel)
        .into()
    };

    components::panel(
        column![
            components::section_header(
                "Catalog Export",
                format!("{exportable} exportable skill(s)")
            ),
            row![
                pick_list(
                    UiCatalogFormat::ALL,
                    Some(app.catalog_format),
                    Message::CatalogFormatSelected
                )
                .padding([9, 12])
                .style(theme::select)
                .width(Length::Fixed(160.0)),
                components::primary_button("Generate", Some(icons::FILE))
                    .on_press_maybe((!app.busy).then_some(Message::GenerateCatalog)),
                components::secondary_button("Copy", Some(icons::COPY)).on_press_maybe(
                    (!app.catalog_output.is_empty()).then_some(Message::CopyCatalog)
                ),
                components::secondary_button("Save", Some(icons::DOWNLOAD)).on_press_maybe(
                    (!app.busy && !app.catalog_output.is_empty()).then_some(Message::SaveCatalog)
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text_input("Save path", &app.catalog_save_path)
                .on_input(Message::CatalogSavePathChanged)
                .padding([10, 12])
                .style(theme::input),
            output,
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}

fn settings_view(app: &App) -> Element<'_, Message> {
    components::panel(
        column![
            components::section_header("Settings", "Open Agent Skills convention"),
            text_input("Project folder", &app.project_path)
                .on_input(Message::ProjectPathChanged)
                .padding([10, 12])
                .style(theme::input),
            components::flat_panel(
                column![
                    row![icons::icon(icons::FOLDER, 16), text("Project scope").size(14)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    text("<project>/.agents/skills").size(13).color(theme::MUTED),
                    text("Project skills take priority and can shadow user skills with the same name.")
                        .size(12)
                        .color(theme::SUBTLE),
                ]
                .spacing(6),
            ),
            components::flat_panel(
                column![
                    row![icons::icon(icons::GLOBE, 16), text("User scope").size(14)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    text("~/.agents/skills").size(13).color(theme::MUTED),
                    text("User skills are available across projects unless a project skill shadows them.")
                        .size(12)
                        .color(theme::SUBTLE),
                ]
                .spacing(6),
            ),
            components::flat_panel(
                column![
                    row![icons::icon(icons::SHIELD, 16), text("Validation").size(14)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    text("The scanner checks required frontmatter, name shape, description length, compatibility notes, resources, and shadowing.")
                        .size(13)
                        .color(theme::MUTED)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(6),
            ),
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}

fn catalog_entries(app: &App) -> Element<'_, Message> {
    if app.catalog_entries.is_empty() {
        return components::empty_state(
            "Catalog entries appear here",
            "Load a catalog, then move a Git entry into the URL installer.",
        )
        .into();
    }

    app.catalog_entries
        .iter()
        .fold(column![].spacing(8), |list, entry| {
            list.push(catalog_entry(entry))
        })
        .into()
}

fn catalog_entry(entry: &CatalogEntryState) -> Element<'_, Message> {
    let action = components::secondary_button("Use", Some(icons::UPLOAD))
        .on_press_maybe(entry.install_url.clone().map(Message::UseCatalogEntry));
    components::flat_panel(
        row![
            column![
                text(&entry.name).size(14).color(theme::TEXT),
                text(&entry.description)
                    .size(12)
                    .color(theme::MUTED)
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(&entry.source_label).size(11).color(theme::SUBTLE),
            ]
            .spacing(4)
            .width(Length::Fill),
            action,
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .into()
}

fn diagnostics_block(skill: &InstalledSkill) -> Element<'_, Message> {
    if skill.diagnostics.is_empty() {
        return detail_row("Diagnostics", "No diagnostics".to_string()).into();
    }

    let diagnostics = skill.diagnostics.iter().fold(
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
    );

    diagnostics.into()
}

fn diagnostics_text<'a>(diagnostics: &'a [String]) -> Element<'a, Message> {
    if diagnostics.is_empty() {
        return text("No diagnostics").size(12).color(theme::SUBTLE).into();
    }

    diagnostics
        .iter()
        .fold(column![].spacing(3), |list, diagnostic| {
            list.push(text(format!("- {diagnostic}")).size(12).color(theme::MUTED))
        })
        .into()
}

fn metadata_block(skill: &InstalledSkill) -> Element<'_, Message> {
    if skill.frontmatter.metadata.is_empty() {
        return detail_row("Metadata", "No extra metadata".to_string()).into();
    }

    let content = skill.frontmatter.metadata.iter().fold(
        column![text("Metadata").size(11).color(theme::SUBTLE)].spacing(3),
        |list, (key, value)| {
            list.push(text(format!("{key}: {value}")).size(12).color(theme::MUTED))
        },
    );
    content.into()
}

fn detail_row<'a>(label: &'a str, value: String) -> iced::widget::Column<'a, Message> {
    column![
        text(label).size(11).color(theme::SUBTLE),
        text(value)
            .size(13)
            .color(theme::TEXT)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(3)
}

fn preview_meta(app: &App) -> String {
    app.preview.as_ref().map_or_else(
        || "No source previewed".to_string(),
        |preview| {
            format!(
                "{} candidate(s) from {} into {} scope",
                preview.candidates.len(),
                preview.source_label,
                preview.scope.label()
            )
        },
    )
}

fn header_subtitle(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Inventory => "Search, validate, enable, disable, and inspect local skills",
        ActiveView::Install => "Preview GitHub, local, or catalog sources before installing",
        ActiveView::Catalog => "Export enabled and usable skills for agent runtimes",
        ActiveView::Settings => "Project path and open Agent Skills storage rules",
    }
}

fn nav_icon(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Inventory => icons::LIST,
        ActiveView::Install => icons::DOWNLOAD,
        ActiveView::Catalog => icons::DATABASE,
        ActiveView::Settings => icons::SETTINGS,
    }
}
