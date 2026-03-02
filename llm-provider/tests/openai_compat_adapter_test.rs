use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use llm_client::{LlmClient, LlmMessage, LlmRequest, LlmRole};
use llm_provider::openai_compat::{ChatCompletionsConfig, OpenAiCompatAdapter};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sample_request() -> LlmRequest {
    LlmRequest {
        model: "gpt-test".to_string(),
        messages: vec![LlmMessage {
            role: LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: false,
        max_tokens: Some(128),
        temperature: Some(0.1),
        top_p: Some(1.0),
        tools: None,
    }
}

fn sse_data(event: serde_json::Value) -> String {
    format!("data: {event}\n\n")
}

fn sse_done() -> String {
    "data: [DONE]\n\n".to_string()
}

#[tokio::test]
async fn openai_compat_adapter_chats_via_chat_completions() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-1",
            "created": 1700000000,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "hello from openai"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })))
        .mount(&mock_server)
        .await;

    let config = ChatCompletionsConfig::new(mock_server.uri(), "test-key", HashMap::new())
        .expect("valid config");
    let adapter = Arc::new(OpenAiCompatAdapter::new(config));

    let client = LlmClient::builder()
        .register_adapter(adapter)
        .default_adapter("openai")
        .build()
        .unwrap();

    let resp = client.chat(sample_request()).await.unwrap();
    assert_eq!(resp.model, "gpt-test");
    assert_eq!(resp.output_text, "hello from openai");
    assert_eq!(resp.usage.unwrap().total_tokens, 15);
}

#[tokio::test]
async fn openai_compat_stream_maps_reasoning_content() {
    let mock_server = MockServer::start().await;

    let sse_body = vec![
        sse_data(serde_json::json!({
            "id": "chatcmpl-2",
            "created": 1700000001,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "delta": { "content": "Hello " },
                "finish_reason": null
            }]
        })),
        sse_data(serde_json::json!({
            "id": "chatcmpl-2",
            "created": 1700000001,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "delta": { "reasoning_content": "thinking..." },
                "finish_reason": null
            }]
        })),
        sse_data(serde_json::json!({
            "id": "chatcmpl-2",
            "created": 1700000001,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "delta": { "content": "world" },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        })),
        sse_done(),
    ]
    .concat();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&mock_server)
        .await;

    let config = ChatCompletionsConfig::new(mock_server.uri(), "test-key", HashMap::new())
        .expect("valid config");
    let adapter = Arc::new(OpenAiCompatAdapter::new(config));

    let client = LlmClient::builder()
        .register_adapter(adapter)
        .default_adapter("openai")
        .build()
        .unwrap();

    let mut stream = client.chat_stream(sample_request()).unwrap();
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut usage_total = None;

    while let Some(item) = stream.next().await {
        let chunk = item.expect("stream chunk");
        if let Some(delta) = chunk.delta_text {
            text.push_str(&delta);
        }
        if let Some(delta) = chunk.delta_reasoning {
            reasoning.push_str(&delta);
        }
        if let Some(usage) = chunk.usage {
            usage_total = Some(usage.total_tokens);
        }
    }

    assert_eq!(text, "Hello world");
    assert_eq!(reasoning, "thinking...");
    assert_eq!(usage_total, Some(18));
}

#[test]
fn config_requires_base_url() {
    let err = ChatCompletionsConfig::new("", "k", HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("base_url"));
}
