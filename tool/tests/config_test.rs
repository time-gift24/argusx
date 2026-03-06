use tool::config::AgentToolConfig;

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
