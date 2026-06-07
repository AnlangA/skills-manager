use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, container, pick_list, row, scrollable, text},
};
use skills_manager_core::{ConflictPolicy, SkillHealth, format_bytes};

use crate::{
    app::{
        App, CatalogEntryState, DownloadedEntryState, InstallSource, Message,
        PreviewCandidateState, UiConflictPolicy, UiScope,
    },
    components, icons, theme,
};

use super::{detail_row, diagnostics_text};

pub fn view(app: &App) -> Element<'_, Message> {
    let can_preview = !app.busy
        && match app.install.install_source {
            InstallSource::Url => !app.install.source_url.trim().is_empty(),
            InstallSource::Local => !app.install.local_source_path.trim().is_empty(),
            InstallSource::Downloaded => app.install.selected_download_root.is_some(),
            InstallSource::Catalog => false,
        };
    let can_download = !app.busy && !app.install.source_url.trim().is_empty();
    let can_install = !app.busy
        && app
            .install
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.has_blocking_conflicts());

    let source_panel = components::panel(scrollable(
        column![
            components::section_header("Source", "URL, local folder, downloaded cache, or catalog"),
            row![
                pick_list(
                    InstallSource::ALL,
                    Some(app.install.install_source),
                    Message::InstallSourceSelected,
                )
                .padding([9, 12])
                .style(theme::select)
                .width(Length::Fill),
            ],
            source_controls(app),
            download_controls(app, can_download),
            components::section_header("Destination", "Scope and conflict handling"),
            destination_controls(app),
            compatibility_matrix(app),
            row![
                components::primary_button("Preview", Some(icons::SEARCH))
                    .on_press_maybe(can_preview.then_some(Message::PreviewInstall)),
                components::primary_button("Install", Some(icons::DOWNLOAD))
                    .on_press_maybe(can_install.then_some(Message::InstallPreview)),
            ]
            .spacing(8),
        ]
        .spacing(12),
    ))
    .width(Length::FillPortion(2))
    .height(Length::Fill);

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

fn source_controls(app: &App) -> Element<'_, Message> {
    match app.install.install_source {
        InstallSource::Url => column![components::field(
            "GitHub source",
            "Repository shorthand, full repository URL, or GitHub tree URL.",
            "github.com/owner/repo or owner/repo",
            &app.install.source_url,
            Message::SourceUrlChanged,
        ),]
        .spacing(10)
        .into(),
        InstallSource::Local => column![components::field(
            "Local source folder",
            "Choose a folder that contains one or more SKILL.md files.",
            "/path/to/folder/containing/SKILL.md",
            &app.install.local_source_path,
            Message::LocalSourcePathChanged,
        ),]
        .spacing(10)
        .into(),
        InstallSource::Downloaded => column![downloaded_entries(app),].spacing(10).into(),
        InstallSource::Catalog => column![
            components::field(
                "Catalog URL",
                "Load entries from skills.json, catalog.json, or marketplace.json.",
                "GitHub URL containing skills.json, catalog.json, or marketplace.json",
                &app.install.catalog_url,
                Message::CatalogUrlChanged,
            ),
            components::secondary_button("Load Catalog", Some(icons::DATABASE)).on_press_maybe(
                (!app.busy && !app.install.catalog_url.trim().is_empty())
                    .then_some(Message::LoadCatalog),
            ),
            catalog_entries(app),
        ]
        .spacing(10)
        .into(),
    }
}

fn download_controls(app: &App, can_download: bool) -> Element<'_, Message> {
    if app.install.install_source != InstallSource::Url {
        return column![].into();
    }

    column![
        components::section_header("Download cache", "Optional reusable local copy"),
        components::compact_field(
            "Override folder",
            "Use saved default download path",
            &app.install.download_path_override,
            Message::DownloadPathOverrideChanged,
        ),
        detail_row("Default folder", app.settings.default_download_path.clone()),
        components::secondary_button("Download URL", Some(icons::DOWNLOAD))
            .on_press_maybe(can_download.then_some(Message::DownloadSource)),
    ]
    .spacing(10)
    .into()
}

fn destination_controls(app: &App) -> Element<'_, Message> {
    let enable = checkbox(app.install.enable_after_install)
        .size(18)
        .on_toggle(Message::EnableAfterInstallChanged);
    let enable_row = row![
        enable,
        text("Enable after install").size(13).color(theme::TEXT),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut controls = column![
        row![
            pick_list(
                UiScope::ALL,
                Some(app.install.install_scope),
                Message::InstallScopeSelected
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(1)),
            pick_list(
                UiConflictPolicy::ALL,
                Some(app.install.conflict_policy),
                Message::ConflictSelected,
            )
            .padding([9, 12])
            .style(theme::select)
            .width(Length::FillPortion(2)),
        ]
        .spacing(10),
        enable_row,
    ]
    .spacing(10);

    if app.install.install_scope == UiScope::Custom {
        controls = controls.push(components::field(
            "Custom install path",
            "This folder becomes a managed Custom root in Library.",
            "/path/to/skills/root",
            &app.install.custom_install_path,
            Message::CustomInstallPathChanged,
        ));
    }

    controls.into()
}

