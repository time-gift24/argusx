// llm-client/src/types.rs
use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::LlmError;

pub type LlmChunkStream = Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send + 'static>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

/// Generic tool definition - provider-agnostic representation of a function tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Generic tool call - represents a tool invocation from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub call_id: Option<String>,
    pub tool_name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub stream: bool,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub tools: Option<Vec<LlmTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub id: String,
    pub created: i64,
    pub model: String,
    pub output_text: String,
    pub finish_reason: Option<String>,
    pub request_id: Option<String>,
    pub usage: Option<LlmUsage>,
    /// Additional provider-specific fields (web_search, video_result, content_filter, etc.)
    #[serde(default)]
    pub extensions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChunk {
    pub id: String,
    pub created: i64,
    pub model: String,
    pub delta_text: Option<String>,
    pub delta_reasoning: Option<String>,
    pub delta_tool_calls: Option<Vec<LlmToolCall>>,
    pub finish_reason: Option<String>,
    pub usage: Option<LlmUsage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that tools can be added to LlmRequest
    #[test]
    fn llm_request_supports_tools_field() {
        let tools = vec![LlmTool {
            name: "echo".to_string(),
            description: "Echo back the input".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                }
            }),
        }];

        let req = LlmRequest {
            model: "glm-5".to_string(),
            messages: vec![LlmMessage {
                role: LlmRole::User,
                content: "hello".to_string(),
            }],
            stream: true,
            max_tokens: Some(128),
            temperature: Some(0.7),
            top_p: Some(0.9),
            tools: Some(tools),
        };

        assert!(req.tools.is_some());
        assert_eq!(req.tools.as_ref().unwrap().len(), 1);
        assert_eq!(req.tools.as_ref().unwrap()[0].name, "echo");
    }

    /// Test that LlmChunk supports delta_tool_calls
    #[test]
    fn llm_chunk_supports_delta_tool_calls() {
        let tool_calls = vec![LlmToolCall {
            call_id: Some("call-123".to_string()),
            tool_name: Some("echo".to_string()),
            arguments: Some(r#"{"text":"hello"}"#.to_string()),
        }];

        let chunk = LlmChunk {
            id: "test-id".to_string(),
            created: 1234567890,
            model: "glm-5".to_string(),
            delta_text: None,
            delta_reasoning: None,
            finish_reason: None,
            usage: None,
            delta_tool_calls: Some(tool_calls),
        };

        let calls = chunk.delta_tool_calls.expect("should have tool calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].call_id.as_deref(), Some("call-123"));
        assert_eq!(calls[0].tool_name.as_deref(), Some("echo"));
    }
}
