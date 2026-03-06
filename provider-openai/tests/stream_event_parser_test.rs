use provider_openai::parse_payload;
use provider_openai::parse_sse_line;
use provider_openai::schema::stream::ChatCompletionsStreamEvent;

#[test]
fn parse_payload_done_event() {
    let event = parse_payload("[DONE]").unwrap();
    assert!(matches!(event, ChatCompletionsStreamEvent::Done));
}

#[test]
fn parse_payload_chunk_event() {
    let payload = r#"{"id":"x","object":"chat.completion.chunk","created":1,"model":"glm-5","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":null}]}"#;
    let event = parse_payload(payload).unwrap();
    assert!(matches!(event, ChatCompletionsStreamEvent::Chunk(_)));
}

#[test]
fn parse_payload_structured_error_event() {
    let payload = r#"{"error":{"message":"rate limited","type":"rate_limit_error","code":"429"}}"#;
    let event = parse_payload(payload).unwrap();
    assert!(matches!(event, ChatCompletionsStreamEvent::Error(_)));
}

#[test]
fn parse_payload_raw_error_string_event() {
    let event = parse_payload("gateway timeout").unwrap();
    assert!(matches!(event, ChatCompletionsStreamEvent::Error(_)));
}

#[test]
fn parse_sse_line_data_done() {
    let event = parse_sse_line("data: [DONE]").unwrap();
    assert!(matches!(event, Some(ChatCompletionsStreamEvent::Done)));
}

#[test]
fn parse_sse_line_open_event() {
    let event = parse_sse_line("event: open").unwrap();
    assert!(matches!(event, Some(ChatCompletionsStreamEvent::Open)));
}

#[test]
fn parse_sse_line_ignores_non_data_lines() {
    let event = parse_sse_line("id: 123").unwrap();
    assert!(event.is_none());
}

#[test]
fn parse_payload_invalid_json_returns_error() {
    assert!(parse_payload("{not-json").is_err());
}
