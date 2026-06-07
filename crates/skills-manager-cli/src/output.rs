use std::path::PathBuf;

use clap::ValueEnum;
use serde::Serialize;
use skills_manager_core::{
    DoctorReport, DownloadedSkillEntry, InstallPreview, InstallResult, InstalledSkill,
    ManagedResource, MarketplaceDocument, MarketplaceSearchResult, MarketplaceSourceRecord,
    PluginInstallPreview, PluginInstallResult, RepairReport, SkillCatalog, SkillScaffoldPreview,
    TargetProfile, WorkspaceSnapshot, format_bytes,
};

/// Supported output formats for command responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    /// Human-readable text output.
    Text,
    /// Stable JSON envelope (legacy variant).
    Json,
    /// Versioned JSON envelope for machine readers.
    JsonV3,
}

/// Public command output payloads serialized by the CLI.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandOutput {
    /// Workspace snapshot command result.
    Workspace { snapshot: WorkspaceSnapshot },
    /// Installed skills listing.
    Skills { skills: Vec<InstalledSkill> },
    /// Validation output with invalid count.
    Validation {
        skills: Vec<InstalledSkill>,
        invalid: usize,
    },
    /// Target profile listing.
    Targets { targets: Vec<TargetProfile> },
    /// Doctor summary and diagnostics.
    Doctor { report: DoctorReport },
    /// Repair action summary.
    Repair { report: RepairReport },
    /// Scaffold preview.
    Scaffold {
        dry_run: bool,
        preview: SkillScaffoldPreview,
    },
    /// Install preview output.
    Preview { preview: InstallPreview },
    /// Install execution output.
    Install { result: InstallResult },
    /// Single downloaded skill entry.
    Download { entry: DownloadedSkillEntry },
    /// All downloaded catalog entries.
    Downloads {
        downloads: Vec<DownloadedSkillEntry>,
    },
    /// Result confirming a download entry was removed.
    RemovedDownload { path: PathBuf },
    /// Serialized catalog export payload.
    CatalogExport { format: String, catalog: String },
    /// Parsed catalog payload loaded from file or URL.
    LoadedCatalog { catalog: SkillCatalog },
    /// Generic status message.
    Status { message: String },
    /// Path backup after removing a skill.
    RemovedSkill { backup: PathBuf },
    /// Inventory resources list.
    Resources { resources: Vec<ManagedResource> },
    /// Discovered plugins list.
    Plugins { plugins: Vec<ManagedResource> },
    /// Plugin install preview for a single plugin target.
    PluginPreview { preview: PluginInstallPreview },
    /// Plugin install execution result.
    PluginInstall { result: PluginInstallResult },
    /// Path backup after plugin removal.
    RemovedPlugin { backup: PathBuf },
    /// Configured marketplace sources.
    MarketplaceSources {
        sources: Vec<MarketplaceSourceRecord>,
    },
    /// Single configured marketplace source record.
    MarketplaceSource { source: MarketplaceSourceRecord },
    /// Removed marketplace source label result.
    RemovedMarketplaceSource { label: String },
    /// Marketplace inspection payload from source.
    MarketplaceInspect { marketplace: MarketplaceDocument },
    /// Marketplace search result.
    MarketplaceSearch { result: MarketplaceSearchResult },
}

/// Write command output using selected mode.
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
        CommandOutput::Resources { resources } => {
            if resources.is_empty() {
                println!("No resources found.");
            }
            for resource in resources {
                print_resource_line(resource);
            }
        }
        CommandOutput::Plugins { plugins } => {
            if plugins.is_empty() {
                println!("No plugins found.");
            }
            for plugin in plugins {
                print_resource_line(plugin);
            }
        }
        CommandOutput::PluginPreview { preview } => {
            println!(
                "Plugin preview: {} [{}] -> {}",
                preview.manifest.name,
                preview.manifest.target.label(),
                preview
                    .operation_plan
                    .destination_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "delegated command".to_string())
            );
            for diagnostic in &preview.operation_plan.diagnostics {
                println!(
                    "  - {}: {}",
                    diagnostic.severity.label(),
                    diagnostic.message
                );
            }
            for command in &preview.operation_plan.commands {
                println!("  command: {command}");
            }
        }
        CommandOutput::PluginInstall { result } => {
            if let Some(path) = &result.installed {
                println!("Installed plugin: {}", path.display());
            }
            if let Some(backup) = &result.backup {
                println!("Backup: {}", backup.display());
            }
            if let Some(output) = &result.command_output {
                print!("{output}");
            }
        }
        CommandOutput::RemovedPlugin { backup } => {
            println!("Removed plugin. Backup: {}", backup.display());
        }
        CommandOutput::MarketplaceSources { sources } => {
            if sources.is_empty() {
                println!("No marketplace sources configured.");
            }
            for source in sources {
                println!(
                    "{} [{}] {}",
                    source.label,
                    source.target.as_deref().unwrap_or("generic"),
                    source.source
                );
            }
        }
        CommandOutput::MarketplaceSource { source } => {
            println!("Marketplace source: {} -> {}", source.label, source.source);
        }
        CommandOutput::RemovedMarketplaceSource { label } => {
            println!("Removed marketplace source: {label}");
        }
        CommandOutput::MarketplaceInspect { marketplace } => {
            println!(
                "Marketplace: {} [{}] {} plugin(s)",
                marketplace.name,
                marketplace.target.label(),
                marketplace.entries.len()
            );
            for entry in &marketplace.entries {
                println!(
                    "- {}: {}",
                    entry.name,
                    entry.description.as_deref().unwrap_or("No description")
                );
            }
        }
        CommandOutput::MarketplaceSearch { result } => {
            println!(
                "{} result(s) from {} for `{}`.",
                result.entries.len(),
                result.provider.label(),
                result.query
            );
            for entry in &result.entries {
                println!(
                    "- {}: {}",
                    entry.name,
                    entry.description.as_deref().unwrap_or("No description")
                );
            }
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

fn print_resource_line(resource: &ManagedResource) {
    println!(
        "{} [{} {}] {} / {} - {} | source: {} | installed: {}",
        resource.display_name,
        resource.target.label(),
        resource.kind.label(),
        resource.enablement.label(),
        resource.health.label(),
        resource.root_dir.display(),
        resource.source_url.as_deref().unwrap_or("unknown"),
        resource
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
