//! UI state structures for each application view.
//!
//! Contains the mutable state for inventory browsing, install workflows,
//! skill scaffolding, catalog export, settings, marketplace management,
//! and install preview.

use std::path::PathBuf;

use iced::widget::text_editor;
use skills_manager_core::{
    ConflictPolicy, DoctorReport, InstallTarget, McpServerTransport, OperationPlan, ResourceKind,
    SkillHealth, SkillScaffoldPreview, SkillScope, TargetProfile,
};

use super::filters::{HealthFilter, PluginTargetFilter, ScopeFilter, SortKey, SourceFilter};
use super::types::{
    ExpandedEditorTarget, InstallSource, UiAgentTarget, UiCatalogFormat, UiConflictPolicy, UiScope,
};

/// State for the inventory/library view filters, selection, and pending actions.
#[derive(Debug, Clone)]
pub struct InventoryState {
    /// Text search query for the skills list.
    pub skill_search_query: String,
    /// Text search query for the plugins list.
    pub plugin_search_query: String,
    /// Text search query for the marketplace list.
    pub marketplace_search_query: String,
    /// ID of the currently selected skill, if any.
    pub selected_skill_id: Option<String>,
    /// ID of the currently selected resource, if any.
    pub selected_resource_id: Option<String>,
    /// Active scope filter.
    pub scope_filter: ScopeFilter,
    /// Active health filter.
    pub health_filter: HealthFilter,
    /// Active source provenance filter.
    pub source_filter: SourceFilter,
    /// Active plugin target filter.
    pub plugin_target_filter: PluginTargetFilter,
    /// Active sort key.
    pub sort_key: SortKey,
    /// Skill root path pending removal confirmation.
    pub pending_remove_skill: Option<PathBuf>,
    /// Plugin ID pending removal confirmation.
    pub pending_remove_plugin: Option<String>,
}

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            skill_search_query: String::new(),
            plugin_search_query: String::new(),
            marketplace_search_query: String::new(),
            selected_skill_id: None,
            selected_resource_id: None,
            scope_filter: ScopeFilter::All,
            health_filter: HealthFilter::All,
            source_filter: SourceFilter::All,
            plugin_target_filter: PluginTargetFilter::All,
            sort_key: SortKey::Priority,
            pending_remove_skill: None,
            pending_remove_plugin: None,
        }
    }
}

/// State for the MCP management view.
#[derive(Debug, Clone)]
pub struct McpState {
    /// Text search query for the MCP server list.
    pub search_query: String,
    /// Active target filter for MCP servers.
    pub target_filter: PluginTargetFilter,
    /// Active health filter for MCP servers.
    pub health_filter: HealthFilter,
    /// Selected agent target for the new MCP server.
    pub target: UiAgentTarget,
    /// Selected transport type (stdio or HTTP).
    pub transport: McpServerTransport,
    /// MCP server name input.
    pub name: String,
    /// Local command input for stdio transport.
    pub command: String,
    /// Command arguments input.
    pub args: String,
    /// Environment variables input.
    pub env: String,
    /// Remote URL input for HTTP transport.
    pub url: String,
    /// HTTP headers input.
    pub headers: String,
    /// Whether the new MCP server should be enabled.
    pub enabled: bool,
    /// MCP server name pending removal confirmation.
    pub pending_remove: Option<String>,
}

impl Default for McpState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            target_filter: PluginTargetFilter::All,
            health_filter: HealthFilter::All,
            target: UiAgentTarget::Codex,
            transport: McpServerTransport::Stdio,
            name: String::new(),
            command: String::new(),
            args: String::new(),
            env: String::new(),
            url: String::new(),
            headers: String::new(),
            enabled: true,
            pending_remove: None,
        }
    }
}

/// State for the right-hand expanded text editor shared by form-heavy pages.
#[derive(Debug, Clone)]
pub struct ExpandedEditorState {
    /// The field currently being edited in the expanded panel.
    pub active: Option<ExpandedEditorTarget>,
    /// Text editor content buffer.
    pub content: text_editor::Content,
}

impl Default for ExpandedEditorState {
    fn default() -> Self {
        Self {
            active: None,
            content: text_editor::Content::new(),
        }
    }
}

