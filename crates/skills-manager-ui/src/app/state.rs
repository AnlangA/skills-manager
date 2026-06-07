//! UI state structures for each application view.
//!
//! Contains the mutable state for inventory browsing, install workflows,
//! skill scaffolding, catalog export, settings, marketplace management,
//! and install preview.

use std::path::PathBuf;

use skills_manager_core::{
    ConflictPolicy, DoctorReport, InstallTarget, OperationPlan, ResourceKind, SkillHealth,
    SkillScaffoldPreview, SkillScope, TargetProfile,
};

use super::filters::{HealthFilter, PluginTargetFilter, ScopeFilter, SortKey, SourceFilter};
use super::types::{InstallSource, UiCatalogFormat, UiConflictPolicy, UiScope};

/// State for the inventory/library view filters, selection, and pending actions.
#[derive(Debug, Clone)]
pub struct InventoryState {
    pub skill_search_query: String,
    pub plugin_search_query: String,
    pub marketplace_search_query: String,
    pub selected_skill_id: Option<String>,
    pub selected_resource_id: Option<String>,
    pub scope_filter: ScopeFilter,
    pub health_filter: HealthFilter,
    pub source_filter: SourceFilter,
    pub plugin_target_filter: PluginTargetFilter,
    pub sort_key: SortKey,
    pub pending_remove_skill: Option<PathBuf>,
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

/// State for the install workflow view, including source selection and preview.
#[derive(Debug, Clone)]
pub struct InstallState {
    pub install_source: InstallSource,
    pub source_url: String,
    pub local_source_path: String,
    pub catalog_url: String,
    pub download_path_override: String,
    pub install_scope: UiScope,
    pub custom_install_path: String,
    pub enable_after_install: bool,
    pub conflict_policy: UiConflictPolicy,
    pub preview: Option<PreviewState>,
    pub downloaded_entries: Vec<DownloadedEntryState>,
    pub selected_download_root: Option<PathBuf>,
    pub catalog_entries: Vec<CatalogEntryState>,
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
    pub name: String,
    pub description: String,
    pub target: UiScope,
    pub custom_path: String,
    pub tags: String,
    pub allowed_tools: String,
    pub compatibility: String,
    pub license: String,
    pub when_to_use: String,
    pub disable_model_invocation: bool,
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
    pub catalog_format: UiCatalogFormat,
    pub catalog_save_path: String,
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
    pub project_path: String,
    pub default_download_path: String,
    pub target_profiles: Vec<TargetProfile>,
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
    pub source_label: String,
    pub source: InstallSource,
    pub source_value: String,
    pub download_dir: Option<String>,
    pub target: InstallTarget,
    pub enable_after_install: bool,
    pub scope: SkillScope,
    pub conflict_policy: ConflictPolicy,
    pub operation_plan: Option<OperationPlan>,
    pub candidates: Vec<PreviewCandidateState>,
}

impl PreviewState {
    pub fn has_blocking_conflicts(&self) -> bool {
        self.conflict_policy == ConflictPolicy::Block
            && self.candidates.iter().any(|candidate| candidate.conflict)
    }
}

/// Per-skill candidate state within an install preview.
#[derive(Debug, Clone)]
pub struct PreviewCandidateState {
    pub name: String,
    pub description: String,
    pub destination_root: PathBuf,
    pub health: SkillHealth,
    pub conflict: bool,
    pub diagnostics: Vec<String>,
    pub resource_count: usize,
    pub resource_bytes: u64,
}

/// State for a single catalog entry loaded from a remote catalog URL.
#[derive(Debug, Clone)]
pub struct CatalogEntryState {
    pub name: String,
    pub description: String,
    pub source_label: String,
    pub install_source: Option<InstallSource>,
    pub source_value: Option<String>,
    pub unavailable_reason: Option<String>,
}

/// Display state for a cached downloaded skill bundle.
#[derive(Debug, Clone)]
pub struct DownloadedEntryState {
    pub source_url: String,
    pub root_dir: PathBuf,
    pub downloaded_at: String,
    pub summary: String,
}

/// Precomputed derived state for inventory filtering and search.
#[derive(Debug, Clone, Default)]
pub struct DerivedInventoryState {
    pub filtered_skill_indices: Vec<usize>,
    pub resource_search: Vec<ResourceSearchEntry>,
    pub counts: SkillCounts,
    pub visible_scopes_by_id: std::collections::BTreeMap<String, Vec<SkillScope>>,
}

/// Search index entry for a managed resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceSearchEntry {
    pub resource_index: usize,
    pub kind: ResourceKind,
    pub haystack: String,
}

/// Aggregate skill counts broken down by enablement, health, scope, and source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillCounts {
    pub enabled: usize,
    pub disabled: usize,
    pub valid: usize,
    pub warning: usize,
    pub invalid: usize,
    pub shadowed: usize,
    pub project: usize,
    pub global: usize,
    pub claude_code: usize,
    pub droid: usize,
    pub opencode: usize,
    pub codex: usize,
    pub zed: usize,
    pub custom: usize,
    pub known_source: usize,
    pub exportable: usize,
}
