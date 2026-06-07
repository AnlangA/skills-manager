//! Plugin/resource indexing, marketplace management, and plugin operations.
//!
//! Provides [`ResourceManager`] as the high-level facade for scanning,
//! installing, enabling, and removing plugins and marketplace documents
//! across Codex, Claude Code, and generic targets. Also contains the
//! marketplace source CRUD helpers and remote search integration.

use std::{
    collections::{BTreeMap, HashMap},
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::{
    AgentToolTarget, ConflictPolicy, DiagnosticSeverity, GitHubTreeSource, InstalledSkill,
    ManagedResource, ManagerConfig, ManagerPaths, ResourceHealth, ResourceKind, Result,
    SkillDiagnostic, SkillEnablement, SkillsManagerError,
    codex_config::CodexConfig,
    download::{download_first_archive, extract_zip_safe, filtered_search_root},
    installed_skill_identity,
    skill::{format_bytes, path_key, sanitize_folder_name, unique_folder_name},
};

const CODEX_PLUGIN_MANIFEST: &str = ".codex-plugin/plugin.json";
const CLAUDE_PLUGIN_MANIFEST: &str = ".claude-plugin/plugin.json";
const CODEX_MARKETPLACE_FILE: &str = ".agents/plugins/marketplace.json";
const CLAUDE_MARKETPLACE_FILE: &str = ".claude-plugin/marketplace.json";

/// Parsed plugin manifest metadata discovered from a plugin tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Target runtime for this plugin.
    pub target: AgentToolTarget,
    /// Canonical plugin root.
    pub root_dir: PathBuf,
    /// Path to manifest file.
    pub manifest_file: PathBuf,
    /// Declared plugin name.
    pub name: String,
    /// Optional plugin version.
    pub version: Option<String>,
    /// Optional plugin description.
    pub description: Option<String>,
    /// Human-friendly display name.
    pub display_name: Option<String>,
    /// Optional category.
    pub category: Option<String>,
    /// Relative path from plugin root to skills directory.
    pub skills_path: Option<String>,
    /// Relative path from plugin root to MCP server directory.
    pub mcp_servers_path: Option<String>,
    /// Relative path from plugin root to app directory.
    pub apps_path: Option<String>,
    /// Relative path from plugin root to hooks directory.
    pub hooks_path: Option<String>,
    /// Count of discovered component types.
    pub component_counts: PluginComponentCounts,
    /// Validation and parse diagnostics for the plugin manifest.
    pub diagnostics: Vec<SkillDiagnostic>,
}

/// Component count breakdown for plugin diagnostics and inventory metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginComponentCounts {
    /// Number of `skills/` entries.
    pub skills: usize,
    /// Number of `mcp_servers/` entries.
    pub mcp_servers: usize,
    /// Number of app declarations.
    pub apps: usize,
    /// Number of hook declarations.
    pub hooks: usize,
    /// Number of non-code assets counted.
    pub assets: usize,
}

/// Plugin marketplace document persisted in local catalog roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceDocument {
    /// Document name/slug.
    pub name: String,
    /// Optional display name.
    pub display_name: Option<String>,
    /// Target runtime associated with the document.
    pub target: AgentToolTarget,
    /// Optional local source root.
    pub root_dir: Option<PathBuf>,
    /// Optional source origin label.
    pub source_label: Option<String>,
    /// Entries advertised by this document.
    pub entries: Vec<MarketplaceEntry>,
    /// Document-level diagnostics.
    pub diagnostics: Vec<SkillDiagnostic>,
}

/// A plugin entry in a marketplace document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    /// Plugin name.
    pub name: String,
    /// Optional human description.
    pub description: Option<String>,
    /// Optional version.
    pub version: Option<String>,
    /// Optional category.
    pub category: Option<String>,
    /// Target runtime for the plugin.
    pub target: AgentToolTarget,
    /// Optional marketplace display label.
    pub marketplace: Option<String>,
    /// Source declaration for installation.
    pub source: MarketplaceSource,
    /// Installation policy hints.
    pub policy_installation: Option<String>,
    /// Authentication policy hints.
    pub policy_authentication: Option<String>,
}

/// Source location encoding in a marketplace entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
pub enum MarketplaceSource {
    /// Filesystem path source.
    Local { path: String },
    /// Git repo root source.
    Git {
        url: String,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        reference: Option<String>,
    },
    /// Git repository with explicit subdirectory.
    GitSubdir {
        url: String,
        path: String,
        #[serde(default)]
        reference: Option<String>,
    },
    /// Direct URL source.
    Url { url: String },
    /// Remote skills marketplace search source.
    SkillsMp { query: String },
    /// Forward-compatible catch-all.
    Unknown { raw: String },
}

/// One configured marketplace source entry persisted in configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceSourceRecord {
    /// Human label for the source.
    pub label: String,
    /// Raw source URI or path.
    pub source: String,
    /// Target override, if configured.
    pub target: Option<String>,
    /// Provider hint.
    pub provider: Option<String>,
    /// Timestamp when source was first added.
    pub added_at: Option<DateTime<Utc>>,
    /// Timestamp of last successful refresh.
    pub last_refreshed_at: Option<DateTime<Utc>>,
}

/// Search provider implementations for marketplace lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceSearchProvider {
    /// Skills Marketplace (`skillsmp`).
    SkillsMp,
}

impl MarketplaceSearchProvider {
    /// Canonical provider label used for display and storage.
    pub fn label(self) -> &'static str {
        match self {
            Self::SkillsMp => "skillsmp",
        }
    }
}

impl fmt::Display for MarketplaceSearchProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Search result container for a marketplace query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceSearchResult {
    /// Provider used for the search.
    pub provider: MarketplaceSearchProvider,
    /// Search query string.
    pub query: String,
    /// Normalized entries returned by provider.
    pub entries: Vec<MarketplaceSearchEntry>,
}

/// Single search hit result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceSearchEntry {
    /// Plugin name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Optional source URL.
    pub source_url: Option<String>,
    /// Optional path within source.
    pub source_path: Option<String>,
    /// Optional author label.
    pub author: Option<String>,
    /// Optional star count for ranking.
    pub stars: Option<u64>,
    /// Resource kind for UI and install dispatch.
    pub kind: ResourceKind,
}

/// Transactional plan for resource operations.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceOperationPlan {
    /// Stable plan identifier.
    pub id: String,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Target runtime.
    pub target: AgentToolTarget,
    /// Source root.
    pub source_root: PathBuf,
    /// Optional destination root.
    pub destination_root: Option<PathBuf>,
    /// Optional marketplace id.
    pub marketplace: Option<String>,
    /// Resource name.
    pub resource_name: String,
    /// Optional version string.
    pub version: Option<String>,
    /// Whether to enable resource after install.
    pub enable_after_install: bool,
    /// Conflict behavior.
    pub conflict_policy: ConflictPolicy,
    /// Commands to apply for this operation when external tooling is needed.
    pub commands: Vec<String>,
    /// Validation and runtime diagnostics.
    pub diagnostics: Vec<SkillDiagnostic>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Preview result for a plugin install.
