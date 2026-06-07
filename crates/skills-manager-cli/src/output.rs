use std::path::PathBuf;

use clap::ValueEnum;
use serde::Serialize;
use skills_manager_core::{
    DoctorReport, DownloadedSkillEntry, InstallPreview, InstallResult, InstalledSkill,
    RepairReport, SkillCatalog, SkillScaffoldPreview, TargetProfile, WorkspaceSnapshot,
    format_bytes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    Text,
    Json,
    JsonV3,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandOutput {
    Workspace {
        snapshot: WorkspaceSnapshot,
    },
    Skills {
        skills: Vec<InstalledSkill>,
    },
    Validation {
        skills: Vec<InstalledSkill>,
        invalid: usize,
    },
    Targets {
        targets: Vec<TargetProfile>,
    },
    Doctor {
        report: DoctorReport,
    },
    Repair {
        report: RepairReport,
    },
    Scaffold {
        dry_run: bool,
        preview: SkillScaffoldPreview,
    },
    Preview {
        preview: InstallPreview,
    },
    Install {
        result: InstallResult,
    },
    Download {
        entry: DownloadedSkillEntry,
    },
    Downloads {
        downloads: Vec<DownloadedSkillEntry>,
    },
    RemovedDownload {
        path: PathBuf,
    },
    CatalogExport {
        format: String,
        catalog: String,
    },
    LoadedCatalog {
        catalog: SkillCatalog,
    },
    Status {
        message: String,
    },
    RemovedSkill {
        backup: PathBuf,
    },
}

pub fn write_output(mode: OutputMode, command: &str, output: &CommandOutput) -> anyhow::Result<()> {
    match mode {
        OutputMode::Text => write_text_output(output),
        OutputMode::Json => {
            serde_json::to_writer_pretty(
                std::io::stdout(),
                &OutputEnvelopeV2::success(command, output),
            )?;
            println!();
            Ok(())
        }
        OutputMode::JsonV3 => {
            serde_json::to_writer_pretty(
                std::io::stdout(),
                &OutputEnvelopeV3::success(command, output),
            )?;
            println!();
            Ok(())
        }
    }
}

#[derive(Debug, Serialize)]
struct OutputEnvelopeV2<'a> {
    schema_version: u16,
    command: &'a str,
    status: &'a str,
    data: &'a CommandOutput,
    #[serde(flatten)]
    legacy: &'a CommandOutput,
    diagnostics: Vec<String>,
}

