//! Derived inventory state computed from the raw skill and resource lists.
//!
//! Provides aggregation functions for skill counts, filtered index
//! computation, sort ordering, and cross-scope visibility lookups
//! used by the library and plugin views.

use skills_manager_core::{
    InstalledSkill, ManagedResource, ResourceHealth, ResourceKind, SkillEnablement, SkillHealth,
    SkillScope,
};

use super::filters::{HealthFilter, PluginTargetFilter, SortKey};
use super::helpers::{compare_skills, resource_search_haystack, skill_search_haystack};
use super::state::{ResourceSearchEntry, SkillCounts};

pub fn counts_from_skills(skills: &[InstalledSkill]) -> SkillCounts {
    skills
        .iter()
        .fold(SkillCounts::default(), |mut counts, skill| {
            match skill.enablement {
                SkillEnablement::Enabled => counts.enabled += 1,
                SkillEnablement::Disabled => counts.disabled += 1,
            }
            match skill.health {
                SkillHealth::Valid => counts.valid += 1,
                SkillHealth::Warning => counts.warning += 1,
                SkillHealth::Invalid => counts.invalid += 1,
                SkillHealth::Shadowed => counts.shadowed += 1,
            }
            match skill.scope {
                SkillScope::Project => counts.project += 1,
                SkillScope::Global => counts.global += 1,
                SkillScope::ClaudeCode => counts.claude_code += 1,
                SkillScope::Droid => counts.droid += 1,
                SkillScope::OpenCode => counts.opencode += 1,
                SkillScope::Codex => counts.codex += 1,
                SkillScope::Zed => counts.zed += 1,
                SkillScope::Custom => counts.custom += 1,
            }
            if skill.source_url.is_some() {
                counts.known_source += 1;
            }
            if skill.is_exportable() {
                counts.exportable += 1;
            }
            counts
        })
}

pub fn filtered_indices(
    skills: &[InstalledSkill],
    search_query: &str,
    scope_filter: super::filters::ScopeFilter,
    health_filter: super::filters::HealthFilter,
    source_filter: super::filters::SourceFilter,
    snapshot: Option<&skills_manager_core::WorkspaceSnapshot>,
) -> Vec<usize> {
    let query = search_query.trim().to_lowercase();
    if query.is_empty() {
        return skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| {
                skill_matches_filters(skill, scope_filter, health_filter, source_filter)
                    .then_some(index)
            })
            .collect();
    }

    if let Some(snapshot) = snapshot {
        return snapshot
            .index
            .search
            .iter()
            .filter(|entry| entry.haystack.contains(&query))
            .filter_map(|entry| {
                skills
                    .get(entry.skill_index)
                    .filter(|skill| {
                        skill_matches_filters(skill, scope_filter, health_filter, source_filter)
                    })
                    .map(|_| entry.skill_index)
            })
            .collect();
    }

    skills
        .iter()
        .enumerate()
        .filter_map(|(index, skill)| {
            (skill_matches_filters(skill, scope_filter, health_filter, source_filter)
                && skill_matches_query(skill, &query))
            .then_some(index)
        })
        .collect()
}

pub fn resource_search_index(resources: &[ManagedResource]) -> Vec<ResourceSearchEntry> {
    resources
        .iter()
        .enumerate()
        .map(|(resource_index, resource)| ResourceSearchEntry {
            resource_index,
            kind: resource.kind,
            haystack: resource_search_haystack(resource),
        })
        .collect()
}

pub fn filtered_plugin_indices(
    resources: &[ManagedResource],
    search_index: &[ResourceSearchEntry],
    search_query: &str,
    target_filter: PluginTargetFilter,
    health_filter: HealthFilter,
    sort_key: SortKey,
) -> Vec<usize> {
    let query = search_query.trim().to_lowercase();
    let mut indices = search_index
        .iter()
        .filter(|entry| entry.kind == ResourceKind::Plugin)
        .filter(|entry| query.is_empty() || entry.haystack.contains(&query))
        .filter_map(|entry| {
            resources
                .get(entry.resource_index)
                .filter(|resource| target_filter.matches(resource.target))
                .filter(|resource| health_filter.matches(resource_health_as_skill(resource.health)))
                .map(|_| entry.resource_index)
        })
        .collect::<Vec<_>>();
    sort_resource_indices(&mut indices, resources, sort_key);
    indices
}

pub fn filtered_marketplace_indices(
    resources: &[ManagedResource],
    search_index: &[ResourceSearchEntry],
    search_query: &str,
) -> Vec<usize> {
    let query = search_query.trim().to_lowercase();
    let mut indices = search_index
        .iter()
        .filter(|entry| entry.kind == ResourceKind::Marketplace)
        .filter(|entry| query.is_empty() || entry.haystack.contains(&query))
        .map(|entry| entry.resource_index)
        .collect::<Vec<_>>();
    sort_resource_indices(&mut indices, resources, SortKey::Name);
    indices
}

fn resource_health_as_skill(health: ResourceHealth) -> SkillHealth {
    match health {
        ResourceHealth::Valid => SkillHealth::Valid,
        ResourceHealth::Warning => SkillHealth::Warning,
        ResourceHealth::Invalid => SkillHealth::Invalid,
    }
}

fn skill_matches_filters(
    skill: &InstalledSkill,
    scope_filter: super::filters::ScopeFilter,
    health_filter: super::filters::HealthFilter,
    source_filter: super::filters::SourceFilter,
) -> bool {
    scope_filter.matches(skill.scope)
        && health_filter.matches(skill.health)
        && source_filter.matches(skill)
}

fn skill_matches_query(skill: &InstalledSkill, query: &str) -> bool {
    skill_search_haystack(skill).contains(query)
}

pub fn sort_skill_indices(indices: &mut [usize], skills: &[InstalledSkill], sort_key: SortKey) {
    indices.sort_by(|left, right| compare_skills(&skills[*left], &skills[*right], sort_key));
}

fn sort_resource_indices(indices: &mut [usize], resources: &[ManagedResource], sort_key: SortKey) {
    indices
        .sort_by(|left, right| compare_resources(&resources[*left], &resources[*right], sort_key));
}

fn compare_resources(
    left: &ManagedResource,
    right: &ManagedResource,
    sort_key: SortKey,
) -> std::cmp::Ordering {
    match sort_key {
        SortKey::Priority => left
            .target
            .cmp(&right.target)
            .then_with(|| {
                resource_health_rank(left.health).cmp(&resource_health_rank(right.health))
            })
            .then_with(|| resource_sort_name(left).cmp(&resource_sort_name(right))),
        SortKey::Name | SortKey::Resources => {
            resource_sort_name(left).cmp(&resource_sort_name(right))
        }
        SortKey::Health => resource_health_rank(left.health)
            .cmp(&resource_health_rank(right.health))
            .then_with(|| resource_sort_name(left).cmp(&resource_sort_name(right))),
    }
}

fn resource_sort_name(resource: &ManagedResource) -> String {
    resource.display_name.to_lowercase()
}

fn resource_health_rank(health: ResourceHealth) -> u8 {
    match health {
        ResourceHealth::Invalid => 0,
        ResourceHealth::Warning => 1,
        ResourceHealth::Valid => 2,
    }
}
