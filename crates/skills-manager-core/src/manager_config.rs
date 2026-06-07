use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ManagerPaths, Result, SkillsManagerError, fs_ops::atomic_write, skill::path_key};

const CONFIG_LOCK_RETRY_DELAY: Duration = Duration::from_millis(25);
const CONFIG_LOCK_TIMEOUT: Duration = Duration::from_secs(60);
const CONFIG_LOCK_STALE_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManagerConfig {
    #[serde(default)]
    pub disabled_skill_files: BTreeSet<String>,
    #[serde(default)]
    pub installed: BTreeMap<String, InstalledMetadata>,
    #[serde(default)]
    pub custom_install_roots: Vec<PathBuf>,
    #[serde(default)]
    pub default_download_dir: Option<PathBuf>,
    #[serde(default)]
    pub downloads: BTreeMap<String, DownloadedMetadata>,
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMetadata {
    pub source_url: Option<String>,
    pub installed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedMetadata {
    pub source_url: String,
    pub downloaded_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub search_root: Option<PathBuf>,
    #[serde(default)]
    pub candidate_count: Option<usize>,
    #[serde(default)]
    pub resource_count: Option<usize>,
    #[serde(default)]
    pub resource_bytes: Option<u64>,
}

#[derive(Debug)]
pub struct ManagerConfigUpdateLock {
    path: PathBuf,
}

impl Drop for ManagerConfigUpdateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl ManagerConfig {
    pub fn load(paths: &ManagerPaths) -> Result<Self> {
        let file = paths.app_config_file();
        if !file.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(file)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self, paths: &ManagerPaths) -> Result<()> {
        let file = paths.app_config_file();
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(
            &file,
            toml::to_string_pretty(self).expect("serializing manager config"),
        )?;
        Ok(())
    }

    pub fn acquire_update_lock(paths: &ManagerPaths) -> Result<ManagerConfigUpdateLock> {
        let lock_path = config_lock_file(paths);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let started = Instant::now();
        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => return Ok(ManagerConfigUpdateLock { path: lock_path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    remove_stale_lock(&lock_path);
                    if started.elapsed() >= CONFIG_LOCK_TIMEOUT {
                        return Err(SkillsManagerError::ConfigLockTimeout(lock_path));
                    }
                    sleep(CONFIG_LOCK_RETRY_DELAY);
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub fn update<T>(
        paths: &ManagerPaths,
        update: impl FnOnce(&mut ManagerConfig) -> Result<T>,
    ) -> Result<T> {
        let _lock = Self::acquire_update_lock(paths)?;
        let mut config = Self::load(paths)?;
        let result = update(&mut config)?;
        config.save(paths)?;
        Ok(result)
    }

    pub fn is_disabled(&self, skill_file: &Path) -> bool {
        self.disabled_skill_files.contains(&path_key(skill_file))
    }

    pub fn set_disabled(&mut self, skill_file: &Path, disabled: bool) {
        let key = path_key(skill_file);
        if disabled {
            self.disabled_skill_files.insert(key);
        } else {
            self.disabled_skill_files.remove(&key);
        }
    }

    pub fn move_install_record(&mut self, from_root: &Path, to_root: &Path) {
        if let Some(metadata) = self.installed.remove(&path_key(from_root)) {
            self.installed.insert(path_key(to_root), metadata);
        }
    }

    pub fn record_install(&mut self, root_dir: &Path, source_url: Option<String>) {
        self.installed.insert(
            path_key(root_dir),
            InstalledMetadata {
                source_url,
                installed_at: Some(Utc::now()),
            },
        );
    }

    pub fn record_custom_install_root(&mut self, root: &Path) {
        let key = path_key(root);
        if !self
            .custom_install_roots
            .iter()
            .any(|existing| path_key(existing) == key)
        {
            self.custom_install_roots.push(root.to_path_buf());
        }
    }

    pub fn effective_download_dir(&self, paths: &ManagerPaths) -> PathBuf {
        self.default_download_dir
            .clone()
            .unwrap_or_else(|| paths.downloads_dir())
    }

    pub fn set_default_download_dir(&mut self, path: Option<PathBuf>) {
        self.default_download_dir = path;
    }

    pub fn record_download(&mut self, root_dir: &Path, source_url: String) {
        self.downloads.insert(
            path_key(root_dir),
            DownloadedMetadata {
                source_url,
                downloaded_at: Some(Utc::now()),
                search_root: None,
                candidate_count: None,
                resource_count: None,
                resource_bytes: None,
            },
        );
    }

    pub fn record_download_summary(
        &mut self,
        root_dir: &Path,
        source_url: String,
        search_root: &Path,
        candidate_count: usize,
        resource_count: usize,
        resource_bytes: u64,
    ) {
        self.downloads.insert(
            path_key(root_dir),
            DownloadedMetadata {
                source_url,
                downloaded_at: Some(Utc::now()),
                search_root: Some(search_root.to_path_buf()),
                candidate_count: Some(candidate_count),
                resource_count: Some(resource_count),
                resource_bytes: Some(resource_bytes),
            },
        );
    }

    pub fn forget_download(&mut self, root_dir: &Path) {
        self.downloads.remove(&path_key(root_dir));
    }

    pub fn forget_install(&mut self, root_dir: &Path, skill_file: &Path) {
        self.installed.remove(&path_key(root_dir));
        self.disabled_skill_files.remove(&path_key(skill_file));
    }
}

fn config_lock_file(paths: &ManagerPaths) -> PathBuf {
    paths.app_config_file().with_extension("toml.lock")
}

fn remove_stale_lock(lock_path: &Path) {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return;
    };
    let Ok(modified) = metadata.modified() else {
        return;
    };
    if modified
        .elapsed()
        .is_ok_and(|age| age >= CONFIG_LOCK_STALE_AFTER)
    {
        let _ = fs::remove_file(lock_path);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, Barrier},
        thread,
    };

    use tempfile::tempdir;

    use crate::{ManagerPaths, ProjectRoot};

    use super::*;

    #[test]
    fn update_lock_preserves_concurrent_config_changes() {
        let dir = tempdir().unwrap();
        let paths = Arc::new(ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(dir.path().join("project"))),
        ));
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = Vec::new();

        for index in 0..8 {
            let paths = Arc::clone(&paths);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                ManagerConfig::update(paths.as_ref(), |config| {
                    config
                        .recent_projects
                        .push(PathBuf::from(format!("project-{index}")));
                    Ok(())
                })
                .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let config = ManagerConfig::load(paths.as_ref()).unwrap();
        assert_eq!(config.recent_projects.len(), 8);
    }
}
