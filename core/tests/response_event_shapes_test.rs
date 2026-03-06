use std::sync::Arc;
use core::{Meta, ResponseEvent, ToolCall, Usage};

#[test]
fn response_event_shape_matches_design() {
    let _ = ResponseEvent::Created(Meta { model: "glm-5".into(), provider: "openai".into() });
    let _ = ResponseEvent::ContentDelta(Arc::<str>::from("hi"));
    let _ = ResponseEvent::ReasoningDelta(Arc::<str>::from("think"));
    let _ = ResponseEvent::ToolDelta(Arc::<str>::from("{\"city\""));
    let _ = ResponseEvent::ToolDone(ToolCall::FunctionCall {
        call_id: "call_1".into(),
        name: "get_weather".into(),
        arguments_json: "{\"city\":\"北京\"}".into(),
    });
    let _ = ResponseEvent::Done(Some(Usage { input_tokens: 1, output_tokens: 2, total_tokens: 3 }));
}
