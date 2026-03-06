use argus_core::{ResponseEvent, ToolCall};
use provider::{Dialect, Mapper};

#[test]
fn zai_mapper_handles_message_tool_calls() {
    let mut mapper = Mapper::new(Dialect::Zai);

    let _ = mapper
        .feed(r#"{"id":"x","object":"chat.completion.chunk","created":1,"model":"MiniMax-M2.5","choices":[{"index":0,"delta":{"reasoning_content":"The user"}}]}"#)
        .unwrap();

    let events = mapper
        .feed(r#"{"id":"x","object":"chat.completion","created":1,"model":"MiniMax-M2.5","choices":[{"index":0,"finish_reason":"tool_calls","message":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"get_weather","arguments":"{\"city\":\"北京\"}"}}]}}],"usage":{"total_tokens":10,"prompt_tokens":4,"completion_tokens":6}}"#)
        .unwrap();

    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::ReasoningDone(_)))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::ToolDone(ToolCall::FunctionCall { .. })))
    );
}
