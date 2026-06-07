use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SkillsManagerError>;

#[derive(Debug, Error)]
pub enum SkillsManagerError {
    #[error("could not find a usable home directory")]
    HomeDirectoryMissing,

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("unsupported URL. v1 supports GitHub repository and tree URLs")]
    UnsupportedUrl,

    #[error("GitHub URL does not include an owner and repository")]
    MissingGitHubRepository,

    #[error("the archive did not contain any folders with SKILL.md")]
    NoSkillsFound,

    #[error("skill destination already exists: {0}")]
    DestinationExists(PathBuf),

    #[error("invalid skill folder `{0}`: missing SKILL.md")]
    MissingSkillFile(PathBuf),

    #[error("archive entry tried to write outside the extraction directory: {0}")]
    UnsafeArchivePath(String),

    #[error("archive is too large: {bytes} bytes (limit {max_bytes} bytes)")]
    ArchiveTooLarge { bytes: u64, max_bytes: u64 },

    #[error("archive entry `{path}` is too large: {bytes} bytes (limit {max_bytes} bytes)")]
    ArchiveEntryTooLarge {
        path: String,
        bytes: u64,
        max_bytes: u64,
    },

    #[error("skill path is not inside the configured global or project skill roots: {0}")]
    UnknownSkillScope(PathBuf),

    #[error("downloaded skills path is not managed by this app: {0}")]
    UnknownDownload(PathBuf),

    #[error("failed to parse {path}: {source}")]
    ParseToml {
        path: PathBuf,
        source: toml_edit::TomlError,
    },

    #[error("failed to parse YAML frontmatter in {path}: {source}")]
    ParseYaml {
        path: PathBuf,
        source: serde_yaml::Error,
    },

    #[error("failed to parse catalog JSON in {path}: {source}")]
    ParseCatalog {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to parse marketplace JSON in {path}: {source}")]
    ParseMarketplace {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml edit error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
