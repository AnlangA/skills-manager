use std::{
    fs,
    path::{Path, PathBuf},
};

use toml_edit::{ArrayOfTables, DocumentMut, Item, Value};

use crate::{ManagerPaths, Result, SkillsManagerError, fs_ops::atomic_write, skill::path_key};

/// In-memory representation of `.codex/config.toml` with mutation helpers.
#[derive(Debug, Clone)]
pub struct CodexConfig {
    path: PathBuf,
    document: DocumentMut,
}

/// Parsed `[[skills.config]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSkillToggle {
    /// Absolute or relative skill manifest path.
    pub path: PathBuf,
    /// Whether the skill is enabled.
    pub enabled: bool,
}

/// Parsed plugin toggle entry under `[plugins]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexPluginToggle {
    /// Plugin identifier.
    pub id: String,
    /// Whether plugin is enabled.
    pub enabled: bool,
}

impl CodexConfig {
    /// Loads codex config from disk, returning an empty document if missing.
    pub fn load(paths: &ManagerPaths) -> Result<Self> {
        let path = paths.codex_config_file();
        if !path.exists() {
            return Ok(Self {
                path,
                document: DocumentMut::new(),
            });
        }

        let raw = fs::read_to_string(&path)?;
        let document =
            raw.parse::<DocumentMut>()
                .map_err(|source| SkillsManagerError::ParseToml {
                    path: path.clone(),
                    source,
                })?;

        Ok(Self { path, document })
    }

    /// Writes the current config to disk atomically.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&self.path, self.document.to_string())?;
        Ok(())
    }

    /// Returns all stored skill toggles from `[[skills.config]]`.
    pub fn toggles(&self) -> Vec<CodexSkillToggle> {
        let Some(configs) = self
            .document
            .get("skills")
            .and_then(|skills| skills.get("config"))
            .and_then(Item::as_array_of_tables)
        else {
            return Vec::new();
        };

        configs
            .iter()
            .filter_map(|table| {
                let path = table
                    .get("path")
                    .and_then(Item::as_value)
                    .and_then(Value::as_str)?;
                let enabled = table
                    .get("enabled")
                    .and_then(Item::as_value)
                    .and_then(Value::as_bool)
                    .unwrap_or(true);

                Some(CodexSkillToggle {
                    path: PathBuf::from(path),
                    enabled,
                })
            })
            .collect()
    }

    /// Returns all stored plugin toggles from `[plugins]`.
    pub fn plugin_toggles(&self) -> Vec<CodexPluginToggle> {
        let Some(plugins) = self.document.get("plugins").and_then(Item::as_table) else {
            return Vec::new();
        };

        plugins
            .iter()
            .filter_map(|(id, item)| {
                let table = item.as_table()?;
                let enabled = table
                    .get("enabled")
                    .and_then(Item::as_value)
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                Some(CodexPluginToggle {
                    id: id.to_string(),
                    enabled,
                })
            })
            .collect()
    }

    /// Checks whether a given skill file is explicitly disabled.
    pub fn is_disabled(&self, skill_file: &Path) -> bool {
        let target = path_key(skill_file);
        self.toggles()
            .into_iter()
            .any(|toggle| path_key(&toggle.path) == target && !toggle.enabled)
    }

    /// Checks whether a given plugin is explicitly disabled.
    pub fn is_plugin_disabled(&self, plugin_id: &str) -> bool {
        self.plugin_toggles()
            .into_iter()
            .any(|toggle| toggle.id == plugin_id && !toggle.enabled)
    }

    /// Sets or updates a skill toggle for the given `SKILL.md` path.
    pub fn set_enabled(&mut self, skill_file: &Path, enabled: bool) {
        let skill_file = skill_file.to_path_buf();
        let target = path_key(&skill_file);
        let configs = ensure_skill_configs(&mut self.document);

        for table in configs.iter_mut() {
            let path_matches = table
                .get("path")
                .and_then(Item::as_value)
                .and_then(Value::as_str)
                .is_some_and(|path| path == target);

            if path_matches {
                table["enabled"] = toml_edit::value(enabled);
                return;
            }
        }

        let mut table = toml_edit::Table::new();
        table["path"] = toml_edit::value(target);
        table["enabled"] = toml_edit::value(enabled);
        configs.push(table);
    }

    /// Sets or updates a plugin toggle under `[plugins]`.
    pub fn set_plugin_enabled(&mut self, plugin_id: &str, enabled: bool) {
        if !self.document.as_table().contains_key("plugins") {
            self.document["plugins"] = Item::Table(toml_edit::Table::new());
        }

        if !self.document["plugins"]
            .as_table()
            .expect("plugins is table")
            .contains_key(plugin_id)
        {
            self.document["plugins"][plugin_id] = Item::Table(toml_edit::Table::new());
        }

        self.document["plugins"][plugin_id]["enabled"] = toml_edit::value(enabled);
    }

    /// Removes a skill toggle entry for a given `SKILL.md` path.
    pub fn forget(&mut self, skill_file: &Path) {
        let target = path_key(skill_file);
        let Some(configs) = self
            .document
            .get_mut("skills")
            .and_then(|skills| skills.get_mut("config"))
            .and_then(Item::as_array_of_tables_mut)
        else {
            return;
        };

        let mut kept = ArrayOfTables::new();
        for table in configs.iter() {
            let path_matches = table
                .get("path")
                .and_then(Item::as_value)
                .and_then(Value::as_str)
                .is_some_and(|path| path == target);

            if !path_matches {
                kept.push(table.clone());
            }
        }
        *configs = kept;
    }

    /// Removes plugin toggle state for a given plugin id.
    pub fn forget_plugin(&mut self, plugin_id: &str) {
        let Some(plugins) = self
            .document
            .get_mut("plugins")
            .and_then(Item::as_table_mut)
        else {
            return;
        };
        plugins.remove(plugin_id);
    }
}

