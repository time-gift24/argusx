use core::{
    Builtin, BuiltinToolCall, FinishReason, McpCall, McpCallType, Meta, ResponseEvent, ToolCall,
    Usage,
};
use std::sync::Arc;

#[test]
fn response_event_shape_matches_design() {
    let _ = ResponseEvent::Created(Meta {
        id: "2026030609392522891edc8ea44775".into(),
        created: 1772761165,
        object: "chat.completion.chunk".into(),
        model: "model-test".into(),
    });
    let _ = ResponseEvent::ContentDelta(Arc::<str>::from("hi"));
    let _ = ResponseEvent::ReasoningDelta(Arc::<str>::from("think"));
    let _ = ResponseEvent::ToolDelta(Arc::<str>::from("{\"city\""));
    let _ = ResponseEvent::ToolDone(ToolCall::FunctionCall {
        sequence: 0,
        call_id: "call_1".into(),
        name: "get_weather".into(),
        arguments_json: "{\"city\":\"北京\"}".into(),
    });
    let _ = ResponseEvent::ToolDone(ToolCall::Builtin(BuiltinToolCall {
        sequence: 1,
        call_id: "call_builtin".into(),
        builtin: Builtin::Read,
        arguments_json: "{\"path\":\"Cargo.toml\"}".into(),
    }));
    let _ = ResponseEvent::ToolDone(ToolCall::Mcp(McpCall {
        sequence: 2,
        id: "call_2".into(),
        mcp_type: McpCallType::McpCall,
        server_label: Some("weather".into()),
        name: Some("get_weather".into()),
        arguments_json: Some("{\"city\":\"北京\"}".into()),
        output_json: None,
        tools_json: None,
        error: None,
    }));
    let _ = ResponseEvent::Done {
        reason: FinishReason::Stop,
        usage: Some(Usage {
            input_tokens: 1,
            output_tokens: 2,
            total_tokens: 3,
        }),
    };
}