#[derive(Debug, Clone, Serialize)]
pub struct PluginInstallPreview {
    /// Parsed manifest for the plugin.
    pub manifest: PluginManifest,
    /// Planned operation for execution.
    pub operation_plan: ResourceOperationPlan,
    /// Destination conflict precheck.
    pub conflict: bool,
}

/// Execution result for plugin install.
#[derive(Debug, Clone, Serialize)]
pub struct PluginInstallResult {
    /// Installed root if installation occurred locally.
    pub installed: Option<PathBuf>,
    /// Backup path when replaced.
    pub backup: Option<PathBuf>,
    /// Optional command output from delegated installers.
    pub command_output: Option<String>,
}

/// Input model for plugin preview/install requests.
#[derive(Debug, Clone)]
pub struct PluginInstallRequest {
    /// Source root prepared for plugin manifest lookup.
    pub source_root: PathBuf,
    /// Optional source URL used for metadata.
    pub source_url: Option<String>,
    /// Target runtime.
    pub target: AgentToolTarget,
    /// Optional marketplace label.
    pub marketplace: Option<String>,
    /// Conflict strategy.
    pub conflict_policy: ConflictPolicy,
    /// Whether plugin should be enabled after install.
    pub enable_after_install: bool,
}

/// High-level façade for plugin/resource operations.
#[derive(Debug, Clone)]
pub struct ResourceManager {
    /// Working manager paths.
    paths: ManagerPaths,
}

#[derive(Debug, Deserialize)]
struct RawPluginManifest {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    skills: Option<Value>,
    #[serde(default, rename = "mcpServers", alias = "mcp_servers")]
    mcp_servers: Option<Value>,
    #[serde(default)]
    apps: Option<Value>,
    #[serde(default)]
    hooks: Option<Value>,
    #[serde(default)]
    interface: Option<RawPluginInterface>,
}

#[derive(Debug, Deserialize)]
struct RawPluginInterface {
    #[serde(default, rename = "displayName", alias = "display_name")]
    display_name: Option<String>,
    #[serde(default, rename = "shortDescription", alias = "short_description")]
    short_description: Option<String>,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMarketplaceDocument {
    name: Option<String>,
    #[serde(default)]
    owner: Option<Value>,
    #[serde(default)]
    interface: Option<RawMarketplaceInterface>,
    #[serde(default)]
    plugins: Vec<RawMarketplaceEntry>,
}

#[derive(Debug, Deserialize)]
struct RawMarketplaceInterface {
    #[serde(default, rename = "displayName", alias = "display_name")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMarketplaceEntry {
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    source: Option<Value>,
    #[serde(default)]
    policy: Option<Value>,
}

impl ResourceManager {
    /// Construct a manager with resolved paths.
    pub fn new(paths: ManagerPaths) -> Self {
        Self { paths }
    }

