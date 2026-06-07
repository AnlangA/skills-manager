use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

use crate::{Result, SkillsManagerError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    path: PathBuf,
}

impl ProjectRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.path.join(".agents").join("skills")
    }
}

#[derive(Debug, Clone)]
pub struct ManagerPaths {
    home_dir: PathBuf,
    data_dir: PathBuf,
    config_dir: PathBuf,
    project: Option<ProjectRoot>,
}

impl ManagerPaths {
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

    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn app_config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.data_dir.join("downloads")
    }

    pub fn user_skills_dir(&self) -> PathBuf {
        self.global_skills_dir()
    }

    pub fn global_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".agents").join("skills")
    }

    pub fn claude_code_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".claude").join("skills")
    }

    pub fn droid_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".droid").join("skills")
    }

    pub fn pencode_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".pencode").join("skills")
    }

    pub fn codex_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".codex").join("skills")
    }

    pub fn zed_skills_dir(&self) -> PathBuf {
        self.home_dir.join(".config").join("zed").join("skills")
    }

    pub fn codex_config_file(&self) -> PathBuf {
        self.home_dir.join(".codex").join("config.toml")
    }

    pub fn project(&self) -> Option<&ProjectRoot> {
        self.project.as_ref()
    }

    pub fn project_skills_dir(&self) -> Option<PathBuf> {
        self.project.as_ref().map(ProjectRoot::skills_dir)
    }

    pub fn skills_dir_for_scope(&self, scope: crate::SkillScope) -> Option<PathBuf> {
        match scope {
            crate::SkillScope::Project => self.project_skills_dir(),
            crate::SkillScope::Global => Some(self.global_skills_dir()),
            crate::SkillScope::ClaudeCode => Some(self.claude_code_skills_dir()),
            crate::SkillScope::Droid => Some(self.droid_skills_dir()),
            crate::SkillScope::Pencode => Some(self.pencode_skills_dir()),
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
