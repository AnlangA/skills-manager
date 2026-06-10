//! MCP server discovery and configuration management.
//!
//! This module normalizes MCP server entries from multiple agent config
//! formats into [`ManagedResource`] records and provides mutation helpers for
//! adding, removing, and toggling servers.

use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use toml_edit::{Array, DocumentMut, Item, Table};

use crate::{
    AgentToolTarget, ManagedResource, ManagerPaths, ResourceHealth, ResourceKind, Result,
    SkillDiagnostic, SkillEnablement, SkillsManagerError, fs_ops::atomic_write,
};

const MCP_SERVERS_KEY: &str = "mcpServers";
const OPENCODE_MCP_KEY: &str = "mcp";
const ZED_CONTEXT_SERVERS_KEY: &str = "context_servers";

/// Transport type for an MCP server declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpServerTransport {
    /// Local process started with a command and arguments.
    Stdio,
    /// Remote HTTP/SSE-style MCP endpoint.
    Http,
}

impl McpServerTransport {
    /// UI-friendly list of selectable transport types.
    pub const ALL: [Self; 2] = [Self::Stdio, Self::Http];

    /// Lowercase display label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
        }
    }
}

impl fmt::Display for McpServerTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Stdio => "Local command",
            Self::Http => "Remote URL",
        })
    }
}

/// Request for adding or replacing an MCP server entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRequest {
    /// Server name/key in the target configuration.
    pub name: String,
    /// Target agent tool.
    pub target: AgentToolTarget,
    /// Server transport.
    pub transport: McpServerTransport,
    /// Local command for stdio servers.
    pub command: Option<String>,
    /// Command arguments for stdio servers.
    pub args: Vec<String>,
    /// Environment variables for stdio servers.
    pub env: BTreeMap<String, String>,
    /// Remote endpoint URL for HTTP servers.
    pub url: Option<String>,
    /// Headers for HTTP servers.
    pub headers: BTreeMap<String, String>,
    /// Whether the server should be active.
    pub enabled: bool,
}

#[derive(Debug, Clone)]
struct McpServerEntry {
    name: String,
    target: AgentToolTarget,
    config_file: PathBuf,
    transport: McpServerTransport,
    command: Option<String>,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    url: Option<String>,
    headers: BTreeMap<String, String>,
    enabled: bool,
    diagnostics: Vec<SkillDiagnostic>,
}

/// Scan all supported MCP server configuration locations.
pub fn scan_mcp_servers(paths: &ManagerPaths) -> Result<Vec<ManagedResource>> {
    let mut servers = Vec::new();

    if let Some(project_mcp) = paths.project_mcp_config_file() {
        servers.extend(scan_json_mcp_file(
            &project_mcp,
            AgentToolTarget::ClaudeCode,
            MCP_SERVERS_KEY,
        )?);
    }
    servers.extend(scan_json_mcp_file(
        &paths.claude_config_file(),
        AgentToolTarget::ClaudeCode,
        MCP_SERVERS_KEY,
    )?);
    servers.extend(scan_codex_mcp_file(&paths.codex_config_file())?);
    servers.extend(scan_json_mcp_file(
        &paths.droid_mcp_config_file(),
        AgentToolTarget::Droid,
        MCP_SERVERS_KEY,
    )?);
    servers.extend(scan_json_mcp_file(
        &opencode_read_config_file(paths),
        AgentToolTarget::OpenCode,
        OPENCODE_MCP_KEY,
    )?);
    servers.extend(scan_json_mcp_file(
        &paths.zed_settings_file(),
        AgentToolTarget::Zed,
        ZED_CONTEXT_SERVERS_KEY,
    )?);

    servers.sort_by(|left, right| {
        left.target
            .cmp(&right.target)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.config_file.cmp(&right.config_file))
    });

    Ok(servers.into_iter().map(mcp_resource).collect())
}