    /// Scan all resources (skills, plugins, and marketplaces) in one result list.
    pub fn scan_resources(&self) -> Result<Vec<ManagedResource>> {
        let mut resources = scan_installed_skills_as_resources(&self.paths)?;
        resources.extend(self.scan_plugins()?);
        resources.extend(self.scan_marketplaces()?);
        resources.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.target.cmp(&right.target))
                .then_with(|| left.display_name.cmp(&right.display_name))
        });
        Ok(resources)
    }

    /// Discover and normalize installed plugins across configured plugin roots.
    pub fn scan_plugins(&self) -> Result<Vec<ManagedResource>> {
        let manifests = discover_plugin_manifests(&self.paths)?;
        let config = ManagerConfig::load(&self.paths)?;
        let codex_config = CodexConfig::load(&self.paths)?;
        let mut resources = Vec::new();

        for manifest in manifests {
            let marketplace = infer_plugin_marketplace(&self.paths, &manifest);
            let display_id = plugin_display_id(&manifest.name, marketplace.as_deref());
            let plugin_id =
                plugin_resource_id(manifest.target, &manifest.name, marketplace.as_deref());
            let metadata = config.resource_installs.get(&plugin_id);
            let enablement = match manifest.target {
                AgentToolTarget::Codex if codex_config.is_plugin_disabled(&display_id) => {
                    SkillEnablement::Disabled
                }
                _ => SkillEnablement::Enabled,
            };
            let mut diagnostics = manifest.diagnostics.clone();
            if manifest.target == AgentToolTarget::Generic {
                diagnostics.push(SkillDiagnostic::warning(
                    "Plugin format is unknown; this manager can inspect it but cannot install or toggle it",
                ));
            }
            let mut metadata_map = BTreeMap::new();
            metadata_map.insert(
                "components".to_string(),
                format!(
                    "{} skill(s), {} MCP server file(s), {} app file(s), {} hook file(s), {} asset(s)",
                    manifest.component_counts.skills,
                    manifest.component_counts.mcp_servers,
                    manifest.component_counts.apps,
                    manifest.component_counts.hooks,
                    manifest.component_counts.assets
                ),
            );
            if let Some(version) = &manifest.version {
                metadata_map.insert("version".to_string(), version.clone());
            }
            metadata_map.insert("plugin_id".to_string(), display_id);
            if let Some(marketplace) = marketplace {
                metadata_map.insert("marketplace".to_string(), marketplace);
            }

            resources.push(ManagedResource {
                id: plugin_id,
                kind: ResourceKind::Plugin,
                target: manifest.target,
                display_name: manifest
                    .display_name
                    .clone()
                    .unwrap_or(manifest.name.clone()),
                description: manifest.description.clone(),
                root_dir: manifest.root_dir.clone(),
                manifest_file: Some(manifest.manifest_file.clone()),
                enablement,
                health: ResourceHealth::from_diagnostics(&diagnostics),
                diagnostics,
                source_url: metadata.and_then(|item| item.source_url.clone()),
                installed_at: metadata.and_then(|item| item.installed_at),
                metadata: metadata_map,
            });
        }

        Ok(resources)
    }

    /// Scan local marketplace documents and configured marketplace source entries.
    pub fn scan_marketplaces(&self) -> Result<Vec<ManagedResource>> {
        let mut resources = Vec::new();
        for document in discover_local_marketplaces(&self.paths)? {
            let file = document
                .root_dir
                .as_ref()
                .map(|root| match document.target {
                    AgentToolTarget::Codex => root.join(CODEX_MARKETPLACE_FILE),
                    AgentToolTarget::ClaudeCode => root.join(CLAUDE_MARKETPLACE_FILE),
                    AgentToolTarget::Generic => root.join("marketplace.json"),
                });
            resources.push(marketplace_resource(document, file));
        }

        for source in list_marketplace_sources(&self.paths)? {
            resources.push(ManagedResource {
                id: format!("marketplace:configured:{}", source.label),
                kind: ResourceKind::Marketplace,
                target: target_from_config(source.target.as_deref()),
                display_name: source.label.clone(),
                description: Some(source.source.clone()),
                root_dir: PathBuf::from(&source.source),
                manifest_file: None,
                enablement: SkillEnablement::Enabled,
                health: ResourceHealth::Valid,
                diagnostics: Vec::new(),
                source_url: Some(source.source),
                installed_at: source.added_at,
                metadata: BTreeMap::new(),
            });
        }

        Ok(resources)
    }

    /// Build a preview for installing a plugin from local source.
    ///
    /// For Claude Code, this returns CLI command metadata and warnings since install
    /// is delegated to the `claude` binary.
    pub fn preview_plugin_install(
        &self,
        request: PluginInstallRequest,
    ) -> Result<PluginInstallPreview> {
        let manifest = read_plugin_manifest_for_target(&request.source_root, request.target)?;
        let marketplace = request
            .marketplace
            .clone()
            .unwrap_or_else(|| "local".to_string());
        let destination = match request.target {
            AgentToolTarget::Codex => Some(codex_plugin_destination(
                &self.paths,
                &marketplace,
                &manifest.name,
                manifest.version.as_deref(),
                request.conflict_policy,
            )),
            AgentToolTarget::ClaudeCode => None,
            AgentToolTarget::Generic => None,
        };
        let conflict = destination.as_ref().is_some_and(|path| path.exists());
        let mut diagnostics = manifest.diagnostics.clone();
        let mut commands = Vec::new();

        match request.target {
            AgentToolTarget::Codex => {
                if manifest.target != AgentToolTarget::Codex {
                    diagnostics.push(SkillDiagnostic::invalid(
                        "Codex install requires a .codex-plugin/plugin.json manifest",
                    ));
                }
            }
            AgentToolTarget::ClaudeCode => {
                commands.push(format!(
                    "claude plugin install {}@{}",
                    manifest.name, marketplace
                ));
                diagnostics.push(SkillDiagnostic::warning(
                    "Claude Code plugin mutations are delegated to the official `claude plugin` command",
                ));
            }
            AgentToolTarget::Generic => diagnostics.push(SkillDiagnostic::invalid(
                "Generic plugins are read-only until a target adapter is selected",
            )),
        }

        let plan = ResourceOperationPlan {
            id: format!(
                "{}:{}:{}",
                request.target.id_prefix(),
                manifest.name,
                Utc::now().timestamp_millis()
            ),
            kind: ResourceKind::Plugin,
            target: request.target,
            source_root: request.source_root,
            destination_root: destination,
            marketplace: Some(marketplace),
            resource_name: manifest.name.clone(),
            version: manifest.version.clone(),
            enable_after_install: request.enable_after_install,
            conflict_policy: request.conflict_policy,
            commands,
            diagnostics,
            created_at: Utc::now(),
        };

        Ok(PluginInstallPreview {
            manifest,
            conflict,
            operation_plan: plan,
        })
    }

    /// Install a plugin according to target adapter policy.
    ///
    /// Codex installs are local copies; Claude Code installs call `claude plugin ...`.
    pub fn install_plugin(&self, request: PluginInstallRequest) -> Result<PluginInstallResult> {
        let preview = self.preview_plugin_install(request.clone())?;
        if preview
            .operation_plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Invalid)
        {
            return Err(SkillsManagerError::InvalidResource(
                "plugin install preview has invalid diagnostics".to_string(),
            ));
        }

        match request.target {
            AgentToolTarget::Codex => self.install_codex_plugin(request, preview),
            AgentToolTarget::ClaudeCode => self.run_claude_plugin_command(
                "install",
                &plugin_display_id(
                    &preview.manifest.name,
                    preview.operation_plan.marketplace.as_deref(),
                ),
            ),
            AgentToolTarget::Generic => Err(SkillsManagerError::InvalidResource(
                "generic plugin install is read-only".to_string(),
            )),
        }
    }

    /// Enable or disable a plugin by id/path, depending on target runtime.
    pub fn set_plugin_enabled(
        &self,
        target: AgentToolTarget,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<()> {
        match target {
            AgentToolTarget::Codex => {
                let _lock = ManagerConfig::acquire_update_lock(&self.paths)?;
                let mut config = CodexConfig::load(&self.paths)?;
                config.set_plugin_enabled(plugin_id, enabled);
                config.save()
            }
            AgentToolTarget::ClaudeCode => {
                let action = if enabled { "enable" } else { "disable" };
                self.run_claude_plugin_command(action, plugin_id)
                    .map(|_| ())
            }
            AgentToolTarget::Generic => Err(SkillsManagerError::InvalidResource(
                "generic plugin enablement is read-only".to_string(),
            )),
        }
    }

    /// Remove an installed plugin and return the backup directory.
    pub fn remove_plugin(
        &self,
        target: AgentToolTarget,
        plugin_id_or_path: &str,
    ) -> Result<PathBuf> {
        match target {
            AgentToolTarget::Codex => self.remove_codex_plugin(plugin_id_or_path),
            AgentToolTarget::ClaudeCode => self
                .run_claude_plugin_command("uninstall", plugin_id_or_path)
                .map(|_| PathBuf::from(plugin_id_or_path)),
            AgentToolTarget::Generic => Err(SkillsManagerError::InvalidResource(
                "generic plugin removal is read-only".to_string(),
            )),
        }
    }

    fn install_codex_plugin(
        &self,
        request: PluginInstallRequest,
        preview: PluginInstallPreview,
    ) -> Result<PluginInstallResult> {
        let destination = preview
            .operation_plan
            .destination_root
            .clone()
            .ok_or_else(|| {
                SkillsManagerError::InvalidResource("missing destination".to_string())
            })?;
        let conflict = destination.exists();
        let mut backup = None;

        if conflict {
            match request.conflict_policy {
                ConflictPolicy::Block => {
                    return Err(SkillsManagerError::DestinationExists(destination));
                }
                ConflictPolicy::Rename => {}
                ConflictPolicy::Replace => {
                    let backup_path = backup_path(&destination);
                    fs::rename(&destination, &backup_path)?;
                    backup = Some(backup_path);
                }
            }
        }

        copy_dir(&preview.manifest.root_dir, &destination)?;
        let plugin_id = plugin_display_id(
            &preview.manifest.name,
            preview.operation_plan.marketplace.as_deref(),
        );
        let resource_id = plugin_resource_id(
            AgentToolTarget::Codex,
            &preview.manifest.name,
            preview.operation_plan.marketplace.as_deref(),
        );
        let _lock = ManagerConfig::acquire_update_lock(&self.paths)?;
        let mut manager_config = ManagerConfig::load(&self.paths)?;
        let mut codex_config = CodexConfig::load(&self.paths)?;
        manager_config.record_resource_install(resource_id, request.source_url);
        codex_config.set_plugin_enabled(&plugin_id, request.enable_after_install);
        manager_config.save(&self.paths)?;
        codex_config.save()?;

        Ok(PluginInstallResult {
            installed: Some(destination),
            backup,
            command_output: None,
        })
    }

    fn remove_codex_plugin(&self, plugin_id_or_path: &str) -> Result<PathBuf> {
        let requested = PathBuf::from(plugin_id_or_path);
        let plugin = if requested.exists() {
            requested
        } else {
            self.scan_plugins()?
                .into_iter()
                .find(|plugin| {
                    plugin.id == plugin_id_or_path
                        || plugin.display_name == plugin_id_or_path
                        || plugin
                            .metadata
                            .get("plugin_id")
                            .is_some_and(|id| id == plugin_id_or_path)
                })
                .map(|plugin| plugin.root_dir)
                .ok_or_else(|| {
                    SkillsManagerError::InvalidResource(format!(
                        "unknown Codex plugin: {plugin_id_or_path}"
                    ))
                })?
        };
        let backup = backup_path(&plugin);
        fs::rename(&plugin, &backup)?;
        let _lock = ManagerConfig::acquire_update_lock(&self.paths)?;
        let mut manager_config = ManagerConfig::load(&self.paths)?;
        let mut codex_config = CodexConfig::load(&self.paths)?;
        if let Ok(manifest) = read_plugin_manifest_for_target(&backup, AgentToolTarget::Codex) {
            let id = plugin_display_id(&manifest.name, None);
            codex_config.forget_plugin(&id);
            manager_config.forget_resource_install(&plugin_resource_id(
                AgentToolTarget::Codex,
                &manifest.name,
                None,
            ));
        }
        manager_config.save(&self.paths)?;
        codex_config.save()?;
        Ok(backup)
    }

    fn run_claude_plugin_command(
        &self,
        action: &str,
        plugin_id: &str,
    ) -> Result<PluginInstallResult> {
        let output = Command::new("claude")
            .arg("plugin")
            .arg(action)
            .arg(plugin_id)
            .output()
            .map_err(|error| {
                SkillsManagerError::InvalidResource(format!(
                    "failed to run `claude plugin {action} {plugin_id}`: {error}"
                ))
            })?;

        if !output.status.success() {
            return Err(SkillsManagerError::InvalidResource(format!(
                "`claude plugin {action} {plugin_id}` failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(PluginInstallResult {
            installed: None,
            backup: None,
            command_output: Some(String::from_utf8_lossy(&output.stdout).to_string()),
        })
    }
}

/// Convenience wrapper around [`ResourceManager::scan_resources`].
pub fn scan_resources(paths: &ManagerPaths) -> Result<Vec<ManagedResource>> {
    ResourceManager::new(paths.clone()).scan_resources()
}

/// Convenience wrapper around [`ResourceManager::scan_plugins`].
pub fn scan_installed_plugins(paths: &ManagerPaths) -> Result<Vec<ManagedResource>> {
    ResourceManager::new(paths.clone()).scan_plugins()
}

/// Convenience wrapper around [`ResourceManager::scan_marketplaces`].
pub fn scan_marketplaces(paths: &ManagerPaths) -> Result<Vec<ManagedResource>> {
    ResourceManager::new(paths.clone()).scan_marketplaces()
}

/// Return all configured marketplace sources sorted by label.
pub fn list_marketplace_sources(paths: &ManagerPaths) -> Result<Vec<MarketplaceSourceRecord>> {
    let config = ManagerConfig::load(paths)?;
    let mut sources = config
        .marketplace_sources
        .values()
        .map(|source| MarketplaceSourceRecord {
            label: source.label.clone(),
            source: source.source.clone(),
            target: source.target.clone(),
            provider: source.provider.clone(),
            added_at: source.added_at,
            last_refreshed_at: source.last_refreshed_at,
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(sources)
}

/// Add a marketplace source to config and return the recorded source record.
pub fn add_marketplace_source(
    paths: &ManagerPaths,
    label: &str,
    source: &str,
    target: Option<AgentToolTarget>,
    provider: Option<String>,
) -> Result<MarketplaceSourceRecord> {
    ManagerConfig::update(paths, |config| {
        config.record_marketplace_source(
            label,
            source,
            target.map(|target| target.id_prefix().to_string()),
            provider,
        );
        Ok(())
    })?;
    list_marketplace_sources(paths)?
        .into_iter()
        .find(|record| record.label == label)
        .ok_or_else(|| SkillsManagerError::InvalidResource(format!("source not recorded: {label}")))
}

/// Remove a marketplace source from config by label.
pub fn remove_marketplace_source(paths: &ManagerPaths, label: &str) -> Result<String> {
    ManagerConfig::update(paths, |config| {
        config.forget_marketplace_source(label);
        Ok(label.to_string())
    })
}

/// Fetch and refresh a configured marketplace source now.
pub async fn refresh_marketplace_source(
    paths: &ManagerPaths,
    label: &str,
) -> Result<MarketplaceDocument> {
    let source = list_marketplace_sources(paths)?
        .into_iter()
        .find(|source| source.label == label)
        .ok_or_else(|| {
            SkillsManagerError::InvalidResource(format!("unknown marketplace source: {label}"))
        })?;
    let document = load_marketplace(
        &source.source,
        source
            .target
            .as_deref()
            .map(|target| target_from_config(Some(target))),
    )
    .await?;
    ManagerConfig::update(paths, |config| {
        config.mark_marketplace_refreshed(label);
        Ok(())
    })?;
    Ok(document)
}

/// Load and parse a marketplace document by source URL/path.
pub async fn inspect_marketplace_source(
    source: &str,
    target: Option<AgentToolTarget>,
) -> Result<MarketplaceDocument> {
    load_marketplace(source, target).await
}

/// Search a marketplace provider for plugin results.
pub async fn search_marketplace(
    provider: MarketplaceSearchProvider,
    query: &str,
) -> Result<MarketplaceSearchResult> {
    match provider {
        MarketplaceSearchProvider::SkillsMp => search_skillsmp(query).await,
    }
}

/// Read plugin manifest metadata from a path that may be a root folder or manifest file.
pub fn read_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let root = if path.is_file() {
        path.parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    if root.join(CODEX_PLUGIN_MANIFEST).exists() {
        read_plugin_manifest_at(&root, AgentToolTarget::Codex)
    } else if root.join(CLAUDE_PLUGIN_MANIFEST).exists() {
        read_plugin_manifest_at(&root, AgentToolTarget::ClaudeCode)
    } else {
        read_generic_plugin_manifest(&root)
    }
}

fn scan_installed_skills_as_resources(paths: &ManagerPaths) -> Result<Vec<ManagedResource>> {
    Ok(crate::scan_installed_skills(paths)?
        .into_iter()
        .map(skill_resource)
        .collect())
}

fn skill_resource(skill: InstalledSkill) -> ManagedResource {
    let mut metadata = BTreeMap::new();
    metadata.insert("identity".to_string(), installed_skill_identity(&skill));
    metadata.insert(
        "resources".to_string(),
        format!(
            "{} file(s), {}",
            skill.resource_count,
            format_bytes(skill.resource_bytes)
        ),
    );
    ManagedResource {
        id: skill.id.clone(),
        kind: ResourceKind::Skill,
        target: match skill.scope {
            crate::SkillScope::Codex => AgentToolTarget::Codex,
            crate::SkillScope::ClaudeCode => AgentToolTarget::ClaudeCode,
            _ => AgentToolTarget::Generic,
        },
        display_name: skill.display_name.clone(),
        description: skill.description.clone(),
        root_dir: skill.root_dir.clone(),
        manifest_file: Some(skill.skill_file.clone()),
        enablement: skill.enablement,
        health: match skill.health {
            crate::SkillHealth::Invalid => ResourceHealth::Invalid,
            crate::SkillHealth::Warning | crate::SkillHealth::Shadowed => ResourceHealth::Warning,
            crate::SkillHealth::Valid => ResourceHealth::Valid,
        },
        diagnostics: skill.diagnostics,
        source_url: skill.source_url,
        installed_at: skill.installed_at,
        metadata,
    }
}

fn marketplace_resource(document: MarketplaceDocument, file: Option<PathBuf>) -> ManagedResource {
    let mut metadata = BTreeMap::new();
    metadata.insert("entries".to_string(), document.entries.len().to_string());
    ManagedResource {
        id: format!(
            "marketplace:{}:{}",
            document.target.id_prefix(),
            document.name
        ),
        kind: ResourceKind::Marketplace,
        target: document.target,
        display_name: document
            .display_name
            .clone()
            .unwrap_or(document.name.clone()),
        description: Some(format!("{} plugin entrie(s)", document.entries.len())),
        root_dir: document.root_dir.clone().unwrap_or_default(),
        manifest_file: file,
        enablement: SkillEnablement::Enabled,
        health: ResourceHealth::from_diagnostics(&document.diagnostics),
        diagnostics: document.diagnostics,
        source_url: document.source_label,
        installed_at: None,
        metadata,
    }
}

fn discover_plugin_manifests(paths: &ManagerPaths) -> Result<Vec<PluginManifest>> {
    let roots = plugin_discovery_roots(paths);
    let mut manifests = Vec::new();
    let mut seen = BTreeMap::new();

    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .min_depth(1)
            .max_depth(7)
            .follow_links(false)
            .into_iter()
        {
            let entry = entry?;
            if !entry.file_type().is_file() || entry.file_name() != "plugin.json" {
                continue;
            }
            let path = entry.path();
            let target = if path.ends_with(CODEX_PLUGIN_MANIFEST) {
                AgentToolTarget::Codex
            } else if path.ends_with(CLAUDE_PLUGIN_MANIFEST) {
                AgentToolTarget::ClaudeCode
            } else {
                continue;
            };
            let Some(root_dir) = path.parent().and_then(Path::parent) else {
                continue;
            };
            let key = path_key(root_dir);
            if seen.insert(key, ()).is_none() {
                manifests.push(read_plugin_manifest_at(root_dir, target)?);
            }
        }
    }

    manifests.sort_by(|left, right| {
        left.target
            .cmp(&right.target)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.root_dir.cmp(&right.root_dir))
    });
    Ok(manifests)
}

fn plugin_discovery_roots(paths: &ManagerPaths) -> Vec<PathBuf> {
    let mut roots = vec![
        paths.codex_plugins_dir(),
        paths.codex_plugin_cache_dir(),
        paths.claude_plugins_dir(),
        paths.claude_plugin_cache_dir(),
    ];
    if let Some(project) = paths.project() {
        roots.push(project.path().join("plugins"));
        roots.push(project.path().join(".agents").join("plugins"));
        roots.push(project.path().join(".claude-plugin"));
    }
    roots
}

fn read_plugin_manifest_for_target(root: &Path, target: AgentToolTarget) -> Result<PluginManifest> {
    match target {
        AgentToolTarget::Codex => read_plugin_manifest_at(root, AgentToolTarget::Codex),
        AgentToolTarget::ClaudeCode => read_plugin_manifest_at(root, AgentToolTarget::ClaudeCode),
        AgentToolTarget::Generic => read_plugin_manifest(root),
    }
}

fn read_plugin_manifest_at(root: &Path, target: AgentToolTarget) -> Result<PluginManifest> {
    let manifest_file = root.join(match target {
        AgentToolTarget::Codex => CODEX_PLUGIN_MANIFEST,
        AgentToolTarget::ClaudeCode => CLAUDE_PLUGIN_MANIFEST,
        AgentToolTarget::Generic => "plugin.json",
    });
    if !manifest_file.exists() {
        return Err(SkillsManagerError::MissingPluginManifest(
            root.to_path_buf(),
        ));
    }
    let raw = fs::read_to_string(&manifest_file)?;
    let parsed = serde_json::from_str::<RawPluginManifest>(&raw).map_err(|source| {
        SkillsManagerError::ParseMarketplace {
            path: manifest_file.clone(),
            source,
        }
    })?;
    let mut diagnostics = Vec::new();
    let name = parsed.name.unwrap_or_else(|| {
        diagnostics.push(SkillDiagnostic::invalid(
            "plugin manifest is missing `name`",
        ));
        root.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unnamed-plugin")
            .to_string()
    });
    if name.trim().is_empty() {
        diagnostics.push(SkillDiagnostic::invalid("plugin `name` is empty"));
    }
    if parsed
        .description
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        diagnostics.push(SkillDiagnostic::warning(
            "plugin manifest should declare a description",
        ));
    }
    let skills_path = string_path(parsed.skills.as_ref()).or_else(|| {
        root.join("skills")
            .exists()
            .then(|| "./skills/".to_string())
    });
    let mcp_servers_path = string_path(parsed.mcp_servers.as_ref());
    let apps_path = string_path(parsed.apps.as_ref());
    let hooks_path = string_path(parsed.hooks.as_ref()).or_else(|| {
        root.join("hooks/hooks.json")
            .exists()
            .then(|| "./hooks/hooks.json".to_string())
    });
    let component_counts = component_counts(
        root,
        skills_path.as_deref(),
        mcp_servers_path.as_deref(),
        apps_path.as_deref(),
        hooks_path.as_deref(),
    );

    Ok(PluginManifest {
        target,
        root_dir: root.to_path_buf(),
        manifest_file,
        name,
        version: parsed.version,
        description: parsed
            .interface
            .as_ref()
            .and_then(|interface| interface.short_description.clone())
            .or(parsed.description),
        display_name: parsed
            .interface
            .as_ref()
            .and_then(|interface| interface.display_name.clone()),
        category: parsed.interface.and_then(|interface| interface.category),
        skills_path,
        mcp_servers_path,
        apps_path,
        hooks_path,
        component_counts,
        diagnostics,
    })
}

fn read_generic_plugin_manifest(root: &Path) -> Result<PluginManifest> {
    let manifest_file = root.join("plugin.json");
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-plugin")
        .to_string();
    Ok(PluginManifest {
        target: AgentToolTarget::Generic,
        root_dir: root.to_path_buf(),
        manifest_file,
        name,
        version: None,
        description: None,
        display_name: None,
        category: None,
        skills_path: None,
        mcp_servers_path: None,
        apps_path: None,
        hooks_path: None,
        component_counts: component_counts(root, None, None, None, None),
        diagnostics: vec![SkillDiagnostic::warning(
            "No Codex or Claude Code plugin manifest was found",
        )],
    })
}

fn string_path(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(path) if path.trim().starts_with("./") => Some(path.clone()),
        Value::String(path) if !path.trim().is_empty() => Some(path.clone()),
        _ => None,
    }
}

