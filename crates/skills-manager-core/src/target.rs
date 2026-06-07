//! Target profiles, diagnostics, and repair actions for managed scopes.
//!
//! Each install scope (Global, Project, Claude Code, Codex, Zed, etc.)
//! is described by a [`TargetProfile`] that records layout policy,
//! enablement strategy, supported frontmatter keys, and budget limits.
//! The [`doctor_report`] function inspects all targets and produces a
//! [`DoctorReport`] with diagnostics and proposed repair actions.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_yaml::Value;
use walkdir::WalkDir;

use crate::{
    DiagnosticSeverity, InstalledSkill, ManagerConfig, ManagerPaths, Result, SkillDiagnostic,
    SkillEnablement, SkillFrontmatter, SkillHealth, SkillScope,
    codex_config::CodexConfig,
    scan_installed_skills,
    skill::{
        LEGACY_DISABLED_SKILLS_DIR, disabled_store_root_for_skills_root, discover_skill_candidates,
        format_bytes, sanitize_folder_name,
    },
};

const ZED_CATALOG_BUDGET_BYTES: u64 = 50 * 1024;

/// Strategy used to represent enabled/disabled state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EnablementStrategy {
    /// Keep a skill present and mark it enabled/disabled in config.
    ConfigToggle,
    /// Enable/disable by moving a directory into a disabled store.
    DirectoryMove,
}

/// Policy describing how nested folders are expected in skill roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LayoutPolicy {
    /// Nested structures are allowed.
    NestedAllowed,
    /// Flat top-level structure is required.
    FlatTopLevel,
}

/// Static metadata for a scope plus diagnostics context for status/doctor output.
#[derive(Debug, Clone, Serialize)]
pub struct TargetProfile {
    /// Scope for this profile.
    pub scope: SkillScope,
    /// Display label.
    pub label: String,
    /// Root folder containing active skill installations.
    pub skills_root: Option<PathBuf>,
    /// Optional disabled store folder.
    pub disabled_store_root: Option<PathBuf>,
    /// Optional legacy disabled store folder.
    pub legacy_disabled_store_root: Option<PathBuf>,
    /// Whether this target uses config-based or directory-move toggling.
    pub enablement_strategy: EnablementStrategy,
    /// Whether nesting is allowed in this target.
    pub layout_policy: LayoutPolicy,
    /// Lower value means earlier in precedence order.
    pub precedence: usize,
    /// Optional catalog budget check for targets that precompute catalogs.
    pub catalog_budget_bytes: Option<u64>,
    /// Frontmatter keys recognized by the target.
    pub supported_frontmatter: Vec<String>,
    /// Human-targeted notes for diagnostics and docs.
    pub notes: Vec<String>,
}