/// State for the install workflow view, including source selection and preview.
#[derive(Debug, Clone)]
pub struct InstallState {
    /// Selected install source type.
    pub install_source: InstallSource,
    /// GitHub URL input.
    pub source_url: String,
    /// Local folder path input.
    pub local_source_path: String,
    /// Catalog URL input.
    pub catalog_url: String,
    /// Optional download cache override path.
    pub download_path_override: String,
    /// Selected install scope.
    pub install_scope: UiScope,
    /// Custom install path input (when scope is Custom).
    pub custom_install_path: String,
    /// Whether to enable skills after installation.
    pub enable_after_install: bool,
    /// Selected conflict resolution policy.
    pub conflict_policy: UiConflictPolicy,
    /// Current install preview state, if loaded.
    pub preview: Option<PreviewState>,
    /// List of cached downloaded entries.
    pub downloaded_entries: Vec<DownloadedEntryState>,
    /// Currently selected download root for install-from-cache.
    pub selected_download_root: Option<PathBuf>,
    /// Loaded catalog entries from a remote catalog URL.
    pub catalog_entries: Vec<CatalogEntryState>,
    /// Download root pending removal confirmation.
    pub pending_remove_download: Option<PathBuf>,
}

impl Default for InstallState {
    fn default() -> Self {
        Self {
            install_source: InstallSource::Url,
            source_url: String::new(),
            local_source_path: String::new(),
            catalog_url: String::new(),
            download_path_override: String::new(),
            install_scope: UiScope::Global,
            custom_install_path: String::new(),
            enable_after_install: true,
            conflict_policy: UiConflictPolicy::Block,
            preview: None,
            downloaded_entries: Vec::new(),
            selected_download_root: None,
            catalog_entries: Vec::new(),
            pending_remove_download: None,
        }
    }
}

/// State for the skill scaffold creation form.
#[derive(Debug, Clone)]
pub struct CreateState {
    /// Skill name input.
    pub name: String,
    /// Skill description input.
    pub description: String,
    /// Target scope for scaffold output.
    pub target: UiScope,
    /// Custom root path for scaffold output.
    pub custom_path: String,
    /// Comma-separated tags input.
    pub tags: String,
    /// Comma-separated allowed tools input.
    pub allowed_tools: String,
    /// Compatibility text input.
    pub compatibility: String,
    /// License identifier input.
    pub license: String,
    /// When-to-use trigger guidance input.
    pub when_to_use: String,
    /// Whether to disable model invocation for the scaffold.
    pub disable_model_invocation: bool,
    /// Scaffold preview result, if generated.
    pub preview: Option<SkillScaffoldPreview>,
}

impl Default for CreateState {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            target: UiScope::Global,
            custom_path: String::new(),
            tags: String::new(),
            allowed_tools: String::new(),
            compatibility: String::new(),
            license: String::new(),
            when_to_use: String::new(),
            disable_model_invocation: false,
            preview: None,
        }
    }
}

/// State for the catalog export view.
#[derive(Debug, Clone)]
pub struct CatalogExportState {
    /// Selected export format.
    pub catalog_format: UiCatalogFormat,
    /// File path for saving the catalog export.
    pub catalog_save_path: String,
    /// Generated catalog output content.
    pub catalog_output: String,
}

impl Default for CatalogExportState {
    fn default() -> Self {
        Self {
            catalog_format: UiCatalogFormat::Json,
            catalog_save_path: "agent-skills-catalog.json".to_string(),
            catalog_output: String::new(),
        }
    }
}

/// State for the settings/targets view.
#[derive(Debug, Clone)]
pub struct AppSettingsState {
    /// Active project root path.
    pub project_path: String,
    /// Default download directory override.
    pub default_download_path: String,
    /// Loaded target profiles for all scopes.
    pub target_profiles: Vec<TargetProfile>,
    /// Latest doctor report, if generated.
    pub doctor_report: Option<DoctorReport>,
}

impl Default for AppSettingsState {
    fn default() -> Self {
        let project_path = std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        Self {
            project_path,
            default_download_path: String::new(),
            target_profiles: Vec::new(),
            doctor_report: None,
        }
    }
}

