use std::path::PathBuf;

use serde::Serialize;

use crate::{
    EnablementStrategy, LayoutPolicy, ManagerPaths, Result, SkillScope, TargetProfile,
    target_profiles,
};

#[derive(Debug, Clone, Serialize)]
pub struct TargetCapabilities {
    pub scope: SkillScope,
    pub label: String,
    pub skills_root: Option<PathBuf>,
    pub disabled_store_root: Option<PathBuf>,
    pub legacy_disabled_store_root: Option<PathBuf>,
    pub enablement_strategy: EnablementStrategy,
    pub layout_policy: LayoutPolicy,
    pub precedence: usize,
    pub catalog_budget_bytes: Option<u64>,
    pub supported_frontmatter: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetRegistry {
    pub targets: Vec<TargetCapabilities>,
}

impl TargetRegistry {
    pub fn load(paths: &ManagerPaths) -> Result<Self> {
        Ok(Self {
            targets: target_profiles(paths)?
                .into_iter()
                .map(TargetCapabilities::from)
                .collect(),
        })
    }

    pub fn get(&self, scope: SkillScope) -> Option<&TargetCapabilities> {
        self.targets.iter().find(|target| target.scope == scope)
    }
}

impl From<TargetProfile> for TargetCapabilities {
    fn from(profile: TargetProfile) -> Self {
        Self {
            scope: profile.scope,
            label: profile.label,
            skills_root: profile.skills_root,
            disabled_store_root: profile.disabled_store_root,
            legacy_disabled_store_root: profile.legacy_disabled_store_root,
            enablement_strategy: profile.enablement_strategy,
            layout_policy: profile.layout_policy,
            precedence: profile.precedence,
            catalog_budget_bytes: profile.catalog_budget_bytes,
            supported_frontmatter: profile.supported_frontmatter,
            notes: profile.notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::{ManagerPaths, ProjectRoot};

    use super::*;

    #[test]
    fn registry_exposes_builtin_target_capabilities() {
        let dir = tempdir().unwrap();
        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(dir.path().join("project"))),
        );

        let registry = TargetRegistry::load(&paths).unwrap();

        let codex = registry.get(SkillScope::Codex).unwrap();
        assert_eq!(codex.enablement_strategy, EnablementStrategy::ConfigToggle);
        assert!(
            registry
                .get(SkillScope::Zed)
                .unwrap()
                .catalog_budget_bytes
                .is_some()
        );
        assert!(registry.targets.len() >= 7);
    }
}