/// Per-target count buckets used in health summaries.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TargetHealthCounts {
    /// Total skills in scope.
    pub total: usize,
    /// Enabled skills.
    pub enabled: usize,
    /// Disabled skills.
    pub disabled: usize,
    /// Skills considered usable.
    pub usable: usize,
    /// Invalid skills.
    pub invalid: usize,
    /// Warning-only skills.
    pub warning: usize,
    /// Shadowed duplicates.
    pub shadowed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorRepairAction {
    /// Short label in reports.
    pub label: String,
    /// Human-readable description and expected outcome.
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetDoctorReport {
    /// Target profile.
    pub profile: TargetProfile,
    /// Aggregate counts for this target.
    pub counts: TargetHealthCounts,
    /// Optional catalog bytes estimate for targets with catalogs.
    pub catalog_bytes: Option<u64>,
    /// Diagnostics discovered for this target.
    pub diagnostics: Vec<SkillDiagnostic>,
    /// Fixes that can be applied for the target.
    pub repair_actions: Vec<DoctorRepairAction>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DoctorSummary {
    /// Number of inspected targets.
    pub targets: usize,
    /// Total skills across all targets.
    pub skills: usize,
    /// Enabled skill count.
    pub enabled: usize,
    /// Disabled skill count.
    pub disabled: usize,
    /// Invalid skills count.
    pub invalid: usize,
    /// Warning count.
    pub warnings: usize,
    /// Total proposed repair actions.
    pub repair_actions: usize,
}

/// High-level doctor output emitted by status checks.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Aggregate summary.
    pub summary: DoctorSummary,
    /// Per-target detail.
    pub targets: Vec<TargetDoctorReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairOutcome {
    /// Repair action label.
    pub label: String,
    /// Path impacted by the action, if applicable.
    pub path: Option<PathBuf>,
    /// Whether action was applied.
    pub applied: bool,
    /// Status message after execution or simulation.
    pub message: String,
}

/// Result for a full repair run.
#[derive(Debug, Clone, Serialize)]
pub struct RepairReport {
    /// Whether this was a dry-run.
    pub dry_run: bool,
    /// All repair attempts and their status.
    pub actions: Vec<RepairOutcome>,
}

/// Returns all installed target profiles from config and known defaults.
pub fn target_profiles(paths: &ManagerPaths) -> Result<Vec<TargetProfile>> {
    let config = ManagerConfig::load(paths)?;
    let mut profiles = Vec::new();

    if let Some(root) = paths.project_skills_dir() {
        profiles.push(directory_profile(
            SkillScope::Project,
            root,
            0,
            "Project skills are local to the active repository.",
        ));
    } else {
        profiles.push(TargetProfile {
            scope: SkillScope::Project,
            label: SkillScope::Project.label().to_string(),
            skills_root: None,
            disabled_store_root: None,
            legacy_disabled_store_root: None,
            enablement_strategy: EnablementStrategy::DirectoryMove,
            layout_policy: LayoutPolicy::NestedAllowed,
            precedence: 0,
            catalog_budget_bytes: None,
            supported_frontmatter: common_frontmatter_fields(),
            notes: vec!["Project skills require a project folder.".to_string()],
        });
    }

    profiles.push(directory_profile(
        SkillScope::Global,
        paths.global_skills_dir(),
        1,
        "Global skills are shared across local agent tools.",
    ));
    profiles.push(directory_profile(
        SkillScope::ClaudeCode,
        paths.claude_code_skills_dir(),
        2,
        "Claude Code uses description and when_to_use text for skill selection.",
    ));
    profiles.push(directory_profile(
        SkillScope::Droid,
        paths.droid_skills_dir(),
        3,
        "Droid currently follows directory-scanned skill semantics.",
    ));
    profiles.push(directory_profile(
        SkillScope::OpenCode,
        paths.opencode_skills_dir(),
        4,
        "OpenCode currently follows directory-scanned skill semantics.",
    ));
    profiles.push(TargetProfile {
        scope: SkillScope::Codex,
        label: SkillScope::Codex.label().to_string(),
        skills_root: Some(paths.codex_skills_dir()),
        disabled_store_root: None,
        legacy_disabled_store_root: None,
        enablement_strategy: EnablementStrategy::ConfigToggle,
        layout_policy: LayoutPolicy::NestedAllowed,
        precedence: 5,
        catalog_budget_bytes: None,
        supported_frontmatter: codex_frontmatter_fields(),
        notes: vec![
            "Codex enablement is stored in ~/.codex/config.toml.".to_string(),
            "Optional agents/openai.yaml can tune invocation metadata.".to_string(),
        ],
    });
    profiles.push(TargetProfile {
        scope: SkillScope::Zed,
        label: SkillScope::Zed.label().to_string(),
        skills_root: Some(paths.zed_skills_dir()),
        disabled_store_root: Some(disabled_store_root_for_skills_root(&paths.zed_skills_dir())),
        legacy_disabled_store_root: Some(paths.zed_skills_dir().join(LEGACY_DISABLED_SKILLS_DIR)),
        enablement_strategy: EnablementStrategy::DirectoryMove,
        layout_policy: LayoutPolicy::FlatTopLevel,
        precedence: 6,
        catalog_budget_bytes: Some(ZED_CATALOG_BUDGET_BYTES),
        supported_frontmatter: zed_frontmatter_fields(),
        notes: vec![
            "Zed scans ~/.config/zed/skills and expects flat skill folders.".to_string(),
            "Zed keeps a compact skill catalog before loading full SKILL.md content.".to_string(),
        ],
    });

    for (index, root) in config.custom_install_roots.iter().enumerate() {
        profiles.push(directory_profile(
            SkillScope::Custom,
            root.clone(),
            100 + index,
            "Custom roots are managed by Skills Manager.",
        ));
    }

    Ok(profiles)
}

/// Builds a complete doctor report for all targets in the given workspace.
pub fn doctor_report(paths: &ManagerPaths) -> Result<DoctorReport> {
    let profiles = target_profiles(paths)?;
    let skills = scan_installed_skills(paths)?;
    doctor_report_for_skills(paths, profiles, &skills)
}

/// Builds a doctor report from supplied targets and skills.
pub fn doctor_report_for_skills(
    paths: &ManagerPaths,
    profiles: Vec<TargetProfile>,
    skills: &[InstalledSkill],
) -> Result<DoctorReport> {
    let codex_config = CodexConfig::load(paths)?;
    let mut targets = Vec::new();

    for profile in profiles {
        let scoped_skills = skills
            .iter()
            .filter(|skill| skill.scope == profile.scope)
            .collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let mut repair_actions = Vec::new();

        inspect_target_storage(&profile, &mut diagnostics, &mut repair_actions)?;
        if profile.scope == SkillScope::Codex {
            inspect_codex_config(&codex_config, &mut diagnostics, &mut repair_actions);
        }

        let catalog_bytes = if profile.scope == SkillScope::Zed {
            Some(zed_catalog_size(&scoped_skills))
        } else {
            None
        };
        if let (Some(bytes), Some(budget)) = (catalog_bytes, profile.catalog_budget_bytes)
            && bytes > budget
        {
            diagnostics.push(SkillDiagnostic::invalid(format!(
                "Zed catalog estimate is {} but the budget is {}",
                format_bytes(bytes),
                format_bytes(budget)
            )));
        }

        targets.push(TargetDoctorReport {
            profile,
            counts: health_counts(&scoped_skills),
            catalog_bytes,
            diagnostics,
            repair_actions,
        });
    }

    let summary = DoctorSummary {
        targets: targets.len(),
        skills: skills.len(),
        enabled: skills.iter().filter(|skill| skill.is_enabled()).count(),
        disabled: skills.iter().filter(|skill| !skill.is_enabled()).count(),
        invalid: skills
            .iter()
            .filter(|skill| skill.health == SkillHealth::Invalid)
            .count(),
        warnings: targets
            .iter()
            .map(|target| {
                target
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
                    .count()
            })
            .sum(),
        repair_actions: targets
            .iter()
            .map(|target| target.repair_actions.len())
            .sum(),
    };

    Ok(DoctorReport { summary, targets })
}

/// Runs repair probes and optionally applies migrations.
///
/// This currently addresses legacy disabled-store cleanup and stale Codex toggles.
pub fn repair_targets(paths: &ManagerPaths, dry_run: bool) -> Result<RepairReport> {
    let profiles = target_profiles(paths)?;
    let mut actions = Vec::new();

    for profile in profiles
        .iter()
        .filter(|profile| profile.enablement_strategy == EnablementStrategy::DirectoryMove)
    {
        actions.extend(repair_legacy_disabled_store(profile, dry_run)?);
    }
    actions.extend(repair_stale_codex_toggles(paths, dry_run)?);

    Ok(RepairReport { dry_run, actions })
}

/// Computes target-aware diagnostics from frontmatter and target constraints.
pub fn target_specific_diagnostics(
    scope: SkillScope,
    root_dir: &Path,
    frontmatter: &SkillFrontmatter,
    resource_count: usize,
    _resource_bytes: u64,
) -> Vec<SkillDiagnostic> {
    let mut diagnostics = Vec::new();

    if matches!(
        scope,
        SkillScope::ClaudeCode | SkillScope::Codex | SkillScope::Zed
    ) {
        diagnostics.extend(description_quality_diagnostics(frontmatter));
    }

    match scope {
        SkillScope::ClaudeCode => {
            if frontmatter
                .when_to_use
                .as_deref()
                .unwrap_or_default()
                .trim()
                .len()
                < 12
            {
                diagnostics.push(SkillDiagnostic::warning(
                    "Claude Code skills should declare `when_to_use` so the trigger is explicit",
                ));
            }
            if frontmatter.disable_model_invocation == Some(true)
                && frontmatter.allowed_tools.is_empty()
                && resource_count == 0
            {
                diagnostics.push(SkillDiagnostic::warning(
                    "`disable-model-invocation` is true but the skill has no allowed tools or bundled resources",
                ));
            }
        }
        SkillScope::Codex => {
            if frontmatter.tags.len() > 20 {
                diagnostics.push(SkillDiagnostic::warning(
                    "`tags` has more than 20 entries; keep Codex discovery metadata compact",
                ));
            }
            for tag in &frontmatter.tags {
                if !is_compact_identifier(tag) {
                    diagnostics.push(SkillDiagnostic::warning(format!(
                        "Codex tag `{tag}` should use lowercase letters, numbers, hyphens, or underscores"
                    )));
                }
            }
            diagnostics.extend(validate_codex_agent_metadata(root_dir));
        }
        SkillScope::Zed => {
            if let Some(name) = frontmatter.name.as_deref() {
                let folder_name = root_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                let expected = sanitize_folder_name(name);
                if expected != folder_name {
                    diagnostics.push(SkillDiagnostic::invalid(format!(
                        "Zed requires folder `{folder_name}` to match normalized skill name `{expected}`"
                    )));
                }
            }
            if has_nested_entries(root_dir) {
                diagnostics.push(SkillDiagnostic::invalid(
                    "Zed skills must use a flat layout with files at the skill folder top level",
                ));
            }
        }
        SkillScope::Project
        | SkillScope::Global
        | SkillScope::Droid
        | SkillScope::OpenCode
        | SkillScope::Custom => {}
    }

    diagnostics
}

fn directory_profile(
    scope: SkillScope,
    root: PathBuf,
    precedence: usize,
    note: &str,
) -> TargetProfile {
    TargetProfile {
        scope,
        label: scope.label().to_string(),
        disabled_store_root: Some(disabled_store_root_for_skills_root(&root)),
        legacy_disabled_store_root: Some(root.join(LEGACY_DISABLED_SKILLS_DIR)),
        skills_root: Some(root),
        enablement_strategy: EnablementStrategy::DirectoryMove,
        layout_policy: LayoutPolicy::NestedAllowed,
        precedence,
        catalog_budget_bytes: None,
        supported_frontmatter: common_frontmatter_fields(),
        notes: vec![note.to_string()],
    }
}

fn common_frontmatter_fields() -> Vec<String> {
    [
        "name",
        "description",
        "license",
        "compatibility",
        "allowed-tools",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn codex_frontmatter_fields() -> Vec<String> {
    ["name", "description", "tags", "allowed-tools", "license"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn zed_frontmatter_fields() -> Vec<String> {
    ["name", "description", "tags"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn inspect_target_storage(
    profile: &TargetProfile,
    diagnostics: &mut Vec<SkillDiagnostic>,
    repair_actions: &mut Vec<DoctorRepairAction>,
) -> Result<()> {
    let Some(root) = &profile.skills_root else {
        diagnostics.push(SkillDiagnostic::warning(
            "No project root is configured, so this target cannot be scanned yet",
        ));
        return Ok(());
    };

    if !root.exists() {
        diagnostics.push(SkillDiagnostic::warning(format!(
            "Skills root does not exist yet: {}",
            root.display()
        )));
    }

    if profile.enablement_strategy == EnablementStrategy::DirectoryMove {
        if let Some(disabled_root) = &profile.disabled_store_root
            && disabled_root.starts_with(root)
        {
            diagnostics.push(SkillDiagnostic::invalid(format!(
                "Disabled store is inside the scan root: {}",
                disabled_root.display()
            )));
        }

        if let Some(legacy_root) = &profile.legacy_disabled_store_root
            && legacy_root.exists()
        {
            let legacy_count = discover_skill_candidates(legacy_root)?.len();
            if legacy_count > 0 {
                diagnostics.push(SkillDiagnostic::warning(format!(
                    "{legacy_count} legacy disabled skill(s) remain under {}",
                    legacy_root.display()
                )));
                repair_actions.push(DoctorRepairAction {
                    label: "Migrate legacy disabled store".to_string(),
                    description: format!(
                        "Move skills from {} to the sibling .skills-disabled store.",
                        legacy_root.display()
                    ),
                });
            }
        }
    }

    Ok(())
}

fn inspect_codex_config(
    codex_config: &CodexConfig,
    diagnostics: &mut Vec<SkillDiagnostic>,
    repair_actions: &mut Vec<DoctorRepairAction>,
) {
    let mut stale = 0;
    for toggle in codex_config.toggles() {
        if !toggle.path.exists() {
            stale += 1;
        }
    }

    if stale > 0 {
        diagnostics.push(SkillDiagnostic::warning(format!(
            "{stale} Codex skill toggle(s) reference missing SKILL.md files"
        )));
        repair_actions.push(DoctorRepairAction {
            label: "Prune stale Codex toggles".to_string(),
            description: "Remove [[skills.config]] entries that point to missing SKILL.md files."
                .to_string(),
        });
    }
}

fn repair_legacy_disabled_store(
    profile: &TargetProfile,
    dry_run: bool,
) -> Result<Vec<RepairOutcome>> {
    let (Some(disabled_root), Some(legacy_root)) = (
        profile.disabled_store_root.as_ref(),
        profile.legacy_disabled_store_root.as_ref(),
    ) else {
        return Ok(Vec::new());
    };
    if !legacy_root.exists() {
        return Ok(Vec::new());
    }

    let mut actions = Vec::new();
    for candidate in discover_skill_candidates(legacy_root)? {
        let relative = candidate
            .root_dir
            .strip_prefix(legacy_root)
            .unwrap_or(candidate.root_dir.as_path());
        let destination = disabled_root.join(relative);
        if destination.exists() {
            actions.push(RepairOutcome {
                label: "Migrate legacy disabled store".to_string(),
                path: Some(candidate.root_dir),
                applied: false,
                message: format!(
                    "Skipped because destination already exists: {}",
                    destination.display()
                ),
            });
            continue;
        }

        if !dry_run {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&candidate.root_dir, &destination)?;
        }

        actions.push(RepairOutcome {
            label: "Migrate legacy disabled store".to_string(),
            path: Some(destination.clone()),
            applied: !dry_run,
            message: if dry_run {
                format!(
                    "Would move legacy disabled skill to {}",
                    destination.display()
                )
            } else {
                format!("Moved legacy disabled skill to {}", destination.display())
            },
        });
    }

    Ok(actions)
}

fn repair_stale_codex_toggles(paths: &ManagerPaths, dry_run: bool) -> Result<Vec<RepairOutcome>> {
    let mut config = CodexConfig::load(paths)?;
    let stale = config
        .toggles()
        .into_iter()
        .filter(|toggle| !toggle.path.exists())
        .collect::<Vec<_>>();
    if stale.is_empty() {
        return Ok(Vec::new());
    }

    if !dry_run {
        for toggle in &stale {
            config.forget(&toggle.path);
        }
        config.save()?;
    }

    Ok(stale
        .into_iter()
        .map(|toggle| RepairOutcome {
            label: "Prune stale Codex toggles".to_string(),
            path: Some(toggle.path.clone()),
            applied: !dry_run,
            message: if dry_run {
                format!(
                    "Would remove Codex toggle for missing file {}",
                    toggle.path.display()
                )
            } else {
                format!(
                    "Removed Codex toggle for missing file {}",
                    toggle.path.display()
                )
            },
        })
        .collect())
}

fn health_counts(skills: &[&InstalledSkill]) -> TargetHealthCounts {
    skills
        .iter()
        .fold(TargetHealthCounts::default(), |mut counts, skill| {
            counts.total += 1;
            match skill.enablement {
                SkillEnablement::Enabled => counts.enabled += 1,
                SkillEnablement::Disabled => counts.disabled += 1,
            }
            if skill.is_exportable() {
                counts.usable += 1;
            }
            match skill.health {
                SkillHealth::Invalid => counts.invalid += 1,
                SkillHealth::Warning => counts.warning += 1,
                SkillHealth::Shadowed => counts.shadowed += 1,
                SkillHealth::Valid => {}
            }
            counts
        })
}

fn description_quality_diagnostics(frontmatter: &SkillFrontmatter) -> Vec<SkillDiagnostic> {
    let Some(description) = frontmatter.description.as_deref().map(str::trim) else {
        return Vec::new();
    };
    let lowered = description.to_ascii_lowercase();
    let mentions_trigger = ["when", "whenever", "use", "for ", "to ", "trigger"]
        .iter()
        .any(|needle| lowered.contains(needle));

    if description.split_whitespace().count() < 6 || !mentions_trigger {
        vec![SkillDiagnostic::warning(
            "`description` should explain what the skill does and when the agent should use it",
        )]
    } else {
        Vec::new()
    }
}

fn validate_codex_agent_metadata(root_dir: &Path) -> Vec<SkillDiagnostic> {
    let path = root_dir.join("agents").join("openai.yaml");
    if !path.exists() {
        return Vec::new();
    }

    let Ok(raw) = fs::read_to_string(&path) else {
        return vec![SkillDiagnostic::invalid(format!(
            "Could not read Codex metadata at {}",
            path.display()
        ))];
    };
    let parsed: Value = match serde_yaml::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            return vec![SkillDiagnostic::invalid(format!(
                "Could not parse Codex metadata agents/openai.yaml: {error}"
            ))];
        }
    };

    let Some(value) = parsed.get("allow_implicit_invocation") else {
        return Vec::new();
    };
    if !matches!(value, Value::Bool(_)) {
        vec![SkillDiagnostic::invalid(
            "`allow_implicit_invocation` in agents/openai.yaml must be a boolean",
        )]
    } else {
        Vec::new()
    }
}

fn has_nested_entries(root_dir: &Path) -> bool {
    WalkDir::new(root_dir)
        .min_depth(2)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|entry| !entry.file_name().to_string_lossy().contains(".backup-"))
}

fn is_compact_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

fn zed_catalog_size(skills: &[&InstalledSkill]) -> u64 {
    skills
        .iter()
        .filter(|skill| skill.is_enabled())
        .map(|skill| {
            let name = skill
                .frontmatter
                .name
                .as_deref()
                .unwrap_or(&skill.display_name);
            let description = skill.description.as_deref().unwrap_or_default();
            let tags = skill.frontmatter.tags.join(",");
            (name.len() + description.len() + tags.len() + 64) as u64
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{ManagerPaths, ProjectRoot};

    use super::*;

    #[test]
    fn zed_target_diagnostics_reject_nested_layout() {
        let dir = tempdir().unwrap();
        let skill = dir.path().join("home/.config/zed/skills/demo");
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: demo\ndescription: Use this skill when testing Zed layout validation\n---\n",
        )
        .unwrap();
        fs::write(skill.join("references/notes.md"), "nested").unwrap();

        let paths = test_paths(dir.path());
        let skills = scan_installed_skills(&paths).unwrap();
        let zed = skills
            .iter()
            .find(|skill| skill.scope == SkillScope::Zed)
            .unwrap();

        assert_eq!(zed.health, SkillHealth::Invalid);
        assert!(
            zed.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("flat layout"))
        );
    }

    #[test]
    fn doctor_reports_legacy_disabled_store() {
        let dir = tempdir().unwrap();
        let skill = dir.path().join("home/.config/zed/skills/.disabled/demo");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: demo\ndescription: Use this skill when testing legacy disabled stores\n---\n",
        )
        .unwrap();

        let paths = test_paths(dir.path());
        let report = doctor_report(&paths).unwrap();
        let zed = report
            .targets
            .iter()
            .find(|target| target.profile.scope == SkillScope::Zed)
            .unwrap();

        assert_eq!(zed.counts.disabled, 1);
        assert!(!zed.repair_actions.is_empty());
        assert!(
            zed.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("legacy disabled"))
        );
    }

    fn test_paths(root: &std::path::Path) -> ManagerPaths {
        ManagerPaths::with_home(
            root.join("home"),
            root.join("data"),
            root.join("config"),
            Some(ProjectRoot::new(root.join("project"))),
        )
    }
}
