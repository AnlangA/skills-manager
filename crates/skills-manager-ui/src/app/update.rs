//! Application update loop and top-level state transitions.
//!
//! Contains the [`App`] struct definition, its initialization methods,
//! and the main `update` function that processes [`Message`] variants.

use iced::{Task, widget::text_editor};

use super::derived::*;
use super::helpers::*;
use super::message::Message;
use super::state::*;
use super::types::*;
use crate::tasks;

/// Top-level application state containing all view state and derived data.
pub struct App {
    pub snapshot: Option<skills_manager_core::WorkspaceSnapshot>,
    pub skills: Vec<skills_manager_core::InstalledSkill>,
    pub resources: Vec<skills_manager_core::ManagedResource>,
    pub derived: DerivedInventoryState,
    pub active_view: ActiveView,
    pub inventory: InventoryState,
    pub mcp: McpState,
    pub form_editor: ExpandedEditorState,
    pub install: InstallState,
    pub create: CreateState,
    pub catalog: CatalogExportState,
    pub settings: AppSettingsState,
    pub status: String,
    pub busy: bool,
    pub smoke_test: bool,
}

impl App {
    /// Initializes the application with default settings and loads the workspace.
    pub fn init() -> (Self, Task<Message>) {
        Self::init_with_smoke_test(false)
    }

    pub fn init_with_smoke_test(smoke_test: bool) -> (Self, Task<Message>) {
        let app = Self {
            snapshot: None,
            skills: Vec::new(),
            resources: Vec::new(),
            derived: DerivedInventoryState::default(),
            active_view: ActiveView::Library,
            inventory: InventoryState::default(),
            mcp: McpState::default(),
            form_editor: ExpandedEditorState::default(),
            install: InstallState::default(),
            create: CreateState::default(),
            catalog: CatalogExportState::default(),
            settings: AppSettingsState::default(),
            status: "Ready".to_string(),
            busy: true,
            smoke_test,
        };

        let task = if smoke_test {
            Task::done(Message::SmokeExit)
        } else {
            tasks::load_workspace_task(app.settings.project_path.clone())
        };
        (app, task)
    }

    pub fn filtered_skills(&self) -> Vec<&skills_manager_core::InstalledSkill> {
        self.derived
            .filtered_skill_indices
            .iter()
            .filter_map(|index| self.skills.get(*index))
            .collect()
    }

    pub fn selected_skill(&self) -> Option<&skills_manager_core::InstalledSkill> {
        self.inventory
            .selected_skill_id
            .as_ref()
            .and_then(|id| {
                self.filtered_skills()
                    .into_iter()
                    .find(|skill| skill.id == *id)
            })
            .or_else(|| {
                self.derived
                    .filtered_skill_indices
                    .first()
                    .and_then(|index| self.skills.get(*index))
            })
    }

    pub fn counts(&self) -> SkillCounts {
        self.derived.counts
    }

