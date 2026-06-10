use std::{
    env,
    path::{Component, Path, PathBuf},
};

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

    /// Returns project-scoped MCP server configuration.
    pub fn mcp_config_file(&self) -> PathBuf {
        self.path.join(".mcp.json")
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
        let home_dir = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| base.home_dir().to_path_buf());
        let home_override = env::var_os("HOME").is_some();
        let (data_dir, config_dir) = if cfg!(windows) && home_override {
            let data_dir = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home_dir.join(".local").join("share"));
            let config_dir = env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home_dir.join(".config"));
            (
                data_dir.join("skills-manager"),
                config_dir.join("skills-manager"),
            )
        } else {
            (
                project_dirs.data_dir().to_path_buf(),
                project_dirs.config_dir().to_path_buf(),
            )
        };

        Ok(Self {
            home_dir: home_dir.clone(),
            data_dir,
            config_dir,
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

    /// Returns Droid skills root (`~/.factory/skills`).
    pub fn droid_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".factory").join("skills")
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

    /// Returns Codex plugin cache directory (`~/.codex/plugins/cache`).
    pub fn codex_plugin_cache_dir(&self) -> PathBuf {
        self.codex_plugins_dir().join("cache")
    }

    /// Returns personal Codex marketplace file (`~/.agents/plugins/marketplace.json`).
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

    /// Returns Claude Code user config file (`~/.claude.json`).
    pub fn claude_config_file(&self) -> PathBuf {
        self.home_dir.join(".claude.json")
    }

    /// Returns Claude plugin cache root.
    pub fn claude_plugin_cache_dir(&self) -> PathBuf {
        self.claude_plugins_dir().join("cache")
    }

    /// Returns Droid MCP server configuration (`~/.factory/mcp.json`).
    pub fn droid_mcp_config_file(&self) -> PathBuf {
        self.home_dir.join(".factory").join("mcp.json")
    }

    /// Returns OpenCode JSON configuration (`~/.config/opencode/opencode.json`).
    pub fn opencode_config_file(&self) -> PathBuf {
        self.home_dir
            .join(".config")
            .join("opencode")
            .join("opencode.json")
    }

    /// Returns OpenCode JSONC configuration (`~/.config/opencode/opencode.jsonc`).
    pub fn opencode_jsonc_config_file(&self) -> PathBuf {
        self.home_dir
            .join(".config")
            .join("opencode")
            .join("opencode.jsonc")
    }

    /// Returns Zed settings configuration (`~/.config/zed/settings.json`).
    pub fn zed_settings_file(&self) -> PathBuf {
        self.home_dir
            .join(".config")
            .join("zed")
            .join("settings.json")
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

    /// Returns the project-scoped Codex marketplace file, if a project root is set.
    pub fn project_codex_marketplace_file(&self) -> Option<PathBuf> {
        self.project
            .as_ref()
            .map(ProjectRoot::codex_marketplace_file)
    }

    /// Returns the project-scoped Claude marketplace file, if a project root is set.
    pub fn project_claude_marketplace_file(&self) -> Option<PathBuf> {
        self.project
            .as_ref()
            .map(ProjectRoot::claude_marketplace_file)
    }

    /// Returns the project-scoped MCP configuration file, if a project root is set.
    pub fn project_mcp_config_file(&self) -> Option<PathBuf> {
        self.project.as_ref().map(ProjectRoot::mcp_config_file)
    }

    /// Returns the skills directory for a given scope, if resolvable.
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

    /// Returns all known skill roots with their associated scopes.
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

/// Validate a filesystem path for common platform constraints.
pub fn validate_path(path: &Path) -> crate::Result<()> {
    if path.as_os_str().is_empty() {
        return Err(crate::SkillsManagerError::InvalidPath(
            "path is empty".to_string(),
        ));
    }

    if path.to_string_lossy().contains('\0') {
        return Err(crate::SkillsManagerError::InvalidPath(
            "path contains NUL character".to_string(),
        ));
    }

    #[cfg(windows)]
    {
        for component in path.components() {
            if let Component::Normal(name) = component {
                validate_windows_path_component(name)?;
            }
        }
    }

    Ok(())
}

#[cfg(windows)]
fn validate_windows_path_component(name: &std::ffi::OsStr) -> crate::Result<()> {
    let name = name.to_string_lossy();
    if name.is_empty() {
        return Ok(());
    }
    if name
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '\"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err(crate::SkillsManagerError::InvalidPath(format!(
            "path component {name:?} contains invalid Windows characters"
        )));
    }

    if name.chars().any(|ch| ch.is_control()) {
        return Err(crate::SkillsManagerError::InvalidPath(format!(
            "path component {name:?} contains invalid control characters"
        )));
    }

    let name = name.trim_end_matches([' ', '.']);
    if name.is_empty() {
        return Err(crate::SkillsManagerError::InvalidPath(format!(
            "path component {name:?} is not allowed on Windows"
        )));
    }

    let stem = name.split('.').next().unwrap_or("");
    if matches_windows_reserved_name(stem) {
        return Err(crate::SkillsManagerError::InvalidPath(format!(
            "path component {name:?} is a reserved Windows device name"
        )));
    }

    Ok(())
}

#[cfg(windows)]
fn matches_windows_reserved_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    )
}

/// Normalize a path into a stable config-key form.
pub fn normalize_path_key(path: &Path) -> String {
    let mut segments = Vec::new();
    let mut prefix = String::new();

    for component in path.components() {
        match component {
            Component::Prefix(path_prefix) => {
                prefix = path_prefix
                    .as_os_str()
                    .to_string_lossy()
                    .replace('\\', "/")
                    .to_ascii_lowercase();
            }
            Component::RootDir => {
                if !path.is_absolute() {
                    continue;
                }
            }
            Component::CurDir => {
                // keep relative `.` for strict identity preservation in non-windows filesystems
            }
            Component::ParentDir => {
                segments.push("..".to_string());
            }
            Component::Normal(name) => {
                let segment = name.to_string_lossy().to_string();
                if segment.is_empty() {
                    continue;
                }
                segments.push(segment);
            }
        }
    }

    let mut key = String::new();
    if !prefix.is_empty() {
        key.push_str(&prefix);
        if !segments.is_empty() {
            key.push('/');
        }
    } else if path.is_absolute() {
        key.push('/');
    }

    for (index, segment) in segments.iter().enumerate() {
        if index > 0 || (!key.is_empty() && !key.ends_with('/')) {
            key.push('/');
        }
        if cfg!(windows) {
            key.push_str(&segment.to_ascii_lowercase());
        } else {
            key.push_str(segment);
        }
    }

    if key.is_empty() { ".".to_string() } else { key }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn normalize_path_key_preserves_forward_paths() {
        assert_eq!(
            normalize_path_key(Path::new("/tmp/skill-manager")),
            "/tmp/skill-manager"
        );
        assert_eq!(normalize_path_key(Path::new("foo")), "foo");
        assert_eq!(normalize_path_key(Path::new("./foo")), "foo");
        assert_eq!(normalize_path_key(Path::new("../foo")), "../foo");
    }

    #[cfg(not(windows))]
    #[test]
    fn validate_path_rejects_invalid_path_inputs() {
        assert!(validate_path(Path::new("")).is_err());
        assert!(validate_path(Path::new("abc\0xyz")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn validate_path_rejects_windows_invalid_components() {
        assert!(validate_path(Path::new("bad<name")).is_err());
        assert!(validate_path(Path::new("COM1\\skill")).is_err());
        assert!(validate_path(Path::new("good\\skill")).is_ok());
    }
}
