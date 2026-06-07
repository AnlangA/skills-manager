//! Inventory filter and sort enums for skill and resource lists.
//!
//! Provides scope, health, source, resource kind, plugin target, and
//! sort key selectors used by the inventory, plugin, and marketplace views.

use std::fmt;

use skills_manager_core::{AgentToolTarget, InstalledSkill, SkillHealth, SkillScope};

/// Scope filter for the inventory list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeFilter {
    /// Show all scopes.
    All,
    /// Show only project-scoped skills.
    Project,
    /// Show only global-scoped skills.
    Global,
    /// Show only Claude Code skills.
    ClaudeCode,
    /// Show only Droid skills.
    Droid,
    /// Show only OpenCode skills.
    OpenCode,
    /// Show only Codex skills.
    Codex,
    /// Show only Zed skills.
    Zed,
    /// Show only custom-root skills.
    Custom,
}

impl ScopeFilter {
    pub const ALL: [Self; 9] = [
        Self::All,
        Self::Project,
        Self::Global,
        Self::ClaudeCode,
        Self::Droid,
        Self::OpenCode,
        Self::Codex,
        Self::Zed,
        Self::Custom,
    ];

    pub fn matches(self, scope: SkillScope) -> bool {
        match self {
            Self::All => true,
            Self::Project => scope == SkillScope::Project,
            Self::Global => scope == SkillScope::Global,
            Self::ClaudeCode => scope == SkillScope::ClaudeCode,
            Self::Droid => scope == SkillScope::Droid,
            Self::OpenCode => scope == SkillScope::OpenCode,
            Self::Codex => scope == SkillScope::Codex,
            Self::Zed => scope == SkillScope::Zed,
            Self::Custom => scope == SkillScope::Custom,
        }
    }
}

impl fmt::Display for ScopeFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All scopes",
            Self::Project => "Project",
            Self::Global => "Global",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom => "Custom",
        })
    }
}

/// Health state filter for the inventory list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthFilter {
    /// Show all health states.
    All,
    /// Show skills that need attention (warning, invalid, or shadowed).
    NeedsAttention,
    /// Show only valid skills.
    Valid,
    /// Show only warning skills.
    Warning,
    /// Show only invalid skills.
    Invalid,
    /// Show only shadowed skills.
    Shadowed,
}

impl HealthFilter {
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::NeedsAttention,
        Self::Valid,
        Self::Warning,
        Self::Invalid,
        Self::Shadowed,
    ];

    pub fn matches(self, health: SkillHealth) -> bool {
        match self {
            Self::All => true,
            Self::NeedsAttention => matches!(
                health,
                SkillHealth::Warning | SkillHealth::Invalid | SkillHealth::Shadowed
            ),
            Self::Valid => health == SkillHealth::Valid,
            Self::Warning => health == SkillHealth::Warning,
            Self::Invalid => health == SkillHealth::Invalid,
            Self::Shadowed => health == SkillHealth::Shadowed,
        }
    }
}

impl fmt::Display for HealthFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All health",
            Self::NeedsAttention => "Needs attention",
            Self::Valid => "Valid",
            Self::Warning => "Warning",
            Self::Invalid => "Invalid",
            Self::Shadowed => "Shadowed",
        })
    }
}

/// Source provenance filter for the inventory list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFilter {
    /// Show all source types.
    All,
    /// Show only skills with a known source URL.
    Managed,
    /// Show only skills without a tracked source.
    Unknown,
}

impl SourceFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::Managed, Self::Unknown];

    pub fn matches(self, skill: &InstalledSkill) -> bool {
        match self {
            Self::All => true,
            Self::Managed => skill.source_url.is_some(),
            Self::Unknown => skill.source_url.is_none(),
        }
    }
}

impl fmt::Display for SourceFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All sources",
            Self::Managed => "Known source",
            Self::Unknown => "Unknown source",
        })
    }
}

/// Agent tool target filter for the plugins list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginTargetFilter {
    /// Show all targets.
    All,
    /// Show only Codex plugins.
    Codex,
    /// Show only Claude Code plugins.
    ClaudeCode,
    /// Show only generic plugins.
    Generic,
}

impl PluginTargetFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Codex, Self::ClaudeCode, Self::Generic];

    pub fn matches(self, target: AgentToolTarget) -> bool {
        match self {
            Self::All => true,
            Self::Codex => target == AgentToolTarget::Codex,
            Self::ClaudeCode => target == AgentToolTarget::ClaudeCode,
            Self::Generic => target == AgentToolTarget::Generic,
        }
    }
}

impl fmt::Display for PluginTargetFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All targets",
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Generic => "Generic",
        })
    }
}

/// Sort key for the inventory list ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Sort by health priority (invalid first, then warning, then valid).
    Priority,
    /// Sort alphabetically by name.
    Name,
    /// Sort by health status.
    Health,
    /// Sort by resource count.
    Resources,
}

impl fmt::Display for SortKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Priority => "Priority",
            Self::Name => "Name",
            Self::Health => "Health",
            Self::Resources => "Resources",
        })
    }
}
