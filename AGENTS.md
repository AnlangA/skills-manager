# AGENTS.md

This file provides guidance for coding agents working in this repository.

## Project overview

`skills-manager` is a Rust workspace for managing Agent Skills locally.

Workspace members:

- `crates/skills-manager-core`: shared domain logic for discovering, validating, installing, exporting, downloading, and removing skills.
- `crates/skills-manager-cli`: CLI entrypoint built on top of the core crate.
- `crates/skills-manager-ui`: desktop UI built with `iced` and `iced_aw`, also built on top of the core crate.

## Architecture guidelines

- Prefer putting business logic in `skills-manager-core`.
- Keep the CLI and UI as thin integration layers over the core crate.
- Reuse existing workspace dependencies from the root `Cargo.toml` when possible.
- Avoid adding new dependencies unless they are clearly justified.
- Preserve the distinction between user scope and project scope when changing install or discovery behavior.
- Be careful with destructive operations: install, replace, disable, and remove flows should remain explicit and reversible where possible.

## UI structure

The UI crate is organized by responsibility:

- `src/app.rs`: state, messages, and update flow
- `src/views.rs`: screen composition
- `src/components.rs`: reusable UI pieces
- `src/tasks.rs`: async/background task integration
- `src/theme.rs`: styling and theme definitions
- `src/icons.rs`: icon helpers

When changing UI behavior, prefer keeping state transitions in `app.rs` and rendering concerns in `views.rs` / `components.rs`.

## Common commands

Run these from the repository root:

- `cargo fmt --all`
- `cargo check`
- `cargo test`
- `cargo test -p skills-manager-core`
- `cargo run -p skills-manager-cli -- --help`
- `cargo run -p skills-manager-ui`

## Change guidelines

- If you change shared models or exported APIs in `skills-manager-core`, update both CLI and UI call sites as needed.
- If you add new install or catalog behavior, make sure both success and error states stay user-readable.
- Keep changes minimal and consistent with the existing code style.
- Prefer explicit error propagation over silent fallback behavior.

## Repository hygiene

- Do not commit `target/`.
- Do not rely on `.agents/` contents being present in the repository; that directory is ignored locally.
- Keep generated or machine-specific files out of version control unless they are intentionally tracked.

## Validation expectations

- For documentation-only changes, no build is required.
- For core logic changes, run at least `cargo test -p skills-manager-core`.
- For CLI/UI changes, run at least `cargo check` and, when practical, the relevant target with `cargo run`.
