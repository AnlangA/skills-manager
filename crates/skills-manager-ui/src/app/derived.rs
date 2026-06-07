use skills_manager_core::{InstalledSkill, SkillEnablement, SkillHealth, SkillScope};

use super::filters::SortKey;
use super::helpers::{compare_skills, skill_search_haystack};
use super::state::SkillCounts;

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
