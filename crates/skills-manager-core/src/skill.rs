use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_yaml::Value;
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::{
    DiagnosticSeverity, ManagerConfig, ManagerPaths, Result, SkillDiagnostic, SkillEnablement,
    SkillFrontmatter, SkillHealth, SkillScope, SkillsManagerError, codex_config::CodexConfig,
    model::InstalledSkill, target_specific_diagnostics,
};

const MAX_NAME_LENGTH: usize = 64;
const MAX_DESCRIPTION_LENGTH: usize = 1024;
const MIN_DESCRIPTION_LENGTH: usize = 12;
const MAX_COMPATIBILITY_LENGTH: usize = 1000;
const RESOURCE_WARNING_BYTES: u64 = 25 * 1024 * 1024;
const RESOURCE_WARNING_FILES: usize = 100;
/// Directory used for disabled skill bundles in current layout.
pub const DISABLED_SKILLS_DIR: &str = ".skills-disabled";
/// Legacy disabled skill directory retained for backward compatibility.
pub const LEGACY_DISABLED_SKILLS_DIR: &str = ".disabled";

/// A candidate discovered from a skill directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCandidate {
    /// Root directory of the candidate.
    pub root_dir: PathBuf,
    /// Absolute path to `SKILL.md`.
    pub skill_file: PathBuf,
    /// Parsed frontmatter metadata.
    pub frontmatter: SkillFrontmatter,
    /// Normalized candidate name used for sorting and duplicate suppression.
    pub normalized_name: String,
    /// Search tokens used for fuzzy/substring skill inventory search.
    pub search_haystack: String,
    /// Frontmatter and structure validation diagnostics.
    pub diagnostics: Vec<SkillDiagnostic>,
    /// Derived health state.
    pub health: SkillHealth,
    /// Resource file count for this candidate.
    pub resource_count: usize,
    /// Resource byte size for this candidate.
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
    #[serde(default)]
    tags: Option<Value>,
    #[serde(default, rename = "disable-model-invocation")]
    disable_model_invocation: Option<Value>,
    #[serde(default, alias = "when-to-use")]
    when_to_use: Option<Value>,
    #[serde(flatten)]
    metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ResourceSummary {
    count: usize,
    bytes: u64,
}

/// Scan all installed skills across known roots.
///
/// This loads manager and Codex configs, discovers enabled/disabled candidates for each root,
/// applies diagnostics, sorts by target priority, and marks shadowed identities.
pub fn scan_installed_skills(paths: &ManagerPaths) -> Result<Vec<InstalledSkill>> {
    info!("scanning installed skills");
    let app_config = ManagerConfig::load(paths)?;
    let codex_config = CodexConfig::load(paths)?;
    let mut skills = Vec::new();

    let mut roots = paths.skill_roots();
    for root in &app_config.custom_install_roots {
        roots.push((SkillScope::Custom, root.clone()));
    }

    for (scope, root) in dedupe_roots(roots) {
        if !root.exists() {
            debug!(scope = scope.label(), root = %root.display(), "skill root does not exist");
            continue;
        }

        debug!(scope = scope.label(), root = %root.display(), "scanning skill root");
        for candidate in discover_skill_candidates(&root)? {
            skills.push(read_installed_skill(
                scope,
                candidate,
                &app_config,
                &codex_config,
            )?);
        }

        for candidate in discover_disabled_skill_candidates(&root)? {
            skills.push(read_disabled_installed_skill(
                scope,
                &root,
                candidate,
                &app_config,
                &codex_config,
            )?);
        }
    }

    sort_skills(&mut skills);
    mark_shadowed_skills(&mut skills);
    info!(count = skills.len(), "scanned installed skills");
    Ok(skills)
}

fn read_installed_skill(
    scope: SkillScope,
    candidate: SkillCandidate,
    app_config: &ManagerConfig,
    codex_config: &CodexConfig,
) -> Result<InstalledSkill> {
    read_installed_skill_with_options(
        InstalledSkillInput {
            scope,
            root_dir: candidate.root_dir,
            frontmatter: candidate.frontmatter,
            diagnostics: candidate.diagnostics,
            resource_count: candidate.resource_count,
            resource_bytes: candidate.resource_bytes,
        },
        app_config,
        codex_config,
        None,
        &[],
    )
}

