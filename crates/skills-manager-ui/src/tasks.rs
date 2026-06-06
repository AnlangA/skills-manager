use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use iced::Task;
use skills_manager_core::{
    CatalogFormat, ConflictPolicy, InstallRequest, Installer, ManagerPaths, ProjectRoot,
    SkillScope, download_github_catalog, download_github_skills, export_installed_catalog,
    scan_installed_skills,
};

use crate::app::{
    CatalogEntryState, InstallSource, Message, PreviewCandidateState, PreviewState,
    catalog_entry_from_source,
};

pub fn refresh_task(project_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            scan_installed_skills(&paths).map_err(|error| error.to_string())
        },
        Message::SkillsLoaded,
    )
}

pub fn preview_task(
    project_path: String,
    source: InstallSource,
    value: String,
    scope: SkillScope,
    conflict_policy: ConflictPolicy,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            match source {
                InstallSource::Url => {
                    let downloaded =
                        download_github_skills(&value).map_err(|error| error.to_string())?;
                    let preview = installer
                        .preview(downloaded.temp_dir.path(), scope, conflict_policy)
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(value, preview, conflict_policy))
                }
                InstallSource::Local => {
                    let preview = installer
                        .preview(
                            &PathBuf::from_str(&value).map_err(|error| error.to_string())?,
                            scope,
                            conflict_policy,
                        )
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(value, preview, conflict_policy))
                }
                InstallSource::Catalog => {
                    Err("Load the catalog, then preview one catalog entry.".to_string())
                }
            }
        },
        Message::PreviewLoaded,
    )
}

pub fn install_task(
    project_path: String,
    source: InstallSource,
    value: String,
    scope: SkillScope,
    conflict_policy: ConflictPolicy,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            let (source_root, source_url, _downloaded) = match source {
                InstallSource::Url => {
                    let downloaded =
                        download_github_skills(&value).map_err(|error| error.to_string())?;
                    (
                        downloaded.temp_dir.path().to_path_buf(),
                        Some(value),
                        Some(downloaded),
                    )
                }
                InstallSource::Local => (PathBuf::from(value), None, None),
                InstallSource::Catalog => {
                    return Err(
                        "Load a catalog entry into the GitHub URL source before installing."
                            .to_string(),
                    );
                }
            };

            let result = installer
                .install(InstallRequest {
                    source_root,
                    source_url,
                    scope,
                    conflict_policy,
                })
                .map_err(|error| error.to_string())?;

            Ok(format!("Installed {} skill(s).", result.installed.len()))
        },
        Message::Installed,
    )
}

pub fn load_catalog_task(url: String) -> Task<Message> {
    Task::perform(
        async move {
            let downloaded = download_github_catalog(&url).map_err(|error| error.to_string())?;
            Ok(downloaded
                .catalog
                .skills
                .into_iter()
                .map(|entry| {
                    catalog_entry_from_source(
                        entry.display_name.unwrap_or(entry.name),
                        entry
                            .description
                            .unwrap_or_else(|| "No description".to_string()),
                        entry.source,
                    )
                })
                .collect::<Vec<CatalogEntryState>>())
        },
        Message::CatalogLoaded,
    )
}

pub fn generate_catalog_task(project_path: String, format: CatalogFormat) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let skills = scan_installed_skills(&paths).map_err(|error| error.to_string())?;
            export_installed_catalog("Agent Skills", &skills, format)
                .map_err(|error| error.to_string())
        },
        Message::CatalogGenerated,
    )
}

pub fn save_catalog_task(project_path: String, save_path: String, output: String) -> Task<Message> {
    Task::perform(
        async move {
            let path = resolve_save_path(&project_path, &save_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&path, output).map_err(|error| error.to_string())?;
            Ok(format!("Saved catalog export to {}.", path.display()))
        },
        Message::CatalogSaved,
    )
}

pub fn toggle_task(project_path: String, skill_file: PathBuf, disabled: bool) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            installer
                .set_disabled(&skill_file, disabled)
                .map_err(|error| error.to_string())?;

            if disabled {
                Ok("Disabled skill.".to_string())
            } else {
                Ok("Enabled skill.".to_string())
            }
        },
        Message::SkillToggled,
    )
}

pub fn remove_task(project_path: String, skill_root: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            let backup = installer
                .remove(&skill_root)
                .map_err(|error| error.to_string())?;
            Ok(format!("Removed skill. Backup: {}", backup.display()))
        },
        Message::SkillRemoved,
    )
}

fn preview_state(
    source_label: String,
    preview: skills_manager_core::InstallPreview,
    conflict_policy: ConflictPolicy,
) -> PreviewState {
    PreviewState {
        source_label,
        scope: preview.scope,
        conflict_policy,
        candidates: preview
            .candidates
            .into_iter()
            .map(|candidate| PreviewCandidateState {
                name: candidate
                    .frontmatter
                    .name
                    .unwrap_or_else(|| "unnamed skill".to_string()),
                description: candidate
                    .frontmatter
                    .description
                    .unwrap_or_else(|| "No description".to_string()),
                source_root: candidate.source_root,
                destination_root: candidate.destination_root,
                health: candidate.health,
                conflict: candidate.conflict,
                diagnostics: candidate
                    .diagnostics
                    .into_iter()
                    .map(|diagnostic| {
                        format!("{}: {}", diagnostic.severity.label(), diagnostic.message)
                    })
                    .collect(),
                resource_count: candidate.resource_count,
                resource_bytes: candidate.resource_bytes,
            })
            .collect(),
    }
}

fn paths_from_project(project_path: &str) -> Result<ManagerPaths, String> {
    let project = if project_path.trim().is_empty() {
        None
    } else {
        Some(ProjectRoot::new(project_path.trim()))
    };

    ManagerPaths::new(project).map_err(|error| error.to_string())
}

fn resolve_save_path(project_path: &str, save_path: &str) -> PathBuf {
    let path = PathBuf::from(save_path.trim());
    if path.is_absolute() || project_path.trim().is_empty() {
        path
    } else {
        Path::new(project_path.trim()).join(path)
    }
}
