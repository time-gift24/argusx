use std::sync::Arc;

use llm_client::{LlmChunkStream, LlmClient, LlmError, LlmRequest, LlmResponse, ProviderAdapter};

struct TestAdapter;

#[async_trait::async_trait]
impl ProviderAdapter for TestAdapter {
    fn id(&self) -> &str {
        "test"
    }

    async fn chat(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            id: "resp".to_string(),
            request_id: None,
            created: 0,
            model: "m".to_string(),
            output_text: "ok".to_string(),
            finish_reason: Some("stop".to_string()),
            usage: None,
            extensions: serde_json::json!({}),
        })
    }

    fn chat_stream(&self, _req: LlmRequest) -> LlmChunkStream {
        Box::pin(futures::stream::empty())
    }
}

#[tokio::test]
async fn build_client_without_default_adapter_fails() {
    let result = LlmClient::builder().build();
    assert!(result.is_err());
}

#[tokio::test]
async fn calling_unknown_adapter_fails() {
    let client = LlmClient::builder()
        .default_adapter("missing")
        .build()
        .unwrap_err();
    assert!(client.to_string().contains("default adapter"));
}

#[tokio::test]
async fn chat_with_unknown_adapter_fails() {
    let client = LlmClient::builder()
        .register_adapter(Arc::new(TestAdapter))
        .default_adapter("test")
        .build()
        .unwrap();
    let req = LlmRequest {
        model: "model".to_string(),
        messages: vec![],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        tools: None,
    };
    let err = client
        .chat_with_adapter("openai", req)
        .await
        .expect_err("unknown adapter should fail");
    assert!(err.to_string().contains("adapter 'openai' not found"));
}
