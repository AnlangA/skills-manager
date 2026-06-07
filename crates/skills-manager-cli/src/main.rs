use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use skills_manager_core::{
    CatalogFormat, ConflictPolicy, DoctorReport, DownloadedSkillEntry, InstallPreview,
    InstallRequest, InstallResult, InstallTarget, InstalledSkill, Installer, ManagerPaths,
    ProjectRoot, RepairReport, SkillCatalog, SkillHealth, SkillScaffoldPreview,
    SkillScaffoldRequest, TargetProfile, WorkspaceSnapshot, create_skill_scaffold, doctor_report,
    download_github_catalog, download_github_skills_to_cache, export_installed_catalog,
    format_bytes, list_downloaded_skills, preview_skill_scaffold, remove_downloaded_skills,
    repair_targets, scan_installed_skills, target_profiles,
};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, about = "Manage local Agent Skills libraries")]
struct Cli {
    #[arg(long, value_name = "DIR", global = true)]
    project: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = OutputMode::Text, global = true)]
    output: OutputMode,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputMode {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
enum Command {
    Workspace,
    Inventory {
        #[command(subcommand)]
        command: InventoryCommand,
    },
    Install {
        #[command(subcommand)]
        command: InstallCommand,
    },
    Scan,
    Validate {
        #[arg(long, value_enum)]
        target: Option<CliTarget>,
    },
    Targets,
    Doctor,
    Repair {
        #[arg(long)]
        apply: bool,
    },
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "allowed-tool")]
        allowed_tools: Vec<String>,
        #[arg(long)]
        compatibility: Option<String>,
        #[arg(long)]
        license: Option<String>,
        #[arg(long)]
        when_to_use: Option<String>,
        #[arg(long)]
        disable_model_invocation: bool,
        #[arg(long)]
        dry_run: bool,
    },
    PreviewInstall {
        source: String,
        #[arg(long)]
        local: bool,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_enum, hide = true)]
        scope: Option<CliScope>,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        download_dir: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    InstallUrl {
        url: String,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_enum, hide = true)]
        scope: Option<CliScope>,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        download_dir: Option<PathBuf>,
        #[arg(long)]
        disable_after_install: bool,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    InstallLocal {
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_enum, hide = true)]
        scope: Option<CliScope>,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long)]
        disable_after_install: bool,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    Downloads {
        #[command(subcommand)]
        command: DownloadCommand,
    },
    Catalog {
        #[arg(long, value_enum, default_value_t = CliCatalogFormat::Json)]
        format: CliCatalogFormat,
    },
    LoadCatalog {
        url: String,
    },
    Enable {
        path: PathBuf,
        #[arg(long, value_enum)]
        target: Option<CliTarget>,
    },
    Disable {
        path: PathBuf,
        #[arg(long, value_enum)]
        target: Option<CliTarget>,
    },
    Remove {
        path: PathBuf,
        #[arg(long, value_enum)]
        target: Option<CliTarget>,
    },
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Inventory { .. } => "inventory",
            Self::Install { .. } => "install",
            Self::Scan => "scan",
            Self::Validate { .. } => "validate",
            Self::Targets => "targets",
            Self::Doctor => "doctor",
            Self::Repair { .. } => "repair",
            Self::Create { .. } => "create",
            Self::PreviewInstall { .. } => "preview-install",
            Self::InstallUrl { .. } => "install-url",
            Self::InstallLocal { .. } => "install-local",
            Self::Downloads { .. } => "downloads",
            Self::Catalog { .. } => "catalog",
            Self::LoadCatalog { .. } => "load-catalog",
            Self::Enable { .. } => "enable",
            Self::Disable { .. } => "disable",
            Self::Remove { .. } => "remove",
        }
    }
}

#[derive(Debug, Subcommand)]
enum InventoryCommand {
    Scan,
    Validate {
        #[arg(long, value_enum)]
        target: Option<CliTarget>,
    },
}

#[derive(Debug, Subcommand)]
enum InstallCommand {
    Preview {
        source: String,
        #[arg(long)]
        local: bool,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_enum, hide = true)]
        scope: Option<CliScope>,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        download_dir: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    Url {
        url: String,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_enum, hide = true)]
        scope: Option<CliScope>,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        download_dir: Option<PathBuf>,
        #[arg(long)]
        disable_after_install: bool,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    Local {
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = CliTarget::Global)]
        target: CliTarget,
        #[arg(long, value_enum, hide = true)]
        scope: Option<CliScope>,
        #[arg(long, value_name = "DIR")]
        dest: Option<PathBuf>,
        #[arg(long)]
        disable_after_install: bool,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
}

