use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

use crate::{Result, SkillsManagerError};

/// Wrapper around `~/.agents`-style roots for a concrete project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    path: PathBuf,
}

impl ProjectRoot {
    /// Creates a project root from any path-like value.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the underlying project path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns `.agents/skills` inside this project.
    pub fn skills_dir(&self) -> PathBuf {
        self.path.join(".agents").join("skills")
    }

    /// Returns `.agents/plugins/marketplace.json` inside this project.
    pub fn codex_marketplace_file(&self) -> PathBuf {
        self.path
            .join(".agents")
            .join("plugins")
            .join("marketplace.json")
    }

    /// Returns `.claude-plugin/marketplace.json` inside this project.
    pub fn claude_marketplace_file(&self) -> PathBuf {
        self.path.join(".claude-plugin").join("marketplace.json")
    }
}

/// Global manager filesystem path resolver for all scopes.
#[derive(Debug, Clone)]
pub struct ManagerPaths {
    home_dir: PathBuf,
    data_dir: PathBuf,
    config_dir: PathBuf,
    project: Option<ProjectRoot>,
}

impl ManagerPaths {
    /// Builds paths using platform defaults and an optional project root.
    pub fn new(project: Option<ProjectRoot>) -> Result<Self> {
        let base = BaseDirs::new().ok_or(SkillsManagerError::HomeDirectoryMissing)?;
        let project_dirs = ProjectDirs::from("dev", "skills-manager", "Skills Manager")
            .ok_or(SkillsManagerError::HomeDirectoryMissing)?;

        Ok(Self {
            home_dir: base.home_dir().to_path_buf(),
            data_dir: project_dirs.data_dir().to_path_buf(),
            config_dir: project_dirs.config_dir().to_path_buf(),
            project,
        })
    }

    /// Builds paths with explicitly provided base directories (tests and adapters).
    pub fn with_home(
        home_dir: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
        config_dir: impl Into<PathBuf>,
        project: Option<ProjectRoot>,
    ) -> Self {
        Self {
            home_dir: home_dir.into(),
            data_dir: data_dir.into(),
            config_dir: config_dir.into(),
            project,
        }
    }

    /// Returns resolved home directory.
    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    /// Returns config directory root.
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Returns app config file path (`config.toml`).
    pub fn app_config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Returns default download cache directory.
    pub fn downloads_dir(&self) -> PathBuf {
        self.data_dir.join("downloads")
    }

    /// Backward-compatible alias for global scope path.
    pub fn user_skills_dir(&self) -> PathBuf {
        self.global_skills_dir()
    }

    /// Returns global skills root (`~/.agents/skills`).
    pub fn global_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".agents").join("skills")
    }

    /// Returns Claude Code skills root (`~/.claude/skills`).
    pub fn claude_code_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".claude").join("skills")
    }

    /// Returns Droid skills root (`~/.droid/skills`).
    pub fn droid_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".droid").join("skills")
    }

    /// Returns OpenCode global skills root (`~/.config/opencode/skills`).
    pub fn opencode_skills_dir(&self) -> PathBuf {
        self.home_dir
            .join(".config")
            .join("opencode")
            .join("skills")
    }

    /// Returns Codex skills root (`~/.codex/skills`).
    pub fn codex_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".codex").join("skills")
    }

    /// Returns Zed skills root (`~/.config/zed/skills`).
    pub fn zed_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".config").join("zed").join("skills")
    }

    /// Returns Codex config file path (`~/.codex/config.toml`).
    pub fn codex_config_file(&self) -> PathBuf {
        self.home_dir.join(".codex").join("config.toml")
    }

    /// Returns Codex plugin installation directory.
    pub fn codex_plugins_dir(&self) -> PathBuf {
        self.home_dir.join(".codex").join("plugins")
    }

    pub fn codex_plugin_cache_dir(&self) -> PathBuf {
        self.codex_plugins_dir().join("cache")
    }

    pub fn personal_codex_marketplace_file(&self) -> PathBuf {
        self.home_dir
            .join(".agents")
            .join("plugins")
            .join("marketplace.json")
    }

    /// Returns Claude plugins root (`~/.claude/plugins`).
    pub fn claude_plugins_dir(&self) -> PathBuf {
        self.home_dir.join(".claude").join("plugins")
    }

    /// Returns Claude plugin cache root.
    pub fn claude_plugin_cache_dir(&self) -> PathBuf {
        self.claude_plugins_dir().join("cache")
    }

    /// Returns project-scoped Claude marketplace file if available.
    pub fn personal_claude_marketplace_file(&self) -> PathBuf {
        self.home_dir
            .join(".claude-plugin")
            .join("marketplace.json")
    }

    /// Returns optional project root wrapper.
    pub fn project(&self) -> Option<&ProjectRoot> {
        self.project.as_ref()
    }

    /// Returns optional project skills root.
    pub fn project_skills_dir(&self) -> Option<PathBuf> {
        self.project.as_ref().map(ProjectRoot::skills_dir)
    }

    pub fn project_codex_marketplace_file(&self) -> Option<PathBuf> {
        self.project
            .as_ref()
            .map(ProjectRoot::codex_marketplace_file)
    }

    pub fn project_claude_marketplace_file(&self) -> Option<PathBuf> {
        self.project
            .as_ref()
            .map(ProjectRoot::claude_marketplace_file)
    }

    pub fn skills_dir_for_scope(&self, scope: crate::SkillScope) -> Option<PathBuf> {
        match scope {
            crate::SkillScope::Project => self.project_skills_dir(),
            crate::SkillScope::Global => Some(self.global_skills_dir()),
            crate::SkillScope::ClaudeCode => Some(self.claude_code_skills_dir()),
            crate::SkillScope::Droid => Some(self.droid_skills_dir()),
            crate::SkillScope::OpenCode => Some(self.opencode_skills_dir()),
            crate::SkillScope::Codex => Some(self.codex_skills_dir()),
            crate::SkillScope::Zed => Some(self.zed_skills_dir()),
            crate::SkillScope::Custom => None,
        }
    }

    pub fn skill_roots(&self) -> Vec<(crate::SkillScope, PathBuf)> {
        let mut roots = Vec::new();
        if let Some(project_skills_dir) = self.project_skills_dir() {
            roots.push((crate::SkillScope::Project, project_skills_dir));
        }
        for scope in crate::SkillScope::INSTALL_TARGETS {
            if scope == crate::SkillScope::Project {
                continue;
            }
            if let Some(root) = self.skills_dir_for_scope(scope) {
                roots.push((scope, root));
            }
        }
        roots
    }
}