/// Add or replace an MCP server entry in the target configuration.
pub fn add_mcp_server(paths: &ManagerPaths, request: McpServerRequest) -> Result<ManagedResource> {
    validate_mcp_request(&request)?;

    match request.target {
        AgentToolTarget::ClaudeCode => {
            write_common_mcp_server(&claude_write_config_file(paths), MCP_SERVERS_KEY, &request)?
        }
        AgentToolTarget::Codex => write_codex_mcp_server(&paths.codex_config_file(), &request)?,
        AgentToolTarget::Droid => {
            write_common_mcp_server(&paths.droid_mcp_config_file(), MCP_SERVERS_KEY, &request)?
        }
        AgentToolTarget::OpenCode => {
            write_opencode_mcp_server(&paths.opencode_config_file(), &request)?
        }
        AgentToolTarget::Zed => write_zed_mcp_server(&paths.zed_settings_file(), &request)?,
        AgentToolTarget::Generic => {
            return Err(SkillsManagerError::InvalidResource(
                "generic MCP target is read-only".to_string(),
            ));
        }
    }

    scan_mcp_servers(paths)?
        .into_iter()
        .find(|server| server.target == request.target && server.display_name == request.name)
        .ok_or_else(|| {
            SkillsManagerError::InvalidResource(format!(
                "MCP server was written but not discovered: {}",
                request.name
            ))
        })
}

/// Enable or disable an MCP server in its owning target configuration.
pub fn set_mcp_server_enabled(
    paths: &ManagerPaths,
    target: AgentToolTarget,
    name: &str,
    enabled: bool,
) -> Result<()> {
    let server = find_mcp_server(paths, target, name)?;
    match target {
        AgentToolTarget::ClaudeCode | AgentToolTarget::Droid => {
            set_json_mcp_enabled(&server.config_file, MCP_SERVERS_KEY, name, enabled)
        }
        AgentToolTarget::Codex => set_codex_mcp_enabled(&server.config_file, name, enabled),
        AgentToolTarget::OpenCode => {
            set_json_mcp_enabled(&server.config_file, OPENCODE_MCP_KEY, name, enabled)
        }
        AgentToolTarget::Zed => {
            set_json_mcp_enabled(&server.config_file, ZED_CONTEXT_SERVERS_KEY, name, enabled)
        }
        AgentToolTarget::Generic => Err(SkillsManagerError::InvalidResource(
            "generic MCP target is read-only".to_string(),
        )),
    }
}

/// Remove an MCP server entry and return the backup file created before mutation.
pub fn remove_mcp_server(
    paths: &ManagerPaths,
    target: AgentToolTarget,
    name: &str,
) -> Result<PathBuf> {
    let server = find_mcp_server(paths, target, name)?;
    match target {
        AgentToolTarget::ClaudeCode | AgentToolTarget::Droid => {
            remove_json_mcp_server(&server.config_file, MCP_SERVERS_KEY, name)
        }
        AgentToolTarget::Codex => remove_codex_mcp_server(&server.config_file, name),
        AgentToolTarget::OpenCode => {
            remove_json_mcp_server(&server.config_file, OPENCODE_MCP_KEY, name)
        }
        AgentToolTarget::Zed => {
            remove_json_mcp_server(&server.config_file, ZED_CONTEXT_SERVERS_KEY, name)
        }
        AgentToolTarget::Generic => Err(SkillsManagerError::InvalidResource(
            "generic MCP target is read-only".to_string(),
        )),
    }
}

fn validate_mcp_request(request: &McpServerRequest) -> Result<()> {
    validate_mcp_name(&request.name)?;
    match request.transport {
        McpServerTransport::Stdio => {
            if request
                .command
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(SkillsManagerError::InvalidResource(
                    "local MCP servers require a command".to_string(),
                ));
            }
        }
        McpServerTransport::Http => {
            let url = request.url.as_deref().unwrap_or_default().trim();
            if url.is_empty() {
                return Err(SkillsManagerError::InvalidResource(
                    "remote MCP servers require a URL".to_string(),
                ));
            }
            let parsed = url::Url::parse(url)?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err(SkillsManagerError::InvalidUrl(
                    "MCP server URL must use http or https".to_string(),
                ));
            }
        }
    }
    if request.target == AgentToolTarget::Generic {
        return Err(SkillsManagerError::InvalidResource(
            "select a concrete MCP target".to_string(),
        ));
    }
    Ok(())
}

