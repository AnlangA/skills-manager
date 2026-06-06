use std::{fmt, path::PathBuf};

use iced::{Element, Task};
use skills_manager_core::{
    CatalogFormat, ConflictPolicy, InstalledSkill, SkillCatalogSource, SkillEnablement,
    SkillHealth, SkillScope, catalog_git_install_url,
};

use crate::{tasks, views};

#[derive(Debug, Clone)]
pub struct App {
    pub project_path: String,
    pub search_query: String,
    pub skills: Vec<InstalledSkill>,
    pub selected_skill_id: Option<String>,
    pub active_view: ActiveView,
    pub scope_filter: ScopeFilter,
    pub health_filter: HealthFilter,
    pub source_filter: SourceFilter,
    pub sort_key: SortKey,
    pub install_source: InstallSource,
    pub source_url: String,
    pub local_source_path: String,
    pub catalog_url: String,
    pub install_scope: UiScope,
    pub conflict_policy: UiConflictPolicy,
    pub preview: Option<PreviewState>,
    pub catalog_entries: Vec<CatalogEntryState>,
    pub catalog_format: UiCatalogFormat,
    pub catalog_save_path: String,
    pub catalog_output: String,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    SkillsLoaded(Result<Vec<InstalledSkill>, String>),
    ProjectPathChanged(String),
    SearchChanged(String),
    ActiveViewSelected(ActiveView),
    ScopeFilterSelected(ScopeFilter),
    HealthFilterSelected(HealthFilter),
    SourceFilterSelected(SourceFilter),
    SortSelected(SortKey),
    SelectSkill(String),
    InstallSourceSelected(InstallSource),
    SourceUrlChanged(String),
    LocalSourcePathChanged(String),
    CatalogUrlChanged(String),
    InstallScopeSelected(UiScope),
    ConflictSelected(UiConflictPolicy),
    PreviewInstall,
    PreviewLoaded(Result<PreviewState, String>),
    InstallPreview,
    Installed(Result<String, String>),
    LoadCatalog,
    CatalogLoaded(Result<Vec<CatalogEntryState>, String>),
    PreviewCatalogEntry(InstallSource, String),
    CatalogFormatSelected(UiCatalogFormat),
    CatalogSavePathChanged(String),
    GenerateCatalog,
    CatalogGenerated(Result<String, String>),
    CopyCatalog,
    SaveCatalog,
    CatalogSaved(Result<String, String>),
    SetSkillEnabled(PathBuf, bool),
    SkillToggled(Result<String, String>),
    RemoveSkill(PathBuf),
    SkillRemoved(Result<String, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Inventory,
    Install,
    Catalog,
    Settings,
}

impl ActiveView {
    pub const ALL: [Self; 4] = [
        Self::Inventory,
        Self::Install,
        Self::Catalog,
        Self::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Inventory => "Inventory",
            Self::Install => "Install",
            Self::Catalog => "Catalog",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeFilter {
    All,
    Project,
    User,
}

impl ScopeFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::Project, Self::User];

    pub fn matches(self, scope: SkillScope) -> bool {
        match self {
            Self::All => true,
            Self::Project => scope == SkillScope::Project,
            Self::User => scope == SkillScope::User,
        }
    }
}

impl fmt::Display for ScopeFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All scopes",
            Self::Project => "Project",
            Self::User => "User",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthFilter {
    All,
    NeedsAttention,
    Valid,
    Warning,
    Invalid,
    Shadowed,
}

impl HealthFilter {
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::NeedsAttention,
        Self::Valid,
        Self::Warning,
        Self::Invalid,
        Self::Shadowed,
    ];

    pub fn matches(self, health: SkillHealth) -> bool {
        match self {
            Self::All => true,
            Self::NeedsAttention => matches!(
                health,
                SkillHealth::Warning | SkillHealth::Invalid | SkillHealth::Shadowed
            ),
            Self::Valid => health == SkillHealth::Valid,
            Self::Warning => health == SkillHealth::Warning,
            Self::Invalid => health == SkillHealth::Invalid,
            Self::Shadowed => health == SkillHealth::Shadowed,
        }
    }
}

