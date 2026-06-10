//! Persistent application configuration for install state, downloads, and sources.
//!
//! Provides [`ManagerConfig`], the TOML-backed settings store that tracks
//! disabled skill files, installed bundles, custom install roots, download
//! cache metadata, and marketplace source records. Also provides
//! [`ManagerConfigUpdateLock`] for coarse-grained concurrent write safety.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    mem,
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

/// Persisted settings for install state, downloads, and marketplace sources.
///
/// This struct is serialized to `~/.config/.../config.toml` via `toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManagerConfig {
    /// `SKILL.md` paths that are explicitly disabled.
    #[serde(default)]
    pub disabled_skill_files: BTreeSet<String>,
    /// Installed skill roots keyed by normalized install path.
    #[serde(default)]
    pub installed: BTreeMap<String, InstalledMetadata>,
    /// User-added skill roots outside the built-in scope list.
    #[serde(default)]
    pub custom_install_roots: Vec<PathBuf>,
    /// Optional override for default download cache location.
    #[serde(default)]
    pub default_download_dir: Option<PathBuf>,
    /// Download cache metadata indexed by bundle path.
    #[serde(default)]
    pub downloads: BTreeMap<String, DownloadedMetadata>,
    /// Recently opened projects for convenience UX.
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
    /// Configured remote marketplace sources.
    #[serde(default)]
    pub marketplace_sources: BTreeMap<String, MarketplaceSourceMetadata>,
    /// Resource installation metadata.
    #[serde(default)]
    pub resource_installs: BTreeMap<String, InstalledMetadata>,
}

/// Metadata for an installed skill or plugin entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMetadata {
    /// Origin URL if the install came from a remote source.
    pub source_url: Option<String>,
    /// Timestamp for UI and cleanup heuristics.
    pub installed_at: Option<DateTime<Utc>>,
}

/// Metadata for a downloaded archive in cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedMetadata {
    /// Source URL used to download the archive.
    pub source_url: String,
    /// Download timestamp.
    pub downloaded_at: Option<DateTime<Utc>>,
    /// Search root in the downloaded bundle used to discover candidates.
    #[serde(default)]
    pub search_root: Option<PathBuf>,
    /// Number of discovered skill candidates.
    #[serde(default)]
    pub candidate_count: Option<usize>,
    /// Number of files included in downloaded candidates.
    #[serde(default)]
    pub resource_count: Option<usize>,
    /// Total bytes across resource files.
    #[serde(default)]
    pub resource_bytes: Option<u64>,
}

/// Metadata record for a configured marketplace source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSourceMetadata {
    /// Human readable label shown in outputs and UI.
    pub label: String,
    /// Raw source string (URL/path).
    pub source: String,
    /// Optional target to constrain the source to one agent target.
    #[serde(default)]
    pub target: Option<String>,
    /// Optional provider identifier (`skillsmp`, etc.).
    #[serde(default)]
    pub provider: Option<String>,
    /// When the source was added.
    pub added_at: Option<DateTime<Utc>>,
    /// When the source was last refreshed.
    #[serde(default)]
    pub last_refreshed_at: Option<DateTime<Utc>>,
}

/// RAII token that deletes an update lock file when dropped.
#[derive(Debug)]
pub struct ManagerConfigUpdateLock {
    path: PathBuf,
}

