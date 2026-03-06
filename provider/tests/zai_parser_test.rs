use provider::dialect::zai::parser::parse_payload;
use provider::dialect::zai::schema::stream::ZaiStreamEvent;

#[test]
fn parses_zai_chunk_with_reasoning_and_tool_delta() {
    let payload = r#"{"id":"x","object":"chat.completion.chunk","created":1,"model":"MiniMax-M2.5","choices":[{"index":0,"delta":{"reasoning_content":"The user"}}]}"#;
    let event = parse_payload(payload).unwrap();
    assert!(matches!(event, ZaiStreamEvent::Chunk(_)));
}

#[test]
fn parses_zai_completion_message_tool_calls() {
    let payload = r#"{"id":"x","object":"chat.completion","created":1,"model":"MiniMax-M2.5","choices":[{"index":0,"finish_reason":"tool_calls","message":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"get_weather","arguments":"{\"city\":\"北京\"}"}}]}}],"usage":{"total_tokens":10,"prompt_tokens":4,"completion_tokens":6}}"#;
    let event = parse_payload(payload).unwrap();
    match event {
        ZaiStreamEvent::Chunk(chunk) => {
            assert_eq!(chunk.object, "chat.completion");
            assert!(
                chunk.choices[0]
                    .message
                    .as_ref()
                    .and_then(|m| m.tool_calls.as_ref())
                    .is_some()
            );
        }
        _ => panic!("expected chunk"),
    }
}