impl fmt::Display for HealthFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All health",
            Self::NeedsAttention => "Needs attention",
            Self::Valid => "Valid",
            Self::Warning => "Warning",
            Self::Invalid => "Invalid",
            Self::Shadowed => "Shadowed",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFilter {
    All,
    Managed,
    Unknown,
}

impl SourceFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::Managed, Self::Unknown];

    pub fn matches(self, skill: &InstalledSkill) -> bool {
        match self {
            Self::All => true,
            Self::Managed => skill.source_url.is_some(),
            Self::Unknown => skill.source_url.is_none(),
        }
    }
}

impl fmt::Display for SourceFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All sources",
            Self::Managed => "Known source",
            Self::Unknown => "Unknown source",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Priority,
    Name,
    Health,
    Scope,
    Resources,
    Installed,
}

impl SortKey {
    pub const ALL: [Self; 6] = [
        Self::Priority,
        Self::Name,
        Self::Health,
        Self::Scope,
        Self::Resources,
        Self::Installed,
    ];
}

impl fmt::Display for SortKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Priority => "Priority",
            Self::Name => "Name",
            Self::Health => "Health",
            Self::Scope => "Scope",
            Self::Resources => "Resources",
            Self::Installed => "Installed",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    Url,
    Local,
    Catalog,
}

impl InstallSource {
    pub const ALL: [Self; 3] = [Self::Url, Self::Local, Self::Catalog];
}

impl fmt::Display for InstallSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Url => "GitHub URL",
            Self::Local => "Local folder",
            Self::Catalog => "Catalog",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScope {
    User,
    Project,
}

impl UiScope {
    pub const ALL: [Self; 2] = [Self::User, Self::Project];
}

impl From<UiScope> for SkillScope {
    fn from(value: UiScope) -> Self {
        match value {
            UiScope::User => Self::User,
            UiScope::Project => Self::Project,
        }
    }
}

impl fmt::Display for UiScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::User => "User",
            Self::Project => "Project",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiConflictPolicy {
    Block,
    Rename,
    Replace,
}

impl UiConflictPolicy {
    pub const ALL: [Self; 3] = [Self::Block, Self::Rename, Self::Replace];
}

impl From<UiConflictPolicy> for ConflictPolicy {
    fn from(value: UiConflictPolicy) -> Self {
        match value {
            UiConflictPolicy::Block => Self::Block,
            UiConflictPolicy::Rename => Self::Rename,
            UiConflictPolicy::Replace => Self::Replace,
        }
    }
}

impl fmt::Display for UiConflictPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Block => "Block conflicts",
            Self::Rename => "Rename new skill",
            Self::Replace => "Replace with backup",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCatalogFormat {
    Json,
    Xml,
    Markdown,
}

impl UiCatalogFormat {
    pub const ALL: [Self; 3] = [Self::Json, Self::Xml, Self::Markdown];
}

impl From<UiCatalogFormat> for CatalogFormat {
    fn from(value: UiCatalogFormat) -> Self {
        match value {
            UiCatalogFormat::Json => Self::Json,
            UiCatalogFormat::Xml => Self::Xml,
            UiCatalogFormat::Markdown => Self::Markdown,
        }
    }
}

impl fmt::Display for UiCatalogFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Json => "JSON",
            Self::Xml => "XML",
            Self::Markdown => "Markdown",
        })
    }
}

#[derive(Debug, Clone)]
pub struct PreviewState {
    pub source_label: String,
    pub scope: SkillScope,
    pub conflict_policy: ConflictPolicy,
    pub candidates: Vec<PreviewCandidateState>,
}

impl PreviewState {
    pub fn has_blocking_conflicts(&self) -> bool {
        self.conflict_policy == ConflictPolicy::Block
            && self.candidates.iter().any(|candidate| candidate.conflict)
    }
}