fn component_counts(
    root: &Path,
    skills_path: Option<&str>,
    mcp_servers_path: Option<&str>,
    apps_path: Option<&str>,
    hooks_path: Option<&str>,
) -> PluginComponentCounts {
    let skills = skills_path
        .and_then(|path| safe_relative_path(root, path))
        .map(|path| count_files_named(&path, "SKILL.md"))
        .unwrap_or_else(|| count_files_named(&root.join("skills"), "SKILL.md"));
    let mcp_servers = mcp_servers_path
        .and_then(|path| safe_relative_path(root, path))
        .filter(|path| path.exists())
        .map(|_| 1)
        .unwrap_or_else(|| root.join(".mcp.json").exists() as usize);
    let apps = apps_path
        .and_then(|path| safe_relative_path(root, path))
        .filter(|path| path.exists())
        .map(|_| 1)
        .unwrap_or_else(|| root.join(".app.json").exists() as usize);
    let hooks = hooks_path
        .and_then(|path| safe_relative_path(root, path))
        .filter(|path| path.exists())
        .map(|_| 1)
        .unwrap_or_else(|| root.join("hooks/hooks.json").exists() as usize);
    let assets = root.join("assets").exists() as usize;

    PluginComponentCounts {
        skills,
        mcp_servers,
        apps,
        hooks,
        assets,
    }
}

