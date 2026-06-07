//! Error types and result alias for `skills-manager-core`.
//!
//! Defines [`SkillsManagerError`], the unified error enum used by every
//! fallible operation in the crate, and the convenience [`Result`] type alias.

use std::path::PathBuf;

use thiserror::Error;

/// Standard result type used throughout `skills-manager-core`.
pub type Result<T> = std::result::Result<T, SkillsManagerError>;

#[derive(Debug, Error)]
/// Domain-level and infrastructure-level errors for skill and resource operations.
pub enum SkillsManagerError {
    /// The current user home directory could not be resolved.
    #[error("could not find a usable home directory")]
    HomeDirectoryMissing,

    /// URL parsing or normalization failed before reaching a concrete action.
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    /// GitHub URL type is unsupported by this client.
    #[error("unsupported URL. v1 supports GitHub repository and tree URLs")]
    UnsupportedUrl,

    /// Parsed URL is missing `owner/repo` components.
    #[error("GitHub URL does not include an owner and repository")]
    MissingGitHubRepository,

    /// Archive did not include any directory containing `SKILL.md`.
    #[error("the archive did not contain any folders with SKILL.md")]
    NoSkillsFound,

    /// A move/copy destination already exists and cannot be overwritten.
    #[error("skill destination already exists: {0}")]
    DestinationExists(PathBuf),

    /// Attempted to read an install target that has no `SKILL.md`.
    #[error("invalid skill folder `{0}`: missing SKILL.md")]
    MissingSkillFile(PathBuf),

    /// Plugin installation/scan path does not contain the expected manifest.
    #[error("invalid plugin folder `{0}`: missing plugin manifest")]
    MissingPluginManifest(PathBuf),

    /// A parsed skill/resource failed structural validation.
    #[error("invalid resource: {0}")]
    InvalidResource(String),

    /// Archive path extraction would escape the extraction root.
    #[error("archive entry tried to write outside the extraction directory: {0}")]
    UnsafeArchivePath(String),

    /// Archive is larger than the hard limit enforced by the downloader.
    #[error("archive is too large: {bytes} bytes (limit {max_bytes} bytes)")]
    ArchiveTooLarge { bytes: u64, max_bytes: u64 },

    /// One archive entry is larger than allowed maximum size.
    #[error("archive entry `{path}` is too large: {bytes} bytes (limit {max_bytes} bytes)")]
    ArchiveEntryTooLarge {
        path: String,
        bytes: u64,
        max_bytes: u64,
    },

    /// Archive contained too many entries to be processed safely.
    #[error("archive contains too many entries: {entries} (limit {max_entries})")]
    ArchiveEntryCountTooLarge { entries: usize, max_entries: usize },

    /// Archive uncompressed data exceeds the configured memory threshold.
    #[error("archive uncompressed contents are too large: {bytes} bytes (limit {max_bytes} bytes)")]
    ArchiveUncompressedTooLarge { bytes: u64, max_bytes: u64 },

    /// A skill file path is outside known global/project install roots.
    #[error("skill path is not inside the configured global or project skill roots: {0}")]
    UnknownSkillScope(PathBuf),

    /// A requested downloaded bundle is not managed by this application.
    #[error("downloaded skills path is not managed by this app: {0}")]
    UnknownDownload(PathBuf),

    /// Configuration lock could not be acquired before timeout.
    #[error("timed out waiting for manager config lock: {0}")]
    ConfigLockTimeout(PathBuf),

    /// Rollback failed after an operation error and one or more cleanup steps could not be completed.
    #[error("{source}; rollback failed: {failures}")]
    RollbackFailed {
        source: Box<SkillsManagerError>,
        failures: String,
    },

    /// TOML parse failed for a manager/config-related file.
    #[error("failed to parse {path}: {source}")]
    ParseToml {
        path: PathBuf,
        source: toml_edit::TomlError,
    },

    /// SKILL frontmatter parse failed.
    #[error("failed to parse YAML frontmatter in {path}: {source}")]
    ParseYaml {
        path: PathBuf,
        source: serde_yaml::Error,
    },

    /// Catalog JSON parse failed.
    #[error("failed to parse catalog JSON in {path}: {source}")]
    ParseCatalog {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Marketplace JSON parse failed.
    #[error("failed to parse marketplace JSON in {path}: {source}")]
    ParseMarketplace {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// I/O layer returned an error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Directory traversal failed while scanning filesystem trees.
    #[error("directory traversal error: {0}")]
    WalkDir(#[from] walkdir::Error),

    /// Network layer returned an HTTP error.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// ZIP parsing or extraction failed.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// URL parser rejected input.
    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),

    /// TOML decoder failed.
    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    /// TOML editor mutation failed.
    #[error("toml edit error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),

    /// JSON parse/serialize failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML parse/serialize failed.
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
