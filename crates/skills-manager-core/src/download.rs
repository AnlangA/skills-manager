use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tempfile::TempDir;
use tracing::{debug, info, warn};
use zip::ZipArchive;

use crate::{
    GitHubTreeSource, ManagerConfig, ManagerPaths, Result, SkillCatalog, SkillsManagerError,
    skill::{
        SkillCandidate, discover_skill_candidates, format_bytes, path_key, sanitize_folder_name,
    },
};

const MAX_ARCHIVE_BYTES: usize = 128 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug)]
pub struct DownloadedSkills {
    pub temp_dir: TempDir,
    pub source: GitHubTreeSource,
    pub candidates: Vec<SkillCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadedSkillEntry {
    pub id: String,
    pub source_url: String,
    pub root_dir: PathBuf,
    pub search_root: PathBuf,
    pub downloaded_at: Option<DateTime<Utc>>,
    pub candidate_count: usize,
    pub resource_count: usize,
    pub resource_bytes: u64,
}

impl DownloadedSkillEntry {
    pub fn resource_summary(&self) -> String {
        format!(
            "{} candidate(s), {} file(s), {}",
            self.candidate_count,
            self.resource_count,
            format_bytes(self.resource_bytes)
        )
    }
}

#[derive(Debug)]
pub struct DownloadedMarketplace {
    pub temp_dir: Option<TempDir>,
    pub source: GitHubTreeSource,
    pub marketplace: SkillCatalog,
}

#[derive(Debug)]
pub struct DownloadedCatalog {
    pub temp_dir: Option<TempDir>,
    pub source: GitHubTreeSource,
    pub catalog: SkillCatalog,
}

pub async fn download_github_skills(url: &str) -> Result<DownloadedSkills> {
    info!(%url, "downloading GitHub skills to temporary directory");
    let source = GitHubTreeSource::parse(url)?;
    let bytes = download_first_archive(&source).await?;
    let temp_dir = tempfile::tempdir()?;
    extract_zip_safe(&bytes, temp_dir.path())?;

    let search_root = filtered_search_root(temp_dir.path(), source.path_filter())?;
    let candidates = discover_skill_candidates(&search_root)?;

    if candidates.is_empty() {
        warn!(%url, "downloaded archive did not contain skill candidates");
        return Err(SkillsManagerError::NoSkillsFound);
    }

    info!(%url, candidates = candidates.len(), "downloaded GitHub skills");
    Ok(DownloadedSkills {
        temp_dir,
        source,
        candidates,
    })
}

pub async fn download_github_skills_to_cache(
    paths: &ManagerPaths,
    url: &str,
    download_dir: Option<&Path>,
) -> Result<DownloadedSkillEntry> {
    info!(%url, "downloading GitHub skills to cache");
    let source = GitHubTreeSource::parse(url)?;
    let bytes = download_first_archive(&source).await?;
    cache_github_skills_archive(paths, url, &bytes, download_dir)
}

pub fn cache_github_skills_archive(
    paths: &ManagerPaths,
    url: &str,
    bytes: &[u8],
    download_dir: Option<&Path>,
) -> Result<DownloadedSkillEntry> {
    let source = GitHubTreeSource::parse(url)?;
    let mut config = ManagerConfig::load(paths)?;
    let download_root = download_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.effective_download_dir(paths));
    fs::create_dir_all(&download_root)?;

    let preferred_name = download_folder_name(&source);
    let bundle_root = unique_download_dir(&download_root, &preferred_name);
    debug!(
        %url,
        download_root = %download_root.display(),
        bundle_root = %bundle_root.display(),
        "caching downloaded skills archive"
    );
    fs::create_dir_all(&bundle_root)?;
    extract_zip_safe(bytes, &bundle_root)?;

    let search_root = filtered_search_root(&bundle_root, source.path_filter())?;
    let candidates = discover_skill_candidates(&search_root)?;
    if candidates.is_empty() {
        warn!(
            %url,
            bundle_root = %bundle_root.display(),
            "cached archive contained no skills; removing bundle"
        );
        let _ = fs::remove_dir_all(&bundle_root);
        return Err(SkillsManagerError::NoSkillsFound);
    }

