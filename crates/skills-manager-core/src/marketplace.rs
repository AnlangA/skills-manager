//! Catalog format, parsing, and export for skill marketplaces.
//!
//! Provides [`SkillCatalog`] for loading and representing collections of
//! skills from JSON catalogs, [`SkillCatalogEntry`] for individual rows,
//! [`SkillCatalogSource`] for discriminated source descriptors, and
//! [`CatalogFormat`] for supported export output formats.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{InstalledSkill, Result, SkillsManagerError};

/// Parsed skills catalog loaded from local files or remote JSON payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCatalog {
    /// Optional catalog display name.
    #[serde(default)]
    pub name: Option<String>,
    /// List of catalog entries, accepting legacy `plugins` aliases.
    #[serde(default, alias = "plugins")]
    pub skills: Vec<SkillCatalogEntry>,
}

/// Single catalog row used for import/export workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCatalogEntry {
    /// Canonical machine name.
    pub name: String,
    /// Optional display label.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Source descriptor (github/local/legacy).
    #[serde(flatten)]
    pub source: SkillCatalogSource,
}

/// Discriminated source specification for a catalog row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
pub enum SkillCatalogSource {
    /// Source from GitHub repository plus optional path.
    Git {
        /// Repository URL.
        url: String,
        /// Optional path inside repository.
        #[serde(default)]
        path: Option<String>,
    },
    /// Source from filesystem.
    Local {
        /// Local path.
        path: String,
    },
    /// Unknown source shape preserved for tolerant parsing.
    #[serde(other)]
    Unknown,
}

/// Supported export output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogFormat {
    /// Pretty JSON.
    Json,
    /// XML string output.
    Xml,
    /// Markdown table.
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ExportedCatalog {
    name: String,
    skills: Vec<ExportedSkill>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ExportedSkill {
    name: String,
    description: String,
    location: String,
}

impl SkillCatalog {
    /// Loads and parses a catalog JSON file.
    pub fn load_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|source| SkillsManagerError::ParseCatalog {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Exports only usable/enableable skills to the selected format.
pub fn export_installed_catalog(
    name: impl Into<String>,
    skills: &[InstalledSkill],
    format: CatalogFormat,
) -> Result<String> {
    let catalog = ExportedCatalog {
        name: name.into(),
        skills: skills
            .iter()
            .filter(|skill| skill.is_exportable())
            .map(|skill| ExportedSkill {
                name: skill.display_name.clone(),
                description: skill.description.clone().unwrap_or_default(),
                location: skill.root_dir.display().to_string(),
            })
            .collect(),
    };

    match format {
        CatalogFormat::Json => serde_json::to_string_pretty(&catalog).map_err(Into::into),
        CatalogFormat::Xml => Ok(export_xml(&catalog)),
        CatalogFormat::Markdown => Ok(export_markdown(&catalog)),
    }
}

fn export_xml(catalog: &ExportedCatalog) -> String {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str(&format!(
        "<skills-catalog name=\"{}\">\n",
        escape_xml(&catalog.name)
    ));
    for skill in &catalog.skills {
        output.push_str("  <skill>\n");
        output.push_str(&format!("    <name>{}</name>\n", escape_xml(&skill.name)));
        output.push_str(&format!(
            "    <description>{}</description>\n",
            escape_xml(&skill.description)
        ));
        output.push_str(&format!(
            "    <location>{}</location>\n",
            escape_xml(&skill.location)
        ));
        output.push_str("  </skill>\n");
    }
    output.push_str("</skills-catalog>\n");
    output
}

fn export_markdown(catalog: &ExportedCatalog) -> String {
    let mut output = format!("# {}\n\n", catalog.name);
    output.push_str("| Skill | Description | Location |\n");
    output.push_str("| --- | --- | --- |\n");
    for skill in &catalog.skills {
        output.push_str(&format!(
            "| {} | {} | {} |\n",
            escape_markdown_table_cell(&skill.name),
            escape_markdown_table_cell(&skill.description),
            escape_markdown_table_cell(&skill.location)
        ));
    }
    output
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

/// Legacy alias for [`SkillCatalog`].
pub type Marketplace = SkillCatalog;
/// Legacy alias for [`SkillCatalogEntry`].
pub type MarketplaceEntry = SkillCatalogEntry;
/// Legacy alias for [`SkillCatalogSource`].
pub type MarketplaceSource = SkillCatalogSource;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{SkillEnablement, SkillFrontmatter, SkillHealth, SkillScope};

    use super::*;

    #[test]
    fn parses_catalog_json_with_skills_key() {
        let catalog: SkillCatalog = serde_json::from_str(
            r#"{
                "name": "Demo",
                "skills": [
                    {
                        "name": "demo-skill",
                        "display_name": "Demo Skill",
                        "source": "git",
                        "url": "https://github.com/example/skills",
                        "path": "demo"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(catalog.name.as_deref(), Some("Demo"));
        assert_eq!(catalog.skills.len(), 1);
    }

    #[test]
    fn parses_legacy_catalog_json_with_plugins_key() {
        let catalog: SkillCatalog = serde_json::from_str(
            r#"{
                "name": "Demo",
                "plugins": [
                    {
                        "name": "demo-plugin",
                        "display_name": "Demo Plugin",
                        "source": "local",
                        "path": "./demo"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(catalog.skills.len(), 1);
    }

    #[test]
    fn exports_enabled_usable_skills_only() {
        let usable = fixture_skill("usable", SkillEnablement::Enabled, SkillHealth::Warning);
        let disabled = fixture_skill("disabled", SkillEnablement::Disabled, SkillHealth::Valid);
        let invalid = fixture_skill("invalid", SkillEnablement::Enabled, SkillHealth::Invalid);
        let exported = export_installed_catalog(
            "Agent Skills",
            &[usable, disabled, invalid],
            CatalogFormat::Json,
        )
        .unwrap();

        assert!(exported.contains("usable"));
        assert!(!exported.contains("disabled"));
        assert!(!exported.contains("invalid"));
    }

    fn fixture_skill(
        name: &str,
        enablement: SkillEnablement,
        health: SkillHealth,
    ) -> InstalledSkill {
        InstalledSkill {
            id: format!("global:/tmp/{name}"),
            display_name: name.to_string(),
            description: Some(format!("{name} description")),
            scope: SkillScope::Global,
            root_dir: PathBuf::from(format!("/tmp/{name}")),
            skill_file: PathBuf::from(format!("/tmp/{name}/SKILL.md")),
            frontmatter: SkillFrontmatter {
                name: Some(name.to_string()),
                description: Some(format!("{name} description")),
                ..SkillFrontmatter::default()
            },
            enablement,
            health,
            diagnostics: Vec::new(),
            resource_count: 0,
            resource_bytes: 0,
            shadowed_by: None,
            source_url: None,
            installed_at: None,
        }
    }
}