#[derive(Debug, Clone)]
pub struct PreviewCandidateState {
    pub name: String,
    pub description: String,
    pub source_root: PathBuf,
    pub destination_root: PathBuf,
    pub health: SkillHealth,
    pub conflict: bool,
    pub diagnostics: Vec<String>,
    pub resource_count: usize,
    pub resource_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CatalogEntryState {
    pub name: String,
    pub description: String,
    pub source_label: String,
    pub install_source: Option<InstallSource>,
    pub source_value: Option<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct SkillCounts {
    pub enabled: usize,
    pub disabled: usize,
    pub valid: usize,
    pub warning: usize,
    pub invalid: usize,
    pub shadowed: usize,
    pub project: usize,
    pub user: usize,
    pub known_source: usize,
    pub exportable: usize,
}

impl App {
    pub fn init() -> (Self, Task<Message>) {
        let project_path = std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        let app = Self {
            project_path,
            search_query: String::new(),
            skills: Vec::new(),
            selected_skill_id: None,
            active_view: ActiveView::Inventory,
            scope_filter: ScopeFilter::All,
            health_filter: HealthFilter::All,
            source_filter: SourceFilter::All,
            sort_key: SortKey::Priority,
            install_source: InstallSource::Url,
            source_url: String::new(),
            local_source_path: String::new(),
            catalog_url: String::new(),
            install_scope: UiScope::User,
            conflict_policy: UiConflictPolicy::Block,
            preview: None,
            catalog_entries: Vec::new(),
            catalog_format: UiCatalogFormat::Json,
            catalog_save_path: "agent-skills-catalog.json".to_string(),
            catalog_output: String::new(),
            status: "Ready".to_string(),
            busy: true,
        };

        let task = tasks::refresh_task(app.project_path.clone());
        (app, task)
    }

    pub fn filtered_skills(&self) -> Vec<&InstalledSkill> {
        let mut skills = self
            .skills
            .iter()
            .filter(|skill| self.skill_matches(skill))
            .collect::<Vec<_>>();
        self.sort_skills(&mut skills);
        skills
    }

    pub fn selected_skill(&self) -> Option<&InstalledSkill> {
        self.selected_skill_id
            .as_ref()
            .and_then(|id| {
                self.skills
                    .iter()
                    .find(|skill| skill.id == *id && self.skill_matches(skill))
            })
            .or_else(|| self.skills.iter().find(|skill| self.skill_matches(skill)))
    }

    pub fn counts(&self) -> SkillCounts {
        self.skills.iter().fold(
            SkillCounts {
                enabled: 0,
                disabled: 0,
                valid: 0,
                warning: 0,
                invalid: 0,
                shadowed: 0,
                project: 0,
                user: 0,
                known_source: 0,
                exportable: 0,
            },
            |mut counts, skill| {
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
                    SkillScope::User => counts.user += 1,
                }
                if skill.source_url.is_some() {
                    counts.known_source += 1;
                }
                if skill.is_exportable() {
                    counts.exportable += 1;
                }
                counts
            },
        )
    }

    pub fn ensure_selection(&mut self) {
        let visible_ids = self
            .filtered_skills()
            .into_iter()
            .map(|skill| skill.id.clone())
            .collect::<Vec<_>>();

        if self
            .selected_skill_id
            .as_ref()
            .is_some_and(|selected| visible_ids.iter().any(|id| id == selected))
        {
            return;
        }

        self.selected_skill_id = visible_ids.into_iter().next();
    }

    fn skill_matches(&self, skill: &InstalledSkill) -> bool {
        if !self.scope_filter.matches(skill.scope)
            || !self.health_filter.matches(skill.health)
            || !self.source_filter.matches(skill)
        {
            return false;
        }

        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }

        skill.display_name.to_lowercase().contains(&query)
            || skill
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase()
                .contains(&query)
            || skill
                .root_dir
                .display()
                .to_string()
                .to_lowercase()
                .contains(&query)
            || skill
                .frontmatter
                .allowed_tools
                .iter()
                .any(|tool| tool.to_lowercase().contains(&query))
    }

    fn sort_skills(&self, skills: &mut Vec<&InstalledSkill>) {
        skills.sort_by(|left, right| match self.sort_key {
            SortKey::Priority => left
                .scope
                .sort_rank()
                .cmp(&right.scope.sort_rank())
                .then_with(|| health_rank(left.health).cmp(&health_rank(right.health)))
                .then_with(|| {
                    left.display_name
                        .to_lowercase()
                        .cmp(&right.display_name.to_lowercase())
                }),
            SortKey::Name => left
                .display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase()),
            SortKey::Health => health_rank(left.health)
                .cmp(&health_rank(right.health))
                .then_with(|| {
                    left.display_name
                        .to_lowercase()
                        .cmp(&right.display_name.to_lowercase())
                }),
            SortKey::Scope => left
                .scope
                .sort_rank()
                .cmp(&right.scope.sort_rank())
                .then_with(|| {
                    left.display_name
                        .to_lowercase()
                        .cmp(&right.display_name.to_lowercase())
                }),
            SortKey::Resources => right
                .resource_bytes
                .cmp(&left.resource_bytes)
                .then_with(|| right.resource_count.cmp(&left.resource_count)),
            SortKey::Installed => right.installed_at.cmp(&left.installed_at).then_with(|| {
                left.display_name
                    .to_lowercase()
                    .cmp(&right.display_name.to_lowercase())
            }),
        });
    }
}