    let entry = downloaded_entry(
        url.to_string(),
        bundle_root.clone(),
        Some(Utc::now()),
        candidates,
        search_root.clone(),
    )?;
    config.record_download_summary(
        &bundle_root,
        url.to_string(),
        &search_root,
        entry.candidate_count,
        entry.resource_count,
        entry.resource_bytes,
    );
    config.save(paths)?;

    Ok(entry)
}

pub fn list_downloaded_skills(paths: &ManagerPaths) -> Result<Vec<DownloadedSkillEntry>> {
    debug!("listing downloaded skills");
    let config = ManagerConfig::load(paths)?;
    let mut entries = Vec::new();

    for (root_key, metadata) in &config.downloads {
        let root_dir = PathBuf::from(root_key);
        if !root_dir.exists() {
            continue;
        }

        if let (
            Some(search_root),
            Some(candidate_count),
            Some(resource_count),
            Some(resource_bytes),
        ) = (
            metadata.search_root.clone().filter(|path| path.exists()),
            metadata.candidate_count,
            metadata.resource_count,
            metadata.resource_bytes,
        ) {
            entries.push(DownloadedSkillEntry {
                id: path_key(&root_dir),
                source_url: metadata.source_url.clone(),
                root_dir,
                search_root,
                downloaded_at: metadata.downloaded_at,
                candidate_count,
                resource_count,
                resource_bytes,
            });
        } else {
            let source = GitHubTreeSource::parse(&metadata.source_url)?;
            let search_root = filtered_search_root(&root_dir, source.path_filter())?;
            let candidates = discover_skill_candidates(&search_root)?;
            entries.push(downloaded_entry(
                metadata.source_url.clone(),
                root_dir,
                metadata.downloaded_at,
                candidates,
                search_root,
            )?);
        }
    }

    entries.sort_by(|left, right| {
        right
            .downloaded_at
            .cmp(&left.downloaded_at)
            .then_with(|| left.root_dir.cmp(&right.root_dir))
    });
    info!(count = entries.len(), "listed downloaded skills");
    Ok(entries)
}

pub fn downloaded_skill_entry(
    paths: &ManagerPaths,
    root_dir: &Path,
) -> Result<DownloadedSkillEntry> {
    let target = path_key(root_dir);
    list_downloaded_skills(paths)?
        .into_iter()
        .find(|entry| path_key(&entry.root_dir) == target || entry.id == target)
        .ok_or_else(|| SkillsManagerError::UnknownDownload(root_dir.to_path_buf()))
}

pub fn remove_downloaded_skills(paths: &ManagerPaths, root_dir: &Path) -> Result<PathBuf> {
    info!(root_dir = %root_dir.display(), "removing downloaded skills cache");
    let requested = root_dir.canonicalize()?;
    let mut config = ManagerConfig::load(paths)?;
    let mut matched = None;

    for root_key in config.downloads.keys() {
        let recorded = PathBuf::from(root_key);
        if recorded.exists() && recorded.canonicalize()? == requested {
            matched = Some(recorded);
            break;
        }
    }

    let matched =
        matched.ok_or_else(|| SkillsManagerError::UnknownDownload(root_dir.to_path_buf()))?;
    fs::remove_dir_all(&matched)?;
    config.forget_download(&matched);
    config.save(paths)?;
    info!(root_dir = %matched.display(), "removed downloaded skills cache");
    Ok(matched)
}

pub async fn download_github_marketplace(url: &str) -> Result<DownloadedMarketplace> {
    let downloaded = download_github_catalog(url).await?;
    Ok(DownloadedMarketplace {
        temp_dir: downloaded.temp_dir,
        source: downloaded.source,
        marketplace: downloaded.catalog,
    })
}

