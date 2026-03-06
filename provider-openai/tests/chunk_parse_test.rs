use provider_openai::parse_chunk;

#[test]
fn parse_reasoning_chunk() {
    let raw = r#"{"id":"x","object":"chat.completion.chunk","created":1,"model":"glm-5","choices":[{"index":0,"delta":{"reasoning_content":"用户"}}]}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert_eq!(chunk.model, "glm-5");
}

#[test]
fn parse_content_chunk() {
    let raw = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1700000000,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}],"system_fingerprint":"fp_1"}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert_eq!(chunk.model, "gpt-4");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
    assert_eq!(chunk.system_fingerprint.as_deref(), Some("fp_1"));
}

#[test]
fn parse_tool_calls_chunk() {
    let raw = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1700000000,"model":"gpt-4","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"get_weather","arguments":"{"}}]}}]}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert!(!chunk.choices[0]
        .delta
        .tool_calls
        .as_ref()
        .unwrap()
        .is_empty());
}
