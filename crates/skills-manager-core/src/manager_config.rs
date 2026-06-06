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
    pub recent_projects: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMetadata {
    pub source_url: Option<String>,
    pub installed_at: Option<DateTime<Utc>>,
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

    pub fn record_install(&mut self, root_dir: &Path, source_url: Option<String>) {
        self.installed.insert(
            path_key(root_dir),
            InstalledMetadata {
                source_url,
                installed_at: Some(Utc::now()),
            },
        );
    }

    pub fn forget_install(&mut self, root_dir: &Path, skill_file: &Path) {
        self.installed.remove(&path_key(root_dir));
        self.disabled_skill_files.remove(&path_key(skill_file));
    }
}
