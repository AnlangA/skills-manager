use url::Url;

use crate::{Result, SkillsManagerError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubTreeSource {
    pub owner: String,
    pub repo: String,
    pub reference: String,
    pub subdir: Option<String>,
    pub original_url: String,
}

impl GitHubTreeSource {
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

    pub fn archive_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/archive/refs/heads/{}.zip",
            self.owner, self.repo, self.reference
        )
    }

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

    pub fn path_filter(&self) -> Option<&str> {
        self.subdir.as_deref()
    }
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
}