#[derive(Debug)]
struct InstalledSkillInput {
    scope: SkillScope,
    root_dir: PathBuf,
    frontmatter: SkillFrontmatter,
    diagnostics: Vec<SkillDiagnostic>,
    resource_count: usize,
    resource_bytes: u64,
}

fn read_disabled_installed_skill(
    scope: SkillScope,
    skills_root: &Path,
    candidate: SkillCandidate,
    app_config: &ManagerConfig,
    codex_config: &CodexConfig,
) -> Result<InstalledSkill> {
    let enabled_root = enabled_root_for_disabled_root(skills_root, &candidate.root_dir);
    read_installed_skill_with_options(
        InstalledSkillInput {
            scope,
            root_dir: candidate.root_dir,
            frontmatter: candidate.frontmatter,
            diagnostics: candidate.diagnostics,
            resource_count: candidate.resource_count,
            resource_bytes: candidate.resource_bytes,
        },
        app_config,
        codex_config,
        Some(SkillEnablement::Disabled),
        &[enabled_root],
    )
}

fn read_installed_skill_with_options(
    input: InstalledSkillInput,
    app_config: &ManagerConfig,
    codex_config: &CodexConfig,
    enablement_override: Option<SkillEnablement>,
    metadata_fallback_roots: &[PathBuf],
) -> Result<InstalledSkill> {
    let InstalledSkillInput {
        scope,
        root_dir,
        frontmatter,
        mut diagnostics,
        resource_count,
        resource_bytes,
    } = input;
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
    let enablement = enablement_override.unwrap_or_else(|| {
        if app_config.is_disabled(&skill_file) || codex_config.is_disabled(&skill_file) {
            SkillEnablement::Disabled
        } else {
            SkillEnablement::Enabled
        }
    });
    let metadata = std::iter::once(&root_dir)
        .chain(metadata_fallback_roots.iter())
        .find_map(|metadata_root| app_config.installed.get(&path_key(metadata_root)));
    diagnostics.extend(target_specific_diagnostics(
        scope,
        &root_dir,
        &frontmatter,
        resource_count,
        resource_bytes,
    ));
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

fn dedupe_roots(roots: Vec<(SkillScope, PathBuf)>) -> Vec<(SkillScope, PathBuf)> {
    let mut seen = HashMap::new();
    let mut deduped = Vec::new();

    for (scope, root) in roots {
        let key = path_key(&root);
        if seen.insert(key, ()).is_none() {
            deduped.push((scope, root));
        }
    }

    deduped
}

/// Discover skill candidates by searching `SKILL.md` files.
///
/// Discovery is depth-limited and skips `.skills-disabled`, `.disabled`, and
/// backup folders.
pub fn discover_skill_candidates(root: &Path) -> Result<Vec<SkillCandidate>> {
    if !root.exists() {
        debug!(root = %root.display(), "candidate root does not exist");
        return Ok(Vec::new());
    }

    debug!(root = %root.display(), "discovering skill candidates");
    let mut candidate_files = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_ignored_discovery_entry(entry.file_name()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
            continue;
        }

        let skill_file = entry.path().to_path_buf();
        let Some(root_dir) = skill_file.parent().map(Path::to_path_buf) else {
            continue;
        };

        candidate_files.push((root_dir, skill_file));
    }

    candidate_files.sort_by(|(left, _), (right, _)| left.cmp(right));
    let candidate_roots = candidate_files
        .iter()
        .map(|(root_dir, _)| root_dir.clone())
        .collect::<Vec<_>>();
    let summaries = resource_summaries_for_candidates(root, &candidate_roots)?;
    let mut candidates = candidate_files
        .into_iter()
        .map(|(root_dir, skill_file)| {
            let summary = summaries.get(&root_dir).copied().unwrap_or_default();
            read_skill_candidate_with_summary(root_dir, skill_file, summary)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.root_dir.cmp(&right.root_dir));
    debug!(
        root = %root.display(),
        count = candidates.len(),
        "discovered skill candidates"
    );
    Ok(candidates)
}

fn discover_disabled_skill_candidates(root: &Path) -> Result<Vec<SkillCandidate>> {
    let mut candidates = discover_skill_candidates(&disabled_store_root_for_skills_root(root))?;
    candidates.extend(discover_skill_candidates(
        &root.join(LEGACY_DISABLED_SKILLS_DIR),
    )?);
    candidates.sort_by(|left, right| left.root_dir.cmp(&right.root_dir));
    Ok(candidates)
}

fn is_ignored_discovery_entry(name: &std::ffi::OsStr) -> bool {
    name == DISABLED_SKILLS_DIR
        || name == LEGACY_DISABLED_SKILLS_DIR
        || name.to_str().is_some_and(|name| name.contains(".backup-"))
}

fn enabled_root_for_disabled_root(skills_root: &Path, disabled_root: &Path) -> PathBuf {
    let disabled_parent = disabled_store_root_for_skills_root(skills_root);
    let legacy_disabled_parent = skills_root.join(LEGACY_DISABLED_SKILLS_DIR);

    disabled_root
        .strip_prefix(&disabled_parent)
        .or_else(|_| disabled_root.strip_prefix(&legacy_disabled_parent))
        .map(|relative| skills_root.join(relative))
        .unwrap_or_else(|_| disabled_root.to_path_buf())
}

/// Compute the current-layout disabled-root sibling for the given skills root.
pub fn disabled_store_root_for_skills_root(skills_root: &Path) -> PathBuf {
    skills_root
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(DISABLED_SKILLS_DIR)
}

/// Read one skill candidate from explicit file and root inputs.
pub fn read_skill_candidate(root_dir: PathBuf, skill_file: PathBuf) -> SkillCandidate {
    let summary = resource_summary(&root_dir);
    read_skill_candidate_with_summary(root_dir, skill_file, summary)
}

fn read_skill_candidate_with_summary(
    root_dir: PathBuf,
    skill_file: PathBuf,
    summary: ResourceSummary,
) -> SkillCandidate {
    let (frontmatter, mut diagnostics) = match parse_skill_frontmatter(&skill_file) {
        Ok(frontmatter) => (frontmatter, Vec::new()),
        Err(error) => (
            SkillFrontmatter::default(),
            vec![SkillDiagnostic::invalid(format!(
                "Could not parse SKILL.md frontmatter: {error}"
            ))],
        ),
    };
    let resource_count = summary.count;
    let resource_bytes = summary.bytes;
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
        normalized_name: normalized_skill_name(&frontmatter),
        search_haystack: candidate_search_haystack(&frontmatter),
        frontmatter,
        diagnostics,
        health,
        resource_count,
        resource_bytes,
    }
}

