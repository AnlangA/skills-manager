use iced::{
    Alignment, Element, Length,
    widget::{column, container, pick_list, row, scrollable, text},
};
use skills_manager_core::{ConflictPolicy, SkillHealth, format_bytes};

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
        InstallSource::Url => column![components::field(
            "GitHub source",
            "Repository shorthand, full repository URL, or GitHub tree URL.",
            "github.com/owner/repo or owner/repo",
            &app.source_url,
            Message::SourceUrlChanged,
        ),],
        InstallSource::Local => column![components::field(
            "Local source folder",
            "Choose a folder that contains one or more SKILL.md files.",
            "/path/to/folder/containing/SKILL.md",
            &app.local_source_path,
            Message::LocalSourcePathChanged,
        ),],
        InstallSource::Catalog => column![
            components::field(
                "Catalog URL",
                "Load entries from skills.json, catalog.json, or marketplace.json.",
                "GitHub URL containing skills.json, catalog.json, or marketplace.json",
                &app.catalog_url,
                Message::CatalogUrlChanged,
            ),
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
            InstallSource::Catalog => false,
        };
    let can_install = !app.busy
        && app.install_source != InstallSource::Catalog
        && app
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.has_blocking_conflicts());

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
            components::inline_status(&app.status, app.busy),
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
            "Preview a GitHub URL, local folder, or catalog entry before installing.",
        )
        .into();
    };

    column![
        preview_table_header(),
        preview
            .candidates
            .iter()
            .fold(column![].spacing(7), |list, candidate| {
                list.push(preview_candidate(candidate, preview.conflict_policy))
            }),
    ]
    .spacing(8)
    .into()
}

fn preview_table_header() -> Element<'static, Message> {
    container(
        row![
            text("Candidate")
                .size(11)
                .color(theme::SUBTLE)
                .width(Length::FillPortion(4)),
            text("Health")
                .size(11)
                .color(theme::SUBTLE)
                .width(Length::FillPortion(2)),
            text("Conflict result")
                .size(11)
                .color(theme::SUBTLE)
                .width(Length::FillPortion(4)),
            text("Resources")
                .size(11)
                .color(theme::SUBTLE)
                .width(Length::FillPortion(2)),
        ]
        .spacing(10),
    )
    .padding([7, 10])
    .style(theme::table_header)
    .into()
}

fn preview_candidate(
    candidate: &PreviewCandidateState,
    conflict_policy: ConflictPolicy,
) -> Element<'_, Message> {
    components::flat_panel(
        column![
            row![
                column![
                    text(&candidate.name).size(15).color(theme::TEXT),
                    text(&candidate.description)
                        .size(12)
                        .color(theme::MUTED)
                        .wrapping(text::Wrapping::WordOrGlyph),
                    text(candidate.destination_root.display().to_string())
                        .size(11)
                        .color(theme::SUBTLE)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(4)
                .width(Length::FillPortion(4)),
                components::health_chip(candidate.health).width(Length::FillPortion(2)),
                column![
                    conflict_chip(candidate, conflict_policy),
                    text(conflict_result(candidate, conflict_policy))
                        .size(12)
                        .color(theme::MUTED)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(5)
                .width(Length::FillPortion(4)),
                column![
                    text(format!("{} file(s)", candidate.resource_count))
                        .size(12)
                        .color(theme::TEXT),
                    text(format_bytes(candidate.resource_bytes))
                        .size(11)
                        .color(theme::SUBTLE),
                ]
                .spacing(4)
                .width(Length::FillPortion(2)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            detail_row("Source", candidate.source_root.display().to_string()),
            diagnostics_text(&candidate.diagnostics),
        ]
        .spacing(7),
    )
    .into()
}

fn conflict_chip<'a>(
    candidate: &PreviewCandidateState,
    conflict_policy: ConflictPolicy,
) -> iced::widget::Container<'a, Message> {
    if candidate.conflict && conflict_policy == ConflictPolicy::Block {
        components::health_chip(SkillHealth::Invalid)
    } else if candidate.conflict {
        components::health_chip(SkillHealth::Warning)
    } else {
        components::health_chip(SkillHealth::Valid)
    }
}

fn conflict_result(candidate: &PreviewCandidateState, conflict_policy: ConflictPolicy) -> String {
    match (candidate.conflict, conflict_policy) {
        (true, ConflictPolicy::Block) => "Blocked: destination already exists.".to_string(),
        (true, ConflictPolicy::Rename) => {
            format!(
                "Conflict found. Will install as `{}`.",
                destination_name(candidate)
            )
        }
        (true, ConflictPolicy::Replace) => {
            "Conflict found. Will replace existing folder and create a backup.".to_string()
        }
        (false, _) => format!("Will install to `{}`.", destination_name(candidate)),
    }
}

fn destination_name(candidate: &PreviewCandidateState) -> String {
    candidate
        .destination_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill")
        .to_string()
}

fn catalog_entries(app: &App) -> Element<'_, Message> {
    if app.catalog_entries.is_empty() {
        return components::empty_state(
            "Catalog entries appear here",
            "Load a catalog, then preview a GitHub or local entry directly.",
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
    let action = components::secondary_button("Preview", Some(icons::SEARCH)).on_press_maybe(
        entry
            .install_source
            .zip(entry.source_value.clone())
            .map(|(source, value)| Message::PreviewCatalogEntry(source, value)),
    );
    components::flat_panel(
        row![
            column![
                text(&entry.name).size(14).color(theme::TEXT),
                text(&entry.description)
                    .size(12)
                    .color(theme::MUTED)
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(&entry.source_label).size(11).color(theme::SUBTLE),
                if let Some(reason) = &entry.unavailable_reason {
                    text(reason).size(11).color(theme::DANGER)
                } else {
                    text("Ready to preview").size(11).color(theme::SUCCESS)
                },
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
