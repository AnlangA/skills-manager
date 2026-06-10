use std::{collections::BTreeMap, fmt, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Shared domain data types for skills, resources, and target platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillScope {
    /// Skills owned by the active project (`.agents/skills` in the current repo).
    Project,
    /// User-global skill scope (`~/.agents/skills`).
    #[serde(alias = "User")]
    Global,
    /// Claude Code scope (`~/.claude/skills`).
    ClaudeCode,
    /// Droid scope (`~/.factory/skills`).
    Droid,
    /// OpenCode scope (`~/.config/opencode/skills`).
    #[serde(alias = "Pencode")]
    OpenCode,
    /// Codex scope (`~/.codex/skills`).
    Codex,
    /// Zed scope (`~/.config/zed/skills`).
    Zed,
    /// User-defined scope configured in `custom_install_roots`.
    Custom,
}

impl SkillScope {
    /// Ordered list of built-in install targets.
    pub const INSTALL_TARGETS: [Self; 7] = [
        Self::Global,
        Self::ClaudeCode,
        Self::Droid,
        Self::OpenCode,
        Self::Codex,
        Self::Zed,
        Self::Project,
    ];

    /// Returns a stable human label for logs and reports.
    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Global => "Global",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom => "Custom",
        }
    }

    /// Returns a stable machine-readable identifier prefix for snapshots and IDs.
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
            Self::ClaudeCode => "claude-code",
            Self::Droid => "droid",
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::Zed => "zed",
            Self::Custom => "custom",
        }
    }

    /// Relative precedence used when ranking scopes.
    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Project => 0,
            Self::Global => 1,
            Self::ClaudeCode => 2,
            Self::Droid => 3,
            Self::OpenCode => 4,
            Self::Codex => 5,
            Self::Zed => 6,
            Self::Custom => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillEnablement {
    /// Skill is enabled.
    Enabled,
    /// Skill is disabled.
    Disabled,
}

impl SkillEnablement {
    /// Returns whether this setting allows runtime discovery.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Lowercase, UI-friendly label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillHealth {
    /// No blocking issues.
    Valid,
    /// Non-blocking warnings remain.
    Warning,
    /// Not usable and usually filtered from export paths.
    Invalid,
    /// Present but shadowed by another higher priority skill.
    Shadowed,
}

impl SkillHealth {
    /// Lowercase, user-facing label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Warning => "warning",
            Self::Invalid => "invalid",
            Self::Shadowed => "shadowed",
        }
    }

    /// Returns `true` for states considered usable by CLI/UI exports.
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Valid | Self::Warning)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Non-blocking warning.
    Warning,
    /// Blocking issue.
    Invalid,
}

impl DiagnosticSeverity {
    /// Lowercase user-facing severity label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Invalid => "invalid",
        }
    }
}

/// Single diagnostic item emitted by validation and scan workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDiagnostic {
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
}

impl SkillDiagnostic {
    /// Builds a warning diagnostic from a message.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
        }
    }

    /// Builds an invalid diagnostic from a message.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Invalid,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Parsed SKILL.md frontmatter fields and metadata.
pub struct SkillFrontmatter {
    /// Optional display name.
    pub name: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Optional license.
    pub license: Option<String>,
    /// Optional compatibility note.
    pub compatibility: Option<String>,
    /// Declared allowed tools list (`allowed-tools`).
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Vec<String>,
    /// Skill tags.
    pub tags: Vec<String>,
    /// `disable-model-invocation` toggle.
    #[serde(rename = "disable-model-invocation")]
    pub disable_model_invocation: Option<bool>,
    /// Optional usage guidance.
    pub when_to_use: Option<String>,
    /// Arbitrary key-value metadata passthrough.
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Fully materialized skill record used by snapshots, validation, and exports.
pub struct InstalledSkill {
    /// Identifier built from scope and install path.
    pub id: String,
    /// Human-friendly display name.
    pub display_name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Scope in which this skill is installed.
    pub scope: SkillScope,
    /// Skill root folder.
    pub root_dir: PathBuf,
    /// Location of `SKILL.md`.
    pub skill_file: PathBuf,
    /// Parsed frontmatter data.
    pub frontmatter: SkillFrontmatter,
    /// Current enablement state.
    pub enablement: SkillEnablement,
    /// Current health state.
    pub health: SkillHealth,
    /// Validation and resolution diagnostics.
    pub diagnostics: Vec<SkillDiagnostic>,
    /// Number of non-manifest resource files in the skill.
    pub resource_count: usize,
    /// Total bytes of non-manifest resources.
    pub resource_bytes: u64,
    /// Path of the shadowing candidate, if any.
    pub shadowed_by: Option<PathBuf>,
    /// Source URL recorded when installed by remote source.
    pub source_url: Option<String>,
    /// Timestamp from config metadata.
    pub installed_at: Option<DateTime<Utc>>,
}

impl InstalledSkill {
    /// Returns the filesystem folder name used as display fallback.
    pub fn destination_name(&self) -> String {
        self.root_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.display_name)
            .to_string()
    }

