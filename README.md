# skills-manager

A Rust workspace for managing Agent Skills locally.

## Overview

`skills-manager` provides shared core logic, a CLI, and a desktop UI for discovering, validating, installing, exporting, downloading, disabling, and removing Agent Skills.

## Workspace crates

- `crates/skills-manager-core`: shared domain logic
- `crates/skills-manager-cli`: command-line interface
- `crates/skills-manager-ui`: desktop UI built with `iced` and `iced_aw`
