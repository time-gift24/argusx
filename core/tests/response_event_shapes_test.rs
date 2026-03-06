use core::{Meta, ResponseEvent, ToolCall, Usage, ZaiMcpCall, ZaiMcpType};
use std::sync::Arc;

#[test]
fn response_event_shape_matches_design() {
    let _ = ResponseEvent::Created(Meta {
        id: "2026030609392522891edc8ea44775".into(),
        created: 1772761165,
        object: "chat.completion.chunk".into(),
        model: "glm-5".into(),
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
    let _ = ResponseEvent::ToolDone(ToolCall::Mcp(ZaiMcpCall {
        sequence: 1,
        id: "call_2".into(),
        mcp_type: ZaiMcpType::McpCall,
        server_label: Some("weather".into()),
        name: Some("get_weather".into()),
        arguments_json: Some("{\"city\":\"北京\"}".into()),
        output_json: None,
        tools_json: None,
        error: None,
    }));
    let _ = ResponseEvent::Done(Some(Usage {
        input_tokens: 1,
        output_tokens: 2,
        total_tokens: 3,
    }));
}
