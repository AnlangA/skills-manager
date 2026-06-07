use url::Url;

use crate::{Result, SkillsManagerError};

/// Parsed GitHub source location supporting repository URLs and tree paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubTreeSource {
    /// Repository owner.
    pub owner: String,
    /// Repository name (without `.git` suffix).
    pub repo: String,
    /// Branch/tag/commit to inspect.
    pub reference: String,
    /// Optional subdirectory path segment under reference.
    pub subdir: Option<String>,
    /// Normalized original URL string.
    pub original_url: String,
}

impl GitHubTreeSource {
    /// Parses input and normalizes to an explicit owner/repo/reference form.
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = normalize_github_input(input);
        let url = Url::parse(&normalized)?;

        if url.host_str() != Some("github.com") {
            return Err(SkillsManagerError::UnsupportedUrl);
        }

        let segments = url
            .path_segments()
            .ok_or_else(|| SkillsManagerError::InvalidUrl(input.to_string()))?
            .collect::<Vec<_>>();

        if segments.len() < 2 {
            return Err(SkillsManagerError::MissingGitHubRepository);
        }

        let owner = segments[0].to_string();
        let repo = segments[1].trim_end_matches(".git").to_string();

        let (reference, subdir) =
            if segments.get(2) == Some(&"tree") || segments.get(2) == Some(&"blob") {
                let reference = segments
                    .get(3)
                    .ok_or_else(|| SkillsManagerError::InvalidUrl(input.to_string()))?
                    .to_string();
                let subdir = if segments.len() > 4 {
                    Some(segments[4..].join("/"))
                } else {
                    None
                };
                (reference, subdir)
            } else {
                ("main".to_string(), None)
            };

        Ok(Self {
            owner,
            repo,
            reference,
            subdir,
            original_url: normalized,
        })
    }

    /// Returns the default archive URL for the selected reference.
    pub fn archive_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/archive/refs/heads/{}.zip",
            self.owner, self.repo, self.reference
        )
    }

    /// Returns several archive candidates to probe by fallback order.
    pub fn archive_url_candidates(&self) -> Vec<String> {
        vec![
            format!(
                "https://github.com/{}/{}/archive/refs/heads/{}.zip",
                self.owner, self.repo, self.reference
            ),
            format!(
                "https://github.com/{}/{}/archive/refs/tags/{}.zip",
                self.owner, self.repo, self.reference
            ),
            format!(
                "https://github.com/{}/{}/archive/{}.zip",
                self.owner, self.repo, self.reference
            ),
        ]
    }

    /// Returns an optional subdir path used for tree scoping.
    pub fn path_filter(&self) -> Option<&str> {
        self.subdir.as_deref()
    }

    /// Builds a GitHub tree URL for a given subpath.
    pub fn tree_url_with_path(&self, path: &str) -> String {
        let mut parts = Vec::new();
        if let Some(subdir) = self
            .subdir
            .as_deref()
            .map(clean_tree_path)
            .filter(|subdir| !subdir.is_empty())
        {
            parts.push(subdir);
        }

        let path = clean_tree_path(path);
        if !path.is_empty() {
            parts.push(path);
        }

        if parts.is_empty() {
            format!(
                "https://github.com/{}/{}/tree/{}",
                self.owner, self.repo, self.reference
            )
        } else {
            format!(
                "https://github.com/{}/{}/tree/{}/{}",
                self.owner,
                self.repo,
                self.reference,
                parts.join("/")
            )
        }
    }
}

/// Builds a normalized catalog install URL from source URL and optional path.
pub fn catalog_git_install_url(url: &str, path: Option<&str>) -> Result<String> {
    let source = GitHubTreeSource::parse(url)?;
    let Some(path) = path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Ok(source.original_url);
    };

    Ok(source.tree_url_with_path(path))
}

fn normalize_github_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.starts_with("github.com/") {
        format!("https://{trimmed}")
    } else {
        format!("https://github.com/{trimmed}")
    }
}

fn clean_tree_path(path: &str) -> String {
    path.trim()
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tree_url() {
        let parsed =
            GitHubTreeSource::parse("https://github.com/openai/skills/tree/main/skills/.curated")
                .unwrap();

        assert_eq!(parsed.owner, "openai");
        assert_eq!(parsed.repo, "skills");
        assert_eq!(parsed.reference, "main");
        assert_eq!(parsed.subdir.as_deref(), Some("skills/.curated"));
    }

    #[test]
    fn parses_shorthand_repo() {
        let parsed = GitHubTreeSource::parse("openai/skills").unwrap();

        assert_eq!(parsed.owner, "openai");
        assert_eq!(parsed.repo, "skills");
        assert_eq!(parsed.reference, "main");
        assert_eq!(parsed.subdir, None);
    }

    #[test]
    fn builds_catalog_install_url_for_plain_repo_path() {
        let url =
            catalog_git_install_url("https://github.com/acme/skills", Some("demo/skill")).unwrap();

        assert_eq!(url, "https://github.com/acme/skills/tree/main/demo/skill");
    }

    #[test]
    fn builds_catalog_install_url_preserving_branch() {
        let url = catalog_git_install_url("https://github.com/acme/skills/tree/dev", Some("demo"))
            .unwrap();

        assert_eq!(url, "https://github.com/acme/skills/tree/dev/demo");
    }

    #[test]
    fn builds_catalog_install_url_preserving_base_subdir() {
        let url = catalog_git_install_url(
            "https://github.com/acme/skills/tree/dev/catalog",
            Some("/demo//skill/"),
        )
        .unwrap();

        assert_eq!(
            url,
            "https://github.com/acme/skills/tree/dev/catalog/demo/skill"
        );
    }

    #[test]
    fn catalog_install_url_without_path_preserves_normalized_source() {
        let url = catalog_git_install_url("acme/skills", None).unwrap();

        assert_eq!(url, "https://github.com/acme/skills");
    }

    #[test]
    fn catalog_install_url_rejects_unsupported_hosts() {
        let error =
            catalog_git_install_url("https://example.com/acme/skills", Some("demo")).unwrap_err();

        assert!(matches!(error, SkillsManagerError::UnsupportedUrl));
    }
}
