pub fn is_mcp_call(call_type: Option<&str>, name: Option<&str>) -> bool {
    matches!(call_type, Some("mcp")) || name.is_some_and(|n| n.starts_with("__mcp__"))
}
