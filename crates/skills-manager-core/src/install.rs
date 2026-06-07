use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::Serialize;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::{
    ManagerConfig, ManagerPaths, Result, SkillDiagnostic, SkillFrontmatter, SkillHealth,
    SkillScope, SkillsManagerError,
    codex_config::CodexConfig,
    discover_skill_candidates,
    operation::{OperationJournal, OperationPlan, rollback_on_error},
    skill::{
        LEGACY_DISABLED_SKILLS_DIR, SkillCandidate, disabled_store_root_for_skills_root,
        health_from_diagnostics, path_key, sanitize_folder_name, unique_folder_name,
    },
    target_specific_diagnostics,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConflictPolicy {
    Block,
    Replace,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum InstallTarget {
    Global,
    #[allow(dead_code)]
    User,
    Project,
    ClaudeCode,
    Droid,
    Pencode,
    Codex,
    Zed,
    Custom(PathBuf),
}

impl InstallTarget {
    pub fn scope(&self) -> SkillScope {
        match self {
            Self::Global | Self::User => SkillScope::Global,
            Self::Project => SkillScope::Project,
            Self::ClaudeCode => SkillScope::ClaudeCode,
            Self::Droid => SkillScope::Droid,
            Self::Pencode => SkillScope::Pencode,
            Self::Codex => SkillScope::Codex,
            Self::Zed => SkillScope::Zed,
            Self::Custom(_) => SkillScope::Custom,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Global | Self::User => "Global",
            Self::Project => "Project",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::Pencode => "Pencode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom(_) => "Custom",
        }
    }

    pub fn destination_root(&self, paths: &ManagerPaths) -> Result<PathBuf> {
        match self {
            Self::Global | Self::User => Ok(paths.global_skills_dir()),
            Self::Project => paths
                .project_skills_dir()
                .ok_or_else(|| SkillsManagerError::UnknownSkillScope(PathBuf::from("project"))),
            Self::ClaudeCode => Ok(paths.claude_code_skills_dir()),
            Self::Droid => Ok(paths.droid_skills_dir()),
            Self::Pencode => Ok(paths.pencode_skills_dir()),
            Self::Codex => Ok(paths.codex_skills_dir()),
            Self::Zed => Ok(paths.zed_skills_dir()),
            Self::Custom(path) if path.as_os_str().is_empty() => {
                Err(SkillsManagerError::UnknownSkillScope(path.clone()))
            }
            Self::Custom(path) => Ok(path.clone()),
        }
    }
}

impl From<SkillScope> for InstallTarget {
    fn from(value: SkillScope) -> Self {
        match value {
            SkillScope::Project => Self::Project,
            SkillScope::Global => Self::Global,
            SkillScope::ClaudeCode => Self::ClaudeCode,
            SkillScope::Droid => Self::Droid,
            SkillScope::Pencode => Self::Pencode,
            SkillScope::Codex => Self::Codex,
            SkillScope::Zed => Self::Zed,
            SkillScope::Custom => Self::Custom(PathBuf::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallRequest {
    pub source_root: PathBuf,
    pub source_url: Option<String>,
    pub target: InstallTarget,
    pub conflict_policy: ConflictPolicy,
    pub enable_after_install: bool,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct InstallPreview {
    pub scope: SkillScope,
    pub destination_root: PathBuf,
    pub candidates: Vec<InstallCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_plan: Option<OperationPlan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallResult {
    pub installed: Vec<PathBuf>,
    pub backups: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct PlannedDestination {
    path: PathBuf,
    conflict: bool,
}

#[derive(Debug, Clone)]
struct ManagedSkillLocation {
    scope: SkillScope,
    enabled_root: PathBuf,
    disabled_root: PathBuf,
    legacy_disabled_root: PathBuf,
    enabled_skill_file: PathBuf,
    disabled_skill_file: PathBuf,
    legacy_disabled_skill_file: PathBuf,
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
        target: InstallTarget,
        conflict_policy: ConflictPolicy,
    ) -> Result<InstallPreview> {
        let plan = self.plan(InstallRequest {
            source_root: source_root.to_path_buf(),
            source_url: None,
            target,
            conflict_policy,
            enable_after_install: true,
        })?;

        Ok(plan.preview())
    }

    pub fn plan(&self, request: InstallRequest) -> Result<OperationPlan> {
        info!(
            source_root = %request.source_root.display(),
            target = request.target.label(),
            ?request.conflict_policy,
            "building install preview"
        );
        let scope = request.target.scope();
        let destination_root = self.destination_for_target(&request.target)?;
        let mut claimed = HashMap::new();
        let mut candidates = Vec::new();

        for candidate in discover_skill_candidates(&request.source_root)? {
            let preferred_name = preferred_folder_name(&candidate);
            let destination = plan_destination(
                &destination_root,
                &preferred_name,
                &mut claimed,
                request.conflict_policy,
            );
            let mut diagnostics = candidate.diagnostics;
            diagnostics.extend(target_specific_diagnostics(
                scope,
                &destination.path,
                &candidate.frontmatter,
                candidate.resource_count,
                candidate.resource_bytes,
            ));
            if !request.enable_after_install && scope != SkillScope::Codex {
                let disabled_root = disabled_root_for_enabled_root(&destination.path);
                if disabled_root.exists() {
                    diagnostics.push(SkillDiagnostic::invalid(format!(
                        "Disabled destination already exists: {}",
                        disabled_root.display()
                    )));
                }
            }
            let health = health_from_diagnostics(&diagnostics, false);

            candidates.push(InstallCandidate {
                source_root: candidate.root_dir,
                destination_root: destination.path,
                frontmatter: candidate.frontmatter,
                diagnostics,
                health,
                resource_count: candidate.resource_count,
                resource_bytes: candidate.resource_bytes,
                conflict: destination.conflict,
            });
        }

        if candidates.is_empty() {
            warn!(
                source_root = %request.source_root.display(),
                "install preview found no skill candidates"
            );
            return Err(SkillsManagerError::NoSkillsFound);
        }

        info!(
            source_root = %request.source_root.display(),
            destination_root = %destination_root.display(),
            candidates = candidates.len(),
            "built install preview"
        );
        Ok(OperationPlan::new(
            request,
            scope,
            destination_root,
            candidates,
        ))
    }

    pub fn install(&self, request: InstallRequest) -> Result<InstallResult> {
        let plan = self.plan(request)?;
        self.install_plan(plan)
    }

    pub fn install_plan(&self, plan: OperationPlan) -> Result<InstallResult> {
        info!(
            source_root = %plan.request.source_root.display(),
            target = plan.request.target.label(),
            ?plan.request.conflict_policy,
            enable_after_install = plan.request.enable_after_install,
            "installing skills"
        );
        let mut journal = OperationJournal::default();
        let result = self.install_plan_inner(plan, &mut journal);
        rollback_on_error(&mut journal, result)
    }

    fn install_plan_inner(
        &self,
        plan: OperationPlan,
        journal: &mut OperationJournal,
    ) -> Result<InstallResult> {
        let request = plan.request;
        let destination_root = plan.destination_root;
        fs::create_dir_all(&destination_root)?;

        if plan.candidates.is_empty() {
            warn!(
                source_root = %request.source_root.display(),
                "install found no skill candidates"
            );
            return Err(SkillsManagerError::NoSkillsFound);
        }

        let _config_lock = ManagerConfig::acquire_update_lock(&self.paths)?;
        let mut config = ManagerConfig::load(&self.paths)?;
        let mut codex_config = CodexConfig::load(&self.paths)?;
        let target_scope = request.target.scope();
        if matches!(request.target, InstallTarget::Custom(_)) {
            config.record_custom_install_root(&destination_root);
        }
        let mut installed = Vec::new();
        let mut backups = Vec::new();

        for candidate in plan.candidates {
            let destination_path = candidate.destination_root.clone();
            let conflict = destination_path.exists();

            if conflict {
                match request.conflict_policy {
                    ConflictPolicy::Block => {
                        warn!(
                            destination = %destination_path.display(),
                            "blocking install because destination exists"
                        );
                        return Err(SkillsManagerError::DestinationExists(destination_path));
                    }
                    ConflictPolicy::Rename => {}
                    ConflictPolicy::Replace => {
                        let backup = backup_path(&destination_path);
                        info!(
                            destination = %destination_path.display(),
                            backup = %backup.display(),
                            "backing up existing skill before replace"
                        );
                        fs::rename(&destination_path, &backup)?;
                        journal.record_move(destination_path.clone(), backup.clone());
                        backups.push(backup);
                    }
                }
            }

            if !request.enable_after_install && target_scope != SkillScope::Codex {
                let disabled_root = disabled_root_for_enabled_root(&destination_path);
                if disabled_root.exists() {
                    return Err(SkillsManagerError::DestinationExists(disabled_root));
                }
            }

            debug!(
                source = %candidate.source_root.display(),
                destination = %destination_path.display(),
                "copying skill folder"
            );
            copy_skill_folder(&candidate.source_root, &destination_path)?;
            journal.record_created(destination_path.clone());
            let skill_file = destination_path.join("SKILL.md");
            if target_scope == SkillScope::Codex {
                config.record_install(&destination_path, request.source_url.clone());
                config.set_disabled(&skill_file, !request.enable_after_install);
                codex_config.set_enabled(&skill_file, request.enable_after_install);
                installed.push(destination_path);
            } else {
                codex_config.forget(&skill_file);
                if request.enable_after_install {
                    config.record_install(&destination_path, request.source_url.clone());
                    config.set_disabled(&skill_file, false);
                    installed.push(destination_path);
                } else {
                    let disabled_root = move_enabled_root_to_disabled(&destination_path)?;
                    journal.record_created(disabled_root.clone());
                    config.record_install(&disabled_root, request.source_url.clone());
                    config.set_disabled(&skill_file, false);
                    config.set_disabled(&disabled_root.join("SKILL.md"), true);
                    installed.push(disabled_root);
                }
            }
        }

        config.save(&self.paths)?;
        codex_config.save()?;
        info!(
            installed = installed.len(),
            backups = backups.len(),
            "installed skills"
        );
        Ok(InstallResult { installed, backups })
    }

    pub fn remove(&self, skill_root: &Path) -> Result<PathBuf> {
        let location = self.resolve_skill_location(skill_root)?;
        let actual_root = if location.enabled_skill_file.exists() {
            location.enabled_root.clone()
        } else if let Some(disabled_root) = existing_disabled_root(&location) {
            disabled_root
        } else {
            return Err(SkillsManagerError::MissingSkillFile(
                location.enabled_root.clone(),
            ));
        };

        info!(skill_root = %actual_root.display(), "removing installed skill");

        let _config_lock = ManagerConfig::acquire_update_lock(&self.paths)?;
        let backup = backup_path(&actual_root);
        fs::rename(&actual_root, &backup)?;
        let mut journal = OperationJournal::default();
        journal.record_move(actual_root.clone(), backup.clone());

        let mut config = ManagerConfig::load(&self.paths)?;
        let mut codex_config = CodexConfig::load(&self.paths)?;
        config.forget_install(&location.enabled_root, &location.enabled_skill_file);
        config.forget_install(&location.disabled_root, &location.disabled_skill_file);
        config.forget_install(
            &location.legacy_disabled_root,
            &location.legacy_disabled_skill_file,
        );
        codex_config.forget(&location.enabled_skill_file);
        codex_config.forget(&location.disabled_skill_file);
        codex_config.forget(&location.legacy_disabled_skill_file);
        let save_result = config.save(&self.paths).and_then(|_| codex_config.save());
        rollback_on_error(&mut journal, save_result)?;

        info!(
            skill_root = %actual_root.display(),
            backup = %backup.display(),
            "removed installed skill"
        );
        Ok(backup)
    }

    pub fn set_disabled(&self, skill_file: &Path, disabled: bool) -> Result<()> {
        let skill_root = skill_root_from_path(skill_file);
        self.set_skill_enabled(&skill_root, !disabled).map(|_| ())
    }

    pub fn set_skill_enabled(&self, skill_root: &Path, enabled: bool) -> Result<PathBuf> {
        let location = self.resolve_skill_location(skill_root)?;
        info!(
            skill_root = %location.enabled_root.display(),
            scope = location.scope.label(),
            enabled,
            "updating skill enablement"
        );
        let _config_lock = ManagerConfig::acquire_update_lock(&self.paths)?;
        let mut config = ManagerConfig::load(&self.paths)?;
        let mut codex_config = CodexConfig::load(&self.paths)?;
        let final_root = if location.scope == SkillScope::Codex {
            if !location.enabled_skill_file.exists() {
                return Err(SkillsManagerError::MissingSkillFile(location.enabled_root));
            }
            codex_config.set_enabled(&location.enabled_skill_file, enabled);
            config.set_disabled(&location.disabled_skill_file, false);
            config.set_disabled(&location.legacy_disabled_skill_file, false);
            config.set_disabled(&location.enabled_skill_file, !enabled);
            location.enabled_root.clone()
        } else if enabled {
            let final_root = enable_by_moving_directory(&location)?;
            codex_config.forget(&location.enabled_skill_file);
            codex_config.forget(&location.disabled_skill_file);
            codex_config.forget(&location.legacy_disabled_skill_file);
            config.move_install_record(&location.disabled_root, &location.enabled_root);
            config.move_install_record(&location.legacy_disabled_root, &location.enabled_root);
            config.set_disabled(&location.disabled_skill_file, false);
            config.set_disabled(&location.legacy_disabled_skill_file, false);
            config.set_disabled(&location.enabled_skill_file, false);
            final_root
        } else {
            let final_root = disable_by_moving_directory(&location)?;
            codex_config.forget(&location.enabled_skill_file);
            codex_config.forget(&location.disabled_skill_file);
            codex_config.forget(&location.legacy_disabled_skill_file);
            config.move_install_record(&location.enabled_root, &location.disabled_root);
            config.move_install_record(&location.legacy_disabled_root, &location.disabled_root);
            config.set_disabled(&location.enabled_skill_file, false);
            config.set_disabled(&location.legacy_disabled_skill_file, false);
            config.set_disabled(&location.disabled_skill_file, true);
            final_root
        };
        config.save(&self.paths)?;
        codex_config.save()?;

        Ok(final_root)
    }

    fn destination_for_target(&self, target: &InstallTarget) -> Result<PathBuf> {
        target.destination_root(&self.paths)
    }

    fn resolve_skill_location(&self, skill_root: &Path) -> Result<ManagedSkillLocation> {
        let requested = skill_root_from_path(skill_root);
        let config = ManagerConfig::load(&self.paths)?;
        let mut roots = self.paths.skill_roots();
        for root in &config.custom_install_roots {
            roots.push((SkillScope::Custom, root.clone()));
        }

        for (scope, root) in roots {
            if let Some(location) = managed_skill_location(scope, &root, &requested) {
                return Ok(location);
            }
        }

        Err(SkillsManagerError::UnknownSkillScope(requested))
    }
}

fn plan_destination(
    destination_root: &Path,
    preferred_name: &str,
    claimed: &mut HashMap<String, ()>,
    conflict_policy: ConflictPolicy,
) -> PlannedDestination {
    let folder_name = unique_claim_name(preferred_name, claimed);
    let path = destination_root.join(&folder_name);
    let conflict = path.exists();

    if conflict && conflict_policy == ConflictPolicy::Rename {
        let renamed = unique_folder_name(destination_root, preferred_name, claimed);
        claimed.insert(renamed.clone(), ());
        return PlannedDestination {
            path: destination_root.join(renamed),
            conflict,
        };
    }

    PlannedDestination { path, conflict }
}

fn skill_root_from_path(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|name| name == "SKILL.md") {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or(path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

fn managed_skill_location(
    scope: SkillScope,
    skills_root: &Path,
    requested_root: &Path,
) -> Option<ManagedSkillLocation> {
    let disabled_parent = disabled_store_root_for_skills_root(skills_root);
    let legacy_disabled_parent = skills_root.join(LEGACY_DISABLED_SKILLS_DIR);

    if requested_root.starts_with(&disabled_parent) {
        let relative = requested_root.strip_prefix(&disabled_parent).ok()?;
        if relative.as_os_str().is_empty() {
            return None;
        }
        let enabled_root = skills_root.join(relative);
        let disabled_root = disabled_parent.join(relative);
        let legacy_disabled_root = legacy_disabled_parent.join(relative);
        return Some(location(
            scope,
            enabled_root,
            disabled_root,
            legacy_disabled_root,
        ));
    }

    if requested_root.starts_with(&legacy_disabled_parent) {
        let relative = requested_root.strip_prefix(&legacy_disabled_parent).ok()?;
        if relative.as_os_str().is_empty() {
            return None;
        }
        let enabled_root = skills_root.join(relative);
        let disabled_root = disabled_parent.join(relative);
        let legacy_disabled_root = legacy_disabled_parent.join(relative);
        return Some(location(
            scope,
            enabled_root,
            disabled_root,
            legacy_disabled_root,
        ));
    }

    if requested_root.starts_with(skills_root) {
        let relative = requested_root.strip_prefix(skills_root).ok()?;
        if relative.as_os_str().is_empty() {
            return None;
        }
        let enabled_root = skills_root.join(relative);
        let disabled_root = disabled_parent.join(relative);
        let legacy_disabled_root = legacy_disabled_parent.join(relative);
        return Some(location(
            scope,
            enabled_root,
            disabled_root,
            legacy_disabled_root,
        ));
    }

    None
}

fn location(
    scope: SkillScope,
    enabled_root: PathBuf,
    disabled_root: PathBuf,
    legacy_disabled_root: PathBuf,
) -> ManagedSkillLocation {
    ManagedSkillLocation {
        scope,
        enabled_skill_file: enabled_root.join("SKILL.md"),
        disabled_skill_file: disabled_root.join("SKILL.md"),
        legacy_disabled_skill_file: legacy_disabled_root.join("SKILL.md"),
        enabled_root,
        disabled_root,
        legacy_disabled_root,
    }
}

fn disabled_root_for_enabled_root(enabled_root: &Path) -> PathBuf {
    let parent = enabled_root.parent().unwrap_or_else(|| Path::new(""));
    let file_name = enabled_root.file_name().unwrap_or_default();
    disabled_store_root_for_skills_root(parent).join(file_name)
}

fn move_enabled_root_to_disabled(enabled_root: &Path) -> Result<PathBuf> {
    let disabled_root = disabled_root_for_enabled_root(enabled_root);
    if disabled_root.exists() {
        return Err(SkillsManagerError::DestinationExists(disabled_root));
    }
    if !enabled_root.join("SKILL.md").exists() {
        return Err(SkillsManagerError::MissingSkillFile(
            enabled_root.to_path_buf(),
        ));
    }
    if let Some(parent) = disabled_root.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(enabled_root, &disabled_root)?;
    Ok(disabled_root)
}

fn disable_by_moving_directory(location: &ManagedSkillLocation) -> Result<PathBuf> {
    if location.disabled_skill_file.exists() {
        if location.enabled_skill_file.exists() {
            return Err(SkillsManagerError::DestinationExists(
                location.disabled_root.clone(),
            ));
        }
        return Ok(location.disabled_root.clone());
    }

    if location.legacy_disabled_skill_file.exists() {
        if location.enabled_skill_file.exists() {
            return Err(SkillsManagerError::DestinationExists(
                location.legacy_disabled_root.clone(),
            ));
        }
        if let Some(parent) = location.disabled_root.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&location.legacy_disabled_root, &location.disabled_root)?;
        return Ok(location.disabled_root.clone());
    }

    move_enabled_root_to_disabled(&location.enabled_root)
}

fn enable_by_moving_directory(location: &ManagedSkillLocation) -> Result<PathBuf> {
    if location.enabled_skill_file.exists() {
        if location.disabled_skill_file.exists() || location.legacy_disabled_skill_file.exists() {
            return Err(SkillsManagerError::DestinationExists(
                location.enabled_root.clone(),
            ));
        }
        return Ok(location.enabled_root.clone());
    }

    if location.enabled_root.exists() {
        return Err(SkillsManagerError::DestinationExists(
            location.enabled_root.clone(),
        ));
    }

    let Some(disabled_root) = existing_disabled_root(location) else {
        return Err(SkillsManagerError::MissingSkillFile(
            location.enabled_root.clone(),
        ));
    };

    if let Some(parent) = location.enabled_root.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(disabled_root, &location.enabled_root)?;
    Ok(location.enabled_root.clone())
}

fn existing_disabled_root(location: &ManagedSkillLocation) -> Option<PathBuf> {
    if location.disabled_skill_file.exists() {
        Some(location.disabled_root.clone())
    } else if location.legacy_disabled_skill_file.exists() {
        Some(location.legacy_disabled_root.clone())
    } else {
        None
    }
}

fn copy_skill_folder(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::create_dir_all(destination)?;

    for entry in WalkDir::new(source).follow_links(false).into_iter() {
        let entry = entry?;
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
    let config = ManagerConfig::load(paths)?;
    let mut roots = paths.skill_roots();
    for root in &config.custom_install_roots {
        roots.push((SkillScope::Custom, root.clone()));
    }

    for (scope, root) in roots {
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

    use crate::{ManagerConfig, ProjectRoot, SkillEnablement, scan_installed_skills};

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
                target: InstallTarget::Project,
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: true,
            })
            .unwrap();

        assert_eq!(result.installed.len(), 1);
        assert!(project.join(".agents/skills/demo/SKILL.md").exists());
    }

    #[test]
    fn operation_plan_can_be_applied_without_repreviewing() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source/demo");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: demo\ndescription: Demo skill plan apply\n---\n",
        )
        .unwrap();

        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(dir.path().join("project"))),
        );
        let installer = Installer::new(paths.clone());
        let plan = installer
            .plan(InstallRequest {
                source_root: dir.path().join("source"),
                source_url: Some("fixture".to_string()),
                target: InstallTarget::Global,
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: false,
            })
            .unwrap();

        assert_eq!(plan.candidates.len(), 1);
        assert!(!plan.request.enable_after_install);
        let result = installer.install_plan(plan).unwrap();

        assert_eq!(result.installed.len(), 1);
        assert!(result.installed[0].join("SKILL.md").exists());
        assert!(result.installed[0].starts_with(dir.path().join("home/.agents/.skills-disabled")));
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
                InstallTarget::User,
                ConflictPolicy::Rename,
            )
            .unwrap();

        assert_eq!(preview.candidates.len(), 1);
        assert!(preview.candidates[0].conflict);
        assert!(preview.candidates[0].destination_root.ends_with("demo-2"));
        assert!(!preview.candidates[0].diagnostics.is_empty());
    }

    #[test]
    fn rename_policy_does_not_rename_without_conflict() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths);

        let preview = installer
            .preview(&source, InstallTarget::User, ConflictPolicy::Rename)
            .unwrap();
        assert_eq!(preview.candidates.len(), 1);
        assert!(!preview.candidates[0].conflict);
        assert!(preview.candidates[0].destination_root.ends_with("demo"));

        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: None,
                target: InstallTarget::User,
                conflict_policy: ConflictPolicy::Rename,
                enable_after_install: true,
            })
            .unwrap();

        assert_eq!(result.installed.len(), 1);
        assert!(result.installed[0].ends_with("demo"));
        assert!(!result.installed[0].ends_with("demo-2"));
        assert!(result.installed[0].join("SKILL.md").exists());
    }

    #[test]
    fn rename_policy_preview_matches_install_with_conflict() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let existing = dir.path().join("home/.agents/skills/demo");
        fs::create_dir_all(&existing).unwrap();
        fs::write(
            existing.join("SKILL.md"),
            "---\nname: demo\ndescription: Existing skill\n---\n",
        )
        .unwrap();

        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths);
        let preview = installer
            .preview(&source, InstallTarget::User, ConflictPolicy::Rename)
            .unwrap();
        let preview_destination = preview.candidates[0].destination_root.clone();

        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: None,
                target: InstallTarget::User,
                conflict_policy: ConflictPolicy::Rename,
                enable_after_install: true,
            })
            .unwrap();

        assert_eq!(result.installed, vec![preview_destination.clone()]);
        assert!(preview_destination.ends_with("demo-2"));
        assert!(preview_destination.join("SKILL.md").exists());
        assert!(existing.join("SKILL.md").exists());
    }

    #[test]
    fn rename_policy_keeps_duplicate_candidates_deterministic() {
        let dir = tempdir().unwrap();
        let source_root = dir.path().join("source");
        create_skill(dir.path(), "source/first", "demo");
        create_skill(dir.path(), "source/second", "demo");
        let existing = dir.path().join("home/.agents/skills/demo");
        fs::create_dir_all(&existing).unwrap();
        fs::write(
            existing.join("SKILL.md"),
            "---\nname: demo\ndescription: Existing skill\n---\n",
        )
        .unwrap();

        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths);
        let preview = installer
            .preview(&source_root, InstallTarget::User, ConflictPolicy::Rename)
            .unwrap();
        let destinations = preview
            .candidates
            .iter()
            .map(|candidate| {
                candidate
                    .destination_root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(destinations, vec!["demo-2", "demo-3"]);
    }

    #[test]
    fn replace_policy_preserves_backup_behavior() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let existing = dir.path().join("home/.agents/skills/demo");
        fs::create_dir_all(&existing).unwrap();
        fs::write(
            existing.join("SKILL.md"),
            "---\nname: demo\ndescription: Existing skill\n---\n",
        )
        .unwrap();
        fs::write(existing.join("old.txt"), "old").unwrap();

        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths);
        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: Some("fixture".to_string()),
                target: InstallTarget::User,
                conflict_policy: ConflictPolicy::Replace,
                enable_after_install: true,
            })
            .unwrap();

        assert_eq!(result.installed.len(), 1);
        assert_eq!(result.backups.len(), 1);
        assert!(result.installed[0].ends_with("demo"));
        assert!(result.installed[0].join("SKILL.md").exists());
        assert!(result.backups[0].join("old.txt").exists());
    }

    #[test]
    fn custom_install_root_is_recorded_and_scanned() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let custom_root = dir.path().join("custom-skills");
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths.clone());

        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: Some("fixture".to_string()),
                target: InstallTarget::Custom(custom_root.clone()),
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: true,
            })
            .unwrap();

        assert_eq!(result.installed, vec![custom_root.join("demo")]);
        let skills = scan_installed_skills(&paths).unwrap();
        let custom = skills
            .iter()
            .find(|skill| skill.scope == SkillScope::Custom)
            .unwrap();
        assert_eq!(custom.root_dir, custom_root.join("demo"));
    }

    #[test]
    fn installs_skill_to_builtin_agent_targets() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths.clone());
        let targets = [
            (
                InstallTarget::Global,
                SkillScope::Global,
                "home/.agents/skills/demo",
            ),
            (
                InstallTarget::ClaudeCode,
                SkillScope::ClaudeCode,
                "home/.claude/skills/demo",
            ),
            (
                InstallTarget::Droid,
                SkillScope::Droid,
                "home/.droid/skills/demo",
            ),
            (
                InstallTarget::Pencode,
                SkillScope::Pencode,
                "home/.pencode/skills/demo",
            ),
            (
                InstallTarget::Codex,
                SkillScope::Codex,
                "home/.codex/skills/demo",
            ),
            (
                InstallTarget::Zed,
                SkillScope::Zed,
                "home/.config/zed/skills/demo",
            ),
        ];

        for (target, _scope, relative_path) in targets.clone() {
            let result = installer
                .install(InstallRequest {
                    source_root: source.clone(),
                    source_url: Some("fixture".to_string()),
                    target,
                    conflict_policy: ConflictPolicy::Block,
                    enable_after_install: true,
                })
                .unwrap();

            assert_eq!(result.installed, vec![dir.path().join(relative_path)]);
        }

        let skills = scan_installed_skills(&paths).unwrap();
        for (_target, scope, relative_path) in targets {
            let skill = skills.iter().find(|skill| skill.scope == scope).unwrap();
            assert_eq!(skill.root_dir, dir.path().join(relative_path));
        }
    }

    #[test]
    fn directory_scanning_targets_disable_outside_scan_root_and_enable_restore() {
        let cases = [
            (
                InstallTarget::Global,
                SkillScope::Global,
                "home/.agents/skills/demo",
                "home/.agents/.skills-disabled/demo",
            ),
            (
                InstallTarget::ClaudeCode,
                SkillScope::ClaudeCode,
                "home/.claude/skills/demo",
                "home/.claude/.skills-disabled/demo",
            ),
            (
                InstallTarget::Droid,
                SkillScope::Droid,
                "home/.droid/skills/demo",
                "home/.droid/.skills-disabled/demo",
            ),
            (
                InstallTarget::Pencode,
                SkillScope::Pencode,
                "home/.pencode/skills/demo",
                "home/.pencode/.skills-disabled/demo",
            ),
            (
                InstallTarget::Zed,
                SkillScope::Zed,
                "home/.config/zed/skills/demo",
                "home/.config/zed/.skills-disabled/demo",
            ),
        ];

        for (target, scope, enabled_relative, disabled_relative) in cases {
            let dir = tempdir().unwrap();
            let source = create_skill(dir.path(), "source/demo", "demo");
            let paths = test_paths(dir.path(), None);
            let installer = Installer::new(paths.clone());

            assert_directory_target_disable_enable_round_trip(
                dir.path(),
                &paths,
                &installer,
                source,
                target,
                scope,
                enabled_relative,
                disabled_relative,
            );
        }

        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let project = dir.path().join("project");
        let paths = test_paths(dir.path(), Some(&project));
        let installer = Installer::new(paths.clone());
        assert_directory_target_disable_enable_round_trip(
            dir.path(),
            &paths,
            &installer,
            source,
            InstallTarget::Project,
            SkillScope::Project,
            "project/.agents/skills/demo",
            "project/.agents/.skills-disabled/demo",
        );

        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let custom_root = dir.path().join("custom-skills");
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths.clone());
        assert_directory_target_disable_enable_round_trip(
            dir.path(),
            &paths,
            &installer,
            source,
            InstallTarget::Custom(custom_root),
            SkillScope::Custom,
            "custom-skills/demo",
            ".skills-disabled/demo",
        );
    }

    #[test]
    fn zed_legacy_disabled_directory_can_be_enabled() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths.clone());
        let enabled_root = dir.path().join("home/.config/zed/skills/demo");
        let legacy_disabled_root = dir.path().join("home/.config/zed/skills/.disabled/demo");
        fs::create_dir_all(&legacy_disabled_root).unwrap();
        fs::write(
            legacy_disabled_root.join("SKILL.md"),
            "---\nname: demo\ndescription: Demo skill\n---\n",
        )
        .unwrap();

        let skills = scan_installed_skills(&paths).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].root_dir, legacy_disabled_root);
        assert_eq!(skills[0].enablement, SkillEnablement::Disabled);

        let enabled = installer.set_skill_enabled(&enabled_root, true).unwrap();

        assert_eq!(enabled, enabled_root);
        assert!(enabled_root.join("SKILL.md").exists());
        assert!(!legacy_disabled_root.exists());
        let skills = scan_installed_skills(&paths).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].root_dir, enabled_root);
        assert_eq!(skills[0].enablement, SkillEnablement::Enabled);
    }

    #[test]
    fn remove_disabled_directory_skill_cleans_manager_state() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths.clone());

        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: Some("fixture".to_string()),
                target: InstallTarget::Zed,
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: true,
            })
            .unwrap();

        let enabled_root = result.installed[0].clone();
        let disabled_root = installer.set_skill_enabled(&enabled_root, false).unwrap();
        let backup = installer.remove(&enabled_root).unwrap();

        assert!(!disabled_root.exists());
        assert!(backup.exists());
        assert!(
            backup
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("demo.backup-")
        );
        assert!(scan_installed_skills(&paths).unwrap().is_empty());
        let config = ManagerConfig::load(&paths).unwrap();
        assert!(config.installed.is_empty());
        assert!(config.disabled_skill_files.is_empty());
    }

    #[test]
    fn codex_disable_uses_config_without_moving_directory() {
        let dir = tempdir().unwrap();
        let source = create_skill(dir.path(), "source/demo", "demo");
        let paths = test_paths(dir.path(), None);
        let installer = Installer::new(paths.clone());

        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: Some("fixture".to_string()),
                target: InstallTarget::Codex,
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: true,
            })
            .unwrap();

        let skill_file = result.installed[0].join("SKILL.md");
        installer
            .set_skill_enabled(&result.installed[0], false)
            .unwrap();

        assert!(skill_file.exists());
        assert!(
            !dir.path()
                .join("home/.codex/.skills-disabled/demo/SKILL.md")
                .exists()
        );
        let skills = scan_installed_skills(&paths).unwrap();
        assert_eq!(skills[0].enablement, SkillEnablement::Disabled);
        let codex_config = fs::read_to_string(paths.codex_config_file()).unwrap();
        assert!(codex_config.contains(skill_file.to_string_lossy().as_ref()));
        assert!(codex_config.contains("enabled = false"));

        installer
            .set_skill_enabled(&result.installed[0], true)
            .unwrap();

        assert!(skill_file.exists());
        let skills = scan_installed_skills(&paths).unwrap();
        assert_eq!(skills[0].enablement, SkillEnablement::Enabled);
        let codex_config = fs::read_to_string(paths.codex_config_file()).unwrap();
        assert!(codex_config.contains(skill_file.to_string_lossy().as_ref()));
        assert!(codex_config.contains("enabled = true"));
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_directory_target_disable_enable_round_trip(
        root: &Path,
        paths: &ManagerPaths,
        installer: &Installer,
        source: PathBuf,
        target: InstallTarget,
        scope: SkillScope,
        enabled_relative: &str,
        disabled_relative: &str,
    ) {
        let result = installer
            .install(InstallRequest {
                source_root: source,
                source_url: Some("fixture".to_string()),
                target,
                conflict_policy: ConflictPolicy::Block,
                enable_after_install: true,
            })
            .unwrap();

        let enabled_root = root.join(enabled_relative);
        let disabled_root = root.join(disabled_relative);
        assert_eq!(result.installed, vec![enabled_root.clone()]);

        let disabled = installer.set_skill_enabled(&enabled_root, false).unwrap();
        assert_eq!(disabled, disabled_root);
        assert!(!enabled_root.exists());
        assert!(disabled_root.join("SKILL.md").exists());
        assert!(!disabled_root.starts_with(enabled_root.parent().unwrap()));
        assert!(
            !enabled_root
                .parent()
                .unwrap()
                .join(".disabled/demo")
                .exists()
        );

        let skills = scan_installed_skills(paths).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].root_dir, disabled_root);
        assert_eq!(skills[0].scope, scope);
        assert_eq!(skills[0].enablement, SkillEnablement::Disabled);
        assert_eq!(skills[0].source_url.as_deref(), Some("fixture"));
        let config = ManagerConfig::load(paths).unwrap();
        assert!(
            config
                .disabled_skill_files
                .contains(&path_key(&skills[0].skill_file))
        );
        assert!(
            !config
                .disabled_skill_files
                .contains(&path_key(&enabled_root.join("SKILL.md")))
        );

        let enabled = installer.set_skill_enabled(&enabled_root, true).unwrap();
        assert_eq!(enabled, enabled_root);
        assert!(enabled_root.join("SKILL.md").exists());
        assert!(!disabled_root.exists());
        let skills = scan_installed_skills(paths).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].root_dir, enabled_root);
        assert_eq!(skills[0].scope, scope);
        assert_eq!(skills[0].enablement, SkillEnablement::Enabled);
    }

    fn create_skill(root: &Path, relative: &str, name: &str) -> PathBuf {
        let source = root.join(relative);
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: Demo skill\n---\n"),
        )
        .unwrap();
        source.parent().unwrap().to_path_buf()
    }

    fn test_paths(root: &Path, project: Option<&Path>) -> ManagerPaths {
        ManagerPaths::with_home(
            root.join("home"),
            root.join("data"),
            root.join("config"),
            project.map(ProjectRoot::new),
        )
    }
}