fn health_rank(health: SkillHealth) -> u8 {
    match health {
        SkillHealth::Invalid => 0,
        SkillHealth::Warning => 1,
        SkillHealth::Shadowed => 2,
        SkillHealth::Valid => 3,
    }
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Refresh => {
            app.busy = true;
            app.status = "Scanning Agent Skills...".to_string();
            tasks::refresh_task(app.project_path.clone())
        }
        Message::SkillsLoaded(result) => {
            app.busy = false;
            match result {
                Ok(skills) => {
                    app.status = format!("Loaded {} skill(s).", skills.len());
                    app.skills = skills;
                    app.ensure_selection();
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::ProjectPathChanged(value) => {
            app.project_path = value;
            Task::none()
        }
        Message::SearchChanged(value) => {
            app.search_query = value;
            app.ensure_selection();
            Task::none()
        }
        Message::ActiveViewSelected(view) => {
            app.active_view = view;
            Task::none()
        }
        Message::ScopeFilterSelected(scope_filter) => {
            app.scope_filter = scope_filter;
            app.ensure_selection();
            Task::none()
        }
        Message::HealthFilterSelected(health_filter) => {
            app.health_filter = health_filter;
            app.ensure_selection();
            Task::none()
        }
        Message::SourceFilterSelected(source_filter) => {
            app.source_filter = source_filter;
            app.ensure_selection();
            Task::none()
        }
        Message::SortSelected(sort_key) => {
            app.sort_key = sort_key;
            app.ensure_selection();
            Task::none()
        }
        Message::SelectSkill(id) => {
            app.selected_skill_id = Some(id);
            Task::none()
        }
        Message::InstallSourceSelected(source) => {
            app.install_source = source;
            app.preview = None;
            Task::none()
        }
        Message::SourceUrlChanged(value) => {
            app.source_url = value;
            Task::none()
        }
        Message::LocalSourcePathChanged(value) => {
            app.local_source_path = value;
            Task::none()
        }
        Message::CatalogUrlChanged(value) => {
            app.catalog_url = value;
            Task::none()
        }
        Message::InstallScopeSelected(scope) => {
            app.install_scope = scope;
            Task::none()
        }
        Message::ConflictSelected(policy) => {
            app.conflict_policy = policy;
            Task::none()
        }
        Message::PreviewInstall => {
            let source = current_source_value(app);
            if source.trim().is_empty() {
                app.status = "Enter a source before previewing.".to_string();
                return Task::none();
            }

            app.busy = true;
            app.preview = None;
            app.status = "Building install preview...".to_string();
            tasks::preview_task(
                app.project_path.clone(),
                app.install_source,
                source,
                app.install_scope.into(),
                app.conflict_policy.into(),
            )
        }
        Message::PreviewLoaded(result) => {
            app.busy = false;
            match result {
                Ok(preview) => {
                    app.status = format!("Previewed {} candidate(s).", preview.candidates.len());
                    app.preview = Some(preview);
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::InstallPreview => {
            let source = current_source_value(app);
            if source.trim().is_empty() {
                app.status = "Enter a source before installing.".to_string();
                return Task::none();
            }

            app.busy = true;
            app.status = "Installing previewed skill(s)...".to_string();
            tasks::install_task(
                app.project_path.clone(),
                app.install_source,
                source,
                app.install_scope.into(),
                app.conflict_policy.into(),
            )
        }
        Message::Installed(result) => {
            app.busy = false;
            app.preview = None;
            app.status = result.unwrap_or_else(|error| error);
            tasks::refresh_task(app.project_path.clone())
        }
        Message::LoadCatalog => {
            let url = app.catalog_url.trim().to_string();
            if url.is_empty() {
                app.status = "Enter a catalog URL first.".to_string();
                return Task::none();
            }
            app.busy = true;
            app.catalog_entries.clear();
            app.status = "Loading catalog...".to_string();
            tasks::load_catalog_task(url)
        }
        Message::CatalogLoaded(result) => {
            app.busy = false;
            match result {
                Ok(entries) => {
                    app.status = format!("Loaded {} catalog entrie(s).", entries.len());
                    app.catalog_entries = entries;
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::PreviewCatalogEntry(source, value) => {
            match source {
                InstallSource::Url => {
                    app.source_url = value.clone();
                }
                InstallSource::Local => {
                    app.local_source_path = value.clone();
                }
                InstallSource::Catalog => {}
            }
            app.install_source = source;
            app.preview = None;
            app.busy = true;
            app.status = "Previewing catalog entry...".to_string();
            tasks::preview_task(
                app.project_path.clone(),
                source,
                value,
                app.install_scope.into(),
                app.conflict_policy.into(),
            )
        }
        Message::CatalogFormatSelected(format) => {
            app.catalog_format = format;
            Task::none()
        }
        Message::CatalogSavePathChanged(value) => {
            app.catalog_save_path = value;
            Task::none()
        }
        Message::GenerateCatalog => {
            app.busy = true;
            app.status = "Generating catalog export...".to_string();
            tasks::generate_catalog_task(app.project_path.clone(), app.catalog_format.into())
        }
        Message::CatalogGenerated(result) => {
            app.busy = false;
            match result {
                Ok(output) => {
                    app.status = "Catalog export generated.".to_string();
                    app.catalog_output = output;
                }
                Err(error) => app.status = error,
            }
            Task::none()
        }
        Message::CopyCatalog => {
            if app.catalog_output.is_empty() {
                app.status = "Generate a catalog before copying.".to_string();
                Task::none()
            } else {
                app.status = "Catalog export copied to clipboard.".to_string();
                iced::clipboard::write(app.catalog_output.clone())
            }
        }
        Message::SaveCatalog => {
            if app.catalog_output.is_empty() {
                app.status = "Generate a catalog before saving.".to_string();
                Task::none()
            } else if app.catalog_save_path.trim().is_empty() {
                app.status = "Enter a save path first.".to_string();
                Task::none()
            } else {
                app.busy = true;
                app.status = "Saving catalog export...".to_string();
                tasks::save_catalog_task(
                    app.project_path.clone(),
                    app.catalog_save_path.clone(),
                    app.catalog_output.clone(),
                )
            }
        }
        Message::CatalogSaved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            Task::none()
        }
        Message::SetSkillEnabled(skill_file, enabled) => {
            app.busy = true;
            app.status = if enabled {
                "Enabling skill...".to_string()
            } else {
                "Disabling skill...".to_string()
            };
            tasks::toggle_task(app.project_path.clone(), skill_file, !enabled)
        }
        Message::SkillToggled(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::refresh_task(app.project_path.clone())
        }
        Message::RemoveSkill(skill_root) => {
            app.busy = true;
            app.status = "Removing skill and creating backup...".to_string();
            if app
                .selected_skill_id
                .as_ref()
                .is_some_and(|selected| selected.contains(&skill_root.display().to_string()))
            {
                app.selected_skill_id = None;
            }
            tasks::remove_task(app.project_path.clone(), skill_root)
        }
        Message::SkillRemoved(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::refresh_task(app.project_path.clone())
        }
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    views::view(app)
}

fn current_source_value(app: &App) -> String {
    match app.install_source {
        InstallSource::Url => app.source_url.trim().to_string(),
        InstallSource::Local => app.local_source_path.trim().to_string(),
        InstallSource::Catalog => app.catalog_url.trim().to_string(),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_entry_preserves_github_branch_with_path() {
        let entry = catalog_entry_from_source(
            "Demo".to_string(),
            "Demo skill".to_string(),
            SkillCatalogSource::Git {
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
            SkillCatalogSource::Git {
                url: "https://example.com/acme/skills".to_string(),
                path: Some("demo".to_string()),
            },
        );

        assert_eq!(entry.install_source, None);
        assert_eq!(entry.source_value, None);
        assert!(entry.unavailable_reason.is_some());
    }
}
