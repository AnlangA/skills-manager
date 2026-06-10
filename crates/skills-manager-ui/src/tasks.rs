//! Async background task builders for the iced runtime.
//!
//! Each public function wraps a core operation in a [`Task::perform`] call,
//! mapping the result into a [`Message`] variant for the update loop.
//! Handles workspace loading, install/preview/download workflows, catalog
//! generation, toggle/remove actions, marketplace operations, and scaffold
//! creation.

use std::{
    fs,
    path::{Path, PathBuf},
};

use iced::Task;
use skills_manager_core::{
    AgentToolTarget, CatalogFormat, ConflictPolicy, DownloadedSkillEntry, InstallRequest,
    InstallTarget, Installer, ManagerConfig, ManagerPaths, McpServerRequest, OperationPlan,
    ProjectRoot, ResourceManager, SkillScaffoldRequest, WorkspaceSnapshot, add_mcp_server,
    create_skill_scaffold, download_github_catalog, download_github_skills,
    download_github_skills_to_cache, downloaded_skill_entry, export_installed_catalog,
    preview_skill_scaffold, remove_downloaded_skills, remove_mcp_server, scan_installed_skills,
    set_mcp_server_enabled, validate_path,
};

use crate::app::{
    CatalogEntryState, DownloadedEntryState, InstallSource, Message, PreviewCandidateState,
    PreviewState, catalog_entry_from_source,
};

pub fn load_workspace_task(project_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            WorkspaceSnapshot::load(&paths).map_err(|error| error.to_string())
        },
        Message::WorkspaceLoaded,
    )
}

pub fn save_default_download_path_task(project_path: String, value: String) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                validate_path(Path::new(trimmed)).map_err(|error| error.to_string())?;
            }
            let resolved = Path::new(trimmed);
            ManagerConfig::update(&paths, |config| {
                if trimmed.is_empty() || resolved == paths.downloads_dir() {
                    config.set_default_download_dir(None);
                } else {
                    config.set_default_download_dir(Some(PathBuf::from(trimmed)));
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
            Ok("Saved default download path.".to_string())
        },
        Message::DefaultDownloadPathSaved,
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
            let override_dir = download_dir
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(Path::new);
            if let Some(override_dir) = override_dir {
                validate_path(override_dir).map_err(|error| error.to_string())?;
                let downloaded = download_github_skills_to_cache(&paths, &url, Some(override_dir))
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(format!(
                    "Downloaded {} candidate(s) to {}.",
                    downloaded.candidate_count,
                    downloaded.root_dir.display()
                ))
            } else {
                let downloaded = download_github_skills_to_cache(&paths, &url, None)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(format!(
                    "Downloaded {} candidate(s) to {}.",
                    downloaded.candidate_count,
                    downloaded.root_dir.display()
                ))
            }
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
    enable_after_install: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths.clone());
            match source {
                InstallSource::Url => {
                    let downloaded = download_github_skills(&value)
                        .await
                        .map_err(|error| error.to_string())?;
                    let plan = installer
                        .plan(InstallRequest {
                            source_root: downloaded.search_root.clone(),
                            source_url: Some(value.clone()),
                            target: target.clone(),
                            conflict_policy,
                            enable_after_install,
                        })
                        .map_err(|error| error.to_string())?;
                    let mut preview = plan.preview();
                    preview.operation_plan = None;
                    Ok(preview_state(
                        PreviewContext {
                            source,
                            source_label: value.clone(),
                            source_value: value,
                            download_dir,
                            target,
                            enable_after_install,
                            conflict_policy,
                        },
                        preview,
                    ))
                }
                InstallSource::Local => {
                    let source_root = PathBuf::from(value.trim());
                    validate_path(&source_root).map_err(|error| error.to_string())?;
                    let preview = installer
                        .plan(InstallRequest {
                            source_root: source_root.clone(),
                            source_url: None,
                            target: target.clone(),
                            conflict_policy,
                            enable_after_install,
                        })
                        .map(|plan| plan.preview())
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(
                        PreviewContext {
                            source,
                            source_label: value.clone(),
                            source_value: value,
                            download_dir,
                            target,
                            enable_after_install,
                            conflict_policy,
                        },
                        preview,
                    ))
                }
                InstallSource::Downloaded => {
                    let source_root = PathBuf::from(value.trim());
                    validate_path(&source_root).map_err(|error| error.to_string())?;
                    let downloaded = downloaded_skill_entry(&paths, &source_root)
                        .map_err(|error| error.to_string())?;
                    let preview = installer
                        .plan(InstallRequest {
                            source_root: downloaded.search_root.clone(),
                            source_url: Some(downloaded.source_url.clone()),
                            target: target.clone(),
                            conflict_policy,
                            enable_after_install,
                        })
                        .map(|plan| plan.preview())
                        .map_err(|error| error.to_string())?;
                    Ok(preview_state(
                        PreviewContext {
                            source,
                            source_label: downloaded.source_url.clone(),
                            source_value: downloaded.root_dir.display().to_string(),
                            download_dir,
                            target,
                            enable_after_install,
                            conflict_policy,
                        },
                        preview,
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

pub fn install_source_task(
    project_path: String,
    source: InstallSource,
    value: String,
    download_dir: Option<String>,
    target: InstallTarget,
    conflict_policy: ConflictPolicy,
    enable_after_install: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths.clone());
            let (source_root, source_url) = match source {
                InstallSource::Url => {
                    let override_dir = download_dir
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(Path::new);
                    if let Some(override_dir) = override_dir {
                        validate_path(override_dir).map_err(|error| error.to_string())?;
                    }
                    let downloaded = download_github_skills_to_cache(&paths, &value, override_dir)
                        .await
                        .map_err(|error| error.to_string())?;
                    (downloaded.search_root, Some(value))
                }
                InstallSource::Local => {
                    let source_root = PathBuf::from(value.trim());
                    validate_path(&source_root).map_err(|error| error.to_string())?;
                    (source_root, None)
                }
                InstallSource::Downloaded => {
                    let source_root = PathBuf::from(value.trim());
                    validate_path(&source_root).map_err(|error| error.to_string())?;
                    let downloaded = downloaded_skill_entry(&paths, &source_root)
                        .map_err(|error| error.to_string())?;
                    (downloaded.search_root, Some(downloaded.source_url))
                }
                InstallSource::Catalog => {
                    return Err("Load the catalog, then preview one catalog entry.".to_string());
                }
            };
            let plan = installer
                .plan(InstallRequest {
                    source_root,
                    source_url,
                    target,
                    conflict_policy,
                    enable_after_install,
                })
                .map_err(|error| error.to_string())?;
            let result = installer
                .install_plan(plan)
                .map_err(|error| error.to_string())?;

            Ok(format!("Installed {} skill(s).", result.installed.len()))
        },
        Message::Installed,
    )
}

pub fn install_task(project_path: String, plan: OperationPlan) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let installer = Installer::new(paths);
            let result = installer
                .install_plan(plan)
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

pub fn toggle_plugin_task(
    project_path: String,
    plugin_id: String,
    target: AgentToolTarget,
    enabled: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let manager = ResourceManager::new(paths);
            manager
                .set_plugin_enabled(target, &plugin_id, enabled)
                .map_err(|error| error.to_string())?;
            Ok(if enabled {
                "Enabled plugin.".to_string()
            } else {
                "Disabled plugin.".to_string()
            })
        },
        Message::PluginToggled,
    )
}

