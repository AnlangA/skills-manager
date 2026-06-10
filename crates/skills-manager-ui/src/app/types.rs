//! Shared UI type definitions for views, scopes, and policies.
//!
//! Provides display-friendly enums for active views, install sources,
//! scope selectors, conflict policies, catalog formats, and tab selectors
//! used across the desktop UI.

use std::{fmt, path::PathBuf};

use skills_manager_core::validate_path;
use skills_manager_core::{AgentToolTarget, CatalogFormat, ConflictPolicy, InstallTarget};

/// Active screen selection in the sidebar navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    /// Skills library view.
    Library,
    /// Plugin management view.
    Plugins,
    /// MCP server management view.
    Mcp,
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
            Self::Mcp => "MCP",
            Self::Install => "Install",
            Self::Create => "Create",
            Self::Marketplace => "Marketplace",
            Self::Catalog => "Catalog",
            Self::Targets => "Targets",
        }
    }
}

/// UI-friendly agent target selector for MCP resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAgentTarget {
    /// Claude Code target.
    ClaudeCode,
    /// Codex target.
    Codex,
    /// Droid target.
    Droid,
    /// OpenCode target.
    OpenCode,
    /// Zed target.
    Zed,
}

impl UiAgentTarget {
    pub const ALL: [Self; 5] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::Droid,
        Self::OpenCode,
        Self::Zed,
    ];
}

impl From<UiAgentTarget> for AgentToolTarget {
    fn from(value: UiAgentTarget) -> Self {
        match value {
            UiAgentTarget::ClaudeCode => Self::ClaudeCode,
            UiAgentTarget::Codex => Self::Codex,
            UiAgentTarget::Droid => Self::Droid,
            UiAgentTarget::OpenCode => Self::OpenCode,
            UiAgentTarget::Zed => Self::Zed,
        }
    }
}

impl fmt::Display for UiAgentTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::Droid => "Droid",
            Self::OpenCode => "OpenCode",
            Self::Zed => "Zed",
        })
    }
}

/// Text fields that can be edited in the wide right-hand editor panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandedEditorTarget {
    /// Install GitHub URL field.
    InstallSourceUrl,
    /// Install local source path field.
    InstallLocalSourcePath,
    /// Install catalog URL field.
    InstallCatalogUrl,
    /// Install download cache override field.
    InstallDownloadPathOverride,
    /// Install custom target path field.
    InstallCustomPath,
    /// MCP server name field.
    McpName,
    /// MCP local command field.
    McpCommand,
    /// MCP command args field.
    McpArgs,
    /// MCP environment variables field.
    McpEnv,
    /// MCP remote URL field.
    McpUrl,
    /// MCP headers field.
    McpHeaders,
    /// Create skill name field.
    CreateName,
    /// Create skill description field.
    CreateDescription,
    /// Create custom root path field.
    CreateCustomPath,
    /// Create tags field.
    CreateTags,
    /// Create allowed tools field.
    CreateAllowedTools,
    /// Create compatibility field.
    CreateCompatibility,
    /// Create license field.
    CreateLicense,
    /// Create when-to-use field.
    CreateWhenToUse,
}

impl ExpandedEditorTarget {
    pub fn is_install(self) -> bool {
        matches!(
            self,
            Self::InstallSourceUrl
                | Self::InstallLocalSourcePath
                | Self::InstallCatalogUrl
                | Self::InstallDownloadPathOverride
                | Self::InstallCustomPath
                | Self::McpName
                | Self::McpCommand
                | Self::McpArgs
                | Self::McpEnv
                | Self::McpUrl
                | Self::McpHeaders
        )
    }

    pub fn is_create(self) -> bool {
        matches!(
            self,
            Self::CreateName
                | Self::CreateDescription
                | Self::CreateCustomPath
                | Self::CreateTags
                | Self::CreateAllowedTools
                | Self::CreateCompatibility
                | Self::CreateLicense
                | Self::CreateWhenToUse
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::InstallSourceUrl => "GitHub source",
            Self::InstallLocalSourcePath => "Local source folder",
            Self::InstallCatalogUrl => "Catalog URL",
            Self::InstallDownloadPathOverride => "Download cache override",
            Self::InstallCustomPath => "Custom install path",
            Self::McpName => "MCP name",
            Self::McpCommand => "MCP command",
            Self::McpArgs => "MCP args",
            Self::McpEnv => "MCP env",
            Self::McpUrl => "MCP URL",
            Self::McpHeaders => "MCP headers",
            Self::CreateName => "Name",
            Self::CreateDescription => "Description",
            Self::CreateCustomPath => "Custom root",
            Self::CreateTags => "Tags",
            Self::CreateAllowedTools => "Allowed tools",
            Self::CreateCompatibility => "Compatibility",
            Self::CreateLicense => "License",
            Self::CreateWhenToUse => "When to use",
        }
    }

    pub fn helper(self) -> &'static str {
        match self {
            Self::InstallSourceUrl => "Repository shorthand, full URL, or tree URL.",
            Self::InstallLocalSourcePath => "Folder containing one or more SKILL.md files.",
            Self::InstallCatalogUrl => "Load from skills.json, catalog.json, or marketplace.json.",
            Self::InstallDownloadPathOverride => "Optional download cache destination.",
            Self::InstallCustomPath => "Custom skills root used for this install.",
            Self::McpName => "Server key written into the selected target configuration.",
            Self::McpCommand => "Local executable or script command.",
            Self::McpArgs => "Whitespace-separated command arguments.",
            Self::McpEnv => "KEY=VALUE entries, separated by commas or new lines.",
            Self::McpUrl => "Remote MCP endpoint URL.",
            Self::McpHeaders => "KEY=VALUE header entries, separated by commas or new lines.",
            Self::CreateName => "Lowercase, numbers, hyphens, or underscores.",
            Self::CreateDescription => "What the skill does and when to use it.",
            Self::CreateCustomPath => "Custom skills root for scaffold output.",
            Self::CreateTags => "Comma-separated discovery tags.",
            Self::CreateAllowedTools => "Comma-separated tools.",
            Self::CreateCompatibility => "Target compatibility text.",
            Self::CreateLicense => "License identifier.",
            Self::CreateWhenToUse => "Claude Code trigger guidance.",
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Self::InstallSourceUrl => "github.com/owner/repo",
            Self::InstallLocalSourcePath => "path/to/folder/containing/SKILL.md",
            Self::InstallCatalogUrl => "GitHub URL for catalog file",
            Self::InstallDownloadPathOverride => "Use saved default download path",
            Self::InstallCustomPath => "path/to/skills/root",
            Self::McpName => "filesystem",
            Self::McpCommand => "node",
            Self::McpArgs => "server.js --stdio",
            Self::McpEnv => "KEY=value, TOKEN=...",
            Self::McpUrl => "https://example.com/mcp",
            Self::McpHeaders => "Authorization=Bearer ...",
            Self::CreateName => "my-skill",
            Self::CreateDescription => "Use this skill when...",
            Self::CreateCustomPath => "path/to/skills/root",
            Self::CreateTags => "analysis,docs,automation",
            Self::CreateAllowedTools => "shell,browser",
            Self::CreateCompatibility => "Claude Code, Codex, Droid, OpenCode, Zed",
            Self::CreateLicense => "MIT",
            Self::CreateWhenToUse => "Use when the user asks to...",
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
                let path = PathBuf::from(path);
                validate_path(&path).map_err(|error| error.to_string())?;
                Ok(InstallTarget::Custom(path))
            }
        }
    }
}
