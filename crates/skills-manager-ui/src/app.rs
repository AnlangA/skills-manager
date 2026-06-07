use std::{collections::BTreeMap, fmt, path::PathBuf, time::Duration};

use iced::{Element, Subscription, Task};
use skills_manager_core::{
    CatalogFormat, ConflictPolicy, DoctorReport, InstallTarget, InstalledSkill, OperationPlan,
    SkillCatalogSource, SkillEnablement, SkillHealth, SkillScaffoldPreview, SkillScaffoldRequest,
    SkillScope, TargetProfile, WorkspaceSnapshot, catalog_git_install_url,
    installed_skill_identity,
};

use crate::{tasks, views};

#[derive(Debug, Clone)]
pub struct App {
    pub snapshot: Option<WorkspaceSnapshot>,
    pub skills: Vec<InstalledSkill>,
    pub derived: DerivedInventoryState,
    pub active_view: ActiveView,
    pub inventory: InventoryState,
    pub install: InstallState,
    pub create: CreateState,
    pub catalog: CatalogExportState,
    pub settings: AppSettingsState,
    pub status: String,
    pub busy: bool,
    pub smoke_test: bool,
}

#[derive(Debug, Clone)]
pub struct InventoryState {
    pub search_query: String,
    pub selected_skill_id: Option<String>,
    pub scope_filter: ScopeFilter,
    pub health_filter: HealthFilter,
    pub source_filter: SourceFilter,
    pub sort_key: SortKey,
    pub detail_tab: DetailTab,
    pub pending_remove_skill: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct InstallState {
    pub install_source: InstallSource,
    pub source_url: String,
    pub local_source_path: String,
    pub catalog_url: String,
    pub download_path_override: String,
    pub install_scope: UiScope,
    pub custom_install_path: String,
    pub enable_after_install: bool,
    pub conflict_policy: UiConflictPolicy,
    pub preview: Option<PreviewState>,
    pub downloaded_entries: Vec<DownloadedEntryState>,
    pub selected_download_root: Option<PathBuf>,
    pub catalog_entries: Vec<CatalogEntryState>,
    pub pending_remove_download: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CreateState {
    pub name: String,
    pub description: String,
    pub target: UiScope,
    pub custom_path: String,
    pub tags: String,
    pub allowed_tools: String,
    pub compatibility: String,
    pub license: String,
    pub when_to_use: String,
    pub disable_model_invocation: bool,
    pub preview: Option<SkillScaffoldPreview>,
}

#[derive(Debug, Clone)]
pub struct CatalogExportState {
    pub catalog_format: UiCatalogFormat,
    pub catalog_save_path: String,
    pub catalog_output: String,
}

#[derive(Debug, Clone)]
pub struct AppSettingsState {
    pub project_path: String,
    pub default_download_path: String,
    pub target_profiles: Vec<TargetProfile>,
    pub doctor_report: Option<DoctorReport>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    WorkspaceLoaded(Result<WorkspaceSnapshot, String>),
    ProjectPathChanged(String),
    SearchChanged(String),
    ActiveViewSelected(ActiveView),
    ScopeFilterSelected(ScopeFilter),
    HealthFilterSelected(HealthFilter),
    SourceFilterSelected(SourceFilter),
    SortSelected(SortKey),
    SelectSkill(String),
    DetailTabSelected(DetailTab),
    InstallSourceSelected(InstallSource),
    SourceUrlChanged(String),
    LocalSourcePathChanged(String),
    CatalogUrlChanged(String),
    DefaultDownloadPathChanged(String),
    SaveDefaultDownloadPath,
    DefaultDownloadPathSaved(Result<String, String>),
    DownloadPathOverrideChanged(String),
    InstallScopeSelected(UiScope),
    CustomInstallPathChanged(String),
    EnableAfterInstallChanged(bool),
    ConflictSelected(UiConflictPolicy),
    DownloadSource,
    Downloaded(Result<String, String>),
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
    RequestRemoveSkill(PathBuf),
    ConfirmRemoveSkill(PathBuf),
    SkillRemoved(Result<String, String>),
    PreviewDownloaded(PathBuf),
    RequestRemoveDownload(PathBuf),
    ConfirmRemoveDownload(PathBuf),
    DownloadRemoved(Result<String, String>),
    CreateNameChanged(String),
    CreateDescriptionChanged(String),
    CreateTargetSelected(UiScope),
    CreateCustomPathChanged(String),
    CreateTagsChanged(String),
    CreateAllowedToolsChanged(String),
    CreateCompatibilityChanged(String),
    CreateLicenseChanged(String),
    CreateWhenToUseChanged(String),
    CreateDisableModelInvocationChanged(bool),
    PreviewScaffold,
    ScaffoldPreviewed(Result<SkillScaffoldPreview, String>),
    CreateSkill,
    SkillCreated(Result<SkillScaffoldPreview, String>),
    SmokeExit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Library,
    Install,
    Create,
    Catalog,
    Targets,
}

impl ActiveView {
    pub const ALL: [Self; 5] = [
        Self::Library,
        Self::Install,
        Self::Create,
        Self::Catalog,
        Self::Targets,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Install => "Install",
            Self::Create => "Create",
            Self::Catalog => "Catalog",
            Self::Targets => "Targets",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Overview,
    Visibility,
    Diagnostics,
    Files,
    Actions,
}

impl DetailTab {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Visibility,
        Self::Diagnostics,
        Self::Files,
        Self::Actions,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Visibility => "Visibility",
            Self::Diagnostics => "Diagnostics",
            Self::Files => "Files",
            Self::Actions => "Actions",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeFilter {
    All,
    Project,
    Global,
    ClaudeCode,
    Droid,
    Pencode,
    Codex,
    Zed,
    Custom,
}

impl ScopeFilter {
    pub const ALL: [Self; 9] = [
        Self::All,
        Self::Project,
        Self::Global,
        Self::ClaudeCode,
        Self::Droid,
        Self::Pencode,
        Self::Codex,
        Self::Zed,
        Self::Custom,
    ];

    pub fn matches(self, scope: SkillScope) -> bool {
        match self {
            Self::All => true,
            Self::Project => scope == SkillScope::Project,
            Self::Global => scope == SkillScope::Global,
            Self::ClaudeCode => scope == SkillScope::ClaudeCode,
            Self::Droid => scope == SkillScope::Droid,
            Self::Pencode => scope == SkillScope::Pencode,
            Self::Codex => scope == SkillScope::Codex,
            Self::Zed => scope == SkillScope::Zed,
            Self::Custom => scope == SkillScope::Custom,
        }
    }
}

impl fmt::Display for ScopeFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "All scopes",
            Self::Project => "Project",
            Self::Global => "Global",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::Pencode => "Pencode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom => "Custom",
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
    Downloaded,
    Catalog,
}

impl InstallSource {
    pub const ALL: [Self; 4] = [Self::Url, Self::Local, Self::Downloaded, Self::Catalog];
}

impl fmt::Display for InstallSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Url => "GitHub URL",
            Self::Local => "Local folder",
            Self::Downloaded => "Downloaded",
            Self::Catalog => "Catalog",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScope {
    Global,
    Project,
    ClaudeCode,
    Droid,
    Pencode,
    Codex,
    Zed,
    Custom,
}

impl UiScope {
    pub const ALL: [Self; 8] = [
        Self::Global,
        Self::Project,
        Self::ClaudeCode,
        Self::Droid,
        Self::Pencode,
        Self::Codex,
        Self::Zed,
        Self::Custom,
    ];
}

impl fmt::Display for UiScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Global => "Global",
            Self::Project => "Project",
            Self::ClaudeCode => "Claude Code",
            Self::Droid => "Droid",
            Self::Pencode => "Pencode",
            Self::Codex => "Codex",
            Self::Zed => "Zed",
            Self::Custom => "Custom",
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
    pub source: InstallSource,
    pub source_value: String,
    pub download_dir: Option<String>,
    pub target: InstallTarget,
    pub enable_after_install: bool,
    pub download_root: Option<PathBuf>,
    pub scope: SkillScope,
    pub destination_root: PathBuf,
    pub conflict_policy: ConflictPolicy,
    pub operation_plan: Option<OperationPlan>,
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

#[derive(Debug, Clone)]
pub struct DownloadedEntryState {
    pub source_url: String,
    pub root_dir: PathBuf,
    pub downloaded_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct DerivedInventoryState {
    pub filtered_skill_indices: Vec<usize>,
    pub counts: SkillCounts,
    pub scope_summaries: BTreeMap<SkillScope, ScopeSummary>,
    pub visible_scopes_by_id: BTreeMap<String, Vec<SkillScope>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopeSummary {
    pub total: usize,
    pub usable: usize,
    pub disabled: usize,
    pub invalid: usize,
    pub attention: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillCounts {
    pub enabled: usize,
    pub disabled: usize,
    pub valid: usize,
    pub warning: usize,
    pub invalid: usize,
    pub shadowed: usize,
    pub project: usize,
    pub global: usize,
    pub claude_code: usize,
    pub droid: usize,
    pub pencode: usize,
    pub codex: usize,
    pub zed: usize,
    pub custom: usize,
    pub known_source: usize,
    pub exportable: usize,
}

impl App {
    pub fn init() -> (Self, Task<Message>) {
        Self::init_with_smoke_test(false)
    }

    pub fn init_with_smoke_test(smoke_test: bool) -> (Self, Task<Message>) {
        let project_path = std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        let app = Self {
            snapshot: None,
            skills: Vec::new(),
            derived: DerivedInventoryState::default(),
            active_view: ActiveView::Library,
            inventory: InventoryState {
                search_query: String::new(),
                selected_skill_id: None,
                scope_filter: ScopeFilter::All,
                health_filter: HealthFilter::All,
                source_filter: SourceFilter::All,
                sort_key: SortKey::Priority,
                detail_tab: DetailTab::Overview,
                pending_remove_skill: None,
            },
            install: InstallState {
                install_source: InstallSource::Url,
                source_url: String::new(),
                local_source_path: String::new(),
                catalog_url: String::new(),
                download_path_override: String::new(),
                install_scope: UiScope::Global,
                custom_install_path: String::new(),
                enable_after_install: true,
                conflict_policy: UiConflictPolicy::Block,
                preview: None,
                downloaded_entries: Vec::new(),
                selected_download_root: None,
                catalog_entries: Vec::new(),
                pending_remove_download: None,
            },
            create: CreateState {
                name: String::new(),
                description: String::new(),
                target: UiScope::Global,
                custom_path: String::new(),
                tags: String::new(),
                allowed_tools: String::new(),
                compatibility: String::new(),
                license: String::new(),
                when_to_use: String::new(),
                disable_model_invocation: false,
                preview: None,
            },
            catalog: CatalogExportState {
                catalog_format: UiCatalogFormat::Json,
                catalog_save_path: "agent-skills-catalog.json".to_string(),
                catalog_output: String::new(),
            },
            settings: AppSettingsState {
                project_path,
                default_download_path: String::new(),
                target_profiles: Vec::new(),
                doctor_report: None,
            },
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

    pub fn filtered_skills(&self) -> Vec<&InstalledSkill> {
        self.derived
            .filtered_skill_indices
            .iter()
            .filter_map(|index| self.skills.get(*index))
            .collect()
    }

    pub fn selected_skill(&self) -> Option<&InstalledSkill> {
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

    pub fn visible_scopes_for_skill(&self, skill: &InstalledSkill) -> Vec<SkillScope> {
        self.derived
            .visible_scopes_by_id
            .get(&skill.id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn scope_summary(&self, scope: SkillScope) -> ScopeSummary {
        self.derived
            .scope_summaries
            .get(&scope)
            .copied()
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
        self.derived.scope_summaries = scope_summaries_from_skills(&self.skills);
        self.derived.visible_scopes_by_id = visible_scopes_by_id(&self.skills);
        self.derived.filtered_skill_indices = self.filtered_indices();
        self.sort_skill_indices();
    }

    fn filtered_skill_id_exists(&self, id: &str) -> bool {
        self.derived.filtered_skill_indices.iter().any(|index| {
            self.skills
                .get(*index)
                .is_some_and(|skill| skill.id.as_str() == id)
        })
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.inventory.search_query.trim().to_lowercase();
        if query.is_empty() {
            return self
                .skills
                .iter()
                .enumerate()
                .filter_map(|(index, skill)| self.skill_matches_filters(skill).then_some(index))
                .collect();
        }

        if let Some(snapshot) = &self.snapshot {
            return snapshot
                .index
                .search
                .iter()
                .filter(|entry| entry.haystack.contains(&query))
                .filter_map(|entry| {
                    self.skills
                        .get(entry.skill_index)
                        .filter(|skill| self.skill_matches_filters(skill))
                        .map(|_| entry.skill_index)
                })
                .collect();
        }

        self.skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| self.skill_matches(skill).then_some(index))
            .collect()
    }

    fn skill_matches(&self, skill: &InstalledSkill) -> bool {
        self.skill_matches_filters(skill) && self.skill_matches_query(skill)
    }

    fn skill_matches_filters(&self, skill: &InstalledSkill) -> bool {
        self.inventory.scope_filter.matches(skill.scope)
            && self.inventory.health_filter.matches(skill.health)
            && self.inventory.source_filter.matches(skill)
    }

    fn skill_matches_query(&self, skill: &InstalledSkill) -> bool {
        let query = self.inventory.search_query.trim().to_lowercase();
        query.is_empty() || skill_search_haystack(skill).contains(&query)
    }

    fn sort_skill_indices(&mut self) {
        let sort_key = self.inventory.sort_key;
        let skills = &self.skills;
        self.derived
            .filtered_skill_indices
            .sort_by(|left, right| compare_skills(&skills[*left], &skills[*right], sort_key));
    }
}

fn counts_from_skills(skills: &[InstalledSkill]) -> SkillCounts {
    skills
        .iter()
        .fold(SkillCounts::default(), |mut counts, skill| {
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
                SkillScope::Global => counts.global += 1,
                SkillScope::ClaudeCode => counts.claude_code += 1,
                SkillScope::Droid => counts.droid += 1,
                SkillScope::Pencode => counts.pencode += 1,
                SkillScope::Codex => counts.codex += 1,
                SkillScope::Zed => counts.zed += 1,
                SkillScope::Custom => counts.custom += 1,
            }
            if skill.source_url.is_some() {
                counts.known_source += 1;
            }
            if skill.is_exportable() {
                counts.exportable += 1;
            }
            counts
        })
}

fn scope_summaries_from_skills(skills: &[InstalledSkill]) -> BTreeMap<SkillScope, ScopeSummary> {
    let mut summaries = BTreeMap::new();

    for skill in skills {
        let summary = summaries
            .entry(skill.scope)
            .or_insert_with(ScopeSummary::default);
        summary.total += 1;
        if skill.is_exportable() {
            summary.usable += 1;
        }
        if !skill.is_enabled() {
            summary.disabled += 1;
        }
        if skill.health == SkillHealth::Invalid {
            summary.invalid += 1;
        }
        if matches!(
            skill.health,
            SkillHealth::Warning | SkillHealth::Invalid | SkillHealth::Shadowed
        ) {
            summary.attention += 1;
        }
    }

    summaries
}

fn visible_scopes_by_id(skills: &[InstalledSkill]) -> BTreeMap<String, Vec<SkillScope>> {
    let mut by_identity = BTreeMap::<String, Vec<SkillScope>>::new();

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

fn skill_search_haystack(skill: &InstalledSkill) -> String {
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

fn compare_skills(
    left: &InstalledSkill,
    right: &InstalledSkill,
    sort_key: SortKey,
) -> std::cmp::Ordering {
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
        SortKey::Scope => left
            .scope
            .sort_rank()
            .cmp(&right.scope.sort_rank())
            .then_with(|| sort_name(left).cmp(&sort_name(right))),
        SortKey::Resources => right
            .resource_bytes
            .cmp(&left.resource_bytes)
            .then_with(|| right.resource_count.cmp(&left.resource_count)),
        SortKey::Installed => right
            .installed_at
            .cmp(&left.installed_at)
            .then_with(|| sort_name(left).cmp(&sort_name(right))),
    }
}

fn sort_name(skill: &InstalledSkill) -> String {
    skill.display_name.to_lowercase()
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
            app.status = "Loading workspace snapshot...".to_string();
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::WorkspaceLoaded(result) => {
            app.busy = false;
            match result {
                Ok(snapshot) => {
                    app.status = format!(
                        "Loaded {} skill(s), {} download(s), and {} target(s).",
                        snapshot.skills.len(),
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
        Message::DetailTabSelected(tab) => {
            app.inventory.detail_tab = tab;
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
                optional_path(&app.install.download_path_override),
            )
        }
        Message::Downloaded(result) => {
            app.busy = false;
            app.status = result.unwrap_or_else(|error| error);
            tasks::load_workspace_task(app.settings.project_path.clone())
        }
        Message::PreviewInstall => {
            let source = current_source_value(app);
            if source.trim().is_empty() {
                app.status = "Enter a source before previewing.".to_string();
                return Task::none();
            }
            let target = match current_install_target(app) {
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
                optional_path(&app.install.download_path_override),
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
            let target = match current_install_target(app) {
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
                optional_path(&app.install.download_path_override),
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
        Message::PreviewDownloaded(root_dir) => {
            let target = match current_install_target(app) {
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
                optional_path(&app.install.download_path_override),
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
            let request = match current_scaffold_request(app) {
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
            let request = match current_scaffold_request(app) {
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

pub fn view(app: &App) -> Element<'_, Message> {
    views::view(app)
}

pub fn subscription(app: &App) -> Subscription<Message> {
    if app.smoke_test {
        iced::time::every(Duration::from_millis(250)).map(|_| Message::SmokeExit)
    } else {
        Subscription::none()
    }
}

fn current_source_value(app: &App) -> String {
    match app.install.install_source {
        InstallSource::Url => app.install.source_url.trim().to_string(),
        InstallSource::Local => app.install.local_source_path.trim().to_string(),
        InstallSource::Downloaded => app
            .install
            .selected_download_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        InstallSource::Catalog => app.install.catalog_url.trim().to_string(),
    }
}

fn optional_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn current_install_target(app: &App) -> Result<InstallTarget, String> {
    match app.install.install_scope {
        UiScope::Global => Ok(InstallTarget::Global),
        UiScope::Project => Ok(InstallTarget::Project),
        UiScope::ClaudeCode => Ok(InstallTarget::ClaudeCode),
        UiScope::Droid => Ok(InstallTarget::Droid),
        UiScope::Pencode => Ok(InstallTarget::Pencode),
        UiScope::Codex => Ok(InstallTarget::Codex),
        UiScope::Zed => Ok(InstallTarget::Zed),
        UiScope::Custom => {
            let path = app.install.custom_install_path.trim();
            if path.is_empty() {
                Err("Enter a custom install path first.".to_string())
            } else {
                Ok(InstallTarget::Custom(PathBuf::from(path)))
            }
        }
    }
}

fn current_scaffold_request(app: &App) -> Result<SkillScaffoldRequest, String> {
    if app.create.name.trim().is_empty() {
        return Err("Enter a skill name first.".to_string());
    }
    if app.create.description.trim().is_empty() {
        return Err("Enter a skill description first.".to_string());
    }

    Ok(SkillScaffoldRequest {
        name: app.create.name.trim().to_string(),
        description: app.create.description.trim().to_string(),
        target: create_target(app)?,
        tags: split_csv(&app.create.tags),
        allowed_tools: split_csv(&app.create.allowed_tools),
        compatibility: optional_string(&app.create.compatibility),
        license: optional_string(&app.create.license),
        when_to_use: optional_string(&app.create.when_to_use),
        disable_model_invocation: app.create.disable_model_invocation.then_some(true),
    })
}

fn create_target(app: &App) -> Result<InstallTarget, String> {
    match app.create.target {
        UiScope::Global => Ok(InstallTarget::Global),
        UiScope::Project => Ok(InstallTarget::Project),
        UiScope::ClaudeCode => Ok(InstallTarget::ClaudeCode),
        UiScope::Droid => Ok(InstallTarget::Droid),
        UiScope::Pencode => Ok(InstallTarget::Pencode),
        UiScope::Codex => Ok(InstallTarget::Codex),
        UiScope::Zed => Ok(InstallTarget::Zed),
        UiScope::Custom => {
            let path = app.create.custom_path.trim();
            if path.is_empty() {
                Err("Enter a custom create path first.".to_string())
            } else {
                Ok(InstallTarget::Custom(PathBuf::from(path)))
            }
        }
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
        .collect()
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
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
    use std::fs;

    use tempfile::tempdir;

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

        let _ = update(&mut app, Message::DetailTabSelected(DetailTab::Diagnostics));
        assert_eq!(app.inventory.detail_tab, DetailTab::Diagnostics);
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
        assert_eq!(app.scope_summary(SkillScope::Project).usable, 1);
        assert_eq!(app.scope_summary(SkillScope::Codex).disabled, 1);
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
            download_root: None,
            scope: SkillScope::Global,
            destination_root: PathBuf::from("destination"),
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
            frontmatter: skills_manager_core::SkillFrontmatter {
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
