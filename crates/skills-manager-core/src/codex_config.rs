use std::{
    fs,
    path::{Path, PathBuf},
};

use toml_edit::{ArrayOfTables, DocumentMut, Item, Value};

use crate::{ManagerPaths, Result, SkillsManagerError, skill::path_key};

#[derive(Debug, Clone)]
pub struct CodexConfig {
    path: PathBuf,
    document: DocumentMut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSkillToggle {
    pub path: PathBuf,
    pub enabled: bool,
}

impl CodexConfig {
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

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, self.document.to_string())?;
        Ok(())
    }

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

    pub fn is_disabled(&self, skill_file: &Path) -> bool {
        let target = path_key(skill_file);
        self.toggles()
            .into_iter()
            .any(|toggle| path_key(&toggle.path) == target && !toggle.enabled)
    }

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
}
