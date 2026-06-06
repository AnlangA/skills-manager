use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use skills_manager_core::{
    CatalogFormat, ConflictPolicy, InstallRequest, Installer, ManagerPaths, ProjectRoot,
    SkillEnablement, SkillHealth, SkillScope, download_github_catalog, download_github_skills,
    export_installed_catalog, format_bytes, scan_installed_skills,
};

#[derive(Debug, Parser)]
#[command(version, about = "Manage local Agent Skills libraries")]
struct Cli {
    #[arg(long, value_name = "DIR", global = true)]
    project: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Scan,
    Validate,
    PreviewInstall {
        source: String,
        #[arg(long)]
        local: bool,
        #[arg(long, value_enum, default_value_t = CliScope::User)]
        scope: CliScope,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    InstallUrl {
        url: String,
        #[arg(long, value_enum, default_value_t = CliScope::User)]
        scope: CliScope,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    InstallLocal {
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = CliScope::User)]
        scope: CliScope,
        #[arg(long, value_enum, default_value_t = CliConflictPolicy::Block)]
        conflict: CliConflictPolicy,
    },
    Catalog {
        #[arg(long, value_enum, default_value_t = CliCatalogFormat::Json)]
        format: CliCatalogFormat,
    },
    LoadCatalog {
        url: String,
    },
    Enable {
        path: PathBuf,
    },
    Disable {
        path: PathBuf,
    },
    Remove {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliScope {
    #[value(alias = "global")]
    User,
    Project,
}

impl From<CliScope> for SkillScope {
    fn from(value: CliScope) -> Self {
        match value {
            CliScope::User => Self::User,
            CliScope::Project => Self::Project,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliConflictPolicy {
    Block,
    Replace,
    Rename,
}

impl From<CliConflictPolicy> for ConflictPolicy {
    fn from(value: CliConflictPolicy) -> Self {
        match value {
            CliConflictPolicy::Block => Self::Block,
            CliConflictPolicy::Replace => Self::Replace,
            CliConflictPolicy::Rename => Self::Rename,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliCatalogFormat {
    Json,
    Xml,
    Markdown,
}

impl From<CliCatalogFormat> for CatalogFormat {
    fn from(value: CliCatalogFormat) -> Self {
        match value {
            CliCatalogFormat::Json => Self::Json,
            CliCatalogFormat::Xml => Self::Xml,
            CliCatalogFormat::Markdown => Self::Markdown,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = ManagerPaths::new(cli.project.map(ProjectRoot::new))?;
    let installer = Installer::new(paths.clone());

    match cli.command {
        Command::Scan => {
            let skills = scan_installed_skills(&paths)?;
            if skills.is_empty() {
                println!("No skills found.");
            }

            for skill in skills {
                print_skill_line(&skill);
            }
        }
        Command::Validate => {
            let skills = scan_installed_skills(&paths)?;
            if skills.is_empty() {
                println!("No skills found.");
            }

            for skill in &skills {
                print_skill_line(skill);
                for diagnostic in &skill.diagnostics {
                    println!(
                        "  - {}: {}",
                        diagnostic.severity.label(),
                        diagnostic.message
                    );
                }
            }

            let invalid = skills
                .iter()
                .filter(|skill| skill.health == SkillHealth::Invalid)
                .count();
            if invalid > 0 {
                println!("{invalid} invalid skill(s) found.");
            } else {
                println!("All scanned skills are usable.");
            }
        }
        Command::PreviewInstall {
            source,
            local,
            scope,
            conflict,
        } => {
            if local {
                print_preview(
                    &installer,
                    Path::new(&source),
                    scope.into(),
                    conflict.into(),
                )?;
            } else {
                let downloaded = download_github_skills(&source)
                    .with_context(|| format!("failed to download skills from {source}"))?;
                print_preview(
                    &installer,
                    downloaded.temp_dir.path(),
                    scope.into(),
                    conflict.into(),
                )?;
            }
        }
        Command::InstallUrl {
            url,
            scope,
            conflict,
        } => {
            let downloaded = download_github_skills(&url)
                .with_context(|| format!("failed to download skills from {url}"))?;
            let result = installer.install(InstallRequest {
                source_root: downloaded.temp_dir.path().to_path_buf(),
                source_url: Some(url),
                scope: scope.into(),
                conflict_policy: conflict.into(),
            })?;
            print_install_result(result.installed, result.backups);
        }
        Command::InstallLocal {
            path,
            scope,
            conflict,
        } => {
            let result = installer.install(InstallRequest {
                source_root: path,
                source_url: None,
                scope: scope.into(),
                conflict_policy: conflict.into(),
            })?;
            print_install_result(result.installed, result.backups);
        }
        Command::Catalog { format } => {
            let skills = scan_installed_skills(&paths)?;
            let exported = export_installed_catalog("Agent Skills", &skills, format.into())?;
            print!("{exported}");
        }
        Command::LoadCatalog { url } => {
            let downloaded = download_github_catalog(&url)
                .with_context(|| format!("failed to download catalog from {url}"))?;
            println!(
                "Catalog: {}",
                downloaded
                    .catalog
                    .name
                    .as_deref()
                    .unwrap_or("Unnamed catalog")
            );
            for skill in downloaded.catalog.skills {
                println!(
                    "- {}: {}",
                    skill.display_name.as_deref().unwrap_or(&skill.name),
                    skill.description.as_deref().unwrap_or("No description")
                );
            }
        }
        Command::Enable { path } => {
            installer.set_disabled(&skill_file(path), false)?;
            println!("Enabled skill.");
        }
        Command::Disable { path } => {
            installer.set_disabled(&skill_file(path), true)?;
            println!("Disabled skill.");
        }
        Command::Remove { path } => {
            let backup = installer.remove(&path)?;
            println!("Removed skill. Backup: {}", backup.display());
        }
    }

    Ok(())
}

fn print_skill_line(skill: &skills_manager_core::InstalledSkill) {
    println!(
        "{} [{}] {} / {} - {} | resources: {} ({}) | source: {} | installed: {}",
        skill.display_name,
        skill.scope.label(),
        enablement_label(skill.enablement),
        skill.health.label(),
        skill.root_dir.display(),
        skill.resource_count,
        format_bytes(skill.resource_bytes),
        skill.source_url.as_deref().unwrap_or("unknown"),
        skill
            .installed_at
            .map(|time| time.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string())
    );
}

fn print_preview(
    installer: &Installer,
    source_root: &Path,
    scope: SkillScope,
    conflict: ConflictPolicy,
) -> Result<()> {
    let preview = installer.preview(source_root, scope, conflict)?;
    println!(
        "Preview: {} candidate(s) for {} scope",
        preview.candidates.len(),
        preview.scope.label()
    );
    for candidate in preview.candidates {
        let name = candidate
            .frontmatter
            .name
            .as_deref()
            .unwrap_or("unnamed skill");
        println!(
            "- {name}: {} -> {} | conflict: {} | health: {} | resources: {} ({})",
            candidate.source_root.display(),
            candidate.destination_root.display(),
            candidate.conflict,
            candidate.health.label(),
            candidate.resource_count,
            format_bytes(candidate.resource_bytes)
        );
        for diagnostic in candidate.diagnostics {
            println!(
                "  - {}: {}",
                diagnostic.severity.label(),
                diagnostic.message
            );
        }
    }
    Ok(())
}

fn enablement_label(enablement: SkillEnablement) -> &'static str {
    match enablement {
        SkillEnablement::Enabled => "enabled",
        SkillEnablement::Disabled => "disabled",
    }
}

fn print_install_result(installed: Vec<PathBuf>, backups: Vec<PathBuf>) {
    for path in installed {
        println!("Installed: {}", path.display());
    }
    for backup in backups {
        println!("Backup: {}", backup.display());
    }
}

fn skill_file(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.join("SKILL.md")
    } else {
        path
    }
}
