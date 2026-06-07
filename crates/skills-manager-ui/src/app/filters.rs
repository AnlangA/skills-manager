use std::fmt;

use skills_manager_core::{AgentToolTarget, InstalledSkill, ResourceKind, SkillHealth, SkillScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKindFilter {
    Skills,
    Plugins,
    Marketplaces,
    All,
}

impl ResourceKindFilter {
    pub const ALL: [Self; 4] = [Self::Skills, Self::Plugins, Self::Marketplaces, Self::All];

    pub fn matches(self, kind: ResourceKind) -> bool {
        match self {
            Self::Skills => kind == ResourceKind::Skill,
            Self::Plugins => kind == ResourceKind::Plugin,
            Self::Marketplaces => kind == ResourceKind::Marketplace,
            Self::All => true,
        }
    }
}

impl fmt::Display for ResourceKindFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Skills => "Skills",
            Self::Plugins => "Plugins",
            Self::Marketplaces => "Marketplaces",
            Self::All => "All resources",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeFilter {
    All,
    Project,
    Global,
    ClaudeCode,
    Droid,
    OpenCode,
    Codex,
    Zed,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthFilter {
    All,
    NeedsAttention,
    Valid,
    Warning,
    Invalid,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFilter {
    All,
    Managed,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginTargetFilter {
    All,
    Codex,
    ClaudeCode,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Priority,
    Name,
    Health,
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