/// Parse skill metadata from `SKILL.md`.
///
/// Returns default metadata when no frontmatter block exists.
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
        tags: raw
            .tags
            .as_ref()
            .map(value_to_string_list)
            .unwrap_or_default(),
        disable_model_invocation: raw
            .disable_model_invocation
            .as_ref()
            .and_then(value_to_bool),
        when_to_use: raw.when_to_use.as_ref().and_then(scalar_to_string),
        metadata: raw
            .metadata
            .iter()
            .filter_map(|(key, value)| value_to_metadata(value).map(|value| (key.clone(), value)))
            .collect(),
    })
}

/// Validate frontmatter fields and resource footprint.
///
/// This checks required fields, naming policy, and warning thresholds for size.
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

/// Derive normalized [`SkillHealth`] from diagnostics and optional shadowed marker.
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

/// Convert a filesystem path to a stable configuration key.
pub fn path_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// Format bytes in human-readable binary units.
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

    for (index, skill) in skills.iter_mut().enumerate() {
        if !skill.is_enabled() {
            continue;
        }

        let key = installed_skill_identity(skill);
        if let Some((winner_path, _winner_index)) = winners.get(&key) {
            skill.shadowed_by = Some(winner_path.clone());
            skill.diagnostics.push(SkillDiagnostic::warning(format!(
                "Shadowed by higher-priority skill at {}",
                winner_path.display()
            )));
            skill.health = health_from_diagnostics(&skill.diagnostics, true);
        } else {
            winners.insert(key, (skill.root_dir.clone(), index));
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

/// Compute identity key for duplicate/scope collision resolution.
///
/// Uses normalized frontmatter name when available, otherwise normalized folder name.
pub fn installed_skill_identity(skill: &InstalledSkill) -> String {
    skill
        .frontmatter
        .name
        .as_deref()
        .map(sanitize_folder_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| sanitize_folder_name(&skill.destination_name()))
}

/// Return enabled scopes that currently expose this skill identity.
pub fn visible_skill_scopes<'a>(
    skills: impl IntoIterator<Item = &'a InstalledSkill>,
    skill: &InstalledSkill,
) -> Vec<SkillScope> {
    let identity = installed_skill_identity(skill);
    let mut scopes = skills
        .into_iter()
        .filter(|candidate| {
            candidate.is_enabled() && installed_skill_identity(candidate) == identity
        })
        .map(|candidate| candidate.scope)
        .collect::<Vec<_>>();

    scopes.sort_by_key(|scope| scope.sort_rank());
    scopes.dedup();
    scopes
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

fn value_to_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        },
        _ => None,
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

fn resource_summaries_for_candidates(
    search_root: &Path,
    candidate_roots: &[PathBuf],
) -> Result<HashMap<PathBuf, ResourceSummary>> {
    let mut summaries = candidate_roots
        .iter()
        .map(|root| (root.clone(), ResourceSummary::default()))
        .collect::<HashMap<_, _>>();

    if candidate_roots.is_empty() {
        return Ok(summaries);
    }

    for entry in WalkDir::new(search_root)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
    {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.file_name() == "SKILL.md" {
            continue;
        }

        let bytes = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        for candidate_root in candidate_roots {
            if entry.path().starts_with(candidate_root)
                && let Some(summary) = summaries.get_mut(candidate_root)
            {
                summary.count += 1;
                summary.bytes += bytes;
            }
        }
    }

    Ok(summaries)
}

fn resource_summary(root_dir: &Path) -> ResourceSummary {
    let mut summary = ResourceSummary::default();

    for entry in WalkDir::new(root_dir)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() || entry.file_name() == "SKILL.md" {
            continue;
        }

        summary.count += 1;
        if let Ok(metadata) = entry.metadata() {
            summary.bytes += metadata.len();
        }
    }

    summary
}