fn compatibility_matrix(app: &App) -> Element<'_, Message> {
    let selected = app.install.install_scope.to_string();
    let Some(profile) = app
        .settings
        .target_profiles
        .iter()
        .find(|profile| profile.label == selected)
    else {
        return components::empty_state(
            "Target profile unavailable",
            "Refresh settings to load target paths and compatibility rules.",
        )
        .into();
    };

    components::flat_panel(
        column![
            text("Compatibility").size(12).color(theme::TEXT),
            detail_row(
                "Skills root",
                profile
                    .skills_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Unavailable".to_string()),
            ),
            detail_row(
                "Disabled store",
                profile
                    .disabled_store_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Uses config toggle".to_string()),
            ),
            detail_row(
                "Enablement",
                strategy_label(profile.enablement_strategy).to_string()
            ),
            detail_row("Layout", layout_label(profile.layout_policy).to_string()),
            text(profile.notes.join(" "))
                .size(11)
                .color(theme::SUBTLE)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(8),
    )
    .into()
}

fn strategy_label(strategy: skills_manager_core::EnablementStrategy) -> &'static str {
    match strategy {
        skills_manager_core::EnablementStrategy::ConfigToggle => "Config toggle",
        skills_manager_core::EnablementStrategy::DirectoryMove => "Directory move",
    }
}

fn layout_label(layout: skills_manager_core::LayoutPolicy) -> &'static str {
    match layout {
        skills_manager_core::LayoutPolicy::NestedAllowed => "Nested resources allowed",
        skills_manager_core::LayoutPolicy::FlatTopLevel => "Flat top-level folder",
    }
}

fn preview_list(app: &App) -> Element<'_, Message> {
    let Some(preview) = &app.install.preview else {
        return components::empty_state(
            "No preview yet",
            "Preview a GitHub URL, local folder, downloaded bundle, or catalog entry before installing.",
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
    let mut content = column![
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
    ]
    .spacing(7);

    if !candidate.diagnostics.is_empty() {
        content = content.push(diagnostics_text(&candidate.diagnostics));
    }

    components::flat_panel(content).into()
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

fn downloaded_entries(app: &App) -> Element<'_, Message> {
    if app.install.downloaded_entries.is_empty() {
        return components::empty_state(
            "No downloaded skills",
            "Download a GitHub source to create a reusable local bundle.",
        )
        .into();
    }

    app.install
        .downloaded_entries
        .iter()
        .fold(column![].spacing(8), |list, entry| {
            list.push(downloaded_entry(app, entry))
        })
        .into()
}

fn downloaded_entry<'a>(app: &'a App, entry: &'a DownloadedEntryState) -> Element<'a, Message> {
    let selected = app
        .install
        .selected_download_root
        .as_ref()
        .is_some_and(|root| root == &entry.root_dir);
    let pending = app
        .install
        .pending_remove_download
        .as_ref()
        .is_some_and(|root| root == &entry.root_dir);
    let preview = if selected {
        components::primary_button("Preview", Some(icons::SEARCH))
    } else {
        components::secondary_button("Preview", Some(icons::SEARCH))
    }
    .on_press_maybe((!app.busy).then_some(Message::PreviewDownloaded(entry.root_dir.clone())));
    let delete_label = if pending { "Confirm" } else { "Delete" };
    let delete_message = if pending {
        Message::ConfirmRemoveDownload(entry.root_dir.clone())
    } else {
        Message::RequestRemoveDownload(entry.root_dir.clone())
    };

    components::flat_panel(
        column![
            row![
                column![
                    text(&entry.source_url)
                        .size(13)
                        .color(theme::TEXT)
                        .wrapping(text::Wrapping::WordOrGlyph),
                    text(&entry.summary).size(11).color(theme::MUTED),
                    text(format!("Downloaded {}", entry.downloaded_at))
                        .size(11)
                        .color(theme::SUBTLE),
                    text(entry.root_dir.display().to_string())
                        .size(11)
                        .color(theme::SUBTLE)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(4)
                .width(Length::Fill),
                row![
                    preview,
                    components::danger_button(delete_label, Some(icons::TRASH))
                        .on_press_maybe((!app.busy).then_some(delete_message)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(7),
    )
    .style(if selected {
        theme::selected_table_row
    } else {
        theme::flat_panel
    })
    .into()
}

fn catalog_entries(app: &App) -> Element<'_, Message> {
    if app.install.catalog_entries.is_empty() {
        return components::empty_state(
            "Catalog entries appear here",
            "Load a catalog, then preview a GitHub or local entry directly.",
        )
        .into();
    }

    app.install
        .catalog_entries
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
    app.install.preview.as_ref().map_or_else(
        || "No source previewed".to_string(),
        |preview| {
            let download = preview
                .download_root
                .as_ref()
                .map(|path| format!("; cached at {}", path.display()))
                .unwrap_or_default();
            format!(
                "{} candidate(s) from {} into {} scope at {}{}",
                preview.candidates.len(),
                preview.source_label,
                preview.scope.label(),
                preview.destination_root.display(),
                download
            )
        },
    )
}