/// State for an install preview showing candidates and conflict information.
#[derive(Debug, Clone)]
pub struct PreviewState {
    /// Display label for the install source.
    pub source_label: String,
    /// Source type used for this install.
    pub source: InstallSource,
    /// Raw source value (URL, path, or root).
    pub source_value: String,
    /// Optional download directory override.
    pub download_dir: Option<String>,
    /// Resolved install target.
    pub target: InstallTarget,
    /// Whether skills will be enabled after install.
    pub enable_after_install: bool,
    /// Resolved scope for the install.
    pub scope: SkillScope,
    /// Conflict resolution policy.
    pub conflict_policy: ConflictPolicy,
    /// Serializable operation plan for applying the install.
    pub operation_plan: Option<OperationPlan>,
    /// Per-candidate preview entries.
    pub candidates: Vec<PreviewCandidateState>,
}

impl PreviewState {
    /// Returns `true` if any candidate has a conflict and the policy is Block.
    pub fn has_blocking_conflicts(&self) -> bool {
        self.conflict_policy == ConflictPolicy::Block
            && self.candidates.iter().any(|candidate| candidate.conflict)
    }
}

/// Per-skill candidate state within an install preview.
#[derive(Debug, Clone)]
pub struct PreviewCandidateState {
    /// Candidate skill name.
    pub name: String,
    /// Candidate skill description.
    pub description: String,
    /// Destination folder for the candidate.
    pub destination_root: PathBuf,
    /// Health status of the candidate.
    pub health: SkillHealth,
    /// Whether a destination conflict was detected.
    pub conflict: bool,
    /// Formatted diagnostic messages.
    pub diagnostics: Vec<String>,
    /// Number of bundled resource files.
    pub resource_count: usize,
    /// Total size of bundled resource files in bytes.
    pub resource_bytes: u64,
}

/// State for a single catalog entry loaded from a remote catalog URL.
#[derive(Debug, Clone)]
pub struct CatalogEntryState {
    /// Catalog entry display name.
    pub name: String,
    /// Catalog entry description.
    pub description: String,
    /// Human-readable source label.
    pub source_label: String,
    /// Resolved install source type, if available.
    pub install_source: Option<InstallSource>,
    /// Resolved source value (URL or path), if available.
    pub source_value: Option<String>,
    /// Reason the entry cannot be installed, if applicable.
    pub unavailable_reason: Option<String>,
}

/// Display state for a cached downloaded skill bundle.
#[derive(Debug, Clone)]
pub struct DownloadedEntryState {
    /// Original GitHub source URL.
    pub source_url: String,
    /// Local cache root directory.
    pub root_dir: PathBuf,
    /// Formatted download timestamp.
    pub downloaded_at: String,
    /// Compact resource summary string.
    pub summary: String,
}

/// Precomputed derived state for inventory filtering and search.
#[derive(Debug, Clone, Default)]
pub struct DerivedInventoryState {
    /// Indices of skills passing current filters.
    pub filtered_skill_indices: Vec<usize>,
    /// Search index for managed resources.
    pub resource_search: Vec<ResourceSearchEntry>,
    /// Aggregate skill counts.
    pub counts: SkillCounts,
    /// Visible scopes per skill identity key.
    pub visible_scopes_by_id: std::collections::BTreeMap<String, Vec<SkillScope>>,
}

/// Search index entry for a managed resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceSearchEntry {
    /// Index into the resources slice.
    pub resource_index: usize,
    /// Resource kind discriminator.
    pub kind: ResourceKind,
    /// Lowercase search haystack text.
    pub haystack: String,
}

/// Aggregate skill counts broken down by enablement, health, scope, and source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillCounts {
    /// Number of enabled skills.
    pub enabled: usize,
    /// Number of disabled skills.
    pub disabled: usize,
    /// Number of valid skills.
    pub valid: usize,
    /// Number of skills with warnings.
    pub warning: usize,
    /// Number of invalid skills.
    pub invalid: usize,
    /// Number of shadowed duplicate skills.
    pub shadowed: usize,
    /// Number of project-scoped skills.
    pub project: usize,
    /// Number of global-scoped skills.
    pub global: usize,
    /// Number of Claude Code skills.
    pub claude_code: usize,
    /// Number of Droid skills.
    pub droid: usize,
    /// Number of OpenCode skills.
    pub opencode: usize,
    /// Number of Codex skills.
    pub codex: usize,
    /// Number of Zed skills.
    pub zed: usize,
    /// Number of custom-scoped skills.
    pub custom: usize,
    /// Number of skills with a known source URL.
    pub known_source: usize,
    /// Number of exportable skills.
    pub exportable: usize,
}