fn safe_relative_path(root: &Path, value: &str) -> Option<PathBuf> {
    let trimmed = value.trim().trim_start_matches("./");
    if trimmed.is_empty() || trimmed.contains("..") {
        return None;
    }
    Some(root.join(trimmed))
}

fn count_files_named(root: &Path, file_name: &str) -> usize {
    if !root.exists() {
        return 0;
    }
    WalkDir::new(root)
        .min_depth(1)
        .max_depth(5)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == file_name)
        .count()
}

fn discover_local_marketplaces(paths: &ManagerPaths) -> Result<Vec<MarketplaceDocument>> {
    let mut candidates = vec![
        (
            paths.personal_codex_marketplace_file(),
            AgentToolTarget::Codex,
            "personal Codex marketplace",
        ),
        (
            paths.personal_claude_marketplace_file(),
            AgentToolTarget::ClaudeCode,
            "personal Claude marketplace",
        ),
    ];
    if let Some(path) = paths.project_codex_marketplace_file() {
        candidates.push((path, AgentToolTarget::Codex, "project Codex marketplace"));
    }
    if let Some(path) = paths.project_claude_marketplace_file() {
        candidates.push((
            path,
            AgentToolTarget::ClaudeCode,
            "project Claude marketplace",
        ));
    }

    let mut documents = Vec::new();
    for (path, target, label) in candidates {
        if path.exists() {
            let mut document = read_marketplace_file(&path, Some(target))?;
            document.root_dir = path.parent().and_then(Path::parent).map(Path::to_path_buf);
            document.source_label = Some(label.to_string());
            documents.push(document);
        }
    }
    Ok(documents)
}

