use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use walkdir::WalkDir;

use crate::{
    ManagerConfig, ManagerPaths, Result, SkillDiagnostic, SkillFrontmatter, SkillHealth,
    SkillScope, SkillsManagerError, discover_skill_candidates,
    skill::{SkillCandidate, path_key, sanitize_folder_name, unique_folder_name},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    Block,
    Replace,
    Rename,
}

#[derive(Debug, Clone)]
pub struct InstallRequest {
    pub source_root: PathBuf,
    pub source_url: Option<String>,
    pub scope: SkillScope,
    pub conflict_policy: ConflictPolicy,
}

#[derive(Debug, Clone)]
pub struct InstallCandidate {
    pub source_root: PathBuf,
    pub destination_root: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub diagnostics: Vec<SkillDiagnostic>,
    pub health: SkillHealth,
    pub resource_count: usize,
    pub resource_bytes: u64,
    pub conflict: bool,
}

#[derive(Debug, Clone)]
pub struct InstallPreview {
    pub scope: SkillScope,
    pub candidates: Vec<InstallCandidate>,
}

#[derive(Debug, Clone)]
pub struct InstallResult {
    pub installed: Vec<PathBuf>,
    pub backups: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Installer {
    paths: ManagerPaths,
}

impl Installer {
    pub fn new(paths: ManagerPaths) -> Self {
        Self { paths }
    }

    pub fn preview(
        &self,
        source_root: &Path,
        scope: SkillScope,
        conflict_policy: ConflictPolicy,
    ) -> Result<InstallPreview> {
        let destination_root = self.destination_for_scope(scope)?;
        let mut claimed = HashMap::new();
        let mut candidates = Vec::new();

        for candidate in discover_skill_candidates(source_root)? {
            let preferred_name = preferred_folder_name(&candidate);
            let folder_name = unique_claim_name(&preferred_name, &mut claimed);
            let planned_destination = destination_root.join(folder_name);
            let conflict = planned_destination.exists();
            let destination = if conflict && conflict_policy == ConflictPolicy::Rename {
                let folder_name = unique_folder_name(&destination_root, &preferred_name, &claimed);
                destination_root.join(folder_name)
            } else {
                planned_destination
            };

            candidates.push(InstallCandidate {
                source_root: candidate.root_dir,
                destination_root: destination,
                frontmatter: candidate.frontmatter,
                diagnostics: candidate.diagnostics,
                health: candidate.health,
                resource_count: candidate.resource_count,
                resource_bytes: candidate.resource_bytes,
                conflict,
            });
        }

        if candidates.is_empty() {
            return Err(SkillsManagerError::NoSkillsFound);
        }

        Ok(InstallPreview { scope, candidates })
    }

    pub fn install(&self, request: InstallRequest) -> Result<InstallResult> {
        let destination_root = self.destination_for_scope(request.scope)?;
        fs::create_dir_all(&destination_root)?;

        let candidates = discover_skill_candidates(&request.source_root)?;
        if candidates.is_empty() {
            return Err(SkillsManagerError::NoSkillsFound);
        }

        let mut config = ManagerConfig::load(&self.paths)?;
        let mut installed = Vec::new();
        let mut backups = Vec::new();
        let mut claimed = HashMap::new();

        for candidate in candidates {
            let preferred_name = preferred_folder_name(&candidate);
            let destination =
                destination_root.join(unique_claim_name(&preferred_name, &mut claimed));

            if destination.exists() {
                match request.conflict_policy {
                    ConflictPolicy::Block => {
                        return Err(SkillsManagerError::DestinationExists(destination));
                    }
                    ConflictPolicy::Rename => {}
                    ConflictPolicy::Replace => {
                        let backup = backup_path(&destination);
                        fs::rename(&destination, &backup)?;
                        backups.push(backup);
                    }
                }
            }

            let final_destination = if request.conflict_policy == ConflictPolicy::Rename {
                let folder_name = unique_folder_name(&destination_root, &preferred_name, &claimed);
                destination_root.join(folder_name)
            } else {
                destination
            };

            copy_skill_folder(&candidate.root_dir, &final_destination)?;
            config.record_install(&final_destination, request.source_url.clone());
            installed.push(final_destination);
        }

        config.save(&self.paths)?;
        Ok(InstallResult { installed, backups })
    }

