use provider::normalize::tool_calls::is_mcp_call;

#[test]
fn classify_type_mcp_as_mcp() {
    assert!(is_mcp_call(Some("mcp"), Some("any")));
}

#[test]
fn classify_prefixed_name_as_mcp() {
    assert!(is_mcp_call(Some("function"), Some("__mcp__filesystem")));
}

#[test]
fn classify_regular_function_as_function() {
    assert!(!is_mcp_call(Some("function"), Some("get_weather")));
}