fn read_marketplace_file(
    path: &Path,
    target: Option<AgentToolTarget>,
) -> Result<MarketplaceDocument> {
    let raw = fs::read_to_string(path)?;
    let mut document = parse_marketplace_document(&raw, target)?;
    document.root_dir = path.parent().map(Path::to_path_buf);
    Ok(document)
}

fn parse_marketplace_document(
    raw: &str,
    target: Option<AgentToolTarget>,
) -> Result<MarketplaceDocument> {
    let parsed = serde_json::from_str::<RawMarketplaceDocument>(raw)?;
    let target = target.unwrap_or(AgentToolTarget::Generic);
    let name = parsed
        .name
        .unwrap_or_else(|| "unnamed-marketplace".to_string());
    let entries = parsed
        .plugins
        .into_iter()
        .map(|entry| marketplace_entry(entry, &name, target))
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    if entries.is_empty() {
        diagnostics.push(SkillDiagnostic::warning(
            "marketplace does not contain any plugin entries",
        ));
    }
    Ok(MarketplaceDocument {
        display_name: parsed
            .interface
            .and_then(|interface| interface.display_name)
            .or_else(|| owner_name(parsed.owner.as_ref())),
        name,
        target,
        root_dir: None,
        source_label: None,
        entries,
        diagnostics,
    })
}

