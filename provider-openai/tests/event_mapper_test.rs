use argus_core::ResponseEvent;
use provider_openai::Mapper;

#[test]
fn assembles_tool_call_on_finish_reason_tool_calls() {
    let mut m = Mapper::new("openai".into());
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"get_weather","arguments":"{\""}}]}}]}"#).unwrap();
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"arguments":"city\":\"北京\"}"}}]}}]}"#).unwrap();
    let events = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"content":""}}]}"#).unwrap();
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::ToolDone(_))));
}

#[test]
fn emits_content_delta() {
    let mut m = Mapper::new("openai".into());
    let events = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":"stop"}]}"#).unwrap();
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::ContentDelta(_))));
}

#[test]
fn emits_reasoning_delta() {
    let mut m = Mapper::new("openai".into());
    let events = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"reasoning_content":"thinking..."},"finish_reason":"stop"}]}"#).unwrap();
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::ReasoningDelta(_))));
}

#[test]
fn emits_created_event_once() {
    let mut m = Mapper::new("openai".into());
    let events1 = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":"Hello"}}]}"#).unwrap();
    let events2 = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":" World"},"finish_reason":"stop"}]}"#).unwrap();

    let created_count = events1.iter().filter(|e| matches!(e, ResponseEvent::Created(_))).count()
        + events2.iter().filter(|e| matches!(e, ResponseEvent::Created(_))).count();
    assert_eq!(created_count, 1);
}
