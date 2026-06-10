//! Helper functions for install target resolution, scaffold requests, and search.
//!
//! Contains pure functions that derive install targets from UI state,
//! build scaffold requests from form fields, compute search haystacks,
//! and convert catalog sources into preview-ready entry states.

use std::collections::BTreeMap;

use skills_manager_core::{
    InstallTarget, InstalledSkill, ManagedResource, McpServerRequest, McpServerTransport,
    SkillCatalogSource, SkillScaffoldRequest, SkillScope, catalog_git_install_url,
    installed_skill_identity,
};

use super::state::{CatalogEntryState, CreateState, InstallState, McpState};
use super::types::{InstallSource, resolve_install_target};

/// Resolves the current install target from the install form state.
pub fn current_install_target(install: &InstallState) -> Result<InstallTarget, String> {
    resolve_install_target(install.install_scope, &install.custom_install_path)
}

/// Returns the active source value (URL, local path, download root, or catalog URL) from the install form.
pub fn current_source_value(install: &InstallState) -> String {
    match install.install_source {
        InstallSource::Url => install.source_url.trim().to_string(),
        InstallSource::Local => install.local_source_path.trim().to_string(),
        InstallSource::Downloaded => install
            .selected_download_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        InstallSource::Catalog => install.catalog_url.trim().to_string(),
    }
}

/// Builds a scaffold request from the create form state, validating required fields.
pub fn current_scaffold_request(create: &CreateState) -> Result<SkillScaffoldRequest, String> {
    if create.name.trim().is_empty() {
        return Err("Enter a skill name first.".to_string());
    }
    if create.description.trim().is_empty() {
        return Err("Enter a skill description first.".to_string());
    }

    Ok(SkillScaffoldRequest {
        name: create.name.trim().to_string(),
        description: create.description.trim().to_string(),
        target: resolve_install_target(create.target, &create.custom_path)?,
        tags: split_csv(&create.tags),
        allowed_tools: split_csv(&create.allowed_tools),
        compatibility: optional_string(&create.compatibility),
        license: optional_string(&create.license),
        when_to_use: optional_string(&create.when_to_use),
        disable_model_invocation: create.disable_model_invocation.then_some(true),
    })
}

/// Builds an MCP server request from the MCP form state, validating required fields.
pub fn current_mcp_request(mcp: &McpState) -> Result<McpServerRequest, String> {
    if mcp.name.trim().is_empty() {
        return Err("Enter an MCP server name first.".to_string());
    }

    let (command, url) = match mcp.transport {
        McpServerTransport::Stdio => {
            if mcp.command.trim().is_empty() {
                return Err("Enter a local MCP command first.".to_string());
            }
            (Some(mcp.command.trim().to_string()), None)
        }
        McpServerTransport::Http => {
            if mcp.url.trim().is_empty() {
                return Err("Enter a remote MCP URL first.".to_string());
            }
            (None, Some(mcp.url.trim().to_string()))
        }
    };

    Ok(McpServerRequest {
        name: mcp.name.trim().to_string(),
        target: mcp.target.into(),
        transport: mcp.transport,
        command,
        args: split_args(&mcp.args),
        env: split_key_values(&mcp.env)?,
        url,
        headers: split_key_values(&mcp.headers)?,
        enabled: mcp.enabled,
    })
}

/// Splits a comma-separated string into trimmed, non-empty tokens.
pub fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
        .collect()
}

/// Splits a whitespace-separated string into trimmed, non-empty tokens.
pub fn split_args(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
        .collect()
}

/// Parses comma- or newline-separated `KEY=VALUE` pairs into a sorted map.
pub fn split_key_values(value: &str) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    for raw in value.split([',', '\n']) {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let Some((key, value)) = raw.split_once('=') else {
            return Err(format!("Expected KEY=VALUE, got `{raw}`."));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("KEY cannot be empty in `{raw}`."));
        }
        map.insert(key.to_string(), value.trim().to_string());
    }
    Ok(map)
}

