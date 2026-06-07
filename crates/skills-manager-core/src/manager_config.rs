use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ManagerPaths, Result, skill::path_key};

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
        fs::write(
            file,
            toml::to_string_pretty(self).expect("serializing manager config"),
        )?;
        Ok(())
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
