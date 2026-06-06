use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_yaml::Value;
use walkdir::WalkDir;

use crate::{
    DiagnosticSeverity, ManagerConfig, ManagerPaths, Result, SkillDiagnostic, SkillEnablement,
    SkillFrontmatter, SkillHealth, SkillScope, SkillsManagerError, model::InstalledSkill,
};

const MAX_NAME_LENGTH: usize = 64;
const MAX_DESCRIPTION_LENGTH: usize = 1024;
const MIN_DESCRIPTION_LENGTH: usize = 12;
const MAX_COMPATIBILITY_LENGTH: usize = 1000;
const RESOURCE_WARNING_BYTES: u64 = 25 * 1024 * 1024;
const RESOURCE_WARNING_FILES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCandidate {
    pub root_dir: PathBuf,
    pub skill_file: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub diagnostics: Vec<SkillDiagnostic>,
    pub health: SkillHealth,
    pub resource_count: usize,
    pub resource_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    name: Option<Value>,
    description: Option<Value>,
    license: Option<Value>,
    compatibility: Option<Value>,
    #[serde(default, rename = "allowed-tools")]
    allowed_tools: Option<Value>,
    #[serde(flatten)]
    metadata: BTreeMap<String, Value>,
}

pub fn scan_installed_skills(paths: &ManagerPaths) -> Result<Vec<InstalledSkill>> {
    let app_config = ManagerConfig::load(paths)?;
    let mut skills = Vec::new();

    for (scope, root) in paths.skill_roots() {
        if !root.exists() {
            continue;
        }

        for candidate in discover_skill_candidates(&root)? {
            skills.push(read_installed_skill(
                scope,
                candidate.root_dir,
                candidate.frontmatter,
                candidate.diagnostics,
                candidate.resource_count,
                candidate.resource_bytes,
                &app_config,
            )?);
        }
    }

    sort_skills(&mut skills);
    mark_shadowed_skills(&mut skills);
    Ok(skills)
}

pub fn read_installed_skill(
    scope: SkillScope,
    root_dir: PathBuf,
    frontmatter: SkillFrontmatter,
    diagnostics: Vec<SkillDiagnostic>,
    resource_count: usize,
    resource_bytes: u64,
    app_config: &ManagerConfig,
) -> Result<InstalledSkill> {
    let skill_file = root_dir.join("SKILL.md");
    if !skill_file.exists() {
        return Err(SkillsManagerError::MissingSkillFile(root_dir));
    }

    let folder_name = root_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-skill")
        .to_string();
    let display_name = frontmatter.name.clone().unwrap_or(folder_name);
    let id = format!("{}:{}", scope.id_prefix(), root_dir.display());
    let enablement = if app_config.is_disabled(&skill_file) {
        SkillEnablement::Disabled
    } else {
        SkillEnablement::Enabled
    };
    let metadata = app_config.installed.get(&path_key(&root_dir));
    let health = health_from_diagnostics(&diagnostics, false);

    Ok(InstalledSkill {
        id,
        display_name,
        description: frontmatter.description.clone(),
        scope,
        root_dir,
        skill_file,
        frontmatter,
        enablement,
        health,
        diagnostics,
        resource_count,
        resource_bytes,
        shadowed_by: None,
        source_url: metadata.and_then(|item| item.source_url.clone()),
        installed_at: metadata.and_then(|item| item.installed_at),
    })
}

pub fn discover_skill_candidates(root: &Path) -> Result<Vec<SkillCandidate>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
            continue;
        }

        let skill_file = entry.path().to_path_buf();
        let Some(root_dir) = skill_file.parent().map(Path::to_path_buf) else {
            continue;
        };

        candidates.push(read_skill_candidate(root_dir, skill_file));
    }

    candidates.sort_by(|left, right| left.root_dir.cmp(&right.root_dir));
    Ok(candidates)
}