#[derive(Debug, Subcommand)]
enum DownloadCommand {
    Add {
        url: String,
        #[arg(long, value_name = "DIR")]
        dir: Option<PathBuf>,
    },
    List,
    Remove {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliScope {
    #[value(alias = "user", alias = "gloab")]
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliTarget {
    #[value(alias = "user", alias = "gloab")]
    Global,
    Project,
    #[value(name = "claude-code", alias = "claude", alias = "claudecode")]
    ClaudeCode,
    Droid,
    #[value(alias = "opencode")]
    Pencode,
    Codex,
    Zed,
}

impl From<CliScope> for CliTarget {
    fn from(value: CliScope) -> Self {
        match value {
            CliScope::Global => Self::Global,
            CliScope::Project => Self::Project,
        }
    }
}

impl From<CliTarget> for InstallTarget {
    fn from(value: CliTarget) -> Self {
        match value {
            CliTarget::Global => Self::Global,
            CliTarget::Project => Self::Project,
            CliTarget::ClaudeCode => Self::ClaudeCode,
            CliTarget::Droid => Self::Droid,
            CliTarget::Pencode => Self::Pencode,
            CliTarget::Codex => Self::Codex,
            CliTarget::Zed => Self::Zed,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliConflictPolicy {
    Block,
    Replace,
    Rename,
}

impl From<CliConflictPolicy> for ConflictPolicy {
    fn from(value: CliConflictPolicy) -> Self {
        match value {
            CliConflictPolicy::Block => Self::Block,
            CliConflictPolicy::Replace => Self::Replace,
            CliConflictPolicy::Rename => Self::Rename,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliCatalogFormat {
    Json,
    Xml,
    Markdown,
}

impl From<CliCatalogFormat> for CatalogFormat {
    fn from(value: CliCatalogFormat) -> Self {
        match value {
            CliCatalogFormat::Json => Self::Json,
            CliCatalogFormat::Xml => Self::Xml,
            CliCatalogFormat::Markdown => Self::Markdown,
        }
    }
}

impl CliCatalogFormat {
    fn label(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CommandOutput {
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

struct PreviewInstallArgs {
    source: String,
    local: bool,
    target: CliTarget,
    scope: Option<CliScope>,
    dest: Option<PathBuf>,
    download_dir: Option<PathBuf>,
    conflict: CliConflictPolicy,
}

struct InstallUrlArgs {
    url: String,
    target: CliTarget,
    scope: Option<CliScope>,
    dest: Option<PathBuf>,
    download_dir: Option<PathBuf>,
    disable_after_install: bool,
    conflict: CliConflictPolicy,
}

struct InstallLocalArgs {
    path: PathBuf,
    target: CliTarget,
    scope: Option<CliScope>,
    dest: Option<PathBuf>,
    disable_after_install: bool,
    conflict: CliConflictPolicy,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let Cli {
        project,
        output,
        command,
    } = Cli::parse();
    tracing::info!(?command, ?output, "running skills-manager command");

    let paths = ManagerPaths::new(project.map(ProjectRoot::new))?;
    let installer = Installer::new(paths.clone());
    let command_name = command.name().to_string();
    let output_value = run_command(command, &paths, &installer).await?;
    write_output(output, &command_name, &output_value)?;

    Ok(())
}

async fn run_command(
    command: Command,
    paths: &ManagerPaths,
    installer: &Installer,
) -> Result<CommandOutput> {
    match command {
        Command::Workspace => {
            let snapshot = WorkspaceSnapshot::load(paths)?;
            Ok(CommandOutput::Workspace { snapshot })
        }
        Command::Inventory { command } => run_inventory_command(command, paths).await,
        Command::Install { command } => run_install_command(command, paths, installer).await,
        Command::Scan => {
            let skills = scan_installed_skills(paths)?;
            Ok(CommandOutput::Skills { skills })
        }
        Command::Validate { target } => {
            let mut skills = scan_installed_skills(paths)?;
            if let Some(target) = target {
                let scope = InstallTarget::from(target).scope();
                skills.retain(|skill| skill.scope == scope);
            }
            let invalid = skills
                .iter()
                .filter(|skill| skill.health == SkillHealth::Invalid)
                .count();
            Ok(CommandOutput::Validation { skills, invalid })
        }
        Command::Targets => {
            let targets = target_profiles(paths)?;
            Ok(CommandOutput::Targets { targets })
        }
        Command::Doctor => {
            let report = doctor_report(paths)?;
            Ok(CommandOutput::Doctor { report })
        }
        Command::Repair { apply } => {
            let report = repair_targets(paths, !apply)?;
            Ok(CommandOutput::Repair { report })
        }
        Command::Create {
            name,
            description,
            target,
            dest,
            tags,
            allowed_tools,
            compatibility,
            license,
            when_to_use,
            disable_model_invocation,
            dry_run,
        } => {
            let request = SkillScaffoldRequest {
                name,
                description,
                target: install_target(target, None, dest),
                tags,
                allowed_tools,
                compatibility,
                license,
                when_to_use,
                disable_model_invocation: disable_model_invocation.then_some(true),
            };
            let preview = if dry_run {
                preview_skill_scaffold(paths, request)?
            } else {
                create_skill_scaffold(paths, request)?
            };
            Ok(CommandOutput::Scaffold { dry_run, preview })
        }
        Command::PreviewInstall {
            source,
            local,
            target,
            scope,
            dest,
            download_dir,
            conflict,
        } => {
            preview_install_command(
                PreviewInstallArgs {
                    source,
                    local,
                    target,
                    scope,
                    dest,
                    download_dir,
                    conflict,
                },
                paths,
                installer,
            )
            .await
        }
        Command::InstallUrl {
            url,
            target,
            scope,
            dest,
            download_dir,
            disable_after_install,
            conflict,
        } => {
            install_url_command(
                InstallUrlArgs {
                    url,
                    target,
                    scope,
                    dest,
                    download_dir,
                    disable_after_install,
                    conflict,
                },
                paths,
                installer,
            )
            .await
        }
        Command::InstallLocal {
            path,
            target,
            scope,
            dest,
            disable_after_install,
            conflict,
        } => install_local_command(
            InstallLocalArgs {
                path,
                target,
                scope,
                dest,
                disable_after_install,
                conflict,
            },
            installer,
        ),
        Command::Downloads { command } => match command {
            DownloadCommand::Add { url, dir } => {
                let downloaded = download_github_skills_to_cache(paths, &url, dir.as_deref())
                    .await
                    .with_context(|| format!("failed to download skills from {url}"))?;
                Ok(CommandOutput::Download { entry: downloaded })
            }
            DownloadCommand::List => {
                let downloads = list_downloaded_skills(paths)?;
                Ok(CommandOutput::Downloads { downloads })
            }
            DownloadCommand::Remove { path } => {
                let removed = remove_downloaded_skills(paths, &path)?;
                Ok(CommandOutput::RemovedDownload { path: removed })
            }
        },
        Command::Catalog { format } => {
            let skills = scan_installed_skills(paths)?;
            let exported = export_installed_catalog("Agent Skills", &skills, format.into())?;
            Ok(CommandOutput::CatalogExport {
                format: format.label().to_string(),
                catalog: exported,
            })
        }
        Command::LoadCatalog { url } => {
            let downloaded = download_github_catalog(&url)
                .await
                .with_context(|| format!("failed to download catalog from {url}"))?;
            Ok(CommandOutput::LoadedCatalog {
                catalog: downloaded.catalog,
            })
        }
        Command::Enable { path, target } => {
            installer.set_skill_enabled(&skill_root(paths, target, path)?, true)?;
            Ok(CommandOutput::Status {
                message: "Enabled skill.".to_string(),
            })
        }
        Command::Disable { path, target } => {
            installer.set_skill_enabled(&skill_root(paths, target, path)?, false)?;
            Ok(CommandOutput::Status {
                message: "Disabled skill.".to_string(),
            })
        }
        Command::Remove { path, target } => {
            let backup = installer.remove(&skill_root(paths, target, path)?)?;
            Ok(CommandOutput::RemovedSkill { backup })
        }
    }
}

async fn run_inventory_command(
    command: InventoryCommand,
    paths: &ManagerPaths,
) -> Result<CommandOutput> {
    match command {
        InventoryCommand::Scan => {
            let snapshot = WorkspaceSnapshot::load(paths)?;
            Ok(CommandOutput::Workspace { snapshot })
        }
        InventoryCommand::Validate { target } => {
            let mut skills = scan_installed_skills(paths)?;
            if let Some(target) = target {
                let scope = InstallTarget::from(target).scope();
                skills.retain(|skill| skill.scope == scope);
            }
            let invalid = skills
                .iter()
                .filter(|skill| skill.health == SkillHealth::Invalid)
                .count();
            Ok(CommandOutput::Validation { skills, invalid })
        }
    }
}

async fn run_install_command(
    command: InstallCommand,
    paths: &ManagerPaths,
    installer: &Installer,
) -> Result<CommandOutput> {
    match command {
        InstallCommand::Preview {
            source,
            local,
            target,
            scope,
            dest,
            download_dir,
            conflict,
        } => {
            preview_install_command(
                PreviewInstallArgs {
                    source,
                    local,
                    target,
                    scope,
                    dest,
                    download_dir,
                    conflict,
                },
                paths,
                installer,
            )
            .await
        }
        InstallCommand::Url {
            url,
            target,
            scope,
            dest,
            download_dir,
            disable_after_install,
            conflict,
        } => {
            install_url_command(
                InstallUrlArgs {
                    url,
                    target,
                    scope,
                    dest,
                    download_dir,
                    disable_after_install,
                    conflict,
                },
                paths,
                installer,
            )
            .await
        }
        InstallCommand::Local {
            path,
            target,
            scope,
            dest,
            disable_after_install,
            conflict,
        } => install_local_command(
            InstallLocalArgs {
                path,
                target,
                scope,
                dest,
                disable_after_install,
                conflict,
            },
            installer,
        ),
    }
}

async fn preview_install_command(
    args: PreviewInstallArgs,
    paths: &ManagerPaths,
    installer: &Installer,
) -> Result<CommandOutput> {
    let install_target = install_target(args.target, args.scope, args.dest);
    let preview = if args.local {
        installer.preview(
            Path::new(&args.source),
            install_target,
            args.conflict.into(),
        )?
    } else {
        let downloaded =
            download_github_skills_to_cache(paths, &args.source, args.download_dir.as_deref())
                .await
                .with_context(|| format!("failed to download skills from {}", args.source))?;
        installer.preview(
            &downloaded.search_root,
            install_target,
            args.conflict.into(),
        )?
    };
    Ok(CommandOutput::Preview { preview })
}

async fn install_url_command(
    args: InstallUrlArgs,
    paths: &ManagerPaths,
    installer: &Installer,
) -> Result<CommandOutput> {
    let downloaded =
        download_github_skills_to_cache(paths, &args.url, args.download_dir.as_deref())
            .await
            .with_context(|| format!("failed to download skills from {}", args.url))?;
    let plan = installer.plan(InstallRequest {
        source_root: downloaded.search_root,
        source_url: Some(args.url),
        target: install_target(args.target, args.scope, args.dest),
        conflict_policy: args.conflict.into(),
        enable_after_install: !args.disable_after_install,
    })?;
    let result = installer.install_plan(plan)?;
    Ok(CommandOutput::Install { result })
}

fn install_local_command(args: InstallLocalArgs, installer: &Installer) -> Result<CommandOutput> {
    let plan = installer.plan(InstallRequest {
        source_root: args.path,
        source_url: None,
        target: install_target(args.target, args.scope, args.dest),
        conflict_policy: args.conflict.into(),
        enable_after_install: !args.disable_after_install,
    })?;
    let result = installer.install_plan(plan)?;
    Ok(CommandOutput::Install { result })
}

fn write_output(mode: OutputMode, command: &str, output: &CommandOutput) -> Result<()> {
    match mode {
        OutputMode::Text => write_text_output(output),
        OutputMode::Json => {
            serde_json::to_writer_pretty(
                std::io::stdout(),
                &OutputEnvelope::success(command, output),
            )?;
            println!();
            Ok(())
        }
    }
}

#[derive(Debug, Serialize)]
struct OutputEnvelope<'a> {
    schema_version: u16,
    command: &'a str,
    status: &'a str,
    data: &'a CommandOutput,
    #[serde(flatten)]
    legacy: &'a CommandOutput,
    diagnostics: Vec<String>,
}

impl<'a> OutputEnvelope<'a> {
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

fn write_text_output(output: &CommandOutput) -> Result<()> {
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

fn install_target(
    target: CliTarget,
    legacy_scope: Option<CliScope>,
    dest: Option<PathBuf>,
) -> InstallTarget {
    if let Some(path) = dest {
        return InstallTarget::Custom(path);
    }

    legacy_scope.map(CliTarget::from).unwrap_or(target).into()
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

fn skill_root(paths: &ManagerPaths, target: Option<CliTarget>, path: PathBuf) -> Result<PathBuf> {
    let resolved = if path.is_absolute() {
        path
    } else if let Some(target) = target {
        InstallTarget::from(target)
            .destination_root(paths)?
            .join(path)
    } else {
        path
    };

    if resolved.file_name().is_some_and(|name| name == "SKILL.md") {
        Ok(resolved.parent().map(Path::to_path_buf).unwrap_or(resolved))
    } else {
        Ok(resolved)
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("off"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .compact()
        .init();
}