pub async fn download_github_catalog(url: &str) -> Result<DownloadedCatalog> {
    info!(%url, "downloading GitHub catalog");
    let source = GitHubTreeSource::parse(url)?;

    if source
        .path_filter()
        .is_some_and(|path| path.ends_with(".json"))
    {
        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            source.owner,
            source.repo,
            source.reference,
            source.path_filter().expect("checked path filter")
        );
        let catalog = reqwest::get(raw_url).await?.json::<SkillCatalog>().await?;
        info!(%url, "downloaded raw GitHub catalog");
        return Ok(DownloadedCatalog {
            temp_dir: None,
            source,
            catalog,
        });
    }

    let bytes = download_first_archive(&source).await?;
    let temp_dir = tempfile::tempdir()?;
    extract_zip_safe(&bytes, temp_dir.path())?;
    let search_root = filtered_search_root(temp_dir.path(), source.path_filter())?;

    for candidate in [
        search_root.join(".agents/plugins/marketplace.json"),
        search_root.join("marketplace.json"),
        search_root.join("skills.json"),
        search_root.join("catalog.json"),
    ] {
        if candidate.exists() {
            let catalog = SkillCatalog::load_file(&candidate)?;
            info!(path = %candidate.display(), "loaded catalog from archive");
            return Ok(DownloadedCatalog {
                temp_dir: Some(temp_dir),
                source,
                catalog,
            });
        }
    }

    Err(SkillsManagerError::NoSkillsFound)
}

async fn download_first_archive(source: &GitHubTreeSource) -> Result<Vec<u8>> {
    let client = github_client()?;
    let mut last_error = None;

    for url in source.archive_url_candidates() {
        debug!(%url, "trying GitHub archive URL");
        let response = match client.get(&url).send().await {
            Ok(response) => response,
            Err(error) => {
                debug!(%url, %error, "archive request failed");
                last_error = Some(error);
                continue;
            }
        };

        if response.status().is_success() {
            debug!(%url, status = %response.status(), "downloaded GitHub archive");
            let bytes = response.bytes().await?;
            if bytes.len() > MAX_ARCHIVE_BYTES {
                return Err(SkillsManagerError::ArchiveTooLarge {
                    bytes: bytes.len() as u64,
                    max_bytes: MAX_ARCHIVE_BYTES as u64,
                });
            }
            return Ok(bytes.to_vec());
        }

        debug!(%url, status = %response.status(), "archive URL returned non-success status");
    }

    match last_error {
        Some(error) => Err(error.into()),
        None => Err(SkillsManagerError::UnsupportedUrl),
    }
}

fn github_client() -> Result<reqwest::Client> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }

    let client = reqwest::Client::builder()
        .user_agent("skills-manager/0.1")
        .timeout(Duration::from_secs(30))
        .build()?;
    let _ = CLIENT.set(client.clone());
    Ok(client)
}

pub fn extract_zip_safe(bytes: &[u8], destination: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| SkillsManagerError::UnsafeArchivePath(file.name().to_string()))?;
        let output = destination.join(&enclosed);

        if file.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }

        if file.size() > MAX_ARCHIVE_ENTRY_BYTES {
            return Err(SkillsManagerError::ArchiveEntryTooLarge {
                path: enclosed.display().to_string(),
                bytes: file.size(),
                max_bytes: MAX_ARCHIVE_ENTRY_BYTES,
            });
        }

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output_file = fs::File::create(output)?;
        std::io::copy(&mut file, &mut output_file)?;
    }

    Ok(())
}

