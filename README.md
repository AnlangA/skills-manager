# skills-manager

Rust workspace for managing local Agent Skills with a CLI and desktop UI.

`skills-manager` helps you discover, validate, install, export, enable, disable, and remove skills across multiple local targets. The repository is organized as a workspace so the core skill-management logic can be shared by both the CLI and the desktop application.

## What it does

- Discover installed skills in user, project, and tool-specific locations
- Validate skill structure and surface diagnostics
- Install skills from local paths or remote URLs
- Enable, disable, remove, and repair installed skills
- Export installed skills as catalogs in JSON, XML, or Markdown
- Cache and manage downloaded skills before installation
- Generate new skill scaffolds from the CLI

## Workspace layout

- `crates/skills-manager-core` - shared domain logic for discovery, validation, install flows, catalogs, downloads, and removal
- `crates/skills-manager-cli` - command-line interface built on the core crate
- `crates/skills-manager-ui` - desktop UI built with `iced` and `iced_aw`

## Supported targets

The CLI currently exposes these built-in targets:

- `global`
- `project`
- `claude-code`
- `droid`
- `pencode`
- `codex`
- `zed`

Target behavior is not identical across tools. For example, Codex uses a config-toggle enablement strategy, while most other targets use directory moves.

## CLI overview

Top-level commands include:

- `workspace`
- `inventory`
- `install`
- `scan`
- `validate`
- `targets`
- `doctor`
- `repair`
- `create`
- `preview-install`
- `install-url`
- `install-local`
- `downloads`
- `catalog`
- `load-catalog`
- `enable`
- `disable`
- `remove`

Machine-readable output is available via:

- `--output text`
- `--output json`
- `--output json-v3`

## Quick start

### Build and inspect the CLI

```bash
cargo run -p skills-manager-cli -- --help
```

### Show available targets

```bash
cargo run -p skills-manager-cli -- targets
```

### Scan and validate installed skills

```bash
cargo run -p skills-manager-cli -- scan
cargo run -p skills-manager-cli -- validate
```

### Install a skill

From a remote URL:

```bash
cargo run -p skills-manager-cli -- install url <URL>
```

From a local directory:

```bash
cargo run -p skills-manager-cli -- install local <PATH>
```

### Export a catalog

```bash
cargo run -p skills-manager-cli -- catalog --format markdown
```

### Create a new skill scaffold

```bash
cargo run -p skills-manager-cli -- create \
  --name my-skill \
  --description "Short skill description"
```

### Run the desktop UI

```bash
cargo run -p skills-manager-ui
```

## Development

Run these from the repository root:

```bash
cargo fmt --all
cargo check
cargo test
cargo test -p skills-manager-core
```

## Design notes

- Business logic should live in `skills-manager-core`
- The CLI and UI should stay thin over the shared core crate
- Install, replace, disable, and remove flows should remain explicit and reversible where possible
- User scope and project scope should stay distinct

## License

MIT
