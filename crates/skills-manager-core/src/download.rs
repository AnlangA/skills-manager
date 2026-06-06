use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use tempfile::TempDir;
use zip::ZipArchive;

use crate::{
    GitHubTreeSource, Result, SkillCatalog, SkillsManagerError,
    skill::{SkillCandidate, discover_skill_candidates},
};

#[derive(Debug)]
pub struct DownloadedSkills {
    pub temp_dir: TempDir,
    pub source: GitHubTreeSource,
    pub candidates: Vec<SkillCandidate>,
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

pub fn download_github_skills(url: &str) -> Result<DownloadedSkills> {
    let source = GitHubTreeSource::parse(url)?;
    let bytes = download_first_archive(&source)?;
    let temp_dir = tempfile::tempdir()?;
    extract_zip_safe(&bytes, temp_dir.path())?;

    let search_root = filtered_search_root(temp_dir.path(), source.path_filter())?;
    let candidates = discover_skill_candidates(&search_root)?;

    if candidates.is_empty() {
        return Err(SkillsManagerError::NoSkillsFound);
    }

    Ok(DownloadedSkills {
        temp_dir,
        source,
        candidates,
    })
}

pub fn download_github_marketplace(url: &str) -> Result<DownloadedMarketplace> {
    let downloaded = download_github_catalog(url)?;
    Ok(DownloadedMarketplace {
        temp_dir: downloaded.temp_dir,
        source: downloaded.source,
        marketplace: downloaded.catalog,
    })
}

pub fn download_github_catalog(url: &str) -> Result<DownloadedCatalog> {
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
        let catalog = reqwest::blocking::get(raw_url)?.json::<SkillCatalog>()?;
        return Ok(DownloadedCatalog {
            temp_dir: None,
            source,
            catalog,
        });
    }

    let bytes = download_first_archive(&source)?;
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
            return Ok(DownloadedCatalog {
                temp_dir: Some(temp_dir),
                source,
                catalog,
            });
        }
    }

    Err(SkillsManagerError::NoSkillsFound)
}

fn download_first_archive(source: &GitHubTreeSource) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder().build()?;
    let mut last_error = None;

    for url in source.archive_url_candidates() {
        let response = match client.get(&url).send() {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };

        if response.status().is_success() {
            return Ok(response.bytes()?.to_vec());
        }
    }

    match last_error {
        Some(error) => Err(error.into()),
        None => Err(SkillsManagerError::UnsupportedUrl),
    }
}

pub fn extract_zip_safe(bytes: &[u8], destination: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| SkillsManagerError::UnsafeArchivePath(file.name().to_string()))?;
        let output = destination.join(enclosed);

        if file.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        fs::write(output, bytes)?;
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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::*;

    #[test]
    fn extracts_zip_without_path_traversal() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut bytes);
            zip.start_file("repo-main/skill/SKILL.md", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"---\nname: demo\ndescription: Demo\n---\n")
                .unwrap();
            zip.finish().unwrap();
        }

        let dir = tempfile::tempdir().unwrap();
        extract_zip_safe(bytes.get_ref(), dir.path()).unwrap();
        assert!(dir.path().join("repo-main/skill/SKILL.md").exists());
    }
}
