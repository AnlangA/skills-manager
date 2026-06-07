//! Install view with source selection, destination configuration, and preview.
//!
//! Provides a step-by-step workflow for choosing a skill source (GitHub URL,
//! local folder, downloaded cache, or catalog), selecting a target scope and
//! conflict policy, previewing candidates, and applying the installation.

use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, container, row, scrollable, text},
};
use skills_manager_core::{ConflictPolicy, SkillHealth, format_bytes};

use crate::theme::*;
use crate::{
    app::{
        App, CatalogEntryState, DownloadedEntryState, InstallSource, Message,
        PreviewCandidateState, UiConflictPolicy, UiScope,
    },
    components, icons, theme,
};

use super::{layout_label, strategy_label};

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

    let setup_panel = container(scrollable(
        column![
            step_header(1, "Source", "Choose where to install from"),
            source_controls(app),
            download_controls(app, can_download),
            step_header(2, "Destination", "Choose where to install"),
            destination_controls(app),
            compatibility_info(app),
            step_header(3, "Install", "Preview and confirm"),
            row![
                components::primary_button("Preview", Some(icons::SEARCH))
                    .on_press_maybe(can_preview.then_some(Message::PreviewInstall)),
                components::primary_button("Install", Some(icons::DOWNLOAD))
                    .on_press_maybe(can_install.then_some(Message::InstallPreview)),
            ]
            .spacing(SPACING_MD),
        ]
        .spacing(SPACING_LG),
    ))
    .padding(SPACING_XL)
    .style(theme::card)
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let preview_panel = components::card(
        column![
            components::section_header("Preview", preview_meta(app)),
            scrollable(preview_list(app)).height(Length::Fill),
        ]
        .spacing(SPACING_MD),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    components::form_preview_layout(setup_panel, preview_panel)
}

fn step_header<'a>(step: u8, title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    row![
        container(text(step.to_string()).size(FONT_CAPTION).color(PRIMARY))
            .padding([SPACING_XS, SPACING_SM + 2.0])
            .style(theme::chip(PRIMARY_SOFT, PRIMARY)),
        column![
            text(title).size(FONT_BODY).color(TEXT),
            text(subtitle).size(FONT_MICRO).color(TEXT_MUTED),
        ]
        .spacing(2),
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center)
    .into()
}

fn source_controls(app: &App) -> Element<'_, Message> {
    column![
        components::styled_pick_list(
            &InstallSource::ALL,
            Some(app.install.install_source),
            Message::InstallSourceSelected,
            Length::Fill,
        ),
        source_input(app),
    ]
    .spacing(SPACING_MD)
    .into()
}

fn source_input(app: &App) -> Element<'_, Message> {
    match app.install.install_source {
        InstallSource::Url => components::field(
            "GitHub source",
            "Repository shorthand, full URL, or tree URL.",
            "github.com/owner/repo",
            &app.install.source_url,
            Message::SourceUrlChanged,
        ),
        InstallSource::Local => components::field(
            "Local source folder",
            "Folder containing one or more SKILL.md files.",
            "/path/to/folder/containing/SKILL.md",
            &app.install.local_source_path,
            Message::LocalSourcePathChanged,
        ),
        InstallSource::Downloaded => downloaded_entries(app),
        InstallSource::Catalog => column![
            components::field(
                "Catalog URL",
                "Load from skills.json, catalog.json, or marketplace.json.",
                "GitHub URL for catalog file",
                &app.install.catalog_url,
                Message::CatalogUrlChanged,
            ),
            components::secondary_button("Load Catalog", Some(icons::DATABASE)).on_press_maybe(
                (!app.busy && !app.install.catalog_url.trim().is_empty())
                    .then_some(Message::LoadCatalog),
            ),
            catalog_entries(app),
        ]
        .spacing(SPACING_MD)
        .into(),
    }
}

fn download_controls(app: &App, can_download: bool) -> Element<'_, Message> {
    if app.install.install_source != InstallSource::Url {
        return column![].into();
    }

    column![
        components::compact_field(
            "Download cache override (optional)",
            "Use saved default download path",
            &app.install.download_path_override,
            Message::DownloadPathOverrideChanged,
        ),
        row![
            text("Default:").size(FONT_MICRO).color(TEXT_MUTED),
            text(&app.settings.default_download_path)
                .size(FONT_MICRO)
                .color(TEXT_SECONDARY)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(SPACING_XS + 2.0),
        components::secondary_button("Download to Cache", Some(icons::DOWNLOAD))
            .on_press_maybe(can_download.then_some(Message::DownloadSource)),
    ]
    .spacing(SPACING_MD)
    .into()
}

fn destination_controls(app: &App) -> Element<'_, Message> {
    let enable = checkbox(app.install.enable_after_install)
        .size(18)
        .on_toggle(Message::EnableAfterInstallChanged);
    let enable_row = row![
        enable,
        text("Enable after install").size(FONT_BODY).color(TEXT),
    ]
    .spacing(SPACING_SM)
    .align_y(Alignment::Center);

    let mut controls = column![
        row![
            components::styled_pick_list(
                &UiScope::ALL,
                Some(app.install.install_scope),
                Message::InstallScopeSelected,
                Length::FillPortion(1),
            ),
            components::styled_pick_list(
                &UiConflictPolicy::ALL,
                Some(app.install.conflict_policy),
                Message::ConflictSelected,
                Length::FillPortion(2),
            ),
        ]
        .spacing(SPACING_MD),
        enable_row,
    ]
    .spacing(SPACING_MD);

    if app.install.install_scope == UiScope::Custom {
        controls = controls.push(components::compact_field(
            "Custom install path",
            "/path/to/skills/root",
            &app.install.custom_install_path,
            Message::CustomInstallPathChanged,
        ));
    }

    controls.into()
}

