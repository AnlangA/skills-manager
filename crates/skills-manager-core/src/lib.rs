pub mod download;
pub mod error;
pub mod github;
pub mod install;
pub mod manager_config;
pub mod marketplace;
pub mod model;
pub mod paths;
pub mod skill;

pub use download::{
    DownloadedCatalog, download_github_catalog, download_github_marketplace, download_github_skills,
};
pub use error::{Result, SkillsManagerError};
pub use github::{GitHubTreeSource, catalog_git_install_url};
pub use install::{
    ConflictPolicy, InstallCandidate, InstallPreview, InstallRequest, InstallResult, Installer,
};
pub use manager_config::ManagerConfig;
pub use marketplace::{
    CatalogFormat, Marketplace, MarketplaceEntry, MarketplaceSource, SkillCatalog,
    SkillCatalogEntry, SkillCatalogSource, export_installed_catalog,
};
pub use model::{
    DiagnosticSeverity, InstalledSkill, SkillDiagnostic, SkillEnablement, SkillFrontmatter,
    SkillHealth, SkillScope,
};
pub use paths::{ManagerPaths, ProjectRoot};
pub use skill::{
    discover_skill_candidates, format_bytes, read_installed_skill, read_skill_candidate,
    scan_installed_skills, validate_skill,
};
