pub mod codex_config;
pub mod download;
pub mod error;
pub mod github;
pub mod install;
pub mod manager_config;
pub mod marketplace;
pub mod model;
pub mod paths;
pub mod scaffold;
pub mod skill;
pub mod target;

pub use download::{
    DownloadedCatalog, DownloadedSkillEntry, cache_github_skills_archive, download_github_catalog,
    download_github_marketplace, download_github_skills, download_github_skills_to_cache,
    downloaded_skill_entry, list_downloaded_skills, remove_downloaded_skills,
};
pub use error::{Result, SkillsManagerError};
pub use github::{GitHubTreeSource, catalog_git_install_url};
pub use install::{
    ConflictPolicy, InstallCandidate, InstallPreview, InstallRequest, InstallResult, InstallTarget,
    Installer,
};
pub use manager_config::{DownloadedMetadata, ManagerConfig};
pub use marketplace::{
    CatalogFormat, Marketplace, MarketplaceEntry, MarketplaceSource, SkillCatalog,
    SkillCatalogEntry, SkillCatalogSource, export_installed_catalog,
};
pub use model::{
    DiagnosticSeverity, InstalledSkill, SkillDiagnostic, SkillEnablement, SkillFrontmatter,
    SkillHealth, SkillScope,
};
pub use paths::{ManagerPaths, ProjectRoot};
pub use scaffold::{
    SkillScaffoldPreview, SkillScaffoldRequest, create_skill_scaffold, preview_skill_scaffold,
};
pub use skill::{
    discover_skill_candidates, format_bytes, installed_skill_identity, read_installed_skill,
    read_skill_candidate, scan_installed_skills, validate_skill, visible_skill_scopes,
};
pub use target::{
    DoctorRepairAction, DoctorReport, DoctorSummary, EnablementStrategy, LayoutPolicy,
    RepairOutcome, RepairReport, TargetDoctorReport, TargetHealthCounts, TargetProfile,
    doctor_report, repair_targets, target_profiles, target_specific_diagnostics,
};
