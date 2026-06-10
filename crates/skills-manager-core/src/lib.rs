//! Core API surface for `skills-manager`.
//!
//! This crate contains the shared domain logic used by both the CLI and desktop UI.
//! It is intentionally split into small modules by responsibility:
//!
//! - `model`, `error`, `paths`: shared data types and error/result definitions.
//! - `skill`, `install`, `scaffold`: skill discovery, installation, and generation.
//! - `download`, `marketplace`: remote catalog and cache ingestion.
//! - `resource`, `target`, `target_registry`: target profiles and additional resource types.
//! - `snapshot`, `manager_config`: workspace view and persistent state serialization.
pub mod codex_config;
pub mod download;
pub mod error;
mod fs_ops;
pub mod github;
pub mod install;
pub mod manager_config;
pub mod marketplace;
pub mod mcp;
pub mod model;
pub mod operation;
pub mod paths;
pub mod resource;
pub mod scaffold;
pub mod skill;
pub mod snapshot;
pub mod target;
pub mod target_registry;

/// Download, cache, and catalog helpers for remote GitHub skill sources.
pub use download::{
    DownloadedCatalog, DownloadedSkillEntry, cache_github_skills_archive, download_github_catalog,
    download_github_marketplace, download_github_skills, download_github_skills_to_cache,
    downloaded_skill_entry, list_downloaded_skills, remove_downloaded_skills,
};
/// Error primitives used by all core operations.
pub use error::{Result, SkillsManagerError};
/// GitHub URL parsing and marketplace URL construction utilities.
pub use github::{GitHubTreeSource, catalog_git_install_url};
/// Install planning and execution primitives.
pub use install::{
    ConflictPolicy, InstallCandidate, InstallPreview, InstallRequest, InstallResult, InstallTarget,
    Installer,
};
/// Persistent config schema persisted by the CLI/UI.
pub use manager_config::{DownloadedMetadata, ManagerConfig};
/// Catalog format, parsing, and export helpers.
pub use marketplace::{
    CatalogFormat, Marketplace, SkillCatalog, SkillCatalogEntry, SkillCatalogSource,
    export_installed_catalog,
};
/// MCP server discovery and configuration helpers.
pub use mcp::{
    McpServerRequest, McpServerTransport, add_mcp_server, remove_mcp_server, scan_mcp_servers,
    set_mcp_server_enabled,
};
/// Shared domain models for skills, diagnostics, resources, and scopes.
pub use model::{
    AgentToolTarget, DiagnosticSeverity, InstalledSkill, ManagedResource, ResourceHealth,
    ResourceKind, SkillDiagnostic, SkillEnablement, SkillFrontmatter, SkillHealth, SkillScope,
};
/// Installation operation journaling and rollback plan primitives.
pub use operation::OperationPlan;
/// Filesystem location helpers and scope helpers.
pub use paths::{ManagerPaths, ProjectRoot, normalize_path_key, validate_path};
/// Plugin/resource indexing types and marketplace/resource management functions.
pub use resource::{
    MarketplaceDocument, MarketplaceEntry, MarketplaceSearchEntry, MarketplaceSearchProvider,
    MarketplaceSearchResult, MarketplaceSource, MarketplaceSourceRecord, PluginComponentCounts,
    PluginInstallPreview, PluginInstallRequest, PluginInstallResult, PluginManifest,
    ResourceManager, ResourceOperationPlan, add_marketplace_source, inspect_marketplace_source,
    list_marketplace_sources, read_plugin_manifest, refresh_marketplace_source,
    remove_marketplace_source, scan_installed_plugins, scan_marketplaces, scan_resources,
    search_marketplace,
};
/// Scaffold generation helpers for creating new skills from templates.
pub use scaffold::{
    SkillScaffoldPreview, SkillScaffoldRequest, create_skill_scaffold, preview_skill_scaffold,
};
/// SKILL discovery, frontmatter parsing, validation, and indexing utilities.
pub use skill::{
    discover_skill_candidates, format_bytes, installed_skill_identity, read_skill_candidate,
    scan_installed_skills, validate_skill, visible_skill_scopes,
};
/// Snapshot helpers used by status commands and the desktop dashboard.
pub use snapshot::{SkillIndex, SkillSearchEntry, WorkspaceCounts, WorkspaceSnapshot};
/// Target profiles and diagnostics for each managed scope.
pub use target::{
    DoctorRepairAction, DoctorReport, DoctorSummary, EnablementStrategy, LayoutPolicy,
    RepairOutcome, RepairReport, TargetDoctorReport, TargetHealthCounts, TargetProfile,
    doctor_report, doctor_report_for_skills, repair_targets, target_profiles,
    target_specific_diagnostics,
};
/// Registry wrapper around all available install targets.
pub use target_registry::{TargetCapabilities, TargetRegistry};
