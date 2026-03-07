use argus_core::{Builtin, ResponseEvent, ToolCall};
use provider::{Dialect, Mapper};

#[test]
fn known_builtin_name_upgrades_to_builtin_call() {
    let mut m = Mapper::new(Dialect::Openai);
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"read","arguments":"{\"path\":\"Cargo.toml\"}"}}]}}]}"#)
        .unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        ResponseEvent::ToolDone(ToolCall::Builtin(call))
            if matches!(call.builtin, Builtin::Read)
    )));
}

#[test]
fn git_builtin_name_upgrades_to_builtin_call() {
    let mut m = Mapper::new(Dialect::Openai);
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"tool_calls":[{"id":"call_2","index":0,"type":"function","function":{"name":"git","arguments":"{\"action\":\"status\",\"repo_path\":\".\"}"}}]}}]}"#)
        .unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        ResponseEvent::ToolDone(ToolCall::Builtin(call))
            if matches!(call.builtin, Builtin::Git)
    )));
}
