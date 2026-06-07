use iced::{
    Alignment, Element, Length,
    widget::{column, row, scrollable, text},
};
use skills_manager_core::{DoctorReport, TargetDoctorReport, format_bytes};

use crate::{app::App, app::Message, components, icons, theme};

pub fn view(app: &App) -> Element<'_, Message> {
    let counts = app.counts();

    components::panel(scrollable(
        column![
            components::section_header("Settings", "Project, target, and download cache paths"),
            components::field(
                "Project folder",
                "Project scope resolves to <project>/.agents/skills.",
                "/path/to/project",
                &app.settings.project_path,
                Message::ProjectPathChanged,
            ),
            components::field(
                "Default download folder",
                "GitHub skill downloads are cached here unless Install uses an override.",
                "/path/to/downloaded/skills",
                &app.settings.default_download_path,
                Message::DefaultDownloadPathChanged,
            ),
            row![
                components::secondary_button("Save download path", Some(icons::DOWNLOAD))
                    .on_press_maybe((!app.busy).then_some(Message::SaveDefaultDownloadPath)),
            ]
            .spacing(8),
            row![
                components::compact_metric("Project", counts.project.to_string(), theme::PRIMARY),
                components::compact_metric("Global", counts.global.to_string(), theme::CYAN),
                components::compact_metric(
                    "Agents",
                    (counts.claude_code
                        + counts.droid
                        + counts.pencode
                        + counts.codex
                        + counts.zed)
                        .to_string(),
                    theme::WARNING
                ),
                components::compact_metric("Custom", counts.custom.to_string(), theme::WARNING),
                components::compact_metric(
                    "Exportable",
                    counts.exportable.to_string(),
                    theme::SUCCESS
                ),
            ]
            .spacing(8),
            doctor_panel(app.settings.doctor_report.as_ref()),
            storage_summary(),
        ]
        .spacing(12),
    ))
    .height(Length::Fill)
    .into()
}

fn doctor_panel(report: Option<&DoctorReport>) -> Element<'_, Message> {
    let Some(report) = report else {
        return components::empty_state(
            "Doctor not loaded",
            "Refresh to inspect target paths, disabled stores, and stale config.",
        )
        .into();
    };

    components::flat_panel(
        column![
            components::section_header(
                "Doctor",
                format!(
                    "{} target(s), {} skill(s), {} invalid, {} repair action(s)",
                    report.summary.targets,
                    report.summary.skills,
                    report.summary.invalid,
                    report.summary.repair_actions
                ),
            ),
            report
                .targets
                .iter()
                .fold(column![].spacing(8), |list, target| {
                    list.push(target_doctor_row(target))
                }),
        ]
        .spacing(10),
    )
    .into()
}

fn target_doctor_row(target: &TargetDoctorReport) -> Element<'_, Message> {
    let diagnostics: Element<'_, Message> = if target.diagnostics.is_empty() {
        text("No target diagnostics")
            .size(11)
            .color(theme::SUBTLE)
            .into()
    } else {
        target
            .diagnostics
            .iter()
            .fold(column![].spacing(3), |list, diagnostic| {
                list.push(
                    text(format!(
                        "{}: {}",
                        diagnostic.severity.label(),
                        diagnostic.message
                    ))
                    .size(11)
                    .color(theme::MUTED),
                )
            })
            .into()
    };
    let repairs: Element<'_, Message> = if target.repair_actions.is_empty() {
        text("No repair actions")
            .size(11)
            .color(theme::SUBTLE)
            .into()
    } else {
        target
            .repair_actions
            .iter()
            .fold(column![].spacing(3), |list, action| {
                list.push(
                    text(format!("Repair: {} - {}", action.label, action.description))
                        .size(11)
                        .color(theme::WARNING)
                        .wrapping(text::Wrapping::WordOrGlyph),
                )
            })
            .into()
    };

    components::flat_panel(
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
                .size(12)
                .color(theme::MUTED),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            storage_line(
                "Root",
                target
                    .profile
                    .skills_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Unavailable".to_string()),
            ),
            storage_line(
                "Disabled",
                target
                    .profile
                    .disabled_store_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Config toggle".to_string()),
            ),
            if let Some(bytes) = target.catalog_bytes {
                storage_line("Catalog", format_bytes(bytes))
            } else {
                storage_line("Catalog", "Not applicable".to_string())
            },
            diagnostics,
            repairs,
        ]
        .spacing(6),
    )
    .into()
}

fn storage_line<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label)
            .size(11)
            .color(theme::SUBTLE)
            .width(Length::Fixed(64.0)),
        text(value)
            .size(11)
            .color(theme::MUTED)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn storage_summary<'a>() -> Element<'a, Message> {
    components::flat_panel(
        column![
            storage_row(
                icons::FOLDER,
                "Project",
                "<project>/.agents/skills",
                "Highest priority"
            ),
            storage_row(
                icons::GLOBE,
                "Global",
                "~/.agents/skills",
                "Shared across projects"
            ),
            storage_row(
                icons::FOLDER,
                "Claude Code",
                "~/.claude/skills",
                "Claude Code target"
            ),
            storage_row(icons::FOLDER, "Droid", "~/.droid/skills", "Droid target"),
            storage_row(
                icons::FOLDER,
                "Pencode",
                "~/.pencode/skills",
                "Pencode target"
            ),
            storage_row(icons::FOLDER, "Codex", "~/.codex/skills", "Codex target"),
            storage_row(icons::FOLDER, "Zed", "~/.config/zed/skills", "Zed target"),
            storage_row(
                icons::FOLDER,
                "Custom",
                "Configured from Install",
                "Scanned after built-in targets"
            ),
            row![
                icons::icon(icons::SHIELD, 16),
                text("Validation checks frontmatter, naming, descriptions, resources, compatibility, and shadowing.")
                    .size(12)
                    .color(theme::MUTED)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    )
    .into()
}

fn storage_row<'a>(
    icon: &'static str,
    label: &'a str,
    path: &'a str,
    note: &'a str,
) -> Element<'a, Message> {
    row![
        icons::icon(icon, 16),
        text(label)
            .size(13)
            .color(theme::TEXT)
            .width(Length::Fixed(70.0)),
        column![
            text(path)
                .size(12)
                .color(theme::MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            text(note)
                .size(11)
                .color(theme::SUBTLE)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