pub fn remove_plugin_task(
    project_path: String,
    plugin_id: String,
    target: AgentToolTarget,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let manager = ResourceManager::new(paths);
            let backup = manager
                .remove_plugin(target, &plugin_id)
                .map_err(|error| error.to_string())?;
            Ok(format!("Removed plugin. Backup: {}", backup.display()))
        },
        Message::PluginRemoved,
    )
}

pub fn add_mcp_server_task(project_path: String, request: McpServerRequest) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let server = add_mcp_server(&paths, request).map_err(|error| error.to_string())?;
            Ok(format!(
                "Added MCP server {} for {}.",
                server.display_name,
                server.target.label()
            ))
        },
        Message::McpServerAdded,
    )
}

pub fn toggle_mcp_server_task(
    project_path: String,
    name: String,
    target: AgentToolTarget,
    enabled: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            set_mcp_server_enabled(&paths, target, &name, enabled)
                .map_err(|error| error.to_string())?;
            Ok(if enabled {
                "Enabled MCP server.".to_string()
            } else {
                "Disabled MCP server.".to_string()
            })
        },
        Message::McpServerToggled,
    )
}

pub fn remove_mcp_server_task(
    project_path: String,
    name: String,
    target: AgentToolTarget,
) -> Task<Message> {
    Task::perform(
        async move {
            let paths = paths_from_project(&project_path)?;
            let backup =
                remove_mcp_server(&paths, target, &name).map_err(|error| error.to_string())?;
            Ok(format!("Removed MCP server. Backup: {}", backup.display()))
        },
        Message::McpServerRemoved,
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

struct PreviewContext {
    source: InstallSource,
    source_label: String,
    source_value: String,
    download_dir: Option<String>,
    target: InstallTarget,
    enable_after_install: bool,
    conflict_policy: ConflictPolicy,
}

fn preview_state(
    context: PreviewContext,
    preview: skills_manager_core::InstallPreview,
) -> PreviewState {
    PreviewState {
        source_label: context.source_label,
        source: context.source,
        source_value: context.source_value,
        download_dir: context.download_dir,
        target: context.target,
        enable_after_install: context.enable_after_install,
        scope: preview.scope,
        conflict_policy: context.conflict_policy,
        operation_plan: preview.operation_plan,
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

pub fn downloaded_entry_state(entry: DownloadedSkillEntry) -> DownloadedEntryState {
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