fn filtered_search_root(extract_dir: &Path, path_filter: Option<&str>) -> Result<PathBuf> {
    let Some(path_filter) = path_filter else {
        return Ok(extract_dir.to_path_buf());
    };

    for entry in fs::read_dir(extract_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let candidate = entry.path().join(path_filter);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(extract_dir.to_path_buf())
}

fn downloaded_entry(
    source_url: String,
    root_dir: PathBuf,
    downloaded_at: Option<DateTime<Utc>>,
    candidates: Vec<SkillCandidate>,
    search_root: PathBuf,
) -> Result<DownloadedSkillEntry> {
    let candidate_count = candidates.len();
    let resource_count = candidates
        .iter()
        .map(|candidate| candidate.resource_count)
        .sum();
    let resource_bytes = candidates
        .iter()
        .map(|candidate| candidate.resource_bytes)
        .sum();
    Ok(DownloadedSkillEntry {
        id: path_key(&root_dir),
        source_url,
        root_dir,
        search_root,
        downloaded_at,
        candidate_count,
        resource_count,
        resource_bytes,
    })
}

fn download_folder_name(source: &GitHubTreeSource) -> String {
    let mut name = format!("{}-{}-{}", source.owner, source.repo, source.reference);
    if let Some(path_filter) = source.path_filter() {
        name.push('-');
        name.push_str(path_filter);
    }

    let sanitized = sanitize_folder_name(&name);
    if sanitized.is_empty() {
        "skills-download".to_string()
    } else {
        sanitized
    }
}

fn unique_download_dir(download_root: &Path, preferred_name: &str) -> PathBuf {
    let first = download_root.join(preferred_name);
    if !first.exists() {
        return first;
    }

    for index in 2.. {
        let candidate = download_root.join(format!("{preferred_name}-{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("the loop always returns")
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;
    use zip::{ZipWriter, write::SimpleFileOptions};

    use crate::ProjectRoot;

    use super::*;

    #[test]
    fn extracts_zip_without_path_traversal() {
        let bytes = fixture_archive();

        let dir = tempfile::tempdir().unwrap();
        extract_zip_safe(bytes.get_ref(), dir.path()).unwrap();
        assert!(dir.path().join("repo-main/skill/SKILL.md").exists());
    }

    #[test]
    fn caches_download_to_default_directory() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path());
        let bytes = fixture_archive();

        let entry = cache_github_skills_archive(
            &paths,
            "https://github.com/acme/skills",
            bytes.get_ref(),
            None,
        )
        .unwrap();

        assert!(entry.root_dir.starts_with(paths.downloads_dir()));
        assert_eq!(entry.candidate_count, 1);
        assert!(entry.search_root.join("repo-main/skill/SKILL.md").exists());

        let listed = list_downloaded_skills(&paths).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].candidate_count, 1);
        assert_eq!(listed[0].resource_count, entry.resource_count);
    }

    #[test]
    fn caches_download_to_override_directory_and_renames_conflicts() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path());
        let override_dir = dir.path().join("custom-downloads");
        let bytes = fixture_archive();

        let first = cache_github_skills_archive(
            &paths,
            "https://github.com/acme/skills",
            bytes.get_ref(),
            Some(&override_dir),
        )
        .unwrap();
        let second = cache_github_skills_archive(
            &paths,
            "https://github.com/acme/skills",
            bytes.get_ref(),
            Some(&override_dir),
        )
        .unwrap();

        assert!(first.root_dir.starts_with(&override_dir));
        assert!(second.root_dir.starts_with(&override_dir));
        assert_ne!(first.root_dir, second.root_dir);
        assert!(second.root_dir.ends_with("acme-skills-main-2"));
    }

    #[test]
    fn removes_only_recorded_downloads() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path());
        let bytes = fixture_archive();
        let entry = cache_github_skills_archive(
            &paths,
            "https://github.com/acme/skills",
            bytes.get_ref(),
            None,
        )
        .unwrap();
        let untracked = dir.path().join("untracked");
        fs::create_dir_all(&untracked).unwrap();

        assert!(remove_downloaded_skills(&paths, &untracked).is_err());
        let removed = remove_downloaded_skills(&paths, &entry.root_dir).unwrap();
        assert_eq!(removed, entry.root_dir);
        assert!(!removed.exists());
        assert!(list_downloaded_skills(&paths).unwrap().is_empty());
    }

    fn fixture_archive() -> Cursor<Vec<u8>> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut bytes);
            zip.start_file("repo-main/skill/SKILL.md", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"---\nname: demo\ndescription: Demo skill\n---\n")
                .unwrap();
            zip.finish().unwrap();
        }
        bytes
    }

    fn test_paths(root: &Path) -> ManagerPaths {
        ManagerPaths::with_home(
            root.join("home"),
            root.join("data"),
            root.join("config"),
            Some(ProjectRoot::new(root.join("project"))),
        )
    }
}
