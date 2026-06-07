use iced::Task;

use super::derived::*;
use super::helpers::*;
use super::message::Message;
use super::state::*;
use super::types::*;
use crate::tasks;

pub struct App {
    pub snapshot: Option<skills_manager_core::WorkspaceSnapshot>,
    pub skills: Vec<skills_manager_core::InstalledSkill>,
    pub resources: Vec<skills_manager_core::ManagedResource>,
    pub derived: DerivedInventoryState,
    pub active_view: ActiveView,
    pub inventory: InventoryState,
    pub install: InstallState,
    pub create: CreateState,
    pub catalog: CatalogExportState,
    pub marketplace: MarketplaceState,
    pub settings: AppSettingsState,
    pub status: String,
    pub busy: bool,
    pub smoke_test: bool,
}

impl App {
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
            install: InstallState::default(),
            create: CreateState::default(),
            catalog: CatalogExportState::default(),
            marketplace: MarketplaceState::default(),
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
        self.derived.filtered_skill_indices = filtered_indices(
            &self.skills,
            &self.inventory.search_query,
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
                    app.marketplace.sources = snapshot.marketplace_sources.clone();
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
        Message::SearchChanged(value) => {
            app.inventory.search_query = value;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::ActiveViewSelected(view) => {
            app.active_view = view;
            Task::none()
        }
        Message::ScopeFilterSelected(scope_filter) => {
            app.inventory.scope_filter = scope_filter;
            app.rebuild_derived();
            app.ensure_selection();
            Task::none()
        }
        Message::ResourceKindSelected(kind_filter) => {
            app.inventory.resource_kind_filter = kind_filter;
            app.inventory.selected_resource_id = None;
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
            Task::none()
        }
        Message::LocalSourcePathChanged(value) => {
            app.install.local_source_path = value;
            Task::none()
        }
        Message::CatalogUrlChanged(value) => {
            app.install.catalog_url = value;
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
        Message::MarketplaceSourceLabelChanged(value) => {
            app.marketplace.source_label = value;
            Task::none()
        }
        Message::MarketplaceSourceValueChanged(value) => {
            app.marketplace.source_value = value;
            Task::none()
        }
        Message::MarketplaceSourceTargetSelected(target) => {
            app.marketplace.source_target = target;
            Task::none()
        }
        Message::MarketplaceSourceProviderChanged(value) => {
            app.marketplace.source_provider = value;
            Task::none()
        }
        Message::AddMarketplaceSource => {
            let label = app.marketplace.source_label.trim().to_string();
            let source = app.marketplace.source_value.trim().to_string();
            if label.is_empty() || source.is_empty() {
                app.status = "Enter a marketplace label and source first.".to_string();
                return Task::none();
            }
            app.busy = true;
            app.status = "Adding marketplace source...".to_string();
            tasks::add_marketplace_source_task(
                app.settings.project_path.clone(),
                label,
                source,
                app.marketplace.source_target,
                optional_string(&app.marketplace.source_provider),
            )
        }
        Message::MarketplaceSourceAdded(result) => {
            app.busy = false;
            match result {
                Ok(source) => {
                    app.status = format!("Added marketplace source {}.", source.label);
                    app.marketplace.source_label.clear();
                    app.marketplace.source_value.clear();
                }
                Err(error) => app.status = error,
            }
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::RefreshMarketplaceSource(label) => {
            app.busy = true;
            app.status = format!("Refreshing marketplace source {label}...");
            tasks::refresh_marketplace_source_task(app.settings.project_path.clone(), label)
        }
        Message::MarketplaceInspected(result) => {
            app.busy = false;
            match result {
                Ok(document) => {
                    app.status = format!(
                        "Loaded marketplace {} with {} plugin(s).",
                        document.name,
                        document.entries.len()
                    );
                    app.marketplace.inspected_marketplace = Some(document);
                }
                Err(error) => app.status = error,
            }
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::RequestRemoveMarketplaceSource(label) => {
            app.marketplace.pending_remove_source = Some(label);
            app.status =
                "Press Confirm remove to delete the marketplace source record.".to_string();
            Task::none()
        }
        Message::ConfirmRemoveMarketplaceSource(label) => {
            app.busy = true;
            app.marketplace.pending_remove_source = None;
            app.status = "Removing marketplace source...".to_string();
            tasks::remove_marketplace_source_task(app.settings.project_path.clone(), label)
        }
        Message::MarketplaceSourceRemoved(result) => {
            app.busy = false;
            app.status = result
                .map(|label| format!("Removed marketplace source {label}."))
                .unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::MarketplaceSearchQueryChanged(value) => {
            app.marketplace.search_query = value;
            Task::none()
        }
        Message::SearchMarketplace => {
            let query = app.marketplace.search_query.trim().to_string();
            if query.is_empty() {
                app.status = "Enter a search query first.".to_string();
                return Task::none();
            }
            app.busy = true;
            app.status = "Searching marketplace provider...".to_string();
            tasks::search_marketplace_task(app.marketplace.search_provider, query)
        }
        Message::MarketplaceSearchLoaded(result) => {
            app.busy = false;
            match result {
                Ok(entries) => {
                    app.status = format!("Loaded {} marketplace result(s).", entries.len());
                    app.marketplace.search_results = entries;
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::PreviewMarketplaceSearchEntry(url) => {
            app.install.install_source = InstallSource::Url;
            app.install.source_url = url;
            app.install.preview = None;
            app.active_view = ActiveView::Install;
            app.status =
                "Marketplace result loaded into Install. Preview before installing.".to_string();
            Task::none()
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
            Task::none()
        }
        Message::CreateDescriptionChanged(value) => {
            app.create.description = value;
            app.create.preview = None;
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
            Task::none()
        }
        Message::CreateTagsChanged(value) => {
            app.create.tags = value;
            app.create.preview = None;
            Task::none()
        }
        Message::CreateAllowedToolsChanged(value) => {
            app.create.allowed_tools = value;
            app.create.preview = None;
            Task::none()
        }
        Message::CreateCompatibilityChanged(value) => {
            app.create.compatibility = value;
            app.create.preview = None;
            Task::none()
        }
        Message::CreateLicenseChanged(value) => {
            app.create.license = value;
            app.create.preview = None;
            Task::none()
        }
        Message::CreateWhenToUseChanged(value) => {
            app.create.when_to_use = value;
            app.create.preview = None;
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;
    use crate::app::filters::{ResourceKindFilter, ScopeFilter};
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
    fn resource_kind_filter_selection_resets_resource_selection() {
        let (mut app, _) = App::init_with_smoke_test(false);
        app.inventory.selected_resource_id = Some("plugin:codex:demo".to_string());

        let _ = update(
            &mut app,
            Message::ResourceKindSelected(ResourceKindFilter::Plugins),
        );

        assert_eq!(
            app.inventory.resource_kind_filter,
            ResourceKindFilter::Plugins
        );
        assert_eq!(app.inventory.selected_resource_id, None);
    }

    #[test]
    fn workspace_loaded_populates_marketplace_sources_and_resources() {
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

        assert_eq!(app.marketplace.sources.len(), 1);
        assert_eq!(app.marketplace.sources[0].label, "local-market");
        assert!(app.resources.iter().any(|resource| {
            resource.kind == ResourceKind::Marketplace && resource.display_name == "local-market"
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

        app.inventory.search_query = "disabled".to_string();
        app.rebuild_derived();
        let visible = app.filtered_skills();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].scope, SkillScope::Codex);
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
