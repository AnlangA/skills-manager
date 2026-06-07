use std::collections::BTreeMap;

use serde::Serialize;

use std::path::PathBuf;

use crate::{
    DoctorReport, DownloadedSkillEntry, InstalledSkill, ManagedResource, ManagerConfig,
    ManagerPaths, MarketplaceSourceRecord, Result, SkillHealth, SkillScope, TargetProfile,
    doctor_report_for_skills, installed_skill_identity, list_downloaded_skills,
    list_marketplace_sources, scan_installed_skills, scan_resources, target_profiles,
};

/// Aggregate counts derived from workspace scan.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WorkspaceCounts {
    /// Total installed skills.
    pub total: usize,
    /// Enabled skills.
    pub enabled: usize,
    /// Disabled skills.
    pub disabled: usize,
    /// Valid skills.
    pub valid: usize,
    /// Warnings-only skills.
    pub warning: usize,
    /// Invalid skills.
    pub invalid: usize,
    /// Shadowed skills.
    pub shadowed: usize,
    /// Skills with source metadata.
    pub known_source: usize,
    /// Exportable skills (enabled and usable).
    pub exportable: usize,
    /// Per-scope skill count map.
    pub by_scope: BTreeMap<SkillScope, usize>,
}

/// Search index entry created per skill.
#[derive(Debug, Clone, Serialize)]
pub struct SkillSearchEntry {
    /// Index into `WorkspaceSnapshot::skills`.
    pub skill_index: usize,
    /// Lowercase concatenated search payload.
    pub haystack: String,
}

/// Internal skill index for fast search and filtering.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SkillIndex {
    /// Skill index by `InstalledSkill::id`.
    pub by_id: BTreeMap<String, usize>,
    /// Skill index keyed by stable identity.
    pub by_identity: BTreeMap<String, Vec<usize>>,
    /// Skill index keyed by scope.
    pub by_scope: BTreeMap<SkillScope, Vec<usize>>,
    /// Skill index keyed by health.
    pub by_health: BTreeMap<SkillHealth, Vec<usize>>,
    /// Skills with known source metadata.
    pub managed: Vec<usize>,
    /// Skills without source metadata.
    pub unknown_source: Vec<usize>,
    /// Flattened lowercase search payload.
    pub search: Vec<SkillSearchEntry>,
}

/// Full workspace snapshot used by CLI output and UI dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSnapshot {
    /// Installed skills across all targets.
    pub skills: Vec<InstalledSkill>,
    /// Managed resources (plugins, marketplaces, etc.).
    pub resources: Vec<ManagedResource>,
    /// Downloaded skill cache entries.
    pub downloads: Vec<DownloadedSkillEntry>,
    /// Configured marketplace sources.
    pub marketplace_sources: Vec<MarketplaceSourceRecord>,
    /// Known target profiles.
    pub target_profiles: Vec<TargetProfile>,
    /// Doctor report generated for the snapshot.
    pub doctor_report: DoctorReport,
    /// Active default download path.
    pub default_download_path: PathBuf,
    /// Precomputed aggregate counts.
    pub counts: WorkspaceCounts,
    /// Precomputed index and lookup caches.
    pub index: SkillIndex,
}

impl WorkspaceSnapshot {
    /// Loads a complete workspace snapshot from manager paths.
    pub fn load(paths: &ManagerPaths) -> Result<Self> {
        let skills = scan_installed_skills(paths)?;
        let resources = scan_resources(paths)?;
        let downloads = list_downloaded_skills(paths)?;
        let marketplace_sources = list_marketplace_sources(paths)?;
        let target_profiles = target_profiles(paths)?;
        let doctor_report = doctor_report_for_skills(paths, target_profiles.clone(), &skills)?;
        let default_download_path = ManagerConfig::load(paths)?.effective_download_dir(paths);
        let counts = WorkspaceCounts::from_skills(&skills);
        let index = SkillIndex::from_skills(&skills);

        Ok(Self {
            skills,
            resources,
            downloads,
            marketplace_sources,
            target_profiles,
            doctor_report,
            default_download_path,
            counts,
            index,
        })
    }
}

