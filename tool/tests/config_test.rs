use tool::config::{AgentToolConfig, ConfigError};

#[test]
fn parses_builtin_whitelist_and_overrides() {
    let raw = r#"
        [tools]
        builtin_tools = ["read", "glob"]

        [tools.defaults]
        allow_parallel = true
        max_concurrency = 4

        [tools.builtin.read]
        max_concurrency = 16
    "#;

    let cfg: AgentToolConfig = toml::from_str(raw).unwrap();
    assert_eq!(cfg.tools.builtin_tools, vec!["read", "glob"]);
    assert_eq!(cfg.tools.defaults.max_concurrency, Some(4));
    assert_eq!(
        cfg.tools
            .builtin
            .get("read")
            .and_then(|cfg| cfg.max_concurrency),
        Some(16)
    );
}

#[test]
fn rejects_unknown_builtin_override() {
    let raw = r#"
        [tools]
        builtin_tools = ["read"]

        [tools.builtin.nope]
        max_concurrency = 2
    "#;

    assert!(AgentToolConfig::parse_and_validate(raw).is_err());
}

#[test]
fn rejects_enabled_mcp_server_without_transport() {
    let raw = r#"
        [mcp.server.filesystem]
        enabled = true
        command = "uvx"
    "#;

    assert!(matches!(
        AgentToolConfig::parse_and_validate(raw),
        Err(ConfigError::MissingMcpTransport(scope)) if scope == "mcp.server.filesystem"
    ));
}

#[test]
fn rejects_enabled_mcp_server_with_unsupported_transport() {
    let raw = r#"
        [mcp.server.filesystem]
        enabled = true
        transport = "http"
        command = "uvx"
    "#;

    assert!(matches!(
        AgentToolConfig::parse_and_validate(raw),
        Err(ConfigError::UnsupportedMcpTransport { scope, transport })
            if scope == "mcp.server.filesystem" && transport == "http"
    ));
}

#[test]
fn rejects_enabled_stdio_mcp_server_without_command() {
    let raw = r#"
        [mcp.server.filesystem]
        enabled = true
        transport = "stdio"
    "#;

    assert!(matches!(
        AgentToolConfig::parse_and_validate(raw),
        Err(ConfigError::MissingMcpCommand(scope)) if scope == "mcp.server.filesystem"
    ));
}

#[test]
fn accepts_git_as_valid_builtin_tool() {
    let raw = r#"
        [tools]
        builtin_tools = ["read", "git"]

        [tools.builtin.git]
        max_concurrency = 2
    "#;

    let result = AgentToolConfig::parse_and_validate(raw);
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert!(cfg.tools.builtin_tools.contains(&"git".to_string()));
}
