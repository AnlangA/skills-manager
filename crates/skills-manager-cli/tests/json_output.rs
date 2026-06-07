use std::{fs, process::Command};

use serde_json::Value;
use tempfile::tempdir;

#[test]
fn scan_supports_json_output() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    write_skill(
        &project,
        "demo",
        "---\nname: demo\ndescription: Demo skill for JSON output\n---\n",
    );

    let output = cli(sandbox.path())
        .arg("--project")
        .arg(&project)
        .arg("--output")
        .arg("json")
        .arg("scan")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["command"], "scan");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["type"], "skills");
    assert_eq!(json["data"]["type"], "skills");
    assert_eq!(json["skills"][0]["display_name"], "demo");
    assert_eq!(json["skills"][0]["scope"], "Project");
}

#[test]
fn inventory_scan_returns_workspace_snapshot_envelope() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    write_skill(
        &project,
        "demo",
        "---\nname: demo\ndescription: Demo skill for workspace output\n---\n",
    );

    let output = cli(sandbox.path())
        .arg("--project")
        .arg(&project)
        .arg("--output")
        .arg("json")
        .arg("inventory")
        .arg("scan")
        .output()
        .unwrap();

    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["command"], "inventory");
    assert_eq!(json["data"]["type"], "workspace");
    assert_eq!(json["data"]["snapshot"]["counts"]["total"], 1);
    assert_eq!(json["snapshot"]["counts"]["total"], 1);
}

#[test]
fn scan_supports_json_v3_output_without_legacy_flattening() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    write_skill(
        &project,
        "demo",
        "---\nname: demo\ndescription: Demo skill for JSON v3 output\n---\n",
    );

    let output = cli(sandbox.path())
        .arg("--project")
        .arg(&project)
        .arg("--output")
        .arg("json-v3")
        .arg("scan")
        .output()
        .unwrap();

    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 3);
    assert_eq!(json["command"], "scan");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["type"], "skills");
    assert_eq!(json["data"]["skills"][0]["display_name"], "demo");
    assert!(json.get("skills").is_none());
    assert_eq!(json["meta"]["format"], "json-v3");
    assert_eq!(json["meta"]["legacy_flattened_fields"], false);
}

#[test]
fn validate_json_reports_invalid_count() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    write_skill(&project, "broken", "---\ndescription: Missing name\n---\n");

    let output = cli(sandbox.path())
        .arg("--project")
        .arg(&project)
        .arg("--output")
        .arg("json")
        .arg("validate")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "validation");
    assert_eq!(json["invalid"], 1);
    assert_eq!(
        json["skills"][0]["diagnostics"][0]["message"],
        "SKILL.md frontmatter is missing `name`"
    );
}

#[test]
fn validate_json_can_filter_by_target() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    write_skill(
        &project,
        "broken",
        "---\ndescription: Missing name for project target\n---\n",
    );
    fs::create_dir_all(sandbox.path().join(".config/zed/skills/demo")).unwrap();
    fs::write(
        sandbox.path().join(".config/zed/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: Use this skill when testing target validation\n---\n",
    )
    .unwrap();

    let output = cli(sandbox.path())
        .arg("--project")
        .arg(&project)
        .arg("--output")
        .arg("json")
        .arg("validate")
        .arg("--target")
        .arg("zed")
        .output()
        .unwrap();

    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "validation");
    assert_eq!(json["skills"].as_array().unwrap().len(), 1);
    assert_eq!(json["skills"][0]["scope"], "Zed");
}

#[test]
fn doctor_supports_json_output() {
    let sandbox = tempdir().unwrap();
    fs::create_dir_all(sandbox.path().join(".codex")).unwrap();
    fs::write(
        sandbox.path().join(".codex/config.toml"),
        "[[skills.config]]\npath = \"/missing/SKILL.md\"\nenabled = false\n",
    )
    .unwrap();

    let output = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("doctor")
        .output()
        .unwrap();

    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "doctor");
    assert!(json["report"]["summary"]["targets"].as_u64().unwrap() >= 7);
    assert!(
        json["report"]["summary"]["repair_actions"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

#[test]
fn repair_dry_run_and_apply_migrates_legacy_disabled_store() {
    let sandbox = tempdir().unwrap();
    let legacy = sandbox.path().join(".config/zed/skills/.disabled/demo");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(
        legacy.join("SKILL.md"),
        "---\nname: demo\ndescription: Use this skill when testing repair\n---\n",
    )
    .unwrap();

    let dry_run = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("repair")
        .output()
        .unwrap();
    assert_success(&dry_run);
    let json: Value = serde_json::from_slice(&dry_run.stdout).unwrap();
    assert_eq!(json["type"], "repair");
    assert_eq!(json["report"]["dry_run"], true);
    assert!(legacy.join("SKILL.md").exists());

    let apply = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("repair")
        .arg("--apply")
        .output()
        .unwrap();
    assert_success(&apply);
    let json: Value = serde_json::from_slice(&apply.stdout).unwrap();
    assert_eq!(json["report"]["dry_run"], false);
    assert!(!legacy.exists());
    assert!(
        sandbox
            .path()
            .join(".config/zed/.skills-disabled/demo/SKILL.md")
            .exists()
    );
}

#[test]
fn create_dry_run_supports_json_output() {
    let sandbox = tempdir().unwrap();
    let output = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("create")
        .arg("--name")
        .arg("demo")
        .arg("--description")
        .arg("Use this skill when testing create dry run")
        .arg("--target")
        .arg("codex")
        .arg("--tag")
        .arg("testing")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "scaffold");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["preview"]["scope"], "Codex");
    assert!(!sandbox.path().join(".codex/skills/demo/SKILL.md").exists());
}

