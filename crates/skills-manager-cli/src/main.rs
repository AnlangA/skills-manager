//! `skills-manager` CLI entrypoint.
//!
//! Provides the command-line interface for discovering, installing, validating,
//! exporting, and managing local Agent Skills libraries. Built on top of
//! `skills-manager-core` for all domain logic.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use skills_manager_core::{
    AgentToolTarget, CatalogFormat, ConflictPolicy, InstallRequest, InstallTarget, Installer,
    ManagerPaths, MarketplaceSearchProvider, PluginInstallRequest, ProjectRoot, ResourceKind,
    ResourceManager, SkillHealth, SkillScaffoldRequest, WorkspaceSnapshot, add_marketplace_source,
    create_skill_scaffold, doctor_report, download_github_catalog, download_github_skills,
    download_github_skills_to_cache, export_installed_catalog, inspect_marketplace_source,
    list_downloaded_skills, list_marketplace_sources, preview_skill_scaffold,
    refresh_marketplace_source, remove_downloaded_skills, remove_marketplace_source,
    repair_targets, scan_installed_plugins, scan_installed_skills, scan_marketplaces,
    scan_resources, search_marketplace, target_profiles,
};
use tracing_subscriber::EnvFilter;

mod output;

use output::{CommandOutput, OutputMode, write_output};

/// Top-level CLI arguments parsed by `clap`.
#[derive(Debug, Parser)]
#[command(version, about = "Manage local Agent Skills libraries")]
struct Cli {
    /// Optional project directory override for project-scoped operations.
    #[arg(long, value_name = "DIR", global = true)]
    project: Option<PathBuf>,

    /// Output format selection.
    #[arg(long, value_enum, default_value_t = OutputMode::Text, global = true)]
    output: OutputMode,

    /// Subcommand to execute.
    #[command(subcommand)]
    command: Command,
}

/// Available CLI subcommands.
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
    Resources {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Plugins {
        #[command(subcommand)]
        command: PluginCommand,
    },
    Marketplaces {
        #[command(subcommand)]
        command: MarketplaceCommand,
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
            Self::Resources { .. } => "resources",
            Self::Plugins { .. } => "plugins",
            Self::Marketplaces { .. } => "marketplaces",
        }
    }
}

/// Resource scan subcommands.
#[derive(Debug, Subcommand)]
enum ResourceCommand {
    /// Scan and list all resources of a given kind.
    Scan {
        #[arg(long, value_enum, default_value_t = CliResourceKind::All)]
        kind: CliResourceKind,
        #[arg(long, value_enum)]
        target: Option<CliAgentTarget>,
    },
}

/// Plugin management subcommands.
#[derive(Debug, Subcommand)]
enum PluginCommand {
    /// Scan and list installed plugins.
    Scan {
        #[arg(long, value_enum)]
        target: Option<CliAgentTarget>,
    },
    Preview {
        source: PathBuf,
        #[arg(long, value_enum, default_value_t = CliAgentTarget::Codex)]
        target: CliAgentTarget,
        #[arg(long, default_value = "local")]
        marketplace: String,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
        #[arg(long)]
        disable_after_install: bool,
    },
    Install {
        source: PathBuf,
        #[arg(long, value_enum, default_value_t = CliAgentTarget::Codex)]
        target: CliAgentTarget,
        #[arg(long, default_value = "local")]
        marketplace: String,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
        #[arg(long)]
        disable_after_install: bool,
    },
    Enable {
        plugin: String,
        #[arg(long, value_enum, default_value_t = CliAgentTarget::Codex)]
        target: CliAgentTarget,
    },
    Disable {
        plugin: String,
        #[arg(long, value_enum, default_value_t = CliAgentTarget::Codex)]
        target: CliAgentTarget,
    },
    Remove {
        plugin: String,
        #[arg(long, value_enum, default_value_t = CliAgentTarget::Codex)]
        target: CliAgentTarget,
    },
}

/// Marketplace management subcommands.
#[derive(Debug, Subcommand)]
enum MarketplaceCommand {
    /// Manage configured marketplace sources.
    Sources {
        #[command(subcommand)]
        command: MarketplaceSourceCommand,
    },
    Search {
        #[arg(long, value_enum, default_value_t = CliMarketplaceProvider::Skillsmp)]
        provider: CliMarketplaceProvider,
        #[arg(long)]
        query: String,
    },
    Inspect {
        source: String,
        #[arg(long, value_enum)]
        target: Option<CliAgentTarget>,
    },
}

/// Marketplace source subcommands.
#[derive(Debug, Subcommand)]
enum MarketplaceSourceCommand {
    /// List all configured marketplace sources.
    List,
    /// Add a new marketplace source.
    Add {
        label: String,
        source: String,
        #[arg(long, value_enum)]
        target: Option<CliAgentTarget>,
        #[arg(long)]
        provider: Option<String>,
    },
    Refresh {
        label: String,
    },
    Remove {
        label: String,
    },
}

