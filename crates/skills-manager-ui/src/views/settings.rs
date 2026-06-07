use iced::{
    Alignment, Element, Length,
    widget::{column, row, scrollable, text},
};
use skills_manager_core::{DoctorReport, TargetDoctorReport, format_bytes};

use crate::theme::*;
use crate::{app::App, app::Message, components, icons};

pub fn view(app: &App) -> Element<'_, Message> {
    components::card(scrollable(
        column![
            path_settings(app),
            target_metrics(app.settings.doctor_report.as_ref()),
            doctor_panel(app.settings.doctor_report.as_ref()),
        ]
        .spacing(SPACING_XL),
    ))
    .height(Length::Fill)
    .into()
}

fn path_settings(app: &App) -> Element<'_, Message> {
    column![
        components::section_label("PATHS"),
        components::field(
            "Project folder",
            "Project scope resolves to <project>/.agents/skills.",
            "/path/to/project",
            &app.settings.project_path,
            Message::ProjectPathChanged,
        ),
        components::field(
            "Default download folder",
            "GitHub downloads are cached here unless overridden.",
            "/path/to/downloaded/skills",
            &app.settings.default_download_path,
            Message::DefaultDownloadPathChanged,
        ),
        components::secondary_button("Save download path", Some(icons::DOWNLOAD))
            .on_press_maybe((!app.busy).then_some(Message::SaveDefaultDownloadPath)),
    ]
    .spacing(SPACING_LG)
    .into()
}

fn target_metrics(report: Option<&DoctorReport>) -> Element<'_, Message> {
    let repair_actions = report
        .map(|report| report.summary.repair_actions)
        .unwrap_or_default();
    let invalid = report
        .map(|report| report.summary.invalid)
        .unwrap_or_default();
    let warnings = report
        .map(|report| report.summary.warnings)
        .unwrap_or_default();
    let targets = report
        .map(|report| report.summary.targets)
        .unwrap_or_default();

    column![
        components::section_label("HEALTH OVERVIEW"),
        row![
            components::metric("Targets", targets.to_string(), PRIMARY),
            components::metric("Invalid", invalid.to_string(), DANGER),
            components::metric("Warnings", warnings.to_string(), WARNING),
            components::metric("Repairs", repair_actions.to_string(), SUCCESS),
        ]
        .spacing(SPACING_MD),
    ]
    .spacing(SPACING_MD)
    .into()
}

fn doctor_panel(report: Option<&DoctorReport>) -> Element<'_, Message> {
    let Some(report) = report else {
        return components::empty_state(
            "Doctor not loaded",
            "Refresh to inspect target paths and diagnostics.",
        )
        .into();
    };

    column![
        components::section_header(
            "Doctor Report",
            format!(
                "{} target(s), {} invalid, {} repair(s)",
                report.summary.targets, report.summary.invalid, report.summary.repair_actions
            ),
        ),
        components::list_column(report.targets.iter(), SPACING_MD, target_doctor_row),
    ]
    .spacing(SPACING_MD)
    .into()
}

fn target_doctor_row(target: &TargetDoctorReport) -> Element<'_, Message> {
    let diagnostics = components::diagnostic_lines(&target.diagnostics, "No diagnostics");
    let repairs = components::text_lines(
        target
            .repair_actions
            .iter()
            .map(|action| format!("Repair: {} - {}", action.label, action.description)),
        "No repair actions",
        WARNING,
        FONT_MICRO,
    );

    components::flat_card(
        column![
            row![
                components::scope_chip(target.profile.scope),
                text(format!(
                    "{} usable / {} disabled / {} invalid / {} total",
                    target.counts.usable,
                    target.counts.disabled,
                    target.counts.invalid,
                    target.counts.total
                ))
                .size(FONT_CAPTION)
                .color(TEXT_SECONDARY),
            ]
            .spacing(SPACING_SM)
            .align_y(Alignment::Center),
            components::detail_row(
                "Root",
                target
                    .profile
                    .skills_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Unavailable".to_string()),
            ),
            components::detail_row(
                "Disabled store",
                target
                    .profile
                    .disabled_store_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Config toggle".to_string()),
            ),
            if let Some(bytes) = target.catalog_bytes {
                components::detail_row("Catalog", format_bytes(bytes))
            } else {
                components::detail_row("Catalog", "N/A".to_string())
            },
            diagnostics,
            repairs,
        ]
        .spacing(SPACING_SM),
    )
    .into()
}
