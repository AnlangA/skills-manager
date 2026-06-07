use std::{fs, path::PathBuf};

use serde::Serialize;

use crate::{
    InstallTarget, ManagerConfig, ManagerPaths, Result, SkillDiagnostic, SkillFrontmatter,
    SkillHealth, SkillsManagerError,
    codex_config::CodexConfig,
    skill::{health_from_diagnostics, sanitize_folder_name, validate_skill},
    target_specific_diagnostics,
};

#[derive(Debug, Clone)]
pub struct SkillScaffoldRequest {
    pub name: String,
    pub description: String,
    pub target: InstallTarget,
    pub tags: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub compatibility: Option<String>,
    pub license: Option<String>,
    pub when_to_use: Option<String>,
    pub disable_model_invocation: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillScaffoldPreview {
    pub scope: crate::SkillScope,
    pub destination_root: PathBuf,
    pub skill_file: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub content: String,
    pub diagnostics: Vec<SkillDiagnostic>,
    pub health: SkillHealth,
    pub conflict: bool,
}

#[derive(Debug, Serialize)]
struct ScaffoldFrontmatter<'a> {
    name: &'a str,
    description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compatibility: Option<&'a str>,
    #[serde(rename = "allowed-tools", skip_serializing_if = "Vec::is_empty")]
    allowed_tools: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when_to_use: Option<&'a str>,
    #[serde(
        rename = "disable-model-invocation",
        skip_serializing_if = "Option::is_none"
    )]
    disable_model_invocation: Option<bool>,
}

pub fn preview_skill_scaffold(
    paths: &ManagerPaths,
    request: SkillScaffoldRequest,
) -> Result<SkillScaffoldPreview> {
    let destination_root = request.target.destination_root(paths)?;
    let folder_name = scaffold_folder_name(&request.name);
    let skill_root = destination_root.join(folder_name);
    let skill_file = skill_root.join("SKILL.md");
    let frontmatter = SkillFrontmatter {
        name: Some(request.name.trim().to_string()),
        description: Some(request.description.trim().to_string()),
        license: request.license.as_deref().map(str::trim).map(String::from),
        compatibility: request
            .compatibility
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(String::from),
        allowed_tools: request
            .allowed_tools
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        tags: request
            .tags
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        disable_model_invocation: request.disable_model_invocation,
        when_to_use: request
            .when_to_use
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(String::from),
        metadata: Default::default(),
    };
    let content = scaffold_content(&frontmatter)?;
    let mut diagnostics = validate_skill(&skill_root, &frontmatter, 0, 0);
    diagnostics.extend(target_specific_diagnostics(
        request.target.scope(),
        &skill_root,
        &frontmatter,
        0,
        0,
    ));
    let health = health_from_diagnostics(&diagnostics, false);

    Ok(SkillScaffoldPreview {
        scope: request.target.scope(),
        destination_root: skill_root,
        skill_file,
        frontmatter,
        content,
        diagnostics,
        health,
        conflict: false,
    })
}

pub fn create_skill_scaffold(
    paths: &ManagerPaths,
    request: SkillScaffoldRequest,
) -> Result<SkillScaffoldPreview> {
    let mut preview = preview_skill_scaffold(paths, request)?;
    preview.conflict = preview.destination_root.exists();
    if preview.conflict {
        return Err(SkillsManagerError::DestinationExists(
            preview.destination_root.clone(),
        ));
    }

    fs::create_dir_all(&preview.destination_root)?;
    fs::write(&preview.skill_file, &preview.content)?;

    let mut config = ManagerConfig::load(paths)?;
    if preview.scope == crate::SkillScope::Custom {
        if let Some(root) = preview.destination_root.parent() {
            config.record_custom_install_root(root);
        }
    }
    config.record_install(&preview.destination_root, None);
    config.set_disabled(&preview.skill_file, false);
    config.save(paths)?;

    if preview.scope == crate::SkillScope::Codex {
        let mut codex_config = CodexConfig::load(paths)?;
        codex_config.set_enabled(&preview.skill_file, true);
        codex_config.save()?;
    }

    Ok(preview)
}

fn scaffold_folder_name(name: &str) -> String {
    let sanitized = sanitize_folder_name(name);
    if sanitized.is_empty() {
        "skill".to_string()
    } else {
        sanitized
    }
}

fn scaffold_content(frontmatter: &SkillFrontmatter) -> Result<String> {
    let yaml = serde_yaml::to_string(&ScaffoldFrontmatter {
        name: frontmatter.name.as_deref().unwrap_or("skill"),
        description: frontmatter.description.as_deref().unwrap_or_default(),
        license: frontmatter.license.as_deref(),
        compatibility: frontmatter.compatibility.as_deref(),
        allowed_tools: frontmatter
            .allowed_tools
            .iter()
            .map(String::as_str)
            .collect(),
        tags: frontmatter.tags.iter().map(String::as_str).collect(),
        when_to_use: frontmatter.when_to_use.as_deref(),
        disable_model_invocation: frontmatter.disable_model_invocation,
    })?;
    Ok(format!(
        "---\n{}---\n\n# {}\n\n{}\n",
        yaml,
        frontmatter.name.as_deref().unwrap_or("Skill"),
        frontmatter.description.as_deref().unwrap_or_default()
    ))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::{InstallTarget, ManagerPaths, ProjectRoot};

    use super::*;

    #[test]
    fn preview_scaffold_includes_target_specific_fields() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path());
        let preview = preview_skill_scaffold(
            &paths,
            SkillScaffoldRequest {
                name: "demo".to_string(),
                description: "Use this skill when testing scaffold generation".to_string(),
                target: InstallTarget::ClaudeCode,
                tags: vec!["docs".to_string()],
                allowed_tools: vec!["shell".to_string()],
                compatibility: Some("Claude Code".to_string()),
                license: Some("MIT".to_string()),
                when_to_use: Some("Use when testing Claude Code scaffolds".to_string()),
                disable_model_invocation: Some(true),
            },
        )
        .unwrap();

        assert_eq!(preview.scope, crate::SkillScope::ClaudeCode);
        assert!(preview.content.contains("disable-model-invocation: true"));
        assert!(preview.content.contains("when_to_use:"));
        assert_eq!(preview.health, SkillHealth::Valid);
    }

    #[test]
    fn create_scaffold_refuses_existing_destination() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path());
        let request = SkillScaffoldRequest {
            name: "demo".to_string(),
            description: "Use this skill when testing scaffold conflicts".to_string(),
            target: InstallTarget::Global,
            tags: Vec::new(),
            allowed_tools: Vec::new(),
            compatibility: None,
            license: None,
            when_to_use: None,
            disable_model_invocation: None,
        };

        create_skill_scaffold(&paths, request.clone()).unwrap();
        let error = create_skill_scaffold(&paths, request).unwrap_err();

        assert!(error.to_string().contains("already exists"));
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
