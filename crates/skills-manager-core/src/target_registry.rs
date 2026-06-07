//! Registry wrapper around all available install targets.
//!
//! Provides [`TargetRegistry`], a serialized snapshot of every
//! [`TargetProfile`] converted to [`TargetCapabilities`] for UI and
//! CLI introspection.

use std::path::PathBuf;

use serde::Serialize;

use crate::{
    EnablementStrategy, LayoutPolicy, ManagerPaths, Result, SkillScope, TargetProfile,
    target_profiles,
};

/// Publicly exposed capabilities for a target profile.
#[derive(Debug, Clone, Serialize)]
pub struct TargetCapabilities {
    /// Scope this capability describes.
    pub scope: SkillScope,
    /// Human-readable label.
    pub label: String,
    /// Optional skills root path.
    pub skills_root: Option<PathBuf>,
    /// Optional active disabled store path.
    pub disabled_store_root: Option<PathBuf>,
    /// Optional legacy disabled store path.
    pub legacy_disabled_store_root: Option<PathBuf>,
    /// Strategy used for enable/disable actions.
    pub enablement_strategy: EnablementStrategy,
    /// Directory layout constraints.
    pub layout_policy: LayoutPolicy,
    /// Scope precedence for collision handling.
    pub precedence: usize,
    /// Optional catalog budget constraints.
    pub catalog_budget_bytes: Option<u64>,
    /// Frontmatter fields this target recognizes.
    pub supported_frontmatter: Vec<String>,
    /// Additional target notes for diagnostics.
    pub notes: Vec<String>,
}

/// Serialized registry snapshot used by UI and CLI introspection.
#[derive(Debug, Clone, Serialize)]
pub struct TargetRegistry {
    /// Capabilities for all known targets.
    pub targets: Vec<TargetCapabilities>,
}

impl TargetRegistry {
    /// Loads and materializes target capabilities from manager paths.
    pub fn load(paths: &ManagerPaths) -> Result<Self> {
        Ok(Self {
            targets: target_profiles(paths)?
                .into_iter()
                .map(TargetCapabilities::from)
                .collect(),
        })
    }

    /// Returns capabilities for a specific scope.
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