pub fn read_skill_candidate(root_dir: PathBuf, skill_file: PathBuf) -> SkillCandidate {
    let (frontmatter, mut diagnostics) = match parse_skill_frontmatter(&skill_file) {
        Ok(frontmatter) => (frontmatter, Vec::new()),
        Err(error) => (
            SkillFrontmatter::default(),
            vec![SkillDiagnostic::invalid(format!(
                "Could not parse SKILL.md frontmatter: {error}"
            ))],
        ),
    };
    let (resource_count, resource_bytes) = resource_summary(&root_dir);
    diagnostics.extend(validate_skill(
        &root_dir,
        &frontmatter,
        resource_count,
        resource_bytes,
    ));
    let health = health_from_diagnostics(&diagnostics, false);

    SkillCandidate {
        root_dir,
        skill_file,
        frontmatter,
        diagnostics,
        health,
        resource_count,
        resource_bytes,
    }
}

pub fn parse_skill_frontmatter(skill_file: &Path) -> Result<SkillFrontmatter> {
    let content = fs::read_to_string(skill_file)?;
    let Some(frontmatter) = extract_frontmatter(&content) else {
        return Ok(SkillFrontmatter::default());
    };

    let raw: RawFrontmatter =
        serde_yaml::from_str(frontmatter).map_err(|source| SkillsManagerError::ParseYaml {
            path: skill_file.to_path_buf(),
            source,
        })?;

    Ok(SkillFrontmatter {
        name: raw.name.as_ref().and_then(scalar_to_string),
        description: raw.description.as_ref().and_then(scalar_to_string),
        license: raw.license.as_ref().and_then(scalar_to_string),
        compatibility: raw.compatibility.as_ref().and_then(scalar_to_string),
        allowed_tools: raw
            .allowed_tools
            .as_ref()
            .map(value_to_string_list)
            .unwrap_or_default(),
        metadata: raw
            .metadata
            .iter()
            .filter_map(|(key, value)| value_to_metadata(value).map(|value| (key.clone(), value)))
            .collect(),
    })
}

pub fn validate_skill(
    root_dir: &Path,
    frontmatter: &SkillFrontmatter,
    resource_count: usize,
    resource_bytes: u64,
) -> Vec<SkillDiagnostic> {
    let mut diagnostics = Vec::new();
    let folder_name = root_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    match frontmatter.name.as_deref().map(str::trim) {
        Some("") | None => {
            diagnostics.push(SkillDiagnostic::invalid(
                "SKILL.md frontmatter is missing `name`",
            ));
        }
        Some(name) => {
            if name.chars().count() > MAX_NAME_LENGTH {
                diagnostics.push(SkillDiagnostic::invalid(format!(
                    "`name` must be {MAX_NAME_LENGTH} characters or fewer"
                )));
            }
            if !is_valid_skill_name(name) {
                diagnostics.push(SkillDiagnostic::invalid(
                    "`name` must use lowercase letters, numbers, hyphens, or underscores",
                ));
            }
            if sanitize_folder_name(name) != folder_name {
                diagnostics.push(SkillDiagnostic::warning(format!(
                    "Folder `{folder_name}` does not match normalized skill name `{}`",
                    sanitize_folder_name(name)
                )));
            }
        }
    }

    match frontmatter.description.as_deref().map(str::trim) {
        Some("") | None => {
            diagnostics.push(SkillDiagnostic::invalid(
                "SKILL.md frontmatter is missing `description`",
            ));
        }
        Some(description) => {
            if description.chars().count() > MAX_DESCRIPTION_LENGTH {
                diagnostics.push(SkillDiagnostic::invalid(format!(
                    "`description` must be {MAX_DESCRIPTION_LENGTH} characters or fewer"
                )));
            } else if description.chars().count() < MIN_DESCRIPTION_LENGTH {
                diagnostics.push(SkillDiagnostic::warning(format!(
                    "`description` is very short; use at least {MIN_DESCRIPTION_LENGTH} characters"
                )));
            }
        }
    }

    if frontmatter
        .compatibility
        .as_deref()
        .is_some_and(|value| value.chars().count() > MAX_COMPATIBILITY_LENGTH)
    {
        diagnostics.push(SkillDiagnostic::warning(format!(
            "`compatibility` is longer than {MAX_COMPATIBILITY_LENGTH} characters"
        )));
    }

    if frontmatter.allowed_tools.len() > 50 {
        diagnostics.push(SkillDiagnostic::warning(
            "`allowed-tools` lists more than 50 entries",
        ));
    }

    if resource_count > RESOURCE_WARNING_FILES {
        diagnostics.push(SkillDiagnostic::warning(format!(
            "Skill contains {resource_count} resource files; keep bundles focused"
        )));
    }
    if resource_bytes > RESOURCE_WARNING_BYTES {
        diagnostics.push(SkillDiagnostic::warning(format!(
            "Skill resources are {}; keep bundles under 25 MiB when possible",
            format_bytes(resource_bytes)
        )));
    }

    diagnostics
}