fn validate_mcp_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SkillsManagerError::InvalidResource(
            "MCP server name is required".to_string(),
        ));
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(SkillsManagerError::InvalidResource(format!(
            "MCP server name contains invalid characters: {name}"
        )));
    }
    Ok(())
}

fn find_mcp_server(
    paths: &ManagerPaths,
    target: AgentToolTarget,
    name: &str,
) -> Result<McpServerEntry> {
    scan_mcp_entries(paths)?
        .into_iter()
        .find(|server| server.target == target && server.name == name)
        .ok_or_else(|| {
            SkillsManagerError::InvalidResource(format!(
                "unknown MCP server `{name}` for {}",
                target.label()
            ))
        })
}

fn scan_mcp_entries(paths: &ManagerPaths) -> Result<Vec<McpServerEntry>> {
    let mut entries = Vec::new();
    if let Some(project_mcp) = paths.project_mcp_config_file() {
        entries.extend(scan_json_mcp_file(
            &project_mcp,
            AgentToolTarget::ClaudeCode,
            MCP_SERVERS_KEY,
        )?);
    }
    entries.extend(scan_json_mcp_file(
        &paths.claude_config_file(),
        AgentToolTarget::ClaudeCode,
        MCP_SERVERS_KEY,
    )?);
    entries.extend(scan_codex_mcp_file(&paths.codex_config_file())?);
    entries.extend(scan_json_mcp_file(
        &paths.droid_mcp_config_file(),
        AgentToolTarget::Droid,
        MCP_SERVERS_KEY,
    )?);
    entries.extend(scan_json_mcp_file(
        &opencode_read_config_file(paths),
        AgentToolTarget::OpenCode,
        OPENCODE_MCP_KEY,
    )?);
    entries.extend(scan_json_mcp_file(
        &paths.zed_settings_file(),
        AgentToolTarget::Zed,
        ZED_CONTEXT_SERVERS_KEY,
    )?);
    Ok(entries)
}

fn scan_json_mcp_file(
    path: &Path,
    target: AgentToolTarget,
    key: &str,
) -> Result<Vec<McpServerEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let value = read_json_document(path)?;
    let Some(map) = value.get(key).and_then(Value::as_object) else {
        return Ok(Vec::new());
    };

    Ok(map
        .iter()
        .map(|(name, value)| mcp_entry_from_json(name, target, path, value))
        .collect())
}

fn scan_codex_mcp_file(path: &Path) -> Result<Vec<McpServerEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)?;
    let document = raw
        .parse::<DocumentMut>()
        .map_err(|source| SkillsManagerError::ParseToml {
            path: path.to_path_buf(),
            source,
        })?;
    let Some(table) = document.get("mcp_servers").and_then(Item::as_table) else {
        return Ok(Vec::new());
    };

    Ok(table
        .iter()
        .filter_map(|(name, item)| {
            item.as_table()
                .map(|server| mcp_entry_from_toml(name, path, server))
        })
        .collect())
}

fn mcp_entry_from_json(
    name: &str,
    target: AgentToolTarget,
    config_file: &Path,
    value: &Value,
) -> McpServerEntry {
    let mut diagnostics = Vec::new();
    let enabled = json_enabled(value);
    let (command, args, env) = json_command(value);
    let url = value
        .get("url")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let headers = json_string_map(value.get("headers"));
    let transport = if url.is_some() && command.is_none() {
        McpServerTransport::Http
    } else {
        McpServerTransport::Stdio
    };

    push_mcp_diagnostics(
        transport,
        command.as_deref(),
        url.as_deref(),
        &mut diagnostics,
    );

    McpServerEntry {
        name: name.to_string(),
        target,
        config_file: config_file.to_path_buf(),
        transport,
        command,
        args,
        env,
        url,
        headers,
        enabled,
        diagnostics,
    }
}