#[test]
fn target_install_disable_enable_and_remove_round_trip() {
    let sandbox = tempdir().unwrap();
    let source_root = sandbox.path().join("source");
    write_source_skill(
        &source_root,
        "demo",
        "---\nname: demo\ndescription: Demo skill for target operations\n---\n",
    );

    let install = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("install-local")
        .arg(&source_root)
        .arg("--target")
        .arg("zed")
        .output()
        .unwrap();
    assert_success(&install);
    let json: Value = serde_json::from_slice(&install.stdout).unwrap();
    assert_eq!(json["type"], "install");

    let installed = sandbox
        .path()
        .join(".config")
        .join("zed")
        .join("skills")
        .join("demo");
    assert!(installed.join("SKILL.md").exists());

    let disable = cli(sandbox.path())
        .arg("disable")
        .arg("demo")
        .arg("--target")
        .arg("zed")
        .output()
        .unwrap();
    assert_success(&disable);
    let scan = scan_json(sandbox.path());
    assert!(!installed.exists());
    assert!(
        installed
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".skills-disabled/demo/SKILL.md")
            .exists()
    );
    assert_eq!(scan["skills"].as_array().unwrap().len(), 1);
    assert_eq!(scan["skills"][0]["scope"], "Zed");
    assert_eq!(scan["skills"][0]["enablement"], "Disabled");
    assert!(
        scan["skills"][0]["root_dir"]
            .as_str()
            .unwrap()
            .ends_with(".config/zed/.skills-disabled/demo")
    );

    let enable = cli(sandbox.path())
        .arg("enable")
        .arg("demo")
        .arg("--target")
        .arg("zed")
        .output()
        .unwrap();
    assert_success(&enable);
    let scan = scan_json(sandbox.path());
    assert_eq!(scan["skills"][0]["scope"], "Zed");
    assert_eq!(scan["skills"][0]["enablement"], "Enabled");

    let remove = cli(sandbox.path())
        .arg("remove")
        .arg("demo")
        .arg("--target")
        .arg("zed")
        .output()
        .unwrap();
    assert_success(&remove);
    assert!(!installed.exists());
    assert!(
        fs::read_dir(installed.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("demo.backup-"))
    );
}

#[test]
fn claude_code_disable_moves_skill_outside_skills_root() {
    let sandbox = tempdir().unwrap();
    let source_root = sandbox.path().join("source");
    write_source_skill(
        &source_root,
        "demo",
        "---\nname: demo\ndescription: Demo skill for Claude Code\n---\n",
    );

    let install = cli(sandbox.path())
        .arg("install-local")
        .arg(&source_root)
        .arg("--target")
        .arg("claude-code")
        .output()
        .unwrap();
    assert_success(&install);
    let installed = sandbox.path().join(".claude/skills/demo");
    assert!(installed.join("SKILL.md").exists());

    let disable = cli(sandbox.path())
        .arg("disable")
        .arg("demo")
        .arg("--target")
        .arg("claude-code")
        .output()
        .unwrap();
    assert_success(&disable);

    assert!(!installed.exists());
    assert!(
        sandbox
            .path()
            .join(".claude/.skills-disabled/demo/SKILL.md")
            .exists()
    );
    assert!(
        !sandbox
            .path()
            .join(".claude/skills/.disabled/demo")
            .exists()
    );
    let scan = scan_json(sandbox.path());
    assert_eq!(scan["skills"][0]["scope"], "ClaudeCode");
    assert_eq!(scan["skills"][0]["enablement"], "Disabled");
    assert!(
        scan["skills"][0]["root_dir"]
            .as_str()
            .unwrap()
            .ends_with(".claude/.skills-disabled/demo")
    );
}

