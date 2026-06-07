//! Shared UI type definitions for views, scopes, and policies.
//!
//! Provides display-friendly enums for active views, install sources,
//! scope selectors, conflict policies, catalog formats, and tab selectors
//! used across the desktop UI.

use std::{fmt, path::PathBuf};

use skills_manager_core::{CatalogFormat, ConflictPolicy, InstallTarget};

/// Active screen selection in the sidebar navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    /// Skills library view.
    Library,
    /// Plugin management view.
    Plugins,
    /// Install workflow view.
    Install,
    /// Skill scaffold creation view.
    Create,
    /// Marketplace source management view.
    Marketplace,
    /// Catalog export view.
    Catalog,
    /// Targets and settings view.
    Targets,
}

impl ActiveView {
    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Skills",
            Self::Plugins => "Plugins",
            Self::Install => "Install",
            Self::Create => "Create",
            Self::Marketplace => "Marketplace",
            Self::Catalog => "Catalog",
            Self::Targets => "Targets",
        }
    }
}

/// Source type for skill installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    /// Install from a GitHub repository URL.
    Url,
    /// Install from a local filesystem path.
    Local,
    /// Install from a previously downloaded cache bundle.
    Downloaded,
    /// Install from a loaded catalog entry.
    Catalog,
}

impl InstallSource {
    pub const ALL: [Self; 4] = [Self::Url, Self::Local, Self::Downloaded, Self::Catalog];
}

impl fmt::Display for InstallSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Url => "GitHub URL",
            Self::Local => "Local folder",
            Self::Downloaded => "Downloaded",
            Self::Catalog => "Catalog",
        })
    }
}

/// UI-friendly scope selector mapping to install targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScope {
    /// Global user scope.
    Global,
    /// Project-local scope.
    Project,
    /// Claude Code scope.
    ClaudeCode,
    /// Droid scope.
    Droid,
    /// OpenCode scope.
    OpenCode,
    /// Codex scope.
    Codex,
    /// Zed scope.
    Zed,
    /// User-defined custom root.
    Custom,
}

impl UiScope {
    pub const ALL: [Self; 8] = [
        Self::Global,
        Self::Project,
        Self::ClaudeCode,
        Self::Droid,
        Self::OpenCode,
        Self::Codex,
        Self::Zed,
        Self::Custom,
    ];
}

impl fmt::Display for UiScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Global => "Global",
            Self::Project => "Project",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom => "Custom",
        })
    }
}

/// UI conflict resolution policy selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiConflictPolicy {
    /// Abort when destination exists.
    Block,
    /// Rename the new skill folder.
    Rename,
    /// Replace existing with a backup.
    Replace,
}

impl UiConflictPolicy {
    pub const ALL: [Self; 3] = [Self::Block, Self::Rename, Self::Replace];
}

impl From<UiConflictPolicy> for ConflictPolicy {
    fn from(value: UiConflictPolicy) -> Self {
        match value {
            UiConflictPolicy::Block => Self::Block,
            UiConflictPolicy::Rename => Self::Rename,
            UiConflictPolicy::Replace => Self::Replace,
        }
    }
}

impl fmt::Display for UiConflictPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Block => "Block conflicts",
            Self::Rename => "Rename new skill",
            Self::Replace => "Replace with backup",
        })
    }
}

/// UI catalog export format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCatalogFormat {
    /// Pretty JSON.
    Json,
    /// XML string.
    Xml,
    /// Markdown table.
    Markdown,
}

impl UiCatalogFormat {
    pub const ALL: [Self; 3] = [Self::Json, Self::Xml, Self::Markdown];
}

impl From<UiCatalogFormat> for CatalogFormat {
    fn from(value: UiCatalogFormat) -> Self {
        match value {
            UiCatalogFormat::Json => Self::Json,
            UiCatalogFormat::Xml => Self::Xml,
            UiCatalogFormat::Markdown => Self::Markdown,
        }
    }
}

impl fmt::Display for UiCatalogFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Json => "JSON",
            Self::Xml => "XML",
            Self::Markdown => "Markdown",
        })
    }
}

/// Resolves a UI scope and optional custom path into an [`InstallTarget`].
pub fn resolve_install_target(scope: UiScope, custom_path: &str) -> Result<InstallTarget, String> {
    match scope {
        UiScope::Global => Ok(InstallTarget::Global),
        UiScope::Project => Ok(InstallTarget::Project),
        UiScope::ClaudeCode => Ok(InstallTarget::ClaudeCode),
        UiScope::Droid => Ok(InstallTarget::Droid),
        UiScope::OpenCode => Ok(InstallTarget::OpenCode),
        UiScope::Codex => Ok(InstallTarget::Codex),
        UiScope::Zed => Ok(InstallTarget::Zed),
        UiScope::Custom => {
            let path = custom_path.trim();
            if path.is_empty() {
                Err("Enter a custom install path first.".to_string())
            } else {
                Ok(InstallTarget::Custom(PathBuf::from(path)))
            }
        }
    }
}
