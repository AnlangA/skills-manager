use iced::{
    Alignment, Element, Length,
    widget::{column, pick_list, row, scrollable, text, text_input},
};
use skills_manager_core::{SkillHealth, format_bytes};

use crate::{
    app::{
        App, CatalogEntryState, InstallSource, Message, PreviewCandidateState, UiConflictPolicy,
        UiScope,
    },
    components, icons, theme,
};

use super::{detail_row, diagnostics_text};

pub fn view(app: &App) -> Element<'_, Message> {
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