pub fn health_from_diagnostics(diagnostics: &[SkillDiagnostic], shadowed: bool) -> SkillHealth {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Invalid)
    {
        SkillHealth::Invalid
    } else if shadowed {
        SkillHealth::Shadowed
    } else if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
    {
        SkillHealth::Warning
    } else {
        SkillHealth::Valid
    }
}

pub fn path_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for next in UNITS.iter().skip(1) {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = *next;
    }

    if unit == "B" {
        format!("{bytes} {unit}")
    } else {
        format!("{value:.1} {unit}")
    }
}

fn mark_shadowed_skills(skills: &mut [InstalledSkill]) {
    let mut winners: HashMap<String, (PathBuf, usize)> = HashMap::new();

    for index in 0..skills.len() {
        let key = skill_identity(&skills[index]);
        if let Some((winner_path, _winner_index)) = winners.get(&key) {
            skills[index].shadowed_by = Some(winner_path.clone());
            skills[index]
                .diagnostics
                .push(SkillDiagnostic::warning(format!(
                    "Shadowed by higher-priority skill at {}",
                    winner_path.display()
                )));
            skills[index].health = health_from_diagnostics(&skills[index].diagnostics, true);
        } else {
            winners.insert(key, (skills[index].root_dir.clone(), index));
        }
    }
}

fn sort_skills(skills: &mut [InstalledSkill]) {
    skills.sort_by(|left, right| {
        left.scope
            .sort_rank()
            .cmp(&right.scope.sort_rank())
            .then_with(|| {
                left.display_name
                    .to_lowercase()
                    .cmp(&right.display_name.to_lowercase())
            })
            .then_with(|| left.root_dir.cmp(&right.root_dir))
    });
}

fn skill_identity(skill: &InstalledSkill) -> String {
    skill
        .frontmatter
        .name
        .as_deref()
        .map(sanitize_folder_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| sanitize_folder_name(&skill.destination_name()))
}

fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;
    let end = content.find("\n---").or_else(|| content.find("\r\n---"))?;
    Some(&content[..end])
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(normalize_scalar(value.clone())),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
    .filter(|value| !value.is_empty())
}

fn value_to_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::Sequence(items) => items.iter().filter_map(scalar_to_string).collect(),
        Value::String(value) => value
            .split(',')
            .map(normalize_scalar)
            .filter(|value| !value.is_empty())
            .collect(),
        other => scalar_to_string(other).into_iter().collect(),
    }
}

