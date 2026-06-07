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
    assert_eq!(json["type"], "skills");
    assert_eq!(json["skills"][0]["display_name"], "demo");
    assert_eq!(json["skills"][0]["scope"], "Project");
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