    /// Returns whether the skill is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enablement.is_enabled()
    }

    /// Returns whether this skill is exportable in catalog output.
    pub fn is_exportable(&self) -> bool {
        self.is_enabled() && self.health.is_usable()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Kind of resource exposed in inventories and scan output.
pub enum ResourceKind {
    /// Traditional skill folders.
    Skill,
    /// Plugin bundle resource.
    Plugin,
    /// MCP server configuration entry.
    McpServer,
    /// Catalog source descriptors.
    Marketplace,
}

impl ResourceKind {
    /// Lowercase display label for UI rendering and CSV output.
    pub fn label(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Plugin => "plugin",
            Self::McpServer => "mcp-server",
            Self::Marketplace => "marketplace",
        }
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Skill => "Skills",
            Self::Plugin => "Plugins",
            Self::McpServer => "MCP servers",
            Self::Marketplace => "Marketplaces",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Supported targets for platform-specific resources.
pub enum AgentToolTarget {
    /// Generic resource target with no platform-specific manifest.
    Generic,
    /// Codex target.
    Codex,
    /// Claude Code target.
    ClaudeCode,
    /// Factory Droid / Droid target.
    Droid,
    /// OpenCode target.
    OpenCode,
    /// Zed target.
    Zed,
}

impl AgentToolTarget {
    /// Human-friendly label used in output and logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Generic => "Generic",
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::OpenCode => "OpenCode",
            Self::Zed => "Zed",
        }
    }

    /// Stable prefix used for IDs and path keys.
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
            Self::Droid => "droid",
            Self::OpenCode => "opencode",
            Self::Zed => "zed",
        }
    }
}

impl fmt::Display for AgentToolTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Health for non-skill resources.
pub enum ResourceHealth {
    /// Resource has no health issues.
    Valid,
    /// Resource has warnings.
    Warning,
    /// Resource has blocking issues.
    Invalid,
}

impl ResourceHealth {
    /// Human-readable lower-case health label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Warning => "warning",
            Self::Invalid => "invalid",
        }
    }

    /// Derives aggregate health from diagnostics.
    pub fn from_diagnostics(diagnostics: &[SkillDiagnostic]) -> Self {
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Invalid)
        {
            Self::Invalid
        } else if diagnostics.is_empty() {
            Self::Valid
        } else {
            Self::Warning
        }
    }
}

/// UI/CLI-facing representation for a scan-managed item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedResource {
    /// Stable resource identifier.
    pub id: String,
    /// Resource type.
    pub kind: ResourceKind,
    /// Host target (Codex/Claude Code/Generic).
    pub target: AgentToolTarget,
    /// Display name.
    pub display_name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Canonical folder or file root.
    pub root_dir: PathBuf,
    /// Optional manifest path.
    pub manifest_file: Option<PathBuf>,
    /// Current enablement state.
    pub enablement: SkillEnablement,
    /// Aggregate resource health state.
    pub health: ResourceHealth,
    /// Diagnostics for this resource.
    pub diagnostics: Vec<SkillDiagnostic>,
    /// Source URL when discovered from remote metadata.
    pub source_url: Option<String>,
    /// Installation timestamp.
    pub installed_at: Option<DateTime<Utc>>,
    /// Additional metadata key-value pairs.
    pub metadata: BTreeMap<String, String>,
}