impl WorkspaceCounts {
    /// Recomputes summary counts from a skill list.
    pub fn from_skills(skills: &[InstalledSkill]) -> Self {
        skills.iter().fold(
            Self {
                total: skills.len(),
                ..Self::default()
            },
            |mut counts, skill| {
                if skill.is_enabled() {
                    counts.enabled += 1;
                } else {
                    counts.disabled += 1;
                }

                match skill.health {
                    SkillHealth::Valid => counts.valid += 1,
                    SkillHealth::Warning => counts.warning += 1,
                    SkillHealth::Invalid => counts.invalid += 1,
                    SkillHealth::Shadowed => counts.shadowed += 1,
                }

                if skill.source_url.is_some() {
                    counts.known_source += 1;
                }
                if skill.is_exportable() {
                    counts.exportable += 1;
                }

                *counts.by_scope.entry(skill.scope).or_default() += 1;
                counts
            },
        )
    }
}

impl SkillIndex {
    /// Builds a workspace search index from installed skills.
    pub fn from_skills(skills: &[InstalledSkill]) -> Self {
        let mut index = Self::default();

        for (skill_index, skill) in skills.iter().enumerate() {
            index.by_id.insert(skill.id.clone(), skill_index);
            index
                .by_identity
                .entry(installed_skill_identity(skill))
                .or_default()
                .push(skill_index);
            index
                .by_scope
                .entry(skill.scope)
                .or_default()
                .push(skill_index);
            index
                .by_health
                .entry(skill.health)
                .or_default()
                .push(skill_index);

            if skill.source_url.is_some() {
                index.managed.push(skill_index);
            } else {
                index.unknown_source.push(skill_index);
            }

            index.search.push(SkillSearchEntry {
                skill_index,
                haystack: search_haystack(skill),
            });
        }

        index
    }

    /// Case-insensitive search against normalized haystack payload.
    pub fn search<'a>(
        &'a self,
        skills: &'a [InstalledSkill],
        query: &str,
    ) -> Vec<&'a InstalledSkill> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return skills.iter().collect();
        }

        self.search
            .iter()
            .filter(|entry| entry.haystack.contains(&query))
            .filter_map(|entry| skills.get(entry.skill_index))
            .collect()
    }
}

fn search_haystack(skill: &InstalledSkill) -> String {
    let mut haystack = String::new();
    haystack.push_str(&skill.display_name);
    haystack.push(' ');
    haystack.push_str(skill.description.as_deref().unwrap_or_default());
    haystack.push(' ');
    haystack.push_str(&skill.root_dir.display().to_string());
    haystack.push(' ');
    haystack.push_str(&skill.frontmatter.allowed_tools.join(" "));
    haystack.push(' ');
    haystack.push_str(&skill.frontmatter.tags.join(" "));
    haystack.to_lowercase()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use crate::{ManagerPaths, ProjectRoot};

    use super::*;

    #[test]
    fn snapshot_builds_counts_and_search_index() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        write_skill(
            &project,
            "demo",
            "---\nname: demo\ndescription: Use this skill when indexing snapshots\ntags: [fast]\n---\n",
        );
        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(&project)),
        );

        let snapshot = WorkspaceSnapshot::load(&paths).unwrap();

        assert_eq!(snapshot.counts.total, 1);
        assert_eq!(snapshot.counts.enabled, 1);
        assert_eq!(snapshot.index.search(&snapshot.skills, "FAST").len(), 1);
        assert!(snapshot.doctor_report.summary.targets >= 7);
    }

    fn write_skill(project: &Path, name: &str, content: &str) {
        let skill_dir = project.join(".agents/skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }
}