fn value_to_metadata(value: &Value) -> Option<String> {
    scalar_to_string(value).or_else(|| {
        serde_yaml::to_string(value).ok().map(|raw| {
            raw.lines()
                .filter(|line| *line != "---")
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
    })
}

fn normalize_scalar(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

fn is_valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

fn resource_summary(root_dir: &Path) -> (usize, u64) {
    let mut count = 0;
    let mut bytes = 0;

    for entry in WalkDir::new(root_dir)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() || entry.file_name() == "SKILL.md" {
            continue;
        }

        count += 1;
        if let Ok(metadata) = entry.metadata() {
            bytes += metadata.len();
        }
    }

    (count, bytes)
}

pub(crate) fn unique_folder_name(
    destination_root: &Path,
    preferred_name: &str,
    installed: &HashMap<String, ()>,
) -> String {
    let mut sanitized = sanitize_folder_name(preferred_name);
    if sanitized.is_empty() {
        sanitized = "skill".to_string();
    }

    if !destination_root.join(&sanitized).exists() && !installed.contains_key(&sanitized) {
        return sanitized;
    }

    for index in 2.. {
        let candidate = format!("{sanitized}-{index}");
        if !destination_root.join(&candidate).exists() && !installed.contains_key(&candidate) {
            return candidate;
        }
    }

    unreachable!("the loop always returns")
}

pub(crate) fn sanitize_folder_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{ProjectRoot, SkillScope};

    use super::*;

    #[test]
    fn parses_frontmatter_with_agent_skill_fields() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("SKILL.md");
        fs::write(
            &file,
            "---\nname: demo-skill\ndescription: Does useful work\nlicense: MIT\ncompatibility: Works everywhere\nallowed-tools:\n  - shell\n  - browser\nowner: team-ai\n---\n# Demo\n",
        )
        .unwrap();

        let parsed = parse_skill_frontmatter(&file).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("demo-skill"));
        assert_eq!(parsed.description.as_deref(), Some("Does useful work"));
        assert_eq!(parsed.license.as_deref(), Some("MIT"));
        assert_eq!(parsed.allowed_tools, vec!["shell", "browser"]);
        assert_eq!(
            parsed.metadata.get("owner").map(String::as_str),
            Some("team-ai")
        );
    }

    #[test]
    fn discovers_nested_skill_folders() {
        let dir = tempdir().unwrap();
        let skill = dir.path().join("skills").join("demo-skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: demo-skill\ndescription: Demo skill\n---\n",
        )
        .unwrap();

        let candidates = discover_skill_candidates(dir.path()).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].root_dir, skill);
    }

    #[test]
    fn validates_missing_description_as_invalid() {
        let dir = tempdir().unwrap();
        let diagnostics = validate_skill(
            dir.path(),
            &SkillFrontmatter {
                name: Some("demo".to_string()),
                ..SkillFrontmatter::default()
            },
            0,
            0,
        );

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Invalid
                && diagnostic.message.contains("description")
        }));
    }

    #[test]
    fn warns_when_folder_name_does_not_match_skill_name() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("different-folder");
        fs::create_dir_all(&root).unwrap();
        let diagnostics = validate_skill(
            &root,
            &SkillFrontmatter {
                name: Some("demo-skill".to_string()),
                description: Some("A useful demo skill".to_string()),
                ..SkillFrontmatter::default()
            },
            0,
            0,
        );

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Warning
                && diagnostic.message.contains("does not match")
        }));
    }

    #[test]
    fn project_skill_shadows_user_skill_with_same_name() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        let user_skill = dir.path().join("home/.agents/skills/demo-skill");
        let project_skill = project.join(".agents/skills/demo-skill");
        fs::create_dir_all(&user_skill).unwrap();
        fs::create_dir_all(&project_skill).unwrap();
        let skill_doc = "---\nname: demo-skill\ndescription: A useful demo skill\n---\n";
        fs::write(user_skill.join("SKILL.md"), skill_doc).unwrap();
        fs::write(project_skill.join("SKILL.md"), skill_doc).unwrap();

        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(&project)),
        );
        let skills = scan_installed_skills(&paths).unwrap();

        let user = skills
            .iter()
            .find(|skill| skill.scope == SkillScope::User)
            .unwrap();
        assert_eq!(user.health, SkillHealth::Shadowed);
        assert!(user.shadowed_by.is_some());
    }

    #[test]
    fn sanitizes_folder_names() {
        assert_eq!(sanitize_folder_name("Demo Skill!"), "demo-skill");
        assert_eq!(sanitize_folder_name("  ___ "), "___");
    }
}
