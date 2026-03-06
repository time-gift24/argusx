use argus_core::{ResponseEvent, ToolCall};
use provider_openai::Mapper;

#[test]
fn assembles_tool_call_on_finish_reason_tool_calls() {
    let mut m = Mapper::new("openai".into());
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"get_weather","arguments":"{\""}}]}}]}"#).unwrap();
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"arguments":"city\":\"北京\"}"}}]}}]}"#).unwrap();
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"content":""}}]}"#)
        .unwrap();

    let tool_done: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ResponseEvent::ToolDone(ToolCall::FunctionCall {
                sequence,
                call_id,
                name,
                arguments_json,
            }) => Some((
                *sequence,
                call_id.as_str(),
                name.as_str(),
                arguments_json.as_str(),
            )),
            _ => None,
        })
        .collect();

    assert_eq!(tool_done.len(), 1);
    let (sequence, call_id, name, args) = tool_done[0];
    assert_eq!(sequence, 0);
    assert_eq!(call_id, "call_1");
    assert_eq!(name, "get_weather");
    assert_eq!(args, "{\"city\":\"北京\"}");
}

#[test]
fn emits_content_delta() {
    let mut m = Mapper::new("openai".into());
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":"stop"}]}"#)
        .unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, ResponseEvent::ContentDelta(_))));
    assert!(events
        .iter()
        .any(|e| matches!(e, ResponseEvent::ContentDone(_))));
}

#[test]
fn emits_reasoning_delta() {
    let mut m = Mapper::new("openai".into());
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"reasoning_content":"thinking..."},"finish_reason":"stop"}]}"#)
        .unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, ResponseEvent::ReasoningDelta(_))));
    assert!(events
        .iter()
        .any(|e| matches!(e, ResponseEvent::ReasoningDone(_))));
}

#[test]
fn emits_created_event_once() {
    let mut m = Mapper::new("openai".into());
    let events1 = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":"Hello"}}]}"#).unwrap();
    let events2 = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":" World"},"finish_reason":"stop"}]}"#).unwrap();

    let created_count = events1
        .iter()
        .filter(|e| matches!(e, ResponseEvent::Created(_)))
        .count()
        + events2
            .iter()
            .filter(|e| matches!(e, ResponseEvent::Created(_)))
            .count();
    assert_eq!(created_count, 1);
}

#[test]
fn emits_tool_done_in_sequence_order() {
    let mut m = Mapper::new("openai".into());
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_2","index":1,"type":"function","function":{"name":"b","arguments":"{\"b\":1}"}}]}}]}"#).unwrap();
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"a","arguments":"{\"a\":1}"}}]}}]}"#).unwrap();
    let events = m
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"content":""}}]}"#)
        .unwrap();

    let sequences: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            ResponseEvent::ToolDone(ToolCall::FunctionCall { sequence, .. }) => Some(*sequence),
            _ => None,
        })
        .collect();
    assert_eq!(sequences, vec![0, 1]);
}