impl<'a> OutputEnvelopeV2<'a> {
    fn success(command: &'a str, data: &'a CommandOutput) -> Self {
        Self {
            schema_version: 2,
            command,
            status: "ok",
            data,
            legacy: data,
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OutputEnvelopeV3<'a> {
    schema_version: u16,
    command: &'a str,
    status: &'a str,
    data: &'a CommandOutput,
    diagnostics: Vec<String>,
    meta: OutputMeta,
}

#[derive(Debug, Serialize)]
struct OutputMeta {
    format: &'static str,
    legacy_flattened_fields: bool,
}

impl<'a> OutputEnvelopeV3<'a> {
    fn success(command: &'a str, data: &'a CommandOutput) -> Self {
        Self {
            schema_version: 3,
            command,
            status: "ok",
            data,
            diagnostics: Vec::new(),
            meta: OutputMeta {
                format: "json-v3",
                legacy_flattened_fields: false,
            },
        }
    }
}

fn write_text_output(output: &CommandOutput) -> anyhow::Result<()> {
    match output {
        CommandOutput::Workspace { snapshot } => {
            println!(
                "Workspace: {} skill(s), {} download(s), {} target(s), {} exportable.",
                snapshot.counts.total,
                snapshot.downloads.len(),
                snapshot.target_profiles.len(),
                snapshot.counts.exportable
            );
        }
        CommandOutput::Skills { skills } => {
            if skills.is_empty() {
                println!("No skills found.");
            }

            for skill in skills {
                print_skill_line(skill);
            }
        }
        CommandOutput::Validation { skills, invalid } => {
            if skills.is_empty() {
                println!("No skills found.");
            }

            for skill in skills {
                print_skill_line(skill);
                for diagnostic in &skill.diagnostics {
                    println!(
                        "  - {}: {}",
                        diagnostic.severity.label(),
                        diagnostic.message
                    );
                }
            }

            if *invalid > 0 {
                println!("{invalid} invalid skill(s) found.");
            } else {
                println!("All scanned skills are usable.");
            }
        }
        CommandOutput::Targets { targets } => {
            for target in targets {
                println!(
                    "{} [{}] root: {} | disabled: {} | strategy: {:?}",
                    target.label,
                    target.scope.id_prefix(),
                    target
                        .skills_root
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "unavailable".to_string()),
                    target
                        .disabled_store_root
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "config toggle".to_string()),
                    target.enablement_strategy
                );
            }
        }
        CommandOutput::Doctor { report } => {
            println!(
                "Doctor: {} target(s), {} skill(s), {} invalid, {} repair action(s).",
                report.summary.targets,
                report.summary.skills,
                report.summary.invalid,
                report.summary.repair_actions
            );
            for target in &report.targets {
                println!(
                    "- {}: {} total / {} usable / {} disabled / {} invalid",
                    target.profile.label,
                    target.counts.total,
                    target.counts.usable,
                    target.counts.disabled,
                    target.counts.invalid
                );
                if let Some(bytes) = target.catalog_bytes {
                    println!("  catalog: {}", format_bytes(bytes));
                }
                for diagnostic in &target.diagnostics {
                    println!(
                        "  - {}: {}",
                        diagnostic.severity.label(),
                        diagnostic.message
                    );
                }
                for action in &target.repair_actions {
                    println!("  repair: {} - {}", action.label, action.description);
                }
            }
        }
        CommandOutput::Repair { report } => {
            let mode = if report.dry_run {
                "Repair dry-run"
            } else {
                "Repair"
            };
            println!("{mode}: {} action(s).", report.actions.len());
            for action in &report.actions {
                println!(
                    "- {}: {}{}",
                    action.label,
                    action.message,
                    if action.applied { " [applied]" } else { "" }
                );
            }
        }
        CommandOutput::Scaffold { dry_run, preview } => {
            let verb = if *dry_run { "Would create" } else { "Created" };
            println!(
                "{verb}: {} [{}] at {}",
                preview
                    .frontmatter
                    .name
                    .as_deref()
                    .unwrap_or("unnamed skill"),
                preview.scope.label(),
                preview.destination_root.display()
            );
            println!("Skill file: {}", preview.skill_file.display());
            for diagnostic in &preview.diagnostics {
                println!(
                    "  - {}: {}",
                    diagnostic.severity.label(),
                    diagnostic.message
                );
            }
        }
        CommandOutput::Preview { preview } => print_preview(preview),
        CommandOutput::Install { result } => print_install_result(result),
        CommandOutput::Download { entry } => print_downloaded_entry(entry),
        CommandOutput::Downloads { downloads } => {
            if downloads.is_empty() {
                println!("No downloaded skills found.");
            }
            for entry in downloads {
                print_downloaded_entry(entry);
            }
        }
        CommandOutput::RemovedDownload { path } => {
            println!("Removed downloaded skills: {}", path.display());
        }
        CommandOutput::CatalogExport { catalog, .. } => {
            print!("{catalog}");
        }
        CommandOutput::LoadedCatalog { catalog } => {
            println!(
                "Catalog: {}",
                catalog.name.as_deref().unwrap_or("Unnamed catalog")
            );
            for skill in &catalog.skills {
                println!(
                    "- {}: {}",
                    skill.display_name.as_deref().unwrap_or(&skill.name),
                    skill.description.as_deref().unwrap_or("No description")
                );
            }
        }
        CommandOutput::Status { message } => {
            println!("{message}");
        }
        CommandOutput::RemovedSkill { backup } => {
            println!("Removed skill. Backup: {}", backup.display());
        }
    }

    Ok(())
}

fn print_skill_line(skill: &InstalledSkill) {
    println!(
        "{} [{}] {} / {} - {} | resources: {} ({}) | source: {} | installed: {}",
        skill.display_name,
        skill.scope.label(),
        skill.enablement.label(),
        skill.health.label(),
        skill.root_dir.display(),
        skill.resource_count,
        format_bytes(skill.resource_bytes),
        skill.source_url.as_deref().unwrap_or("unknown"),
        skill
            .installed_at
            .map(|time| time.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string())
    );
}

fn print_preview(preview: &InstallPreview) {
    println!(
        "Preview: {} candidate(s) for {} scope at {}",
        preview.candidates.len(),
        preview.scope.label(),
        preview.destination_root.display()
    );
    for candidate in &preview.candidates {
        let name = candidate
            .frontmatter
            .name
            .as_deref()
            .unwrap_or("unnamed skill");
        println!(
            "- {name}: {} -> {} | conflict: {} | health: {} | resources: {} ({})",
            candidate.source_root.display(),
            candidate.destination_root.display(),
            candidate.conflict,
            candidate.health.label(),
            candidate.resource_count,
            format_bytes(candidate.resource_bytes)
        );
        for diagnostic in &candidate.diagnostics {
            println!(
                "  - {}: {}",
                diagnostic.severity.label(),
                diagnostic.message
            );
        }
    }
}

fn print_install_result(result: &InstallResult) {
    for path in &result.installed {
        println!("Installed: {}", path.display());
    }
    for backup in &result.backups {
        println!("Backup: {}", backup.display());
    }
}

fn print_downloaded_entry(entry: &DownloadedSkillEntry) {
    println!(
        "Downloaded: {} | source: {} | root: {} | search: {} | {}",
        entry.id,
        entry.source_url,
        entry.root_dir.display(),
        entry.search_root.display(),
        entry.resource_summary()
    );
}