    pub fn visible_scopes_for_skill(
        &self,
        skill: &skills_manager_core::InstalledSkill,
    ) -> Vec<skills_manager_core::SkillScope> {
        self.derived
            .visible_scopes_by_id
            .get(&skill.id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn ensure_selection(&mut self) {
        if self
            .inventory
            .selected_skill_id
            .as_ref()
            .is_some_and(|selected| self.filtered_skill_id_exists(selected))
        {
            return;
        }

        self.inventory.selected_skill_id = self
            .derived
            .filtered_skill_indices
            .first()
            .and_then(|index| self.skills.get(*index))
            .map(|skill| skill.id.clone());
    }

    pub fn rebuild_derived(&mut self) {
        self.derived.counts = counts_from_skills(&self.skills);
        self.derived.visible_scopes_by_id = visible_scopes_by_id(&self.skills);
        self.derived.resource_search = resource_search_index(&self.resources);
        self.derived.filtered_skill_indices = filtered_indices(
            &self.skills,
            &self.inventory.skill_search_query,
            self.inventory.scope_filter,
            self.inventory.health_filter,
            self.inventory.source_filter,
            self.snapshot.as_ref(),
        );
        sort_skill_indices(
            &mut self.derived.filtered_skill_indices,
            &self.skills,
            self.inventory.sort_key,
        );
    }

    fn filtered_skill_id_exists(&self, id: &str) -> bool {
        self.derived.filtered_skill_indices.iter().any(|index| {
            self.skills
                .get(*index)
                .is_some_and(|skill| skill.id.as_str() == id)
        })
    }
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Refresh => {
            app.busy = true;
            app.status = "Loading workspace snapshot...".to_string();
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::WorkspaceLoaded(result) => {
            app.busy = false;
            match result {
                Ok(snapshot) => {
                    app.status = format!(
                        "Loaded {} skill(s), {} resource(s), {} download(s), and {} target(s).",
                        snapshot.skills.len(),
                        snapshot.resources.len(),
                        snapshot.downloads.len(),
                        snapshot.target_profiles.len()
                    );
                    app.install.downloaded_entries = snapshot
                        .downloads
                        .iter()
                        .cloned()
                        .map(tasks::downloaded_entry_state)
                        .collect();
                    if app
                        .install
                        .selected_download_root
                        .as_ref()
                        .is_none_or(|selected| {
                            !app.install
                                .downloaded_entries
                                .iter()
                                .any(|entry| &entry.root_dir == selected)
                        })
                    {
                        app.install.selected_download_root = app
                            .install
                            .downloaded_entries
                            .first()
                            .map(|entry| entry.root_dir.clone());
                    }
                    app.settings.default_download_path =
                        snapshot.default_download_path.display().to_string();
                    app.settings.target_profiles = snapshot.target_profiles.clone();
                    app.settings.doctor_report = Some(snapshot.doctor_report.clone());
                    app.skills = snapshot.skills.clone();
                    app.resources = snapshot.resources.clone();
                    app.snapshot = Some(snapshot);
                    app.rebuild_derived();
                    app.ensure_selection();
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::ProjectPathChanged(value) => {
            app.settings.project_path = value;
            Task::none()
        }
        Message::SkillSearchChanged(value) => {
            app.inventory.skill_search_query = value;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::PluginSearchChanged(value) => {
            app.inventory.plugin_search_query = value;
            Task::none()
        }
        Message::MarketplaceSearchChanged(value) => {
            app.inventory.marketplace_search_query = value;
            Task::none()
        }
        Message::McpSearchChanged(value) => {
            app.mcp.search_query = value;
            Task::none()
        }
        Message::ActiveViewSelected(view) => {
            app.active_view = view;
            Task::none()
        }
        Message::OpenExpandedEditor(target) => {
            open_expanded_editor(app, target);
            Task::none()
        }
        Message::CloseExpandedEditor => {
            app.form_editor.active = None;
            Task::none()
        }
        Message::ExpandedEditorAction(action) => {
            app.form_editor.content.perform(action);
            if let Some(target) = app.form_editor.active {
                let value = app.form_editor.content.text();
                set_expanded_editor_value(app, target, value);
            }
            Task::none()
        }
        Message::ScopeFilterSelected(scope_filter) => {
            app.inventory.scope_filter = scope_filter;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::HealthFilterSelected(health_filter) => {
            app.inventory.health_filter = health_filter;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::SourceFilterSelected(source_filter) => {
            app.inventory.source_filter = source_filter;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::PluginTargetFilterSelected(target_filter) => {
            app.inventory.plugin_target_filter = target_filter;
            Task::none()
        }
        Message::McpTargetFilterSelected(target_filter) => {
            app.mcp.target_filter = target_filter;
            Task::none()
        }
        Message::McpHealthFilterSelected(health_filter) => {
            app.mcp.health_filter = health_filter;
            Task::none()
        }
        Message::SortSelected(sort_key) => {
            app.inventory.sort_key = sort_key;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::SelectSkill(id) => {
            app.inventory.selected_skill_id = Some(id);
            Task::none()
        }
        Message::SelectResource(id) => {
            app.inventory.selected_resource_id = Some(id);
            Task::none()
        }
        Message::InstallSourceSelected(source) => {
            app.install.install_source = source;
            app.install.preview = None;
            Task::none()
        }
        Message::SourceUrlChanged(value) => {
            app.install.source_url = value;
            sync_expanded_editor(app, ExpandedEditorTarget::InstallSourceUrl);
            Task::none()
        }
        Message::LocalSourcePathChanged(value) => {
            app.install.local_source_path = value;
            sync_expanded_editor(app, ExpandedEditorTarget::InstallLocalSourcePath);
            Task::none()
        }
        Message::CatalogUrlChanged(value) => {
            app.install.catalog_url = value;
            sync_expanded_editor(app, ExpandedEditorTarget::InstallCatalogUrl);
            Task::none()
        }
        Message::DefaultDownloadPathChanged(value) => {
            app.settings.default_download_path = value;
            Task::none()
        }
        Message::SaveDefaultDownloadPath => {
            app.busy = true;
            app.status = "Saving default download path...".to_string();
            tasks::save_default_download_path_task(
                app.settings.project_path.clone(),
                app.settings.default_download_path.clone(),
            )
        }
        Message::DefaultDownloadPathSaved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::DownloadPathOverrideChanged(value) => {
            app.install.download_path_override = value;
            sync_expanded_editor(app, ExpandedEditorTarget::InstallDownloadPathOverride);
            Task::none()
        }
        Message::InstallScopeSelected(scope) => {
            app.install.install_scope = scope;
            app.install.preview = None;
            Task::none()
        }
        Message::CustomInstallPathChanged(value) => {
            app.install.custom_install_path = value;
            app.install.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::InstallCustomPath);
            Task::none()
        }
        Message::EnableAfterInstallChanged(value) => {
            app.install.enable_after_install = value;
            Task::none()
        }
        Message::ConflictSelected(policy) => {
            app.install.conflict_policy = policy;
            app.install.preview = None;
            Task::none()
        }
        Message::DownloadSource => {
            let url = app.install.source_url.trim().to_string();
            if url.is_empty() {
                app.status = "Enter a GitHub URL before downloading.".to_string();
                return Task::none();
            }

            app.busy = true;
            app.status = "Downloading skills to local cache...".to_string();
            tasks::download_source_task(
                app.settings.project_path.clone(),
                url,
                optional_string(&app.install.download_path_override),
            )
        }
        Message::Downloaded(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::PreviewInstall => {
            let source = current_source_value(&app.install);
            if source.trim().is_empty() {
                app.status = "Enter a source before previewing.".to_string();
                return Task::none();
            }
            let target = match current_install_target(&app.install) {
                Ok(target) => target,
                Err(error) => {
                    app.status = error;
                    return Task::none();
                }
            };

            app.busy = true;
            app.install.preview = None;
            app.status = "Building install preview...".to_string();
            tasks::preview_task(
                app.settings.project_path.clone(),
                app.install.install_source,
                source,
                optional_string(&app.install.download_path_override),
                target,
                app.install.conflict_policy.into(),
                app.install.enable_after_install,
            )
        }
        Message::PreviewLoaded(result) => {
            app.busy = false;
            match result {
                Ok(preview) => {
                    app.status = format!("Previewed {} candidate(s).", preview.candidates.len());
                    app.install.preview = Some(preview);
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::InstallPreview => {
            let Some(preview) = app.install.preview.clone() else {
                app.status = "Preview a source before installing.".to_string();
                return Task::none();
            };
            let Some(plan) = preview.operation_plan else {
                app.busy = true;
                app.status = "Installing previewed skill(s)...".to_string();
                return tasks::install_source_task(
                    app.settings.project_path.clone(),
                    preview.source,
                    preview.source_value,
                    preview.download_dir,
                    preview.target,
                    preview.conflict_policy,
                    preview.enable_after_install,
                );
            };

            app.busy = true;
            app.status = "Installing previewed skill(s)...".to_string();
            tasks::install_task(app.settings.project_path.clone(), plan)
        }
        Message::Installed(result) => {
            app.busy = false;
            app.install.preview = None;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::LoadCatalog => {
            let url = app.install.catalog_url.trim().to_string();
            if url.is_empty() {
                app.status = "Enter a catalog URL first.".to_string();
                return Task::none();
            }
            app.busy = true;
            app.install.catalog_entries.clear();
            app.status = "Loading catalog...".to_string();
            tasks::load_catalog_task(url)
        }
        Message::CatalogLoaded(result) => {
            app.busy = false;
            match result {
                Ok(entries) => {
                    let noun = if entries.len() == 1 {
                        "entry"
                    } else {
                        "entries"
                    };
                    app.status = format!("Loaded {} catalog {noun}.", entries.len());
                    app.install.catalog_entries = entries;
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::PreviewCatalogEntry(source, value) => {
            match source {
                InstallSource::Url => {
                    app.install.source_url = value.clone();
                }
                InstallSource::Local => {
                    app.install.local_source_path = value.clone();
                }
                InstallSource::Downloaded | InstallSource::Catalog => {}
            }
            let target = match current_install_target(&app.install) {
                Ok(target) => target,
                Err(error) => {
                    app.status = error;
                    return Task::none();
                }
            };
            let preview_source = source;
            let preview_value = value;
            if source != InstallSource::Catalog {
                app.install.install_source = source;
            }
            app.install.preview = None;
            app.busy = true;
            app.status = "Previewing catalog entry...".to_string();
            tasks::preview_task(
                app.settings.project_path.clone(),
                preview_source,
                preview_value,
                optional_string(&app.install.download_path_override),
                target,
                app.install.conflict_policy.into(),
                app.install.enable_after_install,
            )
        }
        Message::CatalogFormatSelected(format) => {
            app.catalog.catalog_format = format;
            Task::none()
        }
        Message::CatalogSavePathChanged(value) => {
            app.catalog.catalog_save_path = value;
            Task::none()
        }
        Message::GenerateCatalog => {
            app.busy = true;
            app.status = "Generating catalog export...".to_string();
            tasks::generate_catalog_task(
                app.settings.project_path.clone(),
                app.catalog.catalog_format.into(),
            )
        }
        Message::CatalogGenerated(result) => {
            app.busy = false;
            match result {
                Ok(output) => {
                    app.status = "Catalog export generated.".to_string();
                    app.catalog.catalog_output = output;
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::CopyCatalog => {
            if app.catalog.catalog_output.is_empty() {
                app.status = "Generate a catalog before copying.".to_string();
                Task::none()
            } else {
                app.status = "Catalog export copied to clipboard.".to_string();
                iced::clipboard::write(app.catalog.catalog_output.clone())
            }
        }
        Message::SaveCatalog => {
            if app.catalog.catalog_output.is_empty() {
                app.status = "Generate a catalog before saving.".to_string();
                Task::none()
            } else if app.catalog.catalog_save_path.trim().is_empty() {
                app.status = "Enter a save path first.".to_string();
                Task::none()
            } else {
                app.busy = true;
                app.status = "Saving catalog export...".to_string();
                tasks::save_catalog_task(
                    app.settings.project_path.clone(),
                    app.catalog.catalog_save_path.clone(),
                    app.catalog.catalog_output.clone(),
                )
            }
        }
        Message::CatalogSaved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            Task::none()
        }
        Message::SetSkillEnabled(skill_root, enabled) => {
            app.busy = true;
            app.inventory.pending_remove_skill = None;
            app.status = if enabled {
                "Enabling skill...".to_string()
            } else {
                "Disabling skill...".to_string()
            };
            tasks::toggle_task(app.settings.project_path.clone(), skill_root, !enabled)
        }
        Message::SkillToggled(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::RequestRemoveSkill(skill_root) => {
            app.inventory.pending_remove_skill = Some(skill_root);
            app.status =
                "Press Confirm remove to move the installed skill to a backup.".to_string();
            Task::none()
        }
        Message::ConfirmRemoveSkill(skill_root) => {
            app.busy = true;
            app.inventory.pending_remove_skill = None;
            app.status = "Removing skill and creating backup...".to_string();
            if app
                .selected_skill()
                .is_some_and(|skill| skill.root_dir.as_path() == skill_root.as_path())
            {
                app.inventory.selected_skill_id = None;
            }
            tasks::remove_task(app.settings.project_path.clone(), skill_root)
        }
        Message::SkillRemoved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::SetPluginEnabled(plugin_id, target, enabled) => {
            app.busy = true;
            app.inventory.pending_remove_plugin = None;
            app.status = if enabled {
                "Enabling plugin...".to_string()
            } else {
                "Disabling plugin...".to_string()
            };
            tasks::toggle_plugin_task(
                app.settings.project_path.clone(),
                plugin_id,
                target,
                enabled,
            )
        }
        Message::PluginToggled(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::RequestRemovePlugin(plugin_id, target) => {
            app.inventory.pending_remove_plugin = Some(plugin_id.clone());
            app.status =
                "Press Confirm remove to move the installed plugin to a backup.".to_string();
            app.inventory.selected_resource_id = Some(plugin_id.clone());
            let _ = target;
            Task::none()
        }
        Message::ConfirmRemovePlugin(plugin_id, target) => {
            app.busy = true;
            app.status = "Removing plugin...".to_string();
            app.inventory.selected_resource_id = None;
            app.inventory.pending_remove_plugin = None;
            tasks::remove_plugin_task(app.settings.project_path.clone(), plugin_id, target)
        }
        Message::PluginRemoved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::McpNameChanged(value) => {
            app.mcp.name = value;
            sync_expanded_editor(app, ExpandedEditorTarget::McpName);
            Task::none()
        }
        Message::McpTargetSelected(value) => {
            app.mcp.target = value;
            Task::none()
        }
        Message::McpTransportSelected(value) => {
            app.mcp.transport = value;
            Task::none()
        }
        Message::McpCommandChanged(value) => {
            app.mcp.command = value;
            sync_expanded_editor(app, ExpandedEditorTarget::McpCommand);
            Task::none()
        }
        Message::McpArgsChanged(value) => {
            app.mcp.args = value;
            sync_expanded_editor(app, ExpandedEditorTarget::McpArgs);
            Task::none()
        }
        Message::McpEnvChanged(value) => {
            app.mcp.env = value;
            sync_expanded_editor(app, ExpandedEditorTarget::McpEnv);
            Task::none()
        }
        Message::McpUrlChanged(value) => {
            app.mcp.url = value;
            sync_expanded_editor(app, ExpandedEditorTarget::McpUrl);
            Task::none()
        }
        Message::McpHeadersChanged(value) => {
            app.mcp.headers = value;
            sync_expanded_editor(app, ExpandedEditorTarget::McpHeaders);
            Task::none()
        }
        Message::McpEnabledChanged(value) => {
            app.mcp.enabled = value;
            Task::none()
        }
        Message::AddMcpServer => {
            let request = match current_mcp_request(&app.mcp) {
                Ok(request) => request,
                Err(error) => {
                    app.status = error;
                    return Task::none();
                }
            };
            app.busy = true;
            app.status = "Adding MCP server...".to_string();
            tasks::add_mcp_server_task(app.settings.project_path.clone(), request)
        }
        Message::McpServerAdded(result) => {
            app.busy = false;
            match result {
                Ok(message) => {
                    app.status = message;
                    app.form_editor.active = None;
                    app.mcp.name.clear();
                    app.mcp.command.clear();
                    app.mcp.args.clear();
                    app.mcp.env.clear();
                    app.mcp.url.clear();
                    app.mcp.headers.clear();
                }
                Err(error) => app.status = error,
            }
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::SetMcpServerEnabled(name, target, enabled) => {
            app.busy = true;
            app.mcp.pending_remove = None;
            app.status = if enabled {
                "Enabling MCP server...".to_string()
            } else {
                "Disabling MCP server...".to_string()
            };
            tasks::toggle_mcp_server_task(app.settings.project_path.clone(), name, target, enabled)
        }
        Message::McpServerToggled(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::RequestRemoveMcpServer(id) => {
            app.mcp.pending_remove = Some(id.clone());
            app.inventory.selected_resource_id = Some(id);
            app.status = "Press Confirm remove to remove the MCP server entry.".to_string();
            Task::none()
        }
        Message::ConfirmRemoveMcpServer(name, target) => {
            app.busy = true;
            app.mcp.pending_remove = None;
            app.inventory.selected_resource_id = None;
            app.status = "Removing MCP server...".to_string();
            tasks::remove_mcp_server_task(app.settings.project_path.clone(), name, target)
        }
        Message::McpServerRemoved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::PreviewDownloaded(root_dir) => {
            let target = match current_install_target(&app.install) {
                Ok(target) => target,
                Err(error) => {
                    app.status = error;
                    return Task::none();
                }
            };
            app.install.selected_download_root = Some(root_dir.clone());
            app.install.install_source = InstallSource::Downloaded;
            app.install.preview = None;
            app.busy = true;
            app.status = "Previewing downloaded skills...".to_string();
            tasks::preview_task(
                app.settings.project_path.clone(),
                InstallSource::Downloaded,
                root_dir.display().to_string(),
                optional_string(&app.install.download_path_override),
                target,
                app.install.conflict_policy.into(),
                app.install.enable_after_install,
            )
        }
        Message::RequestRemoveDownload(root_dir) => {
            app.install.pending_remove_download = Some(root_dir);
            app.status = "Press Confirm delete to remove the downloaded cache.".to_string();
            Task::none()
        }
        Message::ConfirmRemoveDownload(root_dir) => {
            app.busy = true;
            app.install.pending_remove_download = None;
            app.status = "Deleting downloaded skills cache...".to_string();
            tasks::remove_download_task(app.settings.project_path.clone(), root_dir)
        }
        Message::DownloadRemoved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::CreateNameChanged(value) => {
            app.create.name = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateName);
            Task::none()
        }
        Message::CreateDescriptionChanged(value) => {
            app.create.description = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateDescription);
            Task::none()
        }
        Message::CreateTargetSelected(target) => {
            app.create.target = target;
            app.create.preview = None;
            Task::none()
        }
        Message::CreateCustomPathChanged(value) => {
            app.create.custom_path = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateCustomPath);
            Task::none()
        }
        Message::CreateTagsChanged(value) => {
            app.create.tags = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateTags);
            Task::none()
        }
        Message::CreateAllowedToolsChanged(value) => {
            app.create.allowed_tools = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateAllowedTools);
            Task::none()
        }
        Message::CreateCompatibilityChanged(value) => {
            app.create.compatibility = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateCompatibility);
            Task::none()
        }
        Message::CreateLicenseChanged(value) => {
            app.create.license = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateLicense);
            Task::none()
        }
        Message::CreateWhenToUseChanged(value) => {
            app.create.when_to_use = value;
            app.create.preview = None;
            sync_expanded_editor(app, ExpandedEditorTarget::CreateWhenToUse);
            Task::none()
        }
        Message::CreateDisableModelInvocationChanged(value) => {
            app.create.disable_model_invocation = value;
            app.create.preview = None;
            Task::none()
        }
        Message::PreviewScaffold => {
            let request = match current_scaffold_request(&app.create) {
                Ok(request) => request,
                Err(error) => {
                    app.status = error;
                    return Task::none();
                }
            };
            app.busy = true;
            app.status = "Previewing skill scaffold...".to_string();
            tasks::preview_scaffold_task(app.settings.project_path.clone(), request)
        }
        Message::ScaffoldPreviewed(result) => {
            app.busy = false;
            match result {
                Ok(preview) => {
                    app.status = format!(
                        "Previewed {} scaffold.",
                        preview
                            .frontmatter
                            .name
                            .as_deref()
                            .unwrap_or("unnamed skill")
                    );
                    app.create.preview = Some(preview);
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::CreateSkill => {
            let request = match current_scaffold_request(&app.create) {
                Ok(request) => request,
                Err(error) => {
                    app.status = error;
                    return Task::none();
                }
            };
            app.busy = true;
            app.status = "Creating skill scaffold...".to_string();
            tasks::create_scaffold_task(app.settings.project_path.clone(), request)
        }
        Message::SkillCreated(result) => {
            app.busy = false;
            match result {
                Ok(preview) => {
                    app.status =
                        format!("Created skill at {}.", preview.destination_root.display());
                    app.create.preview = Some(preview);
                }
                Err(error) => app.status = error,
            }
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::SmokeExit => iced::exit(),
    }
}

fn open_expanded_editor(app: &mut App, target: ExpandedEditorTarget) {
    let value = expanded_editor_value(app, target);
    app.form_editor.active = Some(target);
    app.form_editor.content = text_editor::Content::with_text(&value);
}

fn sync_expanded_editor(app: &mut App, target: ExpandedEditorTarget) {
    if app.form_editor.active == Some(target) {
        let value = expanded_editor_value(app, target);
        app.form_editor.content = text_editor::Content::with_text(&value);
    }
}

fn expanded_editor_value(app: &App, target: ExpandedEditorTarget) -> String {
    match target {
        ExpandedEditorTarget::InstallSourceUrl => app.install.source_url.clone(),
        ExpandedEditorTarget::InstallLocalSourcePath => app.install.local_source_path.clone(),
        ExpandedEditorTarget::InstallCatalogUrl => app.install.catalog_url.clone(),
        ExpandedEditorTarget::InstallDownloadPathOverride => {
            app.install.download_path_override.clone()
        }
        ExpandedEditorTarget::InstallCustomPath => app.install.custom_install_path.clone(),
        ExpandedEditorTarget::McpName => app.mcp.name.clone(),
        ExpandedEditorTarget::McpCommand => app.mcp.command.clone(),
        ExpandedEditorTarget::McpArgs => app.mcp.args.clone(),
        ExpandedEditorTarget::McpEnv => app.mcp.env.clone(),
        ExpandedEditorTarget::McpUrl => app.mcp.url.clone(),
        ExpandedEditorTarget::McpHeaders => app.mcp.headers.clone(),
        ExpandedEditorTarget::CreateName => app.create.name.clone(),
        ExpandedEditorTarget::CreateDescription => app.create.description.clone(),
        ExpandedEditorTarget::CreateCustomPath => app.create.custom_path.clone(),
        ExpandedEditorTarget::CreateTags => app.create.tags.clone(),
        ExpandedEditorTarget::CreateAllowedTools => app.create.allowed_tools.clone(),
        ExpandedEditorTarget::CreateCompatibility => app.create.compatibility.clone(),
        ExpandedEditorTarget::CreateLicense => app.create.license.clone(),
        ExpandedEditorTarget::CreateWhenToUse => app.create.when_to_use.clone(),
    }
}

fn set_expanded_editor_value(app: &mut App, target: ExpandedEditorTarget, value: String) {
    match target {
        ExpandedEditorTarget::InstallSourceUrl => {
            app.install.source_url = value;
            app.install.preview = None;
        }
        ExpandedEditorTarget::InstallLocalSourcePath => {
            app.install.local_source_path = value;
            app.install.preview = None;
        }
        ExpandedEditorTarget::InstallCatalogUrl => app.install.catalog_url = value,
        ExpandedEditorTarget::InstallDownloadPathOverride => {
            app.install.download_path_override = value;
        }
        ExpandedEditorTarget::InstallCustomPath => {
            app.install.custom_install_path = value;
            app.install.preview = None;
        }
        ExpandedEditorTarget::McpName => app.mcp.name = value,
        ExpandedEditorTarget::McpCommand => app.mcp.command = value,
        ExpandedEditorTarget::McpArgs => app.mcp.args = value,
        ExpandedEditorTarget::McpEnv => app.mcp.env = value,
        ExpandedEditorTarget::McpUrl => app.mcp.url = value,
        ExpandedEditorTarget::McpHeaders => app.mcp.headers = value,
        ExpandedEditorTarget::CreateName => {
            app.create.name = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateDescription => {
            app.create.description = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateCustomPath => {
            app.create.custom_path = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateTags => {
            app.create.tags = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateAllowedTools => {
            app.create.allowed_tools = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateCompatibility => {
            app.create.compatibility = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateLicense => {
            app.create.license = value;
            app.create.preview = None;
        }
        ExpandedEditorTarget::CreateWhenToUse => {
            app.create.when_to_use = value;
            app.create.preview = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;
    use crate::app::filters::ScopeFilter;
    use skills_manager_core::{
        AgentToolTarget, ConflictPolicy, InstallTarget, InstalledSkill, ResourceKind,
        SkillEnablement, SkillFrontmatter, SkillHealth, SkillScope, WorkspaceSnapshot,
    };

    #[test]
    fn catalog_entry_preserves_github_branch_with_path() {
        let entry = catalog_entry_from_source(
            "Demo".to_string(),
            "Demo skill".to_string(),
            skills_manager_core::SkillCatalogSource::Git {
                url: "https://github.com/acme/skills/tree/dev".to_string(),
                path: Some("demo".to_string()),
            },
        );

        assert_eq!(entry.install_source, Some(InstallSource::Url));
        assert_eq!(
            entry.source_value.as_deref(),
            Some("https://github.com/acme/skills/tree/dev/demo")
        );
        assert!(entry.unavailable_reason.is_none());
    }

    #[test]
    fn catalog_entry_marks_unsupported_git_source_unavailable() {
        let entry = catalog_entry_from_source(
            "Demo".to_string(),
            "Demo skill".to_string(),
            skills_manager_core::SkillCatalogSource::Git {
                url: "https://example.com/acme/skills".to_string(),
                path: Some("demo".to_string()),
            },
        );

        assert_eq!(entry.install_source, None);
        assert_eq!(entry.source_value, None);
        assert!(entry.unavailable_reason.is_some());
    }

    #[test]
    fn inventory_state_filters_by_target_and_updates_detail_tab() {
        let (mut app, _) = App::init_with_smoke_test(false);
        app.skills = vec![
            installed_skill(SkillScope::Zed, "zed-demo"),
            installed_skill(SkillScope::Codex, "codex-demo"),
        ];

        app.inventory.scope_filter = ScopeFilter::Zed;
        app.rebuild_derived();
        app.ensure_selection();
        let visible = app.filtered_skills();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].scope, SkillScope::Zed);
        assert_eq!(
            app.inventory.selected_skill_id.as_deref(),
            Some("zed:zed-demo")
        );
    }

    #[test]
    fn workspace_loaded_populates_library_targets_and_download_path() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        let skill_dir = project.join(".agents/skills/demo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: demo\ndescription: Use this skill when loading workspace snapshots\n---\n",
        )
        .unwrap();
        let paths = skills_manager_core::ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(skills_manager_core::ProjectRoot::new(&project)),
        );
        let snapshot = WorkspaceSnapshot::load(&paths).unwrap();

        let (mut app, _) = App::init_with_smoke_test(false);
        let _ = update(&mut app, Message::WorkspaceLoaded(Ok(snapshot)));

        assert_eq!(app.skills.len(), 1);
        assert!(app.snapshot.is_some());
        assert!(!app.settings.target_profiles.is_empty());
        assert!(app.settings.default_download_path.contains("downloads"));
        assert_eq!(app.active_view, ActiveView::Library);
    }

    #[test]
    fn workspace_loaded_populates_marketplace_resources() {
        let dir = tempdir().unwrap();
        let paths = skills_manager_core::ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            None,
        );
        skills_manager_core::add_marketplace_source(
            &paths,
            "local-market",
            "/tmp/marketplace.json",
            Some(AgentToolTarget::Codex),
            Some("file".to_string()),
        )
        .unwrap();
        let snapshot = WorkspaceSnapshot::load(&paths).unwrap();

        let (mut app, _) = App::init_with_smoke_test(false);
        let _ = update(&mut app, Message::WorkspaceLoaded(Ok(snapshot)));

        assert!(app.resources.iter().any(|resource| {
            resource.kind == ResourceKind::Marketplace && resource.display_name == "local-market"
        }));
        assert!(app.derived.resource_search.iter().any(|entry| {
            entry.kind == ResourceKind::Marketplace && entry.haystack.contains("local-market")
        }));
    }

    #[test]
    fn derived_inventory_state_tracks_filters_summaries_and_visibility() {
        let (mut app, _) = App::init_with_smoke_test(false);
        let mut disabled = installed_skill(SkillScope::Codex, "codex-disabled");
        disabled.enablement = SkillEnablement::Disabled;
        app.skills = vec![
            installed_skill(SkillScope::Project, "shared"),
            installed_skill(SkillScope::Global, "shared"),
            disabled,
        ];

        app.rebuild_derived();

        assert_eq!(app.counts().enabled, 2);
        assert_eq!(app.counts().disabled, 1);
        assert_eq!(
            app.visible_scopes_for_skill(&app.skills[0]),
            vec![SkillScope::Project, SkillScope::Global]
        );

        app.inventory.skill_search_query = "disabled".to_string();
        app.rebuild_derived();
        let visible = app.filtered_skills();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].scope, SkillScope::Codex);
    }

    #[test]
    fn page_search_queries_are_independent() {
        let (mut app, _) = App::init_with_smoke_test(false);
        app.skills = vec![
            installed_skill(SkillScope::Zed, "zed-demo"),
            installed_skill(SkillScope::Codex, "codex-demo"),
        ];
        app.inventory.skill_search_query = "codex".to_string();
        app.rebuild_derived();
        assert_eq!(app.filtered_skills().len(), 1);

        let _ = update(
            &mut app,
            Message::PluginSearchChanged("browser".to_string()),
        );

        assert_eq!(app.inventory.skill_search_query, "codex");
        assert_eq!(app.inventory.plugin_search_query, "browser");
        assert_eq!(app.filtered_skills().len(), 1);
    }

    #[test]
    fn install_preview_without_operation_plan_uses_source_parameters() {
        let (mut app, _) = App::init_with_smoke_test(false);
        app.install.preview = Some(PreviewState {
            source_label: "local source".to_string(),
            source: InstallSource::Local,
            source_value: "source".to_string(),
            download_dir: None,
            target: InstallTarget::Global,
            enable_after_install: true,
            scope: SkillScope::Global,
            conflict_policy: ConflictPolicy::Block,
            operation_plan: None,
            candidates: Vec::new(),
        });

        let _task = update(&mut app, Message::InstallPreview);

        assert!(app.busy);
        assert_eq!(app.status, "Installing previewed skill(s)...");
    }

    fn installed_skill(scope: SkillScope, name: &str) -> InstalledSkill {
        InstalledSkill {
            id: format!("{}:{name}", scope.id_prefix()),
            display_name: name.to_string(),
            description: Some(format!("Use this skill when testing {name}")),
            scope,
            root_dir: PathBuf::from(name),
            skill_file: PathBuf::from(name).join("SKILL.md"),
            frontmatter: SkillFrontmatter {
                name: Some(name.to_string()),
                description: Some(format!("Use this skill when testing {name}")),
                ..Default::default()
            },
            enablement: SkillEnablement::Enabled,
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