/// Inventory subcommands.
#[derive(Debug, Subcommand)]
enum InventoryCommand {
    /// Scan and display the full workspace inventory.
    Scan,
    /// Validate installed skills and report diagnostics.
    Validate {
        #[arg(long, value_enum)]
        target: Option<CliTarget>,
    },
}

/// Install subcommands.
#[derive(Debug, Subcommand)]
enum InstallCommand {
    /// Preview an install without applying changes.
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

/// Download cache subcommands.
#[derive(Debug, Subcommand)]
enum DownloadCommand {
    /// Download and cache skills from a GitHub URL.
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

/// Legacy scope selector retained for backward compatibility.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliScope {
    #[value(alias = "user", alias = "gloab")]
    Global,
    Project,
}

/// CLI target scope selector mapping to [`InstallTarget`].
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliTarget {
    #[value(alias = "user", alias = "gloab")]
    Global,
    Project,
    #[value(name = "claude-code", alias = "claude", alias = "claudecode")]
    ClaudeCode,
    Droid,
    #[value(name = "opencode", alias = "open-code", alias = "pencode")]
    OpenCode,
    Codex,
    Zed,
}

/// CLI resource kind filter for resource scan commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliResourceKind {
    All,
    Skill,
    Plugin,
    Marketplace,
}

impl CliResourceKind {
    fn matches(self, kind: ResourceKind) -> bool {
        match self {
            Self::All => true,
            Self::Skill => kind == ResourceKind::Skill,
            Self::Plugin => kind == ResourceKind::Plugin,
            Self::Marketplace => kind == ResourceKind::Marketplace,
        }
    }
}

/// CLI agent target selector for plugin and marketplace commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliAgentTarget {
    Generic,
    Codex,
    #[value(name = "claude-code", alias = "claude", alias = "claudecode")]
    ClaudeCode,
}

impl From<CliAgentTarget> for AgentToolTarget {
    fn from(value: CliAgentTarget) -> Self {
        match value {
            CliAgentTarget::Generic => Self::Generic,
            CliAgentTarget::Codex => Self::Codex,
            CliAgentTarget::ClaudeCode => Self::ClaudeCode,
        }
    }
}

/// CLI marketplace search provider selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliMarketplaceProvider {
    #[value(name = "skillsmp", alias = "skills-mp")]
    Skillsmp,
}

impl From<CliMarketplaceProvider> for MarketplaceSearchProvider {
    fn from(value: CliMarketplaceProvider) -> Self {
        match value {
            CliMarketplaceProvider::Skillsmp => Self::SkillsMp,
        }
    }
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
            CliTarget::OpenCode => Self::OpenCode,
            CliTarget::Codex => Self::Codex,
            CliTarget::Zed => Self::Zed,
        }
    }
}

/// CLI conflict resolution policy selector.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliConflictPolicy {
    /// Abort installation when any collision is encountered.
    Block,
    /// Replace existing destination after moving it to a backup.
    Replace,
    /// Keep existing destination and resolve using a suffixed folder name.
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

/// CLI catalog export format selector.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliCatalogFormat {
    /// Pretty JSON output.
    Json,
    /// XML string output.
    Xml,
    /// Markdown table output.
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

/// Arguments for the preview-install command handler.
struct PreviewInstallArgs {
    source: String,
    local: bool,
    target: CliTarget,
    scope: Option<CliScope>,
    dest: Option<PathBuf>,
    download_dir: Option<PathBuf>,
    conflict: CliConflictPolicy,
}

/// Arguments for the install-url command handler.
struct InstallUrlArgs {
    url: String,
    target: CliTarget,
    scope: Option<CliScope>,
    dest: Option<PathBuf>,
    download_dir: Option<PathBuf>,
    disable_after_install: bool,
    conflict: CliConflictPolicy,
}

/// Arguments for the install-local command handler.
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
        Command::Validate { target } => validation_output(paths, target),
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
        Command::Resources { command } => run_resource_command(command, paths),
        Command::Plugins { command } => run_plugin_command(command, paths).await,
        Command::Marketplaces { command } => run_marketplace_command(command, paths).await,
    }
}

fn run_resource_command(command: ResourceCommand, paths: &ManagerPaths) -> Result<CommandOutput> {
    match command {
        ResourceCommand::Scan { kind, target } => {
            let mut resources = match kind {
                CliResourceKind::Plugin => scan_installed_plugins(paths)?,
                CliResourceKind::Marketplace => scan_marketplaces(paths)?,
                _ => scan_resources(paths)?,
            };
            resources.retain(|resource| kind.matches(resource.kind));
            if let Some(target) = target {
                let target = AgentToolTarget::from(target);
                resources.retain(|resource| resource.target == target);
            }
            Ok(CommandOutput::Resources { resources })
        }
    }
}