fn mcp_entry_from_toml(name: &str, config_file: &Path, table: &Table) -> McpServerEntry {
    let command = table
        .get("command")
        .and_then(Item::as_value)
        .and_then(toml_edit::Value::as_str)
        .map(ToString::to_string);
    let args = table
        .get("args")
        .and_then(Item::as_value)
        .and_then(toml_edit::Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(toml_edit::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    let env = toml_string_table(table.get("env"));
    let url = table
        .get("url")
        .and_then(Item::as_value)
        .and_then(toml_edit::Value::as_str)
        .map(ToString::to_string);
    let headers = toml_string_table(table.get("headers"));
    let enabled = table
        .get("enabled")
        .and_then(Item::as_value)
        .and_then(toml_edit::Value::as_bool)
        .unwrap_or(true);
    let transport = if url.is_some() && command.is_none() {
        McpServerTransport::Http
    } else {
        McpServerTransport::Stdio
    };
    let mut diagnostics = Vec::new();
    push_mcp_diagnostics(
        transport,
        command.as_deref(),
        url.as_deref(),
        &mut diagnostics,
    );

    McpServerEntry {
        name: name.to_string(),
        target: AgentToolTarget::Codex,
        config_file: config_file.to_path_buf(),
        transport,
        command,
        args,
        env,
        url,
        headers,
        enabled,
        diagnostics,
    }
}

fn push_mcp_diagnostics(
    transport: McpServerTransport,
    command: Option<&str>,
    url: Option<&str>,
    diagnostics: &mut Vec<SkillDiagnostic>,
) {
    match transport {
        McpServerTransport::Stdio if command.unwrap_or_default().trim().is_empty() => {
            diagnostics.push(SkillDiagnostic::invalid(
                "local MCP server is missing a command",
            ));
        }
        McpServerTransport::Http if url.unwrap_or_default().trim().is_empty() => {
            diagnostics.push(SkillDiagnostic::invalid(
                "remote MCP server is missing a URL",
            ));
        }
        McpServerTransport::Http => {
            if let Some(url) = url
                && url::Url::parse(url).is_err()
            {
                diagnostics.push(SkillDiagnostic::warning(
                    "remote MCP server URL could not be parsed",
                ));
            }
        }
        McpServerTransport::Stdio => {}
    }
}

fn json_enabled(value: &Value) -> bool {
    if let Some(enabled) = value.get("enabled").and_then(Value::as_bool) {
        return enabled;
    }
    if let Some(disabled) = value.get("disabled").and_then(Value::as_bool) {
        return !disabled;
    }
    true
}

fn json_command(value: &Value) -> (Option<String>, Vec<String>, BTreeMap<String, String>) {
    let env = json_string_map(
        value
            .get("env")
            .or_else(|| value.get("environment"))
            .or_else(|| value.get("envs")),
    );
    let Some(command) = value.get("command") else {
        return (None, Vec::new(), env);
    };

    match command {
        Value::String(command) => (
            Some(command.clone()),
            json_string_array(value.get("args")),
            env,
        ),
        Value::Array(items) => {
            let mut parts = items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if parts.is_empty() {
                (None, Vec::new(), env)
            } else {
                let command = parts.remove(0);
                (Some(command), parts, env)
            }
        }
        Value::Object(map) => {
            let command = map
                .get("path")
                .or_else(|| map.get("command"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let args = json_string_array(map.get("args"));
            let env = json_string_map(map.get("env").or_else(|| map.get("environment")));
            (command, args, env)
        }
        _ => (None, Vec::new(), env),
    }
}

fn json_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn json_string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value
                            .as_str()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| value.to_string()),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn toml_string_table(item: Option<&Item>) -> BTreeMap<String, String> {
    item.and_then(Item::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(key, item)| {
                    item.as_value()
                        .and_then(toml_edit::Value::as_str)
                        .map(|value| (key.to_string(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn mcp_resource(server: McpServerEntry) -> ManagedResource {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "transport".to_string(),
        server.transport.label().to_string(),
    );
    metadata.insert(
        "config".to_string(),
        server.config_file.display().to_string(),
    );
    if let Some(command) = &server.command {
        metadata.insert("command".to_string(), command.clone());
    }
    if !server.args.is_empty() {
        metadata.insert("args".to_string(), server.args.join(" "));
    }
    if let Some(url) = &server.url {
        metadata.insert("url".to_string(), url.clone());
    }
    metadata.insert("env".to_string(), format!("{} entrie(s)", server.env.len()));
    metadata.insert(
        "headers".to_string(),
        format!("{} entrie(s)", server.headers.len()),
    );

    let description = match server.transport {
        McpServerTransport::Stdio => server
            .command
            .as_ref()
            .map(|command| format!("Local MCP command: {command}")),
        McpServerTransport::Http => server
            .url
            .as_ref()
            .map(|url| format!("Remote MCP endpoint: {url}")),
    };

    ManagedResource {
        id: format!("mcp:{}:{}", server.target.id_prefix(), server.name),
        kind: ResourceKind::McpServer,
        target: server.target,
        display_name: server.name,
        description,
        root_dir: server
            .config_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| server.config_file.clone()),
        manifest_file: Some(server.config_file),
        enablement: if server.enabled {
            SkillEnablement::Enabled
        } else {
            SkillEnablement::Disabled
        },
        health: ResourceHealth::from_diagnostics(&server.diagnostics),
        diagnostics: server.diagnostics,
        source_url: server.url,
        installed_at: None,
        metadata,
    }
}

fn write_common_mcp_server(path: &Path, key: &str, request: &McpServerRequest) -> Result<()> {
    let mut document = read_json_or_empty(path)?;
    let root = ensure_json_object(&mut document);
    let servers = ensure_json_object_field(root, key);
    servers.insert(
        request.name.trim().to_string(),
        common_json_server_value(request),
    );
    write_json_with_backup(path, &document)
}

fn write_opencode_mcp_server(path: &Path, request: &McpServerRequest) -> Result<()> {
    let mut document = read_json_or_empty(path)?;
    let root = ensure_json_object(&mut document);
    let servers = ensure_json_object_field(root, OPENCODE_MCP_KEY);
    servers.insert(
        request.name.trim().to_string(),
        opencode_json_server_value(request),
    );
    write_json_with_backup(path, &document)
}

fn write_zed_mcp_server(path: &Path, request: &McpServerRequest) -> Result<()> {
    let mut document = read_json_or_empty(path)?;
    let root = ensure_json_object(&mut document);
    let servers = ensure_json_object_field(root, ZED_CONTEXT_SERVERS_KEY);
    servers.insert(
        request.name.trim().to_string(),
        zed_json_server_value(request),
    );
    write_json_with_backup(path, &document)
}

fn common_json_server_value(request: &McpServerRequest) -> Value {
    let mut map = Map::new();
    map.insert("enabled".to_string(), Value::Bool(request.enabled));
    match request.transport {
        McpServerTransport::Stdio => {
            map.insert(
                "command".to_string(),
                Value::String(request.command.clone().unwrap_or_default()),
            );
            map.insert("args".to_string(), string_array(&request.args));
            if !request.env.is_empty() {
                map.insert("env".to_string(), string_map(&request.env));
            }
        }
        McpServerTransport::Http => {
            map.insert(
                "url".to_string(),
                Value::String(request.url.clone().unwrap_or_default()),
            );
            if !request.headers.is_empty() {
                map.insert("headers".to_string(), string_map(&request.headers));
            }
        }
    }
    Value::Object(map)
}

fn opencode_json_server_value(request: &McpServerRequest) -> Value {
    let mut map = Map::new();
    map.insert("enabled".to_string(), Value::Bool(request.enabled));
    match request.transport {
        McpServerTransport::Stdio => {
            map.insert("type".to_string(), Value::String("local".to_string()));
            let mut command = vec![request.command.clone().unwrap_or_default()];
            command.extend(request.args.clone());
            map.insert("command".to_string(), string_array(&command));
            if !request.env.is_empty() {
                map.insert("environment".to_string(), string_map(&request.env));
            }
        }
        McpServerTransport::Http => {
            map.insert("type".to_string(), Value::String("remote".to_string()));
            map.insert(
                "url".to_string(),
                Value::String(request.url.clone().unwrap_or_default()),
            );
            if !request.headers.is_empty() {
                map.insert("headers".to_string(), string_map(&request.headers));
            }
        }
    }
    Value::Object(map)
}

fn zed_json_server_value(request: &McpServerRequest) -> Value {
    let mut map = Map::new();
    map.insert("enabled".to_string(), Value::Bool(request.enabled));
    match request.transport {
        McpServerTransport::Stdio => {
            let mut command = Map::new();
            command.insert(
                "path".to_string(),
                Value::String(request.command.clone().unwrap_or_default()),
            );
            command.insert("args".to_string(), string_array(&request.args));
            if !request.env.is_empty() {
                command.insert("env".to_string(), string_map(&request.env));
            }
            map.insert("command".to_string(), Value::Object(command));
        }
        McpServerTransport::Http => {
            map.insert(
                "url".to_string(),
                Value::String(request.url.clone().unwrap_or_default()),
            );
            if !request.headers.is_empty() {
                map.insert("headers".to_string(), string_map(&request.headers));
            }
        }
    }
    Value::Object(map)
}

fn set_json_mcp_enabled(path: &Path, key: &str, name: &str, enabled: bool) -> Result<()> {
    let mut document = read_json_document(path)?;
    let Some(server) = document
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .and_then(|servers| servers.get_mut(name))
        .and_then(Value::as_object_mut)
    else {
        return Err(SkillsManagerError::InvalidResource(format!(
            "MCP server not found: {name}"
        )));
    };
    server.insert("enabled".to_string(), Value::Bool(enabled));
    server.remove("disabled");
    write_json_with_backup(path, &document)
}

fn remove_json_mcp_server(path: &Path, key: &str, name: &str) -> Result<PathBuf> {
    let mut document = read_json_document(path)?;
    let Some(servers) = document.get_mut(key).and_then(Value::as_object_mut) else {
        return Err(SkillsManagerError::InvalidResource(format!(
            "MCP server section not found: {key}"
        )));
    };
    if servers.remove(name).is_none() {
        return Err(SkillsManagerError::InvalidResource(format!(
            "MCP server not found: {name}"
        )));
    }
    let backup = backup_config_file(path)?;
    write_json_document(path, &document)?;
    Ok(backup)
}

fn write_codex_mcp_server(path: &Path, request: &McpServerRequest) -> Result<()> {
    let mut document = read_toml_or_empty(path)?;
    let servers = ensure_toml_table(&mut document, "mcp_servers");
    let mut table = Table::new();
    table["enabled"] = toml_edit::value(request.enabled);
    match request.transport {
        McpServerTransport::Stdio => {
            table["command"] = toml_edit::value(request.command.clone().unwrap_or_default());
            table["args"] = Item::Value(string_toml_array(&request.args));
            if !request.env.is_empty() {
                table["env"] = Item::Table(string_toml_table(&request.env));
            }
        }
        McpServerTransport::Http => {
            table["url"] = toml_edit::value(request.url.clone().unwrap_or_default());
            if !request.headers.is_empty() {
                table["headers"] = Item::Table(string_toml_table(&request.headers));
            }
        }
    }
    servers[request.name.trim()] = Item::Table(table);
    write_toml_with_backup(path, &document)
}

fn set_codex_mcp_enabled(path: &Path, name: &str, enabled: bool) -> Result<()> {
    let mut document = read_toml_or_empty(path)?;
    let Some(server) = document
        .get_mut("mcp_servers")
        .and_then(|item| item.get_mut(name))
        .and_then(Item::as_table_mut)
    else {
        return Err(SkillsManagerError::InvalidResource(format!(
            "MCP server not found: {name}"
        )));
    };
    server["enabled"] = toml_edit::value(enabled);
    write_toml_with_backup(path, &document)
}

fn remove_codex_mcp_server(path: &Path, name: &str) -> Result<PathBuf> {
    let mut document = read_toml_or_empty(path)?;
    let Some(servers) = document.get_mut("mcp_servers").and_then(Item::as_table_mut) else {
        return Err(SkillsManagerError::InvalidResource(
            "Codex MCP section not found".to_string(),
        ));
    };
    if servers.remove(name).is_none() {
        return Err(SkillsManagerError::InvalidResource(format!(
            "MCP server not found: {name}"
        )));
    }
    let backup = backup_config_file(path)?;
    write_toml_document(path, &document)?;
    Ok(backup)
}

fn ensure_json_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("value is object")
}

fn ensure_json_object_field<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> &'a mut Map<String, Value> {
    if !root.get(key).is_some_and(Value::is_object) {
        root.insert(key.to_string(), Value::Object(Map::new()));
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("field is object")
}

fn ensure_toml_table<'a>(document: &'a mut DocumentMut, key: &str) -> &'a mut Table {
    if !document.as_table().contains_key(key) {
        document[key] = Item::Table(Table::new());
    }
    document[key].as_table_mut().expect("item is table")
}

fn read_json_or_empty(path: &Path) -> Result<Value> {
    if path.exists() {
        read_json_document(path)
    } else {
        Ok(Value::Object(Map::new()))
    }
}

fn read_json_document(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw)
        .or_else(|_| serde_json::from_str(&strip_json_comments(&raw)))
        .map_err(SkillsManagerError::Json)
}

fn read_toml_or_empty(path: &Path) -> Result<DocumentMut> {
    if !path.exists() {
        return Ok(DocumentMut::new());
    }
    let raw = fs::read_to_string(path)?;
    raw.parse::<DocumentMut>()
        .map_err(|source| SkillsManagerError::ParseToml {
            path: path.to_path_buf(),
            source,
        })
}

fn write_json_with_backup(path: &Path, value: &Value) -> Result<()> {
    if path.exists() {
        backup_config_file(path)?;
    }
    write_json_document(path, value)
}

fn write_json_document(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(value)?;
    atomic_write(path, format!("{raw}\n"))?;
    Ok(())
}

fn write_toml_with_backup(path: &Path, document: &DocumentMut) -> Result<()> {
    if path.exists() {
        backup_config_file(path)?;
    }
    write_toml_document(path, document)
}

fn write_toml_document(path: &Path, document: &DocumentMut) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, document.to_string())?;
    Ok(())
}

fn backup_config_file(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let backup = path.with_file_name(format!("{file_name}.backup-{timestamp}"));
    fs::copy(path, &backup)?;
    Ok(backup)
}

fn string_array(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

fn string_map(values: &BTreeMap<String, String>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect(),
    )
}

fn string_toml_table(values: &BTreeMap<String, String>) -> Table {
    let mut table = Table::new();
    for (key, value) in values {
        table[key] = toml_edit::value(value.clone());
    }
    table
}

fn string_toml_array(values: &[String]) -> toml_edit::Value {
    let mut array = Array::new();
    for value in values {
        array.push(value.clone());
    }
    toml_edit::Value::Array(array)
}

fn claude_write_config_file(paths: &ManagerPaths) -> PathBuf {
    paths
        .project_mcp_config_file()
        .unwrap_or_else(|| paths.claude_config_file())
}

fn opencode_read_config_file(paths: &ManagerPaths) -> PathBuf {
    let json = paths.opencode_config_file();
    if json.exists() {
        json
    } else {
        paths.opencode_jsonc_config_file()
    }
}

fn strip_json_comments(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if in_string {
            result.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            result.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                    continue;
                }
                _ => {}
            }
        }

        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{ManagerPaths, ProjectRoot};

    use super::*;

    #[test]
    fn scans_mcp_servers_from_all_supported_targets() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        let paths = test_paths(dir.path(), Some(&project));
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join(".mcp.json"),
            r#"{"mcpServers":{"claude-demo":{"command":"node","args":["server.js"]}}}"#,
        )
        .unwrap();
        fs::create_dir_all(paths.codex_config_file().parent().unwrap()).unwrap();
        fs::write(
            paths.codex_config_file(),
            "[mcp_servers.codex_demo]\ncommand = \"node\"\nargs = [\"server.js\"]\n",
        )
        .unwrap();
        fs::create_dir_all(paths.droid_mcp_config_file().parent().unwrap()).unwrap();
        fs::write(
            paths.droid_mcp_config_file(),
            r#"{"mcpServers":{"droid-demo":{"url":"https://example.com/mcp"}}}"#,
        )
        .unwrap();
        fs::create_dir_all(paths.opencode_config_file().parent().unwrap()).unwrap();
        fs::write(
            paths.opencode_config_file(),
            r#"{"mcp":{"opencode-demo":{"command":["node","server.js"],"enabled":false}}}"#,
        )
        .unwrap();
        fs::create_dir_all(paths.zed_settings_file().parent().unwrap()).unwrap();
        fs::write(
            paths.zed_settings_file(),
            r#"{"context_servers":{"zed-demo":{"command":{"path":"node","args":["server.js"]}}}}"#,
        )
        .unwrap();

        let servers = scan_mcp_servers(&paths).unwrap();

        assert_eq!(servers.len(), 5);
        assert!(servers.iter().any(|server| {
            server.target == AgentToolTarget::ClaudeCode && server.display_name == "claude-demo"
        }));
        assert!(servers.iter().any(|server| {
            server.target == AgentToolTarget::OpenCode
                && server.display_name == "opencode-demo"
                && server.enablement == SkillEnablement::Disabled
        }));
    }

    #[test]
    fn add_toggle_and_remove_codex_mcp_server() {
        let dir = tempdir().unwrap();
        let paths = test_paths(dir.path(), None);

        let added = add_mcp_server(
            &paths,
            McpServerRequest {
                name: "demo".to_string(),
                target: AgentToolTarget::Codex,
                transport: McpServerTransport::Stdio,
                command: Some("node".to_string()),
                args: vec!["server.js".to_string()],
                env: BTreeMap::from([("TOKEN".to_string(), "secret".to_string())]),
                url: None,
                headers: BTreeMap::new(),
                enabled: true,
            },
        )
        .unwrap();

        assert_eq!(added.kind, ResourceKind::McpServer);
        assert_eq!(added.display_name, "demo");
        assert!(paths.codex_config_file().exists());

        set_mcp_server_enabled(&paths, AgentToolTarget::Codex, "demo", false).unwrap();
        let disabled = scan_mcp_servers(&paths)
            .unwrap()
            .into_iter()
            .find(|server| server.display_name == "demo")
            .unwrap();
        assert_eq!(disabled.enablement, SkillEnablement::Disabled);

        let backup = remove_mcp_server(&paths, AgentToolTarget::Codex, "demo").unwrap();
        assert!(backup.exists());
        assert!(scan_mcp_servers(&paths).unwrap().is_empty());
    }

    fn test_paths(root: &std::path::Path, project: Option<&std::path::Path>) -> ManagerPaths {
        ManagerPaths::with_home(
            root.join("home"),
            root.join("data"),
            root.join("config"),
            project.map(ProjectRoot::new),
        )
    }
}