fn normalized_skill_name(frontmatter: &SkillFrontmatter) -> String {
    frontmatter
        .name
        .as_deref()
        .map(sanitize_folder_name)
        .unwrap_or_default()
}

fn candidate_search_haystack(frontmatter: &SkillFrontmatter) -> String {
    let mut haystack = String::new();
    haystack.push_str(frontmatter.name.as_deref().unwrap_or_default());
    haystack.push(' ');
    haystack.push_str(frontmatter.description.as_deref().unwrap_or_default());
    haystack.push(' ');
    haystack.push_str(&frontmatter.allowed_tools.join(" "));
    haystack.push(' ');
    haystack.push_str(&frontmatter.tags.join(" "));
    haystack.to_lowercase()
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

/// Normalize a free-form skill name into a directory-friendly identifier.
pub fn sanitize_folder_name(name: &str) -> String {
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
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use crate::{ProjectRoot, SkillEnablement, SkillFrontmatter, SkillHealth, SkillScope};

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
    fn parses_target_specific_frontmatter_fields() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("SKILL.md");
        fs::write(
            &file,
            "---\nname: demo-skill\ndescription: Use this skill when testing target metadata\ntags: [codex, zed]\nwhen_to_use: Use when testing Claude Code metadata\ndisable-model-invocation: true\n---\n# Demo\n",
        )
        .unwrap();

        let parsed = parse_skill_frontmatter(&file).unwrap();
        assert_eq!(parsed.tags, vec!["codex", "zed"]);
        assert_eq!(
            parsed.when_to_use.as_deref(),
            Some("Use when testing Claude Code metadata")
        );
        assert_eq!(parsed.disable_model_invocation, Some(true));
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
    fn discovery_summarizes_resources_for_multiple_candidates() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("skills").join("first");
        let second = dir.path().join("skills").join("second");
        fs::create_dir_all(first.join("assets")).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(
            first.join("SKILL.md"),
            "---\nname: First Skill\ndescription: Use this skill when testing resource summaries\ntags: [fast]\n---\n",
        )
        .unwrap();
        fs::write(first.join("assets/data.txt"), "abc").unwrap();
        fs::write(
            second.join("SKILL.md"),
            "---\nname: second\ndescription: Use this skill when testing second summaries\n---\n",
        )
        .unwrap();
        fs::write(second.join("notes.txt"), "hello").unwrap();

        let candidates = discover_skill_candidates(dir.path()).unwrap();

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].resource_count, 1);
        assert_eq!(candidates[0].resource_bytes, 3);
        assert_eq!(candidates[0].normalized_name, "first-skill");
        assert!(candidates[0].search_haystack.contains("fast"));
        assert_eq!(candidates[1].resource_count, 1);
        assert_eq!(candidates[1].resource_bytes, 5);
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
    fn project_skill_shadows_global_skill_with_same_name() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        let global_skill = dir.path().join("home/.agents/skills/demo-skill");
        let project_skill = project.join(".agents/skills/demo-skill");
        fs::create_dir_all(&global_skill).unwrap();
        fs::create_dir_all(&project_skill).unwrap();
        let skill_doc = "---\nname: demo-skill\ndescription: A useful demo skill\n---\n";
        fs::write(global_skill.join("SKILL.md"), skill_doc).unwrap();
        fs::write(project_skill.join("SKILL.md"), skill_doc).unwrap();

        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(&project)),
        );
        let skills = scan_installed_skills(&paths).unwrap();

        let global = skills
            .iter()
            .find(|skill| skill.scope == SkillScope::Global)
            .unwrap();
        assert_eq!(global.health, SkillHealth::Shadowed);
        assert!(global.shadowed_by.is_some());
    }

    #[test]
    fn disabled_project_skill_does_not_shadow_enabled_global_skill() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        let global_skill = dir.path().join("home/.agents/skills/demo-skill");
        let disabled_project_skill = project.join(".agents/skills/.disabled/demo-skill");
        fs::create_dir_all(&global_skill).unwrap();
        fs::create_dir_all(&disabled_project_skill).unwrap();
        let skill_doc = "---\nname: demo-skill\ndescription: A useful demo skill\n---\n";
        fs::write(global_skill.join("SKILL.md"), skill_doc).unwrap();
        fs::write(disabled_project_skill.join("SKILL.md"), skill_doc).unwrap();

        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(&project)),
        );
        let skills = scan_installed_skills(&paths).unwrap();

        let project = skills
            .iter()
            .find(|skill| skill.scope == SkillScope::Project)
            .unwrap();
        assert_eq!(project.enablement, SkillEnablement::Disabled);
        let global = skills
            .iter()
            .find(|skill| skill.scope == SkillScope::Global)
            .unwrap();
        assert_eq!(global.health, SkillHealth::Valid);
        assert!(global.shadowed_by.is_none());
    }

    #[test]
    fn visible_skill_scopes_groups_enabled_targets_by_skill_identity() {
        let skills = [
            installed_skill(SkillScope::Global, "demo-skill", SkillEnablement::Enabled),
            installed_skill(SkillScope::Codex, "demo-skill", SkillEnablement::Disabled),
            installed_skill(SkillScope::Zed, "demo-skill", SkillEnablement::Enabled),
            installed_skill(SkillScope::Droid, "other-skill", SkillEnablement::Enabled),
        ];

        let scopes = visible_skill_scopes(skills.iter(), &skills[0]);

        assert_eq!(scopes, vec![SkillScope::Global, SkillScope::Zed]);
    }

    #[test]
    fn sanitizes_folder_names() {
        assert_eq!(sanitize_folder_name("Demo Skill!"), "demo-skill");
        assert_eq!(sanitize_folder_name("  ___ "), "___");
    }

    fn installed_skill(
        scope: SkillScope,
        name: &str,
        enablement: SkillEnablement,
    ) -> InstalledSkill {
        let root_dir = PathBuf::from(format!("/tmp/{}/{}", scope.id_prefix(), name));

        InstalledSkill {
            id: format!("{}:{}", scope.id_prefix(), root_dir.display()),
            display_name: name.to_string(),
            description: None,
            scope,
            root_dir: root_dir.clone(),
            skill_file: root_dir.join("SKILL.md"),
            frontmatter: SkillFrontmatter {
                name: Some(name.to_string()),
                ..SkillFrontmatter::default()
            },
            enablement,
            health: SkillHealth::Valid,
            diagnostics: Vec::new(),
            resource_count: 0,
            resource_bytes: 0,
            shadowed_by: None,
            source_url: None,
            installed_at: None,
        }
    }
}