/// Returns `Some(trimmed)` for non-empty strings, `None` otherwise.
pub fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Converts a catalog source entry into a UI-ready catalog entry state with install metadata.
pub fn catalog_entry_from_source(
    name: String,
    description: String,
    source: SkillCatalogSource,
) -> CatalogEntryState {
    match source {
        SkillCatalogSource::Git { url, path } => {
            match catalog_git_install_url(&url, path.as_deref()) {
                Ok(install_url) => CatalogEntryState {
                    name,
                    description,
                    source_label: format!("git: {install_url}"),
                    install_source: Some(InstallSource::Url),
                    source_value: Some(install_url),
                    unavailable_reason: None,
                },
                Err(error) => CatalogEntryState {
                    name,
                    description,
                    source_label: format!("git: {url}"),
                    install_source: None,
                    source_value: None,
                    unavailable_reason: Some(format!("Unavailable: {error}")),
                },
            }
        }
        SkillCatalogSource::Local { path } => CatalogEntryState {
            name,
            description,
            source_label: format!("local: {path}"),
            install_source: Some(InstallSource::Local),
            source_value: Some(path),
            unavailable_reason: None,
        },
        SkillCatalogSource::Unknown => CatalogEntryState {
            name,
            description,
            source_label: "unknown".to_string(),
            install_source: None,
            source_value: None,
            unavailable_reason: Some(
                "Unavailable: catalog entry does not declare a supported source.".to_string(),
            ),
        },
    }
}

/// Builds a lowercase search haystack from a skill's display name, description, path, tools, and tags.
pub fn skill_search_haystack(skill: &InstalledSkill) -> String {
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

/// Builds a lowercase search haystack from a managed resource's kind, target, name, description, and metadata.
pub fn resource_search_haystack(resource: &ManagedResource) -> String {
    let mut haystack = String::new();
    haystack.push_str(resource.kind.label());
    haystack.push(' ');
    haystack.push_str(resource.target.label());
    haystack.push(' ');
    haystack.push_str(&resource.display_name);
    haystack.push(' ');
    haystack.push_str(resource.description.as_deref().unwrap_or_default());
    haystack.push(' ');
    haystack.push_str(resource.source_url.as_deref().unwrap_or_default());
    haystack.push(' ');
    haystack.push_str(&resource.root_dir.display().to_string());
    haystack.push(' ');
    haystack.push_str(
        &resource
            .manifest_file
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
    );
    haystack.push(' ');
    for (key, value) in &resource.metadata {
        haystack.push_str(key);
        haystack.push(' ');
        haystack.push_str(value);
        haystack.push(' ');
    }
    haystack.to_lowercase()
}

/// Maps each skill to its visible scopes (sorted, deduplicated) by identity key.
pub fn visible_scopes_by_id(
    skills: &[InstalledSkill],
) -> std::collections::BTreeMap<String, Vec<SkillScope>> {
    let mut by_identity = std::collections::BTreeMap::<String, Vec<SkillScope>>::new();

    for skill in skills.iter().filter(|skill| skill.is_enabled()) {
        by_identity
            .entry(installed_skill_identity(skill))
            .or_default()
            .push(skill.scope);
    }

    for scopes in by_identity.values_mut() {
        scopes.sort_by_key(|scope| scope.sort_rank());
        scopes.dedup();
    }

    skills
        .iter()
        .map(|skill| {
            let scopes = by_identity
                .get(&installed_skill_identity(skill))
                .cloned()
                .unwrap_or_default();
            (skill.id.clone(), scopes)
        })
        .collect()
}

/// Compares two installed skills by the given sort key for stable ordering.
pub fn compare_skills(
    left: &InstalledSkill,
    right: &InstalledSkill,
    sort_key: super::filters::SortKey,
) -> std::cmp::Ordering {
    use super::filters::SortKey;
    match sort_key {
        SortKey::Priority => left
            .scope
            .sort_rank()
            .cmp(&right.scope.sort_rank())
            .then_with(|| health_rank(left.health).cmp(&health_rank(right.health)))
            .then_with(|| sort_name(left).cmp(&sort_name(right))),
        SortKey::Name => sort_name(left).cmp(&sort_name(right)),
        SortKey::Health => health_rank(left.health)
            .cmp(&health_rank(right.health))
            .then_with(|| sort_name(left).cmp(&sort_name(right))),
        SortKey::Resources => right
            .resource_bytes
            .cmp(&left.resource_bytes)
            .then_with(|| right.resource_count.cmp(&left.resource_count)),
    }
}

fn sort_name(skill: &InstalledSkill) -> String {
    skill.display_name.to_lowercase()
}

fn health_rank(health: skills_manager_core::SkillHealth) -> u8 {
    use skills_manager_core::SkillHealth;
    match health {
        SkillHealth::Invalid => 0,
        SkillHealth::Warning => 1,
        SkillHealth::Shadowed => 2,
        SkillHealth::Valid => 3,
    }
}
