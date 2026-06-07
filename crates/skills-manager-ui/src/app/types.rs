use std::{fmt, path::PathBuf};

use skills_manager_core::{CatalogFormat, ConflictPolicy, InstallTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Library,
    Plugins,
    Install,
    Create,
    Marketplace,
    Catalog,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    Url,
    Local,
    Downloaded,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScope {
    Global,
    Project,
    ClaudeCode,
    Droid,
    OpenCode,
    Codex,
    Zed,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiConflictPolicy {
    Block,
    Rename,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCatalogFormat {
    Json,
    Xml,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginViewTab {
    All,
    Codex,
    ClaudeCode,
    Generic,
}

impl PluginViewTab {
    pub const ALL: [Self; 4] = [Self::All, Self::Codex, Self::ClaudeCode, Self::Generic];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Generic => "Generic",
        }
    }

    pub fn matches_target(self, target: skills_manager_core::AgentToolTarget) -> bool {
        match self {
            Self::All => true,
            Self::Codex => target == skills_manager_core::AgentToolTarget::Codex,
            Self::ClaudeCode => target == skills_manager_core::AgentToolTarget::ClaudeCode,
            Self::Generic => target == skills_manager_core::AgentToolTarget::Generic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketplaceViewTab {
    Sources,
    Search,
    Inspected,
}

impl MarketplaceViewTab {
    pub const ALL: [Self; 3] = [Self::Sources, Self::Search, Self::Inspected];

    pub fn label(self) -> &'static str {
        match self {
            Self::Sources => "Sources",
            Self::Search => "Search",
            Self::Inspected => "Inspected",
        }
    }
}

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