async fn run_plugin_command(command: PluginCommand, paths: &ManagerPaths) -> Result<CommandOutput> {
    let manager = ResourceManager::new(paths.clone());
    match command {
        PluginCommand::Scan { target } => {
            let mut plugins = manager.scan_plugins()?;
            if let Some(target) = target {
                let target = AgentToolTarget::from(target);
                plugins.retain(|plugin| plugin.target == target);
            }
            Ok(CommandOutput::Plugins { plugins })
        }
        PluginCommand::Preview {
            source,
            target,
            marketplace,
            conflict,
            disable_after_install,
        } => {
            let preview = manager.preview_plugin_install(PluginInstallRequest {
                source_root: source,
                source_url: None,
                target: target.into(),
                marketplace: Some(marketplace),
                conflict_policy: conflict.into(),
                enable_after_install: !disable_after_install,
            })?;
            Ok(CommandOutput::PluginPreview { preview })
        }
        PluginCommand::Install {
            source,
            target,
            marketplace,
            conflict,
            disable_after_install,
        } => {
            let result = manager.install_plugin(PluginInstallRequest {
                source_root: source,
                source_url: None,
                target: target.into(),
                marketplace: Some(marketplace),
                conflict_policy: conflict.into(),
                enable_after_install: !disable_after_install,
            })?;
            Ok(CommandOutput::PluginInstall { result })
        }
        PluginCommand::Enable { plugin, target } => {
            manager.set_plugin_enabled(target.into(), &plugin, true)?;
            Ok(CommandOutput::Status {
                message: "Enabled plugin.".to_string(),
            })
        }
        PluginCommand::Disable { plugin, target } => {
            manager.set_plugin_enabled(target.into(), &plugin, false)?;
            Ok(CommandOutput::Status {
                message: "Disabled plugin.".to_string(),
            })
        }
        PluginCommand::Remove { plugin, target } => {
            let backup = manager.remove_plugin(target.into(), &plugin)?;
            Ok(CommandOutput::RemovedPlugin { backup })
        }
    }
}

async fn run_marketplace_command(
    command: MarketplaceCommand,
    paths: &ManagerPaths,
) -> Result<CommandOutput> {
    match command {
        MarketplaceCommand::Sources { command } => match command {
            MarketplaceSourceCommand::List => {
                let sources = list_marketplace_sources(paths)?;
                Ok(CommandOutput::MarketplaceSources { sources })
            }
            MarketplaceSourceCommand::Add {
                label,
                source,
                target,
                provider,
            } => {
                let source = add_marketplace_source(
                    paths,
                    &label,
                    &source,
                    target.map(AgentToolTarget::from),
                    provider,
                )?;
                Ok(CommandOutput::MarketplaceSource { source })
            }
            MarketplaceSourceCommand::Refresh { label } => {
                let marketplace = refresh_marketplace_source(paths, &label).await?;
                Ok(CommandOutput::MarketplaceInspect { marketplace })
            }
            MarketplaceSourceCommand::Remove { label } => {
                let label = remove_marketplace_source(paths, &label)?;
                Ok(CommandOutput::RemovedMarketplaceSource { label })
            }
        },
        MarketplaceCommand::Search { provider, query } => {
            let result = search_marketplace(provider.into(), &query).await?;
            Ok(CommandOutput::MarketplaceSearch { result })
        }
        MarketplaceCommand::Inspect { source, target } => {
            let marketplace =
                inspect_marketplace_source(&source, target.map(AgentToolTarget::from)).await?;
            Ok(CommandOutput::MarketplaceInspect { marketplace })
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
        InventoryCommand::Validate { target } => validation_output(paths, target),
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
    installer: &Installer,
) -> Result<CommandOutput> {
    let install_target = install_target(args.target, args.scope, args.dest);
    let _download_dir = args.download_dir;
    let preview = if args.local {
        installer.preview(
            Path::new(&args.source),
            install_target,
            args.conflict.into(),
        )?
    } else {
        let downloaded = download_github_skills(&args.source)
            .await
            .with_context(|| format!("failed to download skills from {}", args.source))?;
        let plan = installer.plan(InstallRequest {
            source_root: downloaded.search_root,
            source_url: Some(args.source),
            target: install_target,
            conflict_policy: args.conflict.into(),
            enable_after_install: true,
        })?;
        let mut preview = plan.preview();
        preview.operation_plan = None;
        preview
    };
    Ok(CommandOutput::Preview { preview })
}

fn validation_output(paths: &ManagerPaths, target: Option<CliTarget>) -> Result<CommandOutput> {
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