fn ensure_skill_configs(document: &mut DocumentMut) -> &mut ArrayOfTables {
    if !document.as_table().contains_key("skills") {
        document["skills"] = Item::Table(toml_edit::Table::new());
    }

    if !document["skills"]
        .as_table()
        .expect("skills is table")
        .contains_key("config")
    {
        document["skills"]["config"] = Item::ArrayOfTables(ArrayOfTables::new());
    }

    document["skills"]["config"]
        .as_array_of_tables_mut()
        .expect("skills.config is array of tables")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::ProjectRoot;

    use super::*;

    #[test]
    fn writes_skill_toggle_without_destroying_existing_config() {
        let dir = tempdir().unwrap();
        let paths = ManagerPaths::with_home(
            dir.path(),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(dir.path().join("project"))),
        );
        fs::create_dir_all(dir.path().join(".codex")).unwrap();
        fs::write(
            paths.codex_config_file(),
            "model = \"gpt-5\"\n\n[[skills.config]]\npath = \"/old/SKILL.md\"\nenabled = true\n",
        )
        .unwrap();

        let skill_file = dir.path().join(".agents/skills/demo/SKILL.md");
        let mut config = CodexConfig::load(&paths).unwrap();
        config.set_enabled(&skill_file, false);
        config.save().unwrap();

        let written = fs::read_to_string(paths.codex_config_file()).unwrap();
        assert!(written.contains("model = \"gpt-5\""));
        assert!(written.contains("/old/SKILL.md"));
        assert!(written.contains(skill_file.to_string_lossy().as_ref()));
        assert!(written.contains("enabled = false"));
    }

    #[test]
    fn writes_plugin_toggle_without_destroying_existing_config() {
        let dir = tempdir().unwrap();
        let paths = ManagerPaths::with_home(
            dir.path(),
            dir.path().join("data"),
            dir.path().join("config"),
            Some(ProjectRoot::new(dir.path().join("project"))),
        );
        fs::create_dir_all(dir.path().join(".codex")).unwrap();
        fs::write(paths.codex_config_file(), "model = \"gpt-5\"\n").unwrap();

        let mut config = CodexConfig::load(&paths).unwrap();
        config.set_plugin_enabled("demo@local", false);
        config.save().unwrap();

        let written = fs::read_to_string(paths.codex_config_file()).unwrap();
        assert!(written.contains("model = \"gpt-5\""));
        assert!(written.contains("[plugins.\"demo@local\"]"));
        assert!(written.contains("enabled = false"));
        assert!(
            CodexConfig::load(&paths)
                .unwrap()
                .is_plugin_disabled("demo@local")
        );
    }
}