impl Drop for ManagerConfigUpdateLock {
    /// Drops the lock file immediately when lock scope ends.
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl ManagerConfig {
    /// Loads config from disk. Returns default config when file is absent.
    pub fn load(paths: &ManagerPaths) -> Result<Self> {
        let file = paths.app_config_file();
        if !file.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(file)?;
        let mut config: Self = toml::from_str(&raw)?;
        config.normalize_stored_paths();
        Ok(config)
    }

    /// Writes config as pretty TOML and creates parent directories as needed.
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

    /// Acquires a coarse-grained lock file while config is being updated.
    ///
    /// The lock retries briefly until `CONFIG_LOCK_TIMEOUT` and removes stale locks older
    /// than `CONFIG_LOCK_STALE_AFTER`.
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

    /// Runs an in-place config edit with exclusive lock acquisition.
    ///
    /// This guarantees serialized writes and persists the result atomically.
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

    /// Returns whether a skill file is explicitly marked disabled in config.
    pub fn is_disabled(&self, skill_file: &Path) -> bool {
        self.disabled_skill_files.contains(&path_key(skill_file))
    }

    /// Updates the disabled marker for a skill path.
    pub fn set_disabled(&mut self, skill_file: &Path, disabled: bool) {
        let key = path_key(skill_file);
        if disabled {
            self.disabled_skill_files.insert(key);
        } else {
            self.disabled_skill_files.remove(&key);
        }
    }

    /// Renames an install metadata entry from `from_root` to `to_root`.
    pub fn move_install_record(&mut self, from_root: &Path, to_root: &Path) {
        if let Some(metadata) = self.installed.remove(&path_key(from_root)) {
            self.installed.insert(path_key(to_root), metadata);
        }
    }

    /// Records a skill install with optional source URL.
    pub fn record_install(&mut self, root_dir: &Path, source_url: Option<String>) {
        self.installed.insert(
            path_key(root_dir),
            InstalledMetadata {
                source_url,
                installed_at: Some(Utc::now()),
            },
        );
    }

    fn normalize_stored_paths(&mut self) {
        self.disabled_skill_files = normalize_path_set(mem::take(&mut self.disabled_skill_files));
        self.installed = normalize_path_map(mem::take(&mut self.installed));
        self.downloads = normalize_path_map(mem::take(&mut self.downloads));
        self.custom_install_roots = normalize_paths_vec(mem::take(&mut self.custom_install_roots));
    }

    /// Records a plugin/resource install keyed by a stable resource identifier.
    pub fn record_resource_install(&mut self, id: impl Into<String>, source_url: Option<String>) {
        self.resource_installs.insert(
            id.into(),
            InstalledMetadata {
                source_url,
                installed_at: Some(Utc::now()),
            },
        );
    }

    /// Removes a tracked resource install entry.
    pub fn forget_resource_install(&mut self, id: &str) {
        self.resource_installs.remove(id);
    }

    /// Adds or updates a configured marketplace source entry.
    pub fn record_marketplace_source(
        &mut self,
        label: impl Into<String>,
        source: impl Into<String>,
        target: Option<String>,
        provider: Option<String>,
    ) {
        let label = label.into();
        self.marketplace_sources.insert(
            label.clone(),
            MarketplaceSourceMetadata {
                label,
                source: source.into(),
                target,
                provider,
                added_at: Some(Utc::now()),
                last_refreshed_at: None,
            },
        );
    }

    /// Updates the `last_refreshed_at` timestamp for an existing source.
    pub fn mark_marketplace_refreshed(&mut self, label: &str) {
        if let Some(source) = self.marketplace_sources.get_mut(label) {
            source.last_refreshed_at = Some(Utc::now());
        }
    }

    /// Removes a marketplace source record.
    pub fn forget_marketplace_source(&mut self, label: &str) {
        self.marketplace_sources.remove(label);
    }

    /// Adds a custom install root when absent.
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

    /// Returns the active download directory (override or default workspace cache).
    pub fn effective_download_dir(&self, paths: &ManagerPaths) -> PathBuf {
        self.default_download_dir
            .clone()
            .unwrap_or_else(|| paths.downloads_dir())
    }

    /// Updates the user-level download cache override.
    pub fn set_default_download_dir(&mut self, path: Option<PathBuf>) {
        self.default_download_dir = path;
    }

    /// Records a downloaded bundle with only minimal metadata.
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

    /// Records full download summary metadata for a cached bundle.
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

    /// Removes a cached download record.
    pub fn forget_download(&mut self, root_dir: &Path) {
        self.downloads.remove(&path_key(root_dir));
    }

    /// Removes all install metadata for a skill directory and its skill file marker.
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

fn normalize_path_set(values: BTreeSet<String>) -> BTreeSet<String> {
    values
        .into_iter()
        .map(|path| path_key(Path::new(&path)))
        .collect()
}

fn normalize_path_map<T>(values: BTreeMap<String, T>) -> BTreeMap<String, T> {
    let mut normalized = BTreeMap::new();
    for (path, metadata) in values {
        let key = path_key(Path::new(&path));
        normalized.entry(key).or_insert(metadata);
    }
    normalized
}

fn normalize_paths_vec(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .scan(BTreeSet::new(), |seen, path| {
            let key = path_key(&path);
            if seen.insert(key) {
                Some(Some(path))
            } else {
                Some(None)
            }
        })
        .flatten()
        .collect()
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

    #[test]
    fn load_normalizes_stored_path_metadata() {
        let dir = tempdir().unwrap();
        let paths = ManagerPaths::with_home(
            dir.path().join("home"),
            dir.path().join("data"),
            dir.path().join("config"),
            None,
        );

        fs::create_dir_all(paths.config_dir()).unwrap();
        let legacy = ManagerConfig {
            disabled_skill_files: ["./skill.md", "skill.md"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            installed: [
                (
                    "skill.md".into(),
                    InstalledMetadata {
                        source_url: Some("a".into()),
                        installed_at: None,
                    },
                ),
                (
                    "SKILL.MD".into(),
                    InstalledMetadata {
                        source_url: Some("b".into()),
                        installed_at: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            custom_install_roots: vec![PathBuf::from("./foo"), PathBuf::from("foo")],
            default_download_dir: None,
            downloads: [
                (
                    "skill_download_dir".into(),
                    DownloadedMetadata {
                        source_url: "c".into(),
                        downloaded_at: None,
                        search_root: None,
                        candidate_count: None,
                        resource_count: None,
                        resource_bytes: None,
                    },
                ),
                (
                    "SKILL_DOWNLOAD_DIR".into(),
                    DownloadedMetadata {
                        source_url: "d".into(),
                        downloaded_at: None,
                        search_root: None,
                        candidate_count: None,
                        resource_count: None,
                        resource_bytes: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            recent_projects: Vec::new(),
            marketplace_sources: BTreeMap::new(),
            resource_installs: BTreeMap::new(),
        };
        fs::write(
            paths.app_config_file(),
            toml::to_string_pretty(&legacy).unwrap().as_bytes(),
        )
        .unwrap();

        let loaded = ManagerConfig::load(&paths).unwrap();
        assert_eq!(loaded.disabled_skill_files.len(), 1);
        if cfg!(windows) {
            assert_eq!(loaded.installed.len(), 1);
        } else {
            assert_eq!(loaded.installed.len(), 2);
        }
        assert_eq!(loaded.custom_install_roots.len(), 1);
        if cfg!(windows) {
            assert_eq!(loaded.downloads.len(), 1);
        } else {
            assert_eq!(loaded.downloads.len(), 2);
        }
    }
}