#[test]
fn codex_disable_uses_config_toggle_without_moving_directory() {
    let sandbox = tempdir().unwrap();
    let source_root = sandbox.path().join("source");
    write_source_skill(
        &source_root,
        "demo",
        "---\nname: demo\ndescription: Demo skill for Codex\n---\n",
    );

    let install = cli(sandbox.path())
        .arg("install-local")
        .arg(&source_root)
        .arg("--target")
        .arg("codex")
        .output()
        .unwrap();
    assert_success(&install);
    let installed = sandbox.path().join(".codex/skills/demo");
    let skill_file = installed.join("SKILL.md");
    assert!(skill_file.exists());

    let disable = cli(sandbox.path())
        .arg("disable")
        .arg("demo")
        .arg("--target")
        .arg("codex")
        .output()
        .unwrap();
    assert_success(&disable);

    assert!(skill_file.exists());
    assert!(
        !sandbox
            .path()
            .join(".codex/.skills-disabled/demo/SKILL.md")
            .exists()
    );
    let scan = scan_json(sandbox.path());
    assert_eq!(scan["skills"][0]["scope"], "Codex");
    assert_eq!(scan["skills"][0]["enablement"], "Disabled");
    let codex_config = fs::read_to_string(sandbox.path().join(".codex/config.toml")).unwrap();
    assert!(codex_config.contains(skill_file.to_string_lossy().as_ref()));
    assert!(codex_config.contains("enabled = false"));

    let enable = cli(sandbox.path())
        .arg("enable")
        .arg("demo")
        .arg("--target")
        .arg("codex")
        .output()
        .unwrap();
    assert_success(&enable);
    let scan = scan_json(sandbox.path());
    assert_eq!(scan["skills"][0]["enablement"], "Enabled");
}

#[test]
fn target_accepts_gloab_alias_for_global() {
    let sandbox = tempdir().unwrap();
    let source_root = sandbox.path().join("source");
    write_source_skill(
        &source_root,
        "demo",
        "---\nname: demo\ndescription: Demo skill for global alias\n---\n",
    );

    let output = cli(sandbox.path())
        .arg("install-local")
        .arg(&source_root)
        .arg("--target")
        .arg("gloab")
        .output()
        .unwrap();

    assert_success(&output);
    assert!(sandbox.path().join(".agents/skills/demo/SKILL.md").exists());
}

#[test]
fn resources_scan_can_filter_plugins_json() {
    let sandbox = tempdir().unwrap();
    let source_root = sandbox.path().join("source-plugin");
    write_codex_plugin(&source_root, "demo-plugin", "1.0.0");

    let install = cli(sandbox.path())
        .arg("plugins")
        .arg("install")
        .arg(&source_root)
        .arg("--target")
        .arg("codex")
        .arg("--marketplace")
        .arg("local")
        .output()
        .unwrap();
    assert_success(&install);

    let output = cli(sandbox.path())
        .arg("--output")
        .arg("json-v3")
        .arg("resources")
        .arg("scan")
        .arg("--kind")
        .arg("plugin")
        .output()
        .unwrap();
    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 3);
    assert_eq!(json["command"], "resources");
    assert_eq!(json["data"]["type"], "resources");
    assert_eq!(json["data"]["resources"].as_array().unwrap().len(), 1);
    assert_eq!(json["data"]["resources"][0]["kind"], "Plugin");
    assert_eq!(json["data"]["resources"][0]["target"], "Codex");
    assert_eq!(
        json["data"]["resources"][0]["metadata"]["plugin_id"],
        "demo-plugin@local"
    );
    assert!(json.get("resources").is_none());
}

#[test]
fn plugin_preview_install_and_scan_json_round_trip() {
    let sandbox = tempdir().unwrap();
    let source_root = sandbox.path().join("source-plugin");
    write_codex_plugin(&source_root, "demo-plugin", "1.0.0");

    let preview = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("plugins")
        .arg("preview")
        .arg(&source_root)
        .arg("--target")
        .arg("codex")
        .arg("--marketplace")
        .arg("local")
        .output()
        .unwrap();
    assert_success(&preview);
    let json: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(json["type"], "plugin_preview");
    assert_eq!(json["preview"]["manifest"]["name"], "demo-plugin");
    assert_eq!(json["preview"]["operation_plan"]["marketplace"], "local");
    assert!(
        json["preview"]["operation_plan"]["destination_root"]
            .as_str()
            .unwrap()
            .ends_with(".codex/plugins/cache/local/demo-plugin/1.0.0")
    );

    let install = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("plugins")
        .arg("install")
        .arg(&source_root)
        .arg("--target")
        .arg("codex")
        .arg("--marketplace")
        .arg("local")
        .output()
        .unwrap();
    assert_success(&install);
    let json: Value = serde_json::from_slice(&install.stdout).unwrap();
    assert_eq!(json["type"], "plugin_install");
    assert!(
        sandbox
            .path()
            .join(".codex/plugins/cache/local/demo-plugin/1.0.0/.codex-plugin/plugin.json")
            .exists()
    );

    let scan = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("plugins")
        .arg("scan")
        .arg("--target")
        .arg("codex")
        .output()
        .unwrap();
    assert_success(&scan);
    let json: Value = serde_json::from_slice(&scan.stdout).unwrap();
    assert_eq!(json["type"], "plugins");
    assert_eq!(json["plugins"][0]["display_name"], "Demo Plugin");
    assert_eq!(json["plugins"][0]["enablement"], "Enabled");
}

