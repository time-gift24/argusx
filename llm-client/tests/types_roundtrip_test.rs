// llm-client/tests/types_roundtrip_test.rs
use llm_client::{LlmChunk, LlmRequest, LlmRole};

#[test]
fn llm_request_and_chunk_are_constructible() {
    let req = LlmRequest {
        model: "glm-5".to_string(),
        messages: vec![llm_client::LlmMessage {
            role: LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: true,
        max_tokens: Some(128),
        temperature: Some(0.7),
        top_p: Some(0.9),
        tools: None,
    };

    assert!(req.stream);

    let chunk = LlmChunk {
        id: "test-id".to_string(),
        created: 1234567890,
        model: "glm-5".to_string(),
        delta_text: Some("hi".to_string()),
        delta_reasoning: None,
        delta_tool_calls: None,
        finish_reason: None,
        usage: None,
    };
    assert_eq!(chunk.delta_text.as_deref(), Some("hi"));
    assert_eq!(chunk.id, "test-id");
}