fn compatibility_info(app: &App) -> Element<'_, Message> {
    let selected = app.install.install_scope.to_string();
    let Some(profile) = app
        .settings
        .target_profiles
        .iter()
        .find(|profile| profile.label == selected)
    else {
        return text("Refresh to load target profiles.")
            .size(FONT_MICRO)
            .color(TEXT_MUTED)
            .into();
    };

    components::flat_card(
        column![
            row![
                text("Target:").size(FONT_MICRO).color(TEXT_MUTED),
                text(
                    profile
                        .skills_root
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "Unavailable".to_string())
                )
                .size(FONT_MICRO)
                .color(TEXT_SECONDARY),
            ]
            .spacing(SPACING_XS + 2.0),
            text(format!(
                "{} - {}",
                strategy_label(profile.enablement_strategy),
                layout_label(profile.layout_policy)
            ))
            .size(FONT_MICRO)
            .color(TEXT_MUTED)
            .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(SPACING_XS + 2.0),
    )
    .into()
}

fn preview_list(app: &App) -> Element<'_, Message> {
    let Some(preview) = &app.install.preview else {
        return components::empty_state("No preview yet", "Preview a source to see details here.")
            .into();
    };

    components::list_column(preview.candidates.iter(), SPACING_SM, |candidate| {
        preview_candidate(candidate, preview.conflict_policy)
    })
    .into()
}

fn preview_candidate(
    candidate: &PreviewCandidateState,
    conflict_policy: ConflictPolicy,
) -> Element<'_, Message> {
    let mut content = column![
        row![
            column![
                text(&candidate.name).size(FONT_BODY).color(TEXT),
                text(&candidate.description)
                    .size(FONT_CAPTION)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            components::health_dot(candidate.health),
            conflict_chip(candidate, conflict_policy),
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
        row![
            text(format!(
                "{} file(s), {}",
                candidate.resource_count,
                format_bytes(candidate.resource_bytes)
            ))
            .size(FONT_MICRO)
            .color(TEXT_MUTED),
            text(conflict_result(candidate, conflict_policy))
                .size(FONT_MICRO)
                .color(TEXT_SECONDARY)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(SPACING_MD),
    ]
    .spacing(SPACING_SM);

    if !candidate.diagnostics.is_empty() {
        content = content.push(components::bullet_lines(
            candidate.diagnostics.iter().cloned(),
            "No diagnostics",
        ));
    }

    components::flat_card(content).into()
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
        (true, ConflictPolicy::Block) => "Blocked: destination exists.".to_string(),
        (true, ConflictPolicy::Rename) => {
            format!("Will rename to `{}`.", destination_name(candidate))
        }
        (true, ConflictPolicy::Replace) => "Will replace with backup.".to_string(),
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
            "Download a GitHub URL to create a reusable local bundle.",
        )
        .into();
    }

    components::list_column(app.install.downloaded_entries.iter(), SPACING_SM, |entry| {
        downloaded_entry(app, entry)
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
    let preview = components::small_ghost_button("Preview", Some(icons::SEARCH))
        .on_press_maybe((!app.busy).then_some(Message::PreviewDownloaded(entry.root_dir.clone())));
    let delete = components::confirm_button(
        pending,
        "Delete",
        "Confirm",
        Some(icons::TRASH),
        Message::RequestRemoveDownload(entry.root_dir.clone()),
        Message::ConfirmRemoveDownload(entry.root_dir.clone()),
        app.busy,
    );

    container(
        row![
            column![
                text(&entry.source_url)
                    .size(FONT_CAPTION)
                    .color(TEXT)
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(&entry.summary).size(FONT_MICRO).color(TEXT_SECONDARY),
                text(format!(
                    "{} - {}",
                    entry.downloaded_at,
                    entry.root_dir.display()
                ))
                .size(FONT_MICRO)
                .color(TEXT_MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            row![preview, delete]
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
        theme::flat_card
    })
    .into()
}

fn catalog_entries(app: &App) -> Element<'_, Message> {
    if app.install.catalog_entries.is_empty() {
        return components::empty_state(
            "No catalog entries",
            "Load a catalog to see entries here.",
        )
        .into();
    }

    components::list_column(
        app.install.catalog_entries.iter(),
        SPACING_SM,
        catalog_entry,
    )
    .into()
}

fn catalog_entry(entry: &CatalogEntryState) -> Element<'_, Message> {
    let action = components::small_ghost_button("Preview", Some(icons::SEARCH)).on_press_maybe(
        entry
            .install_source
            .zip(entry.source_value.clone())
            .map(|(source, value)| Message::PreviewCatalogEntry(source, value)),
    );
    components::flat_card(
        row![
            column![
                text(&entry.name).size(FONT_BODY).color(TEXT),
                text(&entry.description)
                    .size(FONT_MICRO)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::WordOrGlyph),
                if let Some(reason) = &entry.unavailable_reason {
                    text(reason).size(FONT_MICRO).color(DANGER)
                } else {
                    text(&entry.source_label).size(FONT_MICRO).color(TEXT_MUTED)
                },
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            action,
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .into()
}

fn preview_meta(app: &App) -> String {
    app.install.preview.as_ref().map_or_else(
        || "No source previewed".to_string(),
        |preview| {
            format!(
                "{} candidate(s) - {} - {}",
                preview.candidates.len(),
                preview.source_label,
                preview.scope.label(),
            )
        },
    )
}
