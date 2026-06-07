use std::{
    fs,
    path::{Path, PathBuf},
};

use iced::Task;
use skills_manager_core::{
    CatalogFormat, ConflictPolicy, DownloadedSkillEntry, InstallRequest, InstallTarget, Installer,
    ManagerConfig, ManagerPaths, ProjectRoot, SkillScaffoldRequest, create_skill_scaffold,
    doctor_report, download_github_catalog, download_github_skills_to_cache,
    downloaded_skill_entry, export_installed_catalog, list_downloaded_skills,
    preview_skill_scaffold, remove_downloaded_skills, scan_installed_skills, target_profiles,
};

use crate::app::{
    CatalogEntryState, DownloadedEntryState, InstallSource, ManagerSettingsState, Message,
    PreviewCandidateState, PreviewState, catalog_entry_from_source,
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

pub fn load_settings_task(project_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let config = ManagerConfig::load(&paths).map_err(|error| error.to_string())?;
            let target_profiles = target_profiles(&paths).map_err(|error| error.to_string())?;
            let doctor_report = doctor_report(&paths).map_err(|error| error.to_string())?;
            Ok(ManagerSettingsState {
                default_download_path: config.effective_download_dir(&paths).display().to_string(),
                target_profiles,
                doctor_report,
            })
        },
        Message::SettingsLoaded,
    )
}

pub fn save_default_download_path_task(project_path: String, value: String) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let mut config = ManagerConfig::load(&paths).map_err(|error| error.to_string())?;
            let trimmed = value.trim();
            if trimmed.is_empty() || PathBuf::from(trimmed) == paths.downloads_dir() {
                config.set_default_download_dir(None);
            } else {
                config.set_default_download_dir(Some(PathBuf::from(trimmed)));
            }
            config.save(&paths).map_err(|error| error.to_string())?;
            Ok("Saved default download path.".to_string())
        },
        Message::DefaultDownloadPathSaved,
    )
}

pub fn load_downloads_task(project_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            list_downloaded_skills(&paths)
                .map(|entries| entries.into_iter().map(downloaded_entry_state).collect())
                .map_err(|error| error.to_string())
        },
        Message::DownloadsLoaded,
    )
}

pub fn download_source_task(
    project_path: String,
    url: String,
    download_dir: Option<String>,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let override_dir = download_dir.as_deref().map(Path::new);
            let downloaded = download_github_skills_to_cache(&paths, &url, override_dir)
                .await
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "Downloaded {} candidate(s) to {}.",
                downloaded.candidate_count,
                downloaded.root_dir.display()
            ))
        },
        Message::Downloaded,
    )
}

pub fn preview_task(
    project_path: String,
    source: InstallSource,
    value: String,
    download_dir: Option<String>,
    target: InstallTarget,
    conflict_policy: ConflictPolicy,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths.clone());
            match source {
                InstallSource::Url => {
                    let override_dir = download_dir.as_deref().map(Path::new);
                    let downloaded = download_github_skills_to_cache(&paths, &value, override_dir)
                        .await
                        .map_err(|error| error.to_string())?;
                    let preview = installer
                        .preview(&downloaded.search_root, target, conflict_policy)
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(
                        value.clone(),
                        downloaded.search_root,
                        Some(value),
                        Some(downloaded.root_dir),
                        preview,
                        conflict_policy,
                    ))
                }
                InstallSource::Local => {
                    let source_root = PathBuf::from(&value);
                    let preview = installer
                        .preview(&source_root, target, conflict_policy)
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(
                        value,
                        source_root,
                        None,
                        None,
                        preview,
                        conflict_policy,
                    ))
                }
                InstallSource::Downloaded => {
                    let downloaded = downloaded_skill_entry(&paths, &PathBuf::from(&value))
                        .map_err(|error| error.to_string())?;
                    let preview = installer
                        .preview(&downloaded.search_root, target, conflict_policy)
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(
                        downloaded.source_url.clone(),
                        downloaded.search_root,
                        Some(downloaded.source_url),
                        Some(downloaded.root_dir),
                        preview,
                        conflict_policy,
                    ))
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
    source_root: PathBuf,
    source_url: Option<String>,
    target: InstallTarget,
    conflict_policy: ConflictPolicy,
    enable_after_install: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            let result = installer
                .install(InstallRequest {
                    source_root,
                    source_url,
                    target,
                    conflict_policy,
                    enable_after_install,
                })
                .map_err(|error| error.to_string())?;

            Ok(format!("Installed {} skill(s).", result.installed.len()))
        },
        Message::Installed,
    )
}

pub fn remove_download_task(project_path: String, root_dir: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let removed =
                remove_downloaded_skills(&paths, &root_dir).map_err(|error| error.to_string())?;
            Ok(format!(
                "Deleted downloaded skills at {}.",
                removed.display()
            ))
        },
        Message::DownloadRemoved,
    )
}

pub fn load_catalog_task(url: String) -> Task<Message> {
    Task::perform(
        async move {
            let downloaded = download_github_catalog(&url)
                .await
                .map_err(|error| error.to_string())?;
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

pub fn toggle_task(project_path: String, skill_root: PathBuf, disabled: bool) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            installer
                .set_skill_enabled(&skill_root, !disabled)
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

pub fn preview_scaffold_task(project_path: String, request: SkillScaffoldRequest) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            preview_skill_scaffold(&paths, request).map_err(|error| error.to_string())
        },
        Message::ScaffoldPreviewed,
    )
}

pub fn create_scaffold_task(project_path: String, request: SkillScaffoldRequest) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            create_skill_scaffold(&paths, request).map_err(|error| error.to_string())
        },
        Message::SkillCreated,
    )
}

fn preview_state(
    source_label: String,
    source_root: PathBuf,
    source_url: Option<String>,
    download_root: Option<PathBuf>,
    preview: skills_manager_core::InstallPreview,
    conflict_policy: ConflictPolicy,
) -> PreviewState {
    PreviewState {
        source_label,
        source_root,
        source_url,
        download_root,
        scope: preview.scope,
        destination_root: preview.destination_root,
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

fn downloaded_entry_state(entry: DownloadedSkillEntry) -> DownloadedEntryState {
    let summary = entry.resource_summary();
    DownloadedEntryState {
        source_url: entry.source_url,
        root_dir: entry.root_dir,
        downloaded_at: entry
            .downloaded_at
            .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        summary,
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
