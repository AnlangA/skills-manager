//! Message variants dispatched by UI views and processed by the update loop.
//!
//! Each [`Message`] variant represents a user action, async task completion,
//! or navigation event that triggers a state transition in the application.

use std::path::PathBuf;

use skills_manager_core::{AgentToolTarget, SkillScaffoldPreview, WorkspaceSnapshot};

use super::filters::{HealthFilter, PluginTargetFilter, ScopeFilter, SortKey, SourceFilter};
use super::state::{CatalogEntryState, PreviewState};
use super::types::{InstallSource, UiCatalogFormat, UiConflictPolicy, UiScope};

/// All messages dispatched by UI views and processed by the update loop.
#[derive(Debug, Clone)]
pub enum Message {
    /// Reload workspace snapshot from disk.
    Refresh,
    /// Result of loading the workspace snapshot.
    WorkspaceLoaded(Result<WorkspaceSnapshot, String>),
    /// Project path input changed.
    ProjectPathChanged(String),
    /// Skill search input changed.
    SkillSearchChanged(String),
    /// Plugin search input changed.
    PluginSearchChanged(String),
    /// Marketplace search input changed.
    MarketplaceSearchChanged(String),
    /// Sidebar navigation selection changed.
    ActiveViewSelected(super::types::ActiveView),
    /// Scope filter changed in the inventory view.
    ScopeFilterSelected(ScopeFilter),
    /// Health filter changed in the inventory view.
    HealthFilterSelected(HealthFilter),
    /// Source filter changed in the inventory view.
    SourceFilterSelected(SourceFilter),
    /// Plugin target filter changed in the plugins view.
    PluginTargetFilterSelected(PluginTargetFilter),
    /// Sort order changed in the inventory view.
    SortSelected(SortKey),
    /// Skill selected in the inventory list.
    SelectSkill(String),
    /// Resource selected in the inventory list.
    SelectResource(String),
    /// Install source type changed.
    InstallSourceSelected(InstallSource),
    /// GitHub URL input changed.
    SourceUrlChanged(String),
    /// Local folder path input changed.
    LocalSourcePathChanged(String),
    /// Catalog URL input changed.
    CatalogUrlChanged(String),
    /// Default download path input changed.
    DefaultDownloadPathChanged(String),
    /// Persist the default download path to config.
    SaveDefaultDownloadPath,
    /// Result of saving the default download path.
    DefaultDownloadPathSaved(Result<String, String>),
    /// Download cache path override input changed.
    DownloadPathOverrideChanged(String),
    /// Install scope selector changed.
    InstallScopeSelected(UiScope),
    /// Custom install path input changed.
    CustomInstallPathChanged(String),
    /// Enable-after-install toggle changed.
    EnableAfterInstallChanged(bool),
    /// Conflict policy selector changed.
    ConflictSelected(UiConflictPolicy),
    /// Start downloading from the selected source.
    DownloadSource,
    /// Result of the download operation.
    Downloaded(Result<String, String>),
    /// Start the install preview.
    PreviewInstall,
    /// Result of the install preview computation.
    PreviewLoaded(Result<PreviewState, String>),
    /// Apply the install from the preview.
    InstallPreview,
    /// Result of the install operation.
    Installed(Result<String, String>),
    /// Load catalog entries from the catalog URL.
    LoadCatalog,
    /// Result of loading catalog entries.
    CatalogLoaded(Result<Vec<CatalogEntryState>, String>),
    /// Preview a specific catalog entry before installing.
    PreviewCatalogEntry(InstallSource, String),
    /// Catalog export format selector changed.
    CatalogFormatSelected(UiCatalogFormat),
    /// Catalog save path input changed.
    CatalogSavePathChanged(String),
    /// Generate the catalog output.
    GenerateCatalog,
    /// Result of catalog generation.
    CatalogGenerated(Result<String, String>),
    /// Copy catalog output to clipboard.
    CopyCatalog,
    /// Save catalog output to file.
    SaveCatalog,
    /// Result of saving the catalog to file.
    CatalogSaved(Result<String, String>),
    /// Toggle skill enablement.
    SetSkillEnabled(PathBuf, bool),
    /// Result of toggling a skill.
    SkillToggled(Result<String, String>),
    /// Request removal of a skill (show confirmation).
    RequestRemoveSkill(PathBuf),
    /// Confirm and execute skill removal.
    ConfirmRemoveSkill(PathBuf),
    /// Result of removing a skill.
    SkillRemoved(Result<String, String>),
    /// Toggle plugin enablement.
    SetPluginEnabled(String, AgentToolTarget, bool),
    /// Result of toggling a plugin.
    PluginToggled(Result<String, String>),
    /// Request removal of a plugin (show confirmation).
    RequestRemovePlugin(String, AgentToolTarget),
    /// Confirm and execute plugin removal.
    ConfirmRemovePlugin(String, AgentToolTarget),
    /// Result of removing a plugin.
    PluginRemoved(Result<String, String>),
    /// Preview a downloaded bundle before installing.
    PreviewDownloaded(PathBuf),
    /// Request removal of a downloaded bundle (show confirmation).
    RequestRemoveDownload(PathBuf),
    /// Confirm and execute downloaded bundle removal.
    ConfirmRemoveDownload(PathBuf),
    /// Result of removing a downloaded bundle.
    DownloadRemoved(Result<String, String>),
    /// Scaffold name input changed.
    CreateNameChanged(String),
    /// Scaffold description input changed.
    CreateDescriptionChanged(String),
    /// Scaffold target scope selector changed.
    CreateTargetSelected(UiScope),
    /// Scaffold custom path input changed.
    CreateCustomPathChanged(String),
    /// Scaffold tags input changed.
    CreateTagsChanged(String),
    /// Scaffold allowed-tools input changed.
    CreateAllowedToolsChanged(String),
    /// Scaffold compatibility input changed.
    CreateCompatibilityChanged(String),
    /// Scaffold license input changed.
    CreateLicenseChanged(String),
    /// Scaffold when-to-use input changed.
    CreateWhenToUseChanged(String),
    /// Scaffold disable-model-invocation toggle changed.
    CreateDisableModelInvocationChanged(bool),
    /// Request a scaffold preview.
    PreviewScaffold,
    /// Result of the scaffold preview computation.
    ScaffoldPreviewed(Result<SkillScaffoldPreview, String>),
    /// Create the skill scaffold on disk.
    CreateSkill,
    /// Result of creating the skill scaffold.
    SkillCreated(Result<SkillScaffoldPreview, String>),
    /// Exit with success code for CI smoke testing.
    SmokeExit,
}
