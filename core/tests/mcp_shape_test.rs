use core::{McpCall, McpCallType, ToolCall};

#[test]
fn mcp_shape_is_provider_agnostic() {
    let _ = ToolCall::Mcp(McpCall {
        sequence: 0,
        id: "call_1".into(),
        mcp_type: McpCallType::McpCall,
        server_label: Some("fs".into()),
        name: Some("read_file".into()),
        arguments_json: Some("{\"path\":\"./config.yaml\"}".into()),
        output_json: None,
        tools_json: None,
        error: None,
    });
}
