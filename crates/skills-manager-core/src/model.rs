use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillScope {
    Project,
    #[serde(alias = "User")]
    Global,
    ClaudeCode,
    Droid,
    Pencode,
    Codex,
    Zed,
    Custom,
}

impl SkillScope {
    pub const INSTALL_TARGETS: [Self; 7] = [
        Self::Global,
        Self::ClaudeCode,
        Self::Droid,
        Self::Pencode,
        Self::Codex,
        Self::Zed,
        Self::Project,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Global => "Global",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::Pencode => "Pencode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom => "Custom",
        }
    }

    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
            Self::ClaudeCode => "claude-code",
            Self::Droid => "droid",
            Self::Pencode => "pencode",
            Self::Codex => "codex",
            Self::Zed => "zed",
            Self::Custom => "custom",
        }
    }

    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Project => 0,
            Self::Global => 1,
            Self::ClaudeCode => 2,
            Self::Droid => 3,
            Self::Pencode => 4,
            Self::Codex => 5,
            Self::Zed => 6,
            Self::Custom => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillEnablement {
    Enabled,
    Disabled,
}

impl SkillEnablement {
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillHealth {
    Valid,
    Warning,
    Invalid,
    Shadowed,
}

impl SkillHealth {
    pub fn label(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Warning => "warning",
            Self::Invalid => "invalid",
            Self::Shadowed => "shadowed",
        }
    }

    pub fn is_usable(self) -> bool {
        matches!(self, Self::Valid | Self::Warning)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Warning,
    Invalid,
}

impl DiagnosticSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
}

impl SkillDiagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Invalid,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Vec<String>,
    pub tags: Vec<String>,
    #[serde(rename = "disable-model-invocation")]
    pub disable_model_invocation: Option<bool>,
    pub when_to_use: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub scope: SkillScope,
    pub root_dir: PathBuf,
    pub skill_file: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub enablement: SkillEnablement,
    pub health: SkillHealth,
    pub diagnostics: Vec<SkillDiagnostic>,
    pub resource_count: usize,
    pub resource_bytes: u64,
    pub shadowed_by: Option<PathBuf>,
    pub source_url: Option<String>,
    pub installed_at: Option<DateTime<Utc>>,
}

impl InstalledSkill {
    pub fn destination_name(&self) -> String {
        self.root_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.display_name)
            .to_string()
    }

    pub fn is_enabled(&self) -> bool {
        self.enablement.is_enabled()
    }

    pub fn is_exportable(&self) -> bool {
        self.is_enabled() && self.health.is_usable()
    }
}