fn owner_name(value: Option<&Value>) -> Option<String> {
    value?
        .get("name")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn marketplace_entry(
    raw: RawMarketplaceEntry,
    marketplace_name: &str,
    target: AgentToolTarget,
) -> MarketplaceEntry {
    let source = raw
        .source
        .as_ref()
        .map(marketplace_source)
        .unwrap_or_else(|| MarketplaceSource::Unknown {
            raw: "missing source".to_string(),
        });
    let (policy_installation, policy_authentication) = policy_fields(raw.policy.as_ref());
    MarketplaceEntry {
        name: raw.name.unwrap_or_else(|| "unnamed-plugin".to_string()),
        description: raw.description,
        version: raw.version,
        category: raw.category,
        target,
        marketplace: Some(marketplace_name.to_string()),
        source,
        policy_installation,
        policy_authentication,
    }
}

fn marketplace_source(value: &Value) -> MarketplaceSource {
    match value {
        Value::String(path) => {
            if path.starts_with("http://") || path.starts_with("https://") {
                MarketplaceSource::Url { url: path.clone() }
            } else {
                MarketplaceSource::Local { path: path.clone() }
            }
        }
        Value::Object(map) => {
            let source_type = map.get("source").and_then(Value::as_str).unwrap_or("local");
            match source_type {
                "local" => map
                    .get("path")
                    .and_then(Value::as_str)
                    .map(|path| MarketplaceSource::Local {
                        path: path.to_string(),
                    })
                    .unwrap_or_else(|| MarketplaceSource::Unknown {
                        raw: value.to_string(),
                    }),
                "git" => map
                    .get("url")
                    .or_else(|| map.get("repo"))
                    .and_then(Value::as_str)
                    .map(|url| MarketplaceSource::Git {
                        url: url.to_string(),
                        path: map
                            .get("path")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        reference: map
                            .get("ref")
                            .or_else(|| map.get("sha"))
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                    })
                    .unwrap_or_else(|| MarketplaceSource::Unknown {
                        raw: value.to_string(),
                    }),
                "git-subdir" => map
                    .get("url")
                    .and_then(Value::as_str)
                    .zip(map.get("path").and_then(Value::as_str))
                    .map(|(url, path)| MarketplaceSource::GitSubdir {
                        url: url.to_string(),
                        path: path.to_string(),
                        reference: map
                            .get("ref")
                            .or_else(|| map.get("sha"))
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                    })
                    .unwrap_or_else(|| MarketplaceSource::Unknown {
                        raw: value.to_string(),
                    }),
                "url" => map
                    .get("url")
                    .and_then(Value::as_str)
                    .map(|url| MarketplaceSource::Url {
                        url: url.to_string(),
                    })
                    .unwrap_or_else(|| MarketplaceSource::Unknown {
                        raw: value.to_string(),
                    }),
                _ => MarketplaceSource::Unknown {
                    raw: value.to_string(),
                },
            }
        }
        _ => MarketplaceSource::Unknown {
            raw: value.to_string(),
        },
    }
}

fn policy_fields(policy: Option<&Value>) -> (Option<String>, Option<String>) {
    let installation = policy
        .and_then(|policy| policy.get("installation"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let authentication = policy
        .and_then(|policy| policy.get("authentication"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    (installation, authentication)
}

async fn load_marketplace(
    source: &str,
    target: Option<AgentToolTarget>,
) -> Result<MarketplaceDocument> {
    if source.starts_with("http://") || source.starts_with("https://") {
        if source.ends_with(".json") || source.contains("raw.githubusercontent.com") {
            let raw = reqwest::get(source).await?.text().await?;
            return parse_marketplace_document(&raw, target);
        }
        if source.contains("github.com") {
            return load_github_marketplace(source, target).await;
        }
        let raw = reqwest::get(source).await?.text().await?;
        return parse_marketplace_document(&raw, target);
    }

    let path = PathBuf::from(source);
    if path.is_file() {
        read_marketplace_file(&path, target)
    } else if path.join(CODEX_MARKETPLACE_FILE).exists() {
        read_marketplace_file(
            &path.join(CODEX_MARKETPLACE_FILE),
            Some(AgentToolTarget::Codex),
        )
    } else if path.join(CLAUDE_MARKETPLACE_FILE).exists() {
        read_marketplace_file(
            &path.join(CLAUDE_MARKETPLACE_FILE),
            Some(AgentToolTarget::ClaudeCode),
        )
    } else if path.join("marketplace.json").exists() {
        read_marketplace_file(&path.join("marketplace.json"), target)
    } else {
        Err(SkillsManagerError::InvalidResource(format!(
            "could not find marketplace.json in {source}"
        )))
    }
}

async fn load_github_marketplace(
    source: &str,
    target: Option<AgentToolTarget>,
) -> Result<MarketplaceDocument> {
    let source = GitHubTreeSource::parse(source)?;
    let bytes = download_first_archive(&source).await?;
    let temp_dir = tempfile::tempdir()?;
    extract_zip_safe(&bytes, temp_dir.path())?;
    let search_root = filtered_search_root(temp_dir.path(), source.path_filter())?;
    load_marketplace_from_extracted(search_root, target, temp_dir)
}

fn load_marketplace_from_extracted(
    search_root: PathBuf,
    target: Option<AgentToolTarget>,
    _temp_dir: TempDir,
) -> Result<MarketplaceDocument> {
    for (candidate, candidate_target) in [
        (
            search_root.join(CODEX_MARKETPLACE_FILE),
            AgentToolTarget::Codex,
        ),
        (
            search_root.join(CLAUDE_MARKETPLACE_FILE),
            AgentToolTarget::ClaudeCode,
        ),
        (
            search_root.join("marketplace.json"),
            target.unwrap_or(AgentToolTarget::Generic),
        ),
    ] {
        if candidate.exists() {
            return read_marketplace_file(&candidate, Some(candidate_target));
        }
    }
    Err(SkillsManagerError::InvalidResource(
        "GitHub source did not contain a supported marketplace.json".to_string(),
    ))
}

async fn search_skillsmp(query: &str) -> Result<MarketplaceSearchResult> {
    let url = format!(
        "https://skillsmp.com/api/v1/skills/search?q={}",
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
    );
    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if let Ok(token) = std::env::var("SKILLSMP_API_KEY") {
        if !token.trim().is_empty() {
            request = request.bearer_auth(token);
        }
    }
    let value = request
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    let entries = search_entries_from_value(&value);
    Ok(MarketplaceSearchResult {
        provider: MarketplaceSearchProvider::SkillsMp,
        query: query.to_string(),
        entries,
    })
}

fn search_entries_from_value(value: &Value) -> Vec<MarketplaceSearchEntry> {
    let items = value
        .as_array()
        .cloned()
        .or_else(|| value.get("data").and_then(Value::as_array).cloned())
        .or_else(|| value.get("skills").and_then(Value::as_array).cloned())
        .or_else(|| value.get("results").and_then(Value::as_array).cloned())
        .unwrap_or_default();

    items
        .into_iter()
        .filter_map(|item| {
            let name = string_field(&item, &["name", "title", "slug"])?;
            Some(MarketplaceSearchEntry {
                name,
                description: string_field(&item, &["description", "summary"]),
                source_url: string_field(
                    &item,
                    &[
                        "source_url",
                        "github_url",
                        "repository_url",
                        "repo_url",
                        "url",
                    ],
                ),
                source_path: string_field(&item, &["path", "source_path"]),
                author: string_field(&item, &["author", "creator", "owner"]),
                stars: number_field(&item, &["stars", "star_count"]),
                kind: ResourceKind::Skill,
            })
        })
        .collect()
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = value.get(key).and_then(Value::as_str) {
            return Some(value.to_string());
        }
    }
    None
}

fn number_field(value: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(value) = value.get(key).and_then(Value::as_u64) {
            return Some(value);
        }
    }
    None
}

fn target_from_config(value: Option<&str>) -> AgentToolTarget {
    match value {
        Some("codex") => AgentToolTarget::Codex,
        Some("claude-code") | Some("claude") => AgentToolTarget::ClaudeCode,
        _ => AgentToolTarget::Generic,
    }
}

fn codex_plugin_destination(
    paths: &ManagerPaths,
    marketplace: &str,
    name: &str,
    version: Option<&str>,
    conflict_policy: ConflictPolicy,
) -> PathBuf {
    let marketplace = sanitize_folder_name(marketplace);
    let name = sanitize_folder_name(name);
    let version = sanitize_plugin_version_segment(version.unwrap_or("local"));
    let parent = paths.codex_plugin_cache_dir().join(marketplace).join(name);
    let preferred = if version.is_empty() {
        "local"
    } else {
        &version
    };
    if conflict_policy == ConflictPolicy::Rename && parent.join(preferred).exists() {
        parent.join(unique_folder_name(&parent, preferred, &HashMap::new()))
    } else {
        parent.join(preferred)
    }
}

fn sanitize_plugin_version_segment(version: &str) -> String {
    let sanitized = version
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches(|ch| ch == '-' || ch == '.')
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if sanitized.is_empty() {
        "local".to_string()
    } else {
        sanitized
    }
}

fn infer_plugin_marketplace(paths: &ManagerPaths, manifest: &PluginManifest) -> Option<String> {
    if manifest.target != AgentToolTarget::Codex {
        return None;
    }
    let relative = manifest
        .root_dir
        .strip_prefix(paths.codex_plugin_cache_dir())
        .ok()?;
    let mut components = relative.components();
    components
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .map(ToString::to_string)
}

fn plugin_resource_id(target: AgentToolTarget, name: &str, marketplace: Option<&str>) -> String {
    format!(
        "plugin:{}:{}",
        target.id_prefix(),
        plugin_display_id(name, marketplace)
    )
}

fn plugin_display_id(name: &str, marketplace: Option<&str>) -> String {
    marketplace
        .filter(|marketplace| !marketplace.trim().is_empty())
        .map(|marketplace| format!("{name}@{marketplace}"))
        .unwrap_or_else(|| name.to_string())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir_all(destination)?;
    for entry in WalkDir::new(source).follow_links(false).into_iter() {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .expect("entry starts with source");
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("resource");
    path.with_file_name(format!("{file_name}.backup-{timestamp}"))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn parses_codex_plugin_manifest_and_counts_components() {
        let dir = tempdir().unwrap();
        let plugin = dir.path().join("plugin");
        fs::create_dir_all(plugin.join(".codex-plugin")).unwrap();
        fs::create_dir_all(plugin.join("skills/demo")).unwrap();
        fs::write(
            plugin.join(".codex-plugin/plugin.json"),
            r#"{
                "name": "demo-plugin",
                "version": "1.2.3",
                "description": "Demo plugin",
                "skills": "./skills/",
                "mcpServers": "./.mcp.json",
                "interface": {"displayName": "Demo Plugin"}
            }"#,
        )
        .unwrap();
        fs::write(
            plugin.join("skills/demo/SKILL.md"),
            "---\nname: demo\ndescription: Demo\n---\n",
        )
        .unwrap();
        fs::write(plugin.join(".mcp.json"), "{}").unwrap();

        let manifest = read_plugin_manifest(&plugin).unwrap();

        assert_eq!(manifest.target, AgentToolTarget::Codex);
        assert_eq!(manifest.name, "demo-plugin");
        assert_eq!(manifest.display_name.as_deref(), Some("Demo Plugin"));
        assert_eq!(manifest.component_counts.skills, 1);
        assert_eq!(manifest.component_counts.mcp_servers, 1);
    }

    #[test]
    fn parses_claude_marketplace_string_source() {
        let raw = r#"{
            "name": "team-tools",
            "owner": {"name": "Team"},
            "plugins": [
                {
                    "name": "review",
                    "source": "./plugins/review",
                    "description": "Review code"
                }
            ]
        }"#;

        let document = parse_marketplace_document(raw, Some(AgentToolTarget::ClaudeCode)).unwrap();

        assert_eq!(document.name, "team-tools");
        assert_eq!(document.display_name.as_deref(), Some("Team"));
        assert_eq!(document.entries.len(), 1);
        assert!(matches!(
            document.entries[0].source,
            MarketplaceSource::Local { .. }
        ));
    }

    #[test]
    fn codex_plugin_destination_uses_cache_layout() {
        let dir = tempdir().unwrap();
        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            None,
        );
        let path = codex_plugin_destination(
            &paths,
            "my-market",
            "demo-plugin",
            Some("1.0.0"),
            ConflictPolicy::Block,
        );

        assert!(path.ends_with(".codex/plugins/cache/my-market/demo-plugin/1.0.0"));
        assert_eq!(sanitize_plugin_version_segment("../1.0.0 "), "1.0.0");
    }

    #[test]
    fn codex_plugin_toggle_is_preserved_after_install() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source");
        fs::create_dir_all(source.join(".codex-plugin")).unwrap();
        fs::write(
            source.join(".codex-plugin/plugin.json"),
            r#"{"name":"demo-plugin","version":"local","description":"Demo plugin"}"#,
        )
        .unwrap();
        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            None,
        );
        let manager = ResourceManager::new(paths.clone());

        manager
            .install_plugin(PluginInstallRequest {
                source_root: source,
                source_url: Some("fixture".to_string()),
                target: AgentToolTarget::Codex,
                marketplace: Some("local".to_string()),
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: false,
            })
            .unwrap();

        assert!(
            paths
                .codex_plugin_cache_dir()
                .join("local/demo-plugin/local/.codex-plugin/plugin.json")
                .exists()
        );
        assert!(
            CodexConfig::load(&paths)
                .unwrap()
                .is_plugin_disabled("demo-plugin@local")
        );
    }
}