#[test]
fn marketplace_sources_and_inspect_support_json() {
    let sandbox = tempdir().unwrap();
    let marketplace = sandbox.path().join("marketplace.json");
    write_marketplace(&marketplace);

    let add = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("marketplaces")
        .arg("sources")
        .arg("add")
        .arg("local-market")
        .arg(marketplace.to_string_lossy().as_ref())
        .arg("--target")
        .arg("codex")
        .arg("--provider")
        .arg("file")
        .output()
        .unwrap();
    assert_success(&add);
    let json: Value = serde_json::from_slice(&add.stdout).unwrap();
    assert_eq!(json["type"], "marketplace_source");
    assert_eq!(json["source"]["label"], "local-market");
    assert_eq!(json["source"]["target"], "codex");

    let list = cli(sandbox.path())
        .arg("--output")
        .arg("json")
        .arg("marketplaces")
        .arg("sources")
        .arg("list")
        .output()
        .unwrap();
    assert_success(&list);
    let json: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(json["type"], "marketplace_sources");
    assert_eq!(json["sources"][0]["label"], "local-market");

    let inspect = cli(sandbox.path())
        .arg("--output")
        .arg("json-v3")
        .arg("marketplaces")
        .arg("inspect")
        .arg(marketplace.to_string_lossy().as_ref())
        .arg("--target")
        .arg("codex")
        .output()
        .unwrap();
    assert_success(&inspect);
    let json: Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(json["schema_version"], 3);
    assert_eq!(json["data"]["type"], "marketplace_inspect");
    assert_eq!(json["data"]["marketplace"]["name"], "demo-market");
    assert_eq!(json["data"]["marketplace"]["target"], "Codex");
    assert_eq!(
        json["data"]["marketplace"]["entries"][0]["source"]["source"],
        "git"
    );
    assert!(json.get("marketplace").is_none());
}

fn cli(home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_skills-manager"));
    command.env("HOME", home).env("RUST_LOG", "off");
    command
}

fn scan_json(home: &std::path::Path) -> Value {
    let output = cli(home)
        .arg("--output")
        .arg("json")
        .arg("scan")
        .output()
        .unwrap();
    assert_success(&output);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_skill(project: &std::path::Path, folder: &str, content: &str) {
    let skill_dir = project.join(".agents").join("skills").join(folder);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), content).unwrap();
}

fn write_source_skill(root: &std::path::Path, folder: &str, content: &str) {
    let skill_dir = root.join(folder);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), content).unwrap();
}

fn write_codex_plugin(root: &std::path::Path, name: &str, version: &str) {
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::create_dir_all(root.join("skills/demo")).unwrap();
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        format!(
            r#"{{
  "name": "{name}",
  "version": "{version}",
  "description": "Demo plugin for CLI tests",
  "skills": "./skills/",
  "interface": {{
    "displayName": "Demo Plugin",
    "shortDescription": "Demo plugin for CLI tests",
    "category": "testing"
  }}
}}"#
        ),
    )
    .unwrap();
    fs::write(
        root.join("skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: Demo bundled skill\n---\n",
    )
    .unwrap();
}

fn write_marketplace(path: &std::path::Path) {
    fs::write(
        path,
        r#"{
  "name": "demo-market",
  "interface": {
    "displayName": "Demo Market"
  },
  "plugins": [
    {
      "name": "demo-plugin",
      "description": "Demo plugin listing",
      "version": "1.0.0",
      "category": "testing",
      "source": {
        "source": "git",
        "url": "https://github.com/acme/demo-plugin",
        "path": "plugins/demo",
        "ref": "main"
      },
      "policy": {
        "installation": "preview required",
        "authentication": "none"
      }
    }
  ]
}"#,
    )
    .unwrap();
}