    pub fn remove(&self, skill_root: &Path) -> Result<PathBuf> {
        let skill_file = skill_root.join("SKILL.md");
        if !skill_file.exists() {
            return Err(SkillsManagerError::MissingSkillFile(
                skill_root.to_path_buf(),
            ));
        }

        let backup = backup_path(skill_root);
        fs::rename(skill_root, &backup)?;

        let mut config = ManagerConfig::load(&self.paths)?;
        config.forget_install(skill_root, &skill_file);
        config.save(&self.paths)?;

        Ok(backup)
    }

    pub fn set_disabled(&self, skill_file: &Path, disabled: bool) -> Result<()> {
        let mut config = ManagerConfig::load(&self.paths)?;
        config.set_disabled(skill_file, disabled);
        config.save(&self.paths)?;

        Ok(())
    }

    fn destination_for_scope(&self, scope: SkillScope) -> Result<PathBuf> {
        match scope {
            SkillScope::User => Ok(self.paths.user_skills_dir()),
            SkillScope::Project => self
                .paths
                .project_skills_dir()
                .ok_or_else(|| SkillsManagerError::UnknownSkillScope(PathBuf::from("project"))),
        }
    }
}

fn copy_skill_folder(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::create_dir_all(destination)?;

    for entry in WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry
            .path()
            .strip_prefix(source)
            .expect("walkdir entry starts with source");
        if relative.as_os_str().is_empty() {
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }

    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill");
    path.with_file_name(format!("{file_name}.backup-{timestamp}"))
}

fn preferred_folder_name(candidate: &SkillCandidate) -> String {
    let preferred_name = candidate
        .frontmatter
        .name
        .as_deref()
        .or_else(|| {
            candidate
                .root_dir
                .file_name()
                .and_then(|name| name.to_str())
        })
        .unwrap_or("skill");
    let sanitized = sanitize_folder_name(preferred_name);

    if sanitized.is_empty() {
        "skill".to_string()
    } else {
        sanitized
    }
}

fn unique_claim_name(name: &str, claimed: &mut HashMap<String, ()>) -> String {
    if !claimed.contains_key(name) {
        claimed.insert(name.to_string(), ());
        return name.to_string();
    }

    for index in 2.. {
        let candidate = format!("{name}-{index}");
        if !claimed.contains_key(&candidate) {
            claimed.insert(candidate.clone(), ());
            return candidate;
        }
    }

    unreachable!("the loop always returns")
}

pub fn scope_for_skill(paths: &ManagerPaths, skill_file: &Path) -> Result<SkillScope> {
    let skill_file = skill_file
        .canonicalize()
        .unwrap_or_else(|_| skill_file.to_path_buf());
    for (scope, root) in paths.skill_roots() {
        let root = root.canonicalize().unwrap_or(root);
        if skill_file.starts_with(&root) {
            return Ok(scope);
        }
    }

    Err(SkillsManagerError::UnknownSkillScope(skill_file))
}

pub fn installed_key(path: &Path) -> String {
    path_key(path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::ProjectRoot;

    use super::*;

    #[test]
    fn installs_skill_to_project_scope() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source/demo");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: demo\ndescription: Demo skill\n---\n",
        )
        .unwrap();

        let project = dir.path().join("project");
        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(&project)),
        );

        let installer = Installer::new(paths);
        let result = installer
            .install(InstallRequest {
                source_root: dir.path().join("source"),
                source_url: Some("fixture".to_string()),
                scope: SkillScope::Project,
                conflict_policy: ConflictPolicy::Block,
            })
            .unwrap();

        assert_eq!(result.installed.len(), 1);
        assert!(project.join(".agents/skills/demo/SKILL.md").exists());
    }

    #[test]
    fn previews_conflict_and_diagnostics() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source/demo");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: demo\ndescription: Demo skill\n---\n",
        )
        .unwrap();
        let existing = dir.path().join("home/.agents/skills/demo");
        fs::create_dir_all(&existing).unwrap();
        fs::write(
            existing.join("SKILL.md"),
            "---\nname: demo\ndescription: Existing skill\n---\n",
        )
        .unwrap();

        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            None,
        );
        let installer = Installer::new(paths);
        let preview = installer
            .preview(
                &dir.path().join("source"),
                SkillScope::User,
                ConflictPolicy::Rename,
            )
            .unwrap();

        assert_eq!(preview.candidates.len(), 1);
        assert!(preview.candidates[0].conflict);
        assert!(preview.candidates[0].destination_root.ends_with("demo-2"));
        assert!(!preview.candidates[0].diagnostics.is_empty());
    }
}
