use argus_core::{ResponseEvent, ToolCall, ZaiMcpType};
use provider::{Dialect, Mapper};

#[test]
fn prefixed_openai_function_becomes_mcp() {
    let mut m = Mapper::new(Dialect::Openai);
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"__mcp__filesystem","arguments":"{\"type\":\"mcp_call\",\"server_label\":\"filesystem\",\"name\":\"read_file\",\"arguments\":\"{\\\"path\\\":\\\"./config.yaml\\\"}\"}"}}]}}]}"#)
        .unwrap();

    let mcp = events.iter().find_map(|event| match event {
        ResponseEvent::ToolDone(ToolCall::Mcp(call)) => Some(call),
        _ => None,
    });

    assert!(mcp.is_some(), "expected ToolDone(Mcp)");
    let mcp = mcp.unwrap();
    assert_eq!(mcp.sequence, 0);
    assert_eq!(mcp.id, "call_1");
    assert_eq!(mcp.mcp_type, ZaiMcpType::McpCall);
    assert_eq!(mcp.server_label.as_deref(), Some("filesystem"));
    assert_eq!(mcp.name.as_deref(), Some("read_file"));
}

#[test]
fn invalid_mcp_json_returns_protocol_error() {
    let mut m = Mapper::new(Dialect::Openai);
    let err = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"__mcp__filesystem","arguments":"{\"type\":\"mcp_call\""}}]}}]}"#)
        .unwrap_err();

    assert!(matches!(
        err,
        provider::Error::Openai(provider::dialect::openai::mapper::Error::Protocol(_))
    ));
}
