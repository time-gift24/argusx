use argus_core::Builtin;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use crate::catalog::EffectiveToolPolicy;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentToolConfig {
    #[serde(default)]
    pub tools: ToolConfigSection,
    #[serde(default)]
    pub mcp: McpConfigSection,
}

impl AgentToolConfig {
    pub fn parse_and_validate(raw: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(raw)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_builtin_names(&self.tools.builtin_tools)?;
        validate_tool_policy("tools.defaults", &self.tools.defaults)?;

        let enabled: BTreeSet<&str> = self
            .tools
            .builtin_tools
            .iter()
            .map(String::as_str)
            .collect();

        for (name, config) in &self.tools.builtin {
            if Builtin::from_name(name).is_none() {
                return Err(ConfigError::UnknownBuiltin(name.clone()));
            }
            if !enabled.contains(name.as_str()) {
                return Err(ConfigError::OverrideForDisabledBuiltin(name.clone()));
            }
            validate_tool_policy(&format!("tools.builtin.{name}"), config)?;
        }

        validate_tool_policy("mcp.defaults", &self.mcp.defaults)?;
        for (label, server) in &self.mcp.server {
            validate_mcp_server_policy(&format!("mcp.server.{label}"), server)?;
        }

        Ok(())
    }

    pub fn effective_builtin_policy(&self, builtin: Builtin) -> Option<EffectiveToolPolicy> {
        let name = builtin.canonical_name();
        if !self
            .tools
            .builtin_tools
            .iter()
            .any(|enabled| enabled == name)
        {
            return None;
        }

        Some(resolve_tool_policy(
            &self.tools.defaults,
            self.tools.builtin.get(name),
        ))
    }

    pub fn effective_mcp_policy(&self, server_label: &str) -> Option<EffectiveToolPolicy> {
        let server = self.mcp.server.get(server_label)?;
        if !server.enabled {
            return None;
        }

        Some(resolve_mcp_policy(&self.mcp.defaults, server))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolConfigSection {
    #[serde(default)]
    pub builtin_tools: Vec<String>,
    #[serde(default)]
    pub defaults: ToolPolicyConfig,
    #[serde(default)]
    pub builtin: BTreeMap<String, ToolPolicyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct McpConfigSection {
    #[serde(default)]
    pub defaults: ToolPolicyConfig,
    #[serde(default)]
    pub server: BTreeMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolPolicyConfig {
    #[serde(default)]
    pub allow_parallel: Option<bool>,
    #[serde(default)]
    pub max_concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct McpServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub transport: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub allow_parallel: Option<bool>,
    #[serde(default)]
    pub max_concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid config: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unknown builtin tool: {0}")]
    UnknownBuiltin(String),
    #[error("override provided for builtin tool that is not enabled: {0}")]
    OverrideForDisabledBuiltin(String),
    #[error("max_concurrency must be >= 1 for {0}")]
    InvalidMaxConcurrency(String),
    #[error("enabled MCP server requires a transport for {0}")]
    MissingMcpTransport(String),
    #[error("unsupported MCP transport `{transport}` for {scope}")]
    UnsupportedMcpTransport { scope: String, transport: String },
    #[error("enabled MCP stdio server requires a command for {0}")]
    MissingMcpCommand(String),
}

fn validate_builtin_names(names: &[String]) -> Result<(), ConfigError> {
    for name in names {
        if Builtin::from_name(name).is_none() {
            return Err(ConfigError::UnknownBuiltin(name.clone()));
        }
    }
    Ok(())
}

fn validate_tool_policy(scope: &str, config: &ToolPolicyConfig) -> Result<(), ConfigError> {
    if matches!(config.max_concurrency, Some(0)) {
        return Err(ConfigError::InvalidMaxConcurrency(scope.to_string()));
    }
    Ok(())
}

fn validate_mcp_server_policy(scope: &str, config: &McpServerConfig) -> Result<(), ConfigError> {
    if matches!(config.max_concurrency, Some(0)) {
        return Err(ConfigError::InvalidMaxConcurrency(scope.to_string()));
    }

    if !config.enabled {
        return Ok(());
    }

    let transport = config
        .transport
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ConfigError::MissingMcpTransport(scope.to_string()))?;

    if transport != "stdio" {
        return Err(ConfigError::UnsupportedMcpTransport {
            scope: scope.to_string(),
            transport: transport.to_string(),
        });
    }

    let has_command = config
        .command
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());

    if !has_command {
        return Err(ConfigError::MissingMcpCommand(scope.to_string()));
    }

    Ok(())
}

fn resolve_tool_policy(
    defaults: &ToolPolicyConfig,
    override_policy: Option<&ToolPolicyConfig>,
) -> EffectiveToolPolicy {
    let allow_parallel = override_policy
        .and_then(|policy| policy.allow_parallel)
        .or(defaults.allow_parallel)
        .unwrap_or(true);
    let max_concurrency = override_policy
        .and_then(|policy| policy.max_concurrency)
        .or(defaults.max_concurrency)
        .unwrap_or(1);

    EffectiveToolPolicy {
        allow_parallel,
        max_concurrency,
    }
}

fn resolve_mcp_policy(
    defaults: &ToolPolicyConfig,
    server: &McpServerConfig,
) -> EffectiveToolPolicy {
    let allow_parallel = server
        .allow_parallel
        .or(defaults.allow_parallel)
        .unwrap_or(true);
    let max_concurrency = server
        .max_concurrency
        .or(defaults.max_concurrency)
        .unwrap_or(1);

    EffectiveToolPolicy {
        allow_parallel,
        max_concurrency,
    }
}
