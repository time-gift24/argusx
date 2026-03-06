use core::{ToolCall, ZaiMcpCall, ZaiMcpType};

#[test]
fn mcp_shape_is_zai_aligned() {
    let _ = ToolCall::Mcp(ZaiMcpCall {
        sequence: 0,
        id: "call_1".into(),
        mcp_type: ZaiMcpType::McpCall,
        server_label: Some("fs".into()),
        name: Some("read_file".into()),
        arguments_json: Some("{\"path\":\"./config.yaml\"}".into()),
        output_json: None,
        tools_json: None,
        error: None,
    });
}
