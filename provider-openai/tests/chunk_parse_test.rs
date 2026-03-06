use provider_openai::parse_chunk;

#[test]
fn parse_reasoning_chunk() {
    let raw = r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"reasoning_content":"用户"}}]}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert_eq!(chunk.model, "glm-5");
}

#[test]
fn parse_content_chunk() {
    let raw = r#"{"id":"chatcmpl-123","created":1700000000,"object":"chat.completion.chunk","model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert_eq!(chunk.model, "gpt-4");
    assert_eq!(chunk.choices[0].delta.content.as_ref().unwrap(), "Hello");
}

#[test]
fn parse_tool_calls_chunk() {
    let raw = r#"{"id":"chatcmpl-123","created":1700000000,"object":"chat.completion.chunk","model":"gpt-4","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","type":"function","function":{"name":"get_weather","arguments":"{"}}]}}]}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert!(!chunk.choices[0].delta.tool_calls.as_ref().unwrap().is_empty());
}
