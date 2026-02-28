use std::sync::Arc;

use agent_core::{
    new_id, AgentError, InputEnvelope, InputPart, InputSource, LanguageModel, ModelEventStream,
    ModelOutputEvent, ModelRequest, NoteLevel, ToolCall, TranscriptItem, TransientError, Usage,
};
use async_trait::async_trait;
use bigmodel_api::{
    FunctionDefinition, FunctionTool, Tool as BigModelTool,
};
use futures::StreamExt;
use llm_client::{LlmChunk, LlmClient, LlmMessage, LlmRequest, LlmRole, LlmUsage};
use llm_client::LlmError;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Debug, Clone)]
pub struct BigModelAdapterConfig {
    pub model: String,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

impl Default for BigModelAdapterConfig {
    fn default() -> Self {
        Self {
            model: "glm-5".to_string(),
            system_prompt: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
        }
    }
}

pub struct BigModelModelAdapter {
    client: Arc<LlmClient>,
    config: BigModelAdapterConfig,
}

impl BigModelModelAdapter {
    pub fn new(client: Arc<LlmClient>) -> Self {
        Self {
            client,
            config: BigModelAdapterConfig::default(),
        }
    }

    pub fn with_config(mut self, config: BigModelAdapterConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl LanguageModel for BigModelModelAdapter {
    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn stream(&self, request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        let request = convert_model_request(request, &self.config);
        let client = Arc::clone(&self.client);
        let (tx, rx) = mpsc::unbounded_channel::<Result<ModelOutputEvent, AgentError>>();

        tokio::spawn(async move {
            let stream_result = client.chat_stream(request);
            let mut stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(map_llm_error(e)));
                    return;
                }
            };
            let mut usage: Option<Usage> = None;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        usage = extract_usage_from_chunk(&chunk).or(usage);
                        emit_chunk(chunk, &tx);
                    }
                    Err(err) => {
                        let _ = tx.send(Err(map_llm_error(err)));
                        return;
                    }
                }
            }

            let _ = tx.send(Ok(ModelOutputEvent::Completed { usage }));
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }
}

fn emit_chunk(
    chunk: LlmChunk,
    tx: &mpsc::UnboundedSender<Result<ModelOutputEvent, AgentError>>,
) {
    // For now, we only emit the text delta and reasoning delta
    // Tool calls would require additional mapping
    if let Some(reasoning_delta) = chunk.delta_reasoning {
        if !reasoning_delta.is_empty() {
            let _ = tx.send(Ok(ModelOutputEvent::ReasoningDelta {
                delta: reasoning_delta,
            }));
        }
    }

    if let Some(content_delta) = chunk.delta_text {
        if !content_delta.is_empty() {
            let _ = tx.send(Ok(ModelOutputEvent::TextDelta {
                delta: content_delta,
            }));
        }
    }
}

fn extract_usage_from_chunk(chunk: &LlmChunk) -> Option<Usage> {
    chunk.usage.as_ref().map(|u| Usage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
        total_tokens: u.total_tokens,
    })
}

fn convert_model_request(request: ModelRequest, cfg: &BigModelAdapterConfig) -> LlmRequest {
    let ModelRequest {
        transcript,
        inputs,
        tools,
        ..
    } = request;
    let mut messages = Vec::new();

    if let Some(prompt) = cfg.system_prompt.as_ref() {
        messages.push(LlmMessage {
            role: LlmRole::System,
            content: prompt.clone(),
        });
    }

    for item in transcript {
        if let Some(message) = transcript_item_to_message(item) {
            messages.push(message);
        }
    }

    for input in inputs {
        messages.push(input_envelope_to_message(input));
    }

    // Note: tools are not yet supported in the generic LlmRequest
    // They would need to be added as a separate field

    LlmRequest {
        model: cfg.model.clone(),
        messages,
        stream: true,
        max_tokens: cfg.max_tokens,
        temperature: cfg.temperature,
        top_p: cfg.top_p,
    }
}

fn transcript_item_to_message(item: TranscriptItem) -> Option<LlmMessage> {
    match item {
        TranscriptItem::UserMessage { input, .. } => Some(input_envelope_to_message(input)),
        TranscriptItem::AssistantMessage { text, .. } => Some(LlmMessage {
            role: LlmRole::Assistant,
            content: text,
        }),
        TranscriptItem::Reasoning { text, .. } => Some(LlmMessage {
            role: LlmRole::Assistant,
            content: text, // Note: This loses reasoning_content separation
        }),
        TranscriptItem::ToolCall { call, .. } => Some(LlmMessage {
            role: LlmRole::Assistant,
            content: format!(
                "[tool_call] id={} name={} args={}",
                call.call_id, call.tool_name, call.arguments
            ),
        }),
        TranscriptItem::ToolResult { result, .. } => Some(LlmMessage {
            role: LlmRole::Tool,
            content: result.output.to_string(),
        }),
        TranscriptItem::SystemNote { level, message, .. } => Some(LlmMessage {
            role: LlmRole::System,
            content: format!("{} {}", note_prefix(level), message),
        }),
    }
}

fn input_envelope_to_message(input: InputEnvelope) -> LlmMessage {
    let text = format_input_parts(input.parts);
    let role = match input.source {
        InputSource::User => LlmRole::User,
        InputSource::Tool => LlmRole::Tool,
        InputSource::System => LlmRole::System,
    };

    LlmMessage {
        role,
        content: text,
    }
}

fn note_prefix(level: NoteLevel) -> &'static str {
    match level {
        NoteLevel::Info => "[INFO]",
        NoteLevel::Warn => "[WARN]",
        NoteLevel::Error => "[ERROR]",
    }
}

fn format_input_parts(parts: Vec<InputPart>) -> String {
    let mut text_parts = Vec::new();
    for part in parts {
        match part {
            InputPart::Text { text } => text_parts.push(text),
            InputPart::Json { value } => text_parts.push(value.to_string()),
        }
    }
    text_parts.join("\n")
}

fn map_llm_error(err: LlmError) -> AgentError {
    match err {
        LlmError::RateLimit {
            message,
            retry_after,
        } => AgentError::Transient(TransientError::RateLimit {
            message,
            retry_after_ms: retry_after.map(|d| d.as_millis() as u64),
        }),
        LlmError::NetworkError { message } => AgentError::Transient(TransientError::Network {
            message,
            retry_after_ms: None,
        }),
        LlmError::ServerError { message, .. } => {
            AgentError::Transient(TransientError::ServiceUnavailable {
                message,
                retry_after_ms: None,
            })
        }
        LlmError::Timeout | LlmError::StreamIdleTimeout => {
            AgentError::Transient(TransientError::Network {
                message: "request timeout".to_string(),
                retry_after_ms: None,
            })
        }
        LlmError::AuthError { message } | LlmError::InvalidRequest { message } => {
            AgentError::Model { message }
        }
        LlmError::ContextOverflow { message } => AgentError::Model { message },
        LlmError::QuotaExceeded { message } => AgentError::Model { message },
        LlmError::StreamError { message } => AgentError::Model { message },
        LlmError::ParseError { message } => AgentError::Model { message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::tools::ToolExecutionPolicy;
    use agent_core::{new_id, InputEnvelope, ToolResult};

    #[test]
    fn convert_request_includes_system_prompt_and_streaming() {
        let request = ModelRequest {
            epoch: 0,
            transcript: vec![TranscriptItem::assistant_message("previous")],
            inputs: vec![InputEnvelope::user_text("hello")],
            tools: vec![],
        };
        let cfg = BigModelAdapterConfig {
            model: "glm-test".to_string(),
            system_prompt: Some("be helpful".to_string()),
            max_tokens: Some(512),
            temperature: Some(0.5),
            top_p: Some(0.9),
        };

        let converted = convert_model_request(request, &cfg);

        assert_eq!(converted.model, "glm-test");
        assert!(converted.stream);
        assert_eq!(converted.max_tokens, Some(512));
        assert_eq!(converted.temperature, Some(0.5));
        assert_eq!(converted.top_p, Some(0.9));
        assert_eq!(converted.messages.len(), 3);
        assert!(matches!(converted.messages[0].role, LlmRole::System));
        assert_eq!(converted.messages[0].content, "be helpful");
        assert!(matches!(converted.messages[1].role, LlmRole::Assistant));
        assert_eq!(converted.messages[1].content, "previous");
        assert!(matches!(converted.messages[2].role, LlmRole::User));
        assert_eq!(converted.messages[2].content, "hello");
    }

    #[test]
    fn default_config_uses_supported_model_name() {
        let cfg = BigModelAdapterConfig::default();
        assert_eq!(cfg.model, "glm-5");
    }

    #[test]
    fn transcript_tool_result_maps_to_tool_message() {
        let item = TranscriptItem::ToolResult {
            id: new_id(),
            epoch: 2,
            result: ToolResult::ok("call-1", serde_json::json!({"ok": true})),
        };

        let message = transcript_item_to_message(item).expect("message");
        assert!(matches!(message.role, LlmRole::Tool));
        assert_eq!(message.content, "{\"ok\":true}");
    }

    #[test]
    fn stream_chunk_emits_text_and_reasoning_deltas() {
        let chunk = LlmChunk {
            id: "chunk-1".to_string(),
            created: 0,
            model: "glm-test".to_string(),
            delta_text: Some("hello".to_string()),
            delta_reasoning: Some("thinking".to_string()),
            finish_reason: None,
            usage: None,
        };

        let (tx, mut rx) = mpsc::unbounded_channel();
        emit_chunk(chunk, &tx);

        let first = rx.try_recv().expect("first event").expect("ok");
        let second = rx.try_recv().expect("second event").expect("ok");

        assert_eq!(
            first,
            ModelOutputEvent::ReasoningDelta {
                delta: "thinking".to_string()
            }
        );
        assert_eq!(
            second,
            ModelOutputEvent::TextDelta {
                delta: "hello".to_string()
            }
        );
    }

    #[test]
    fn map_errors_to_agent_error_classes() {
        let retryable = map_llm_error(LlmError::RateLimit {
            message: "busy".to_string(),
            retry_after: None,
        });
        assert!(matches!(retryable, AgentError::Transient(_)));

        let fatal = map_llm_error(LlmError::InvalidRequest {
            message: "bad".to_string(),
        });
        assert!(matches!(fatal, AgentError::Model { .. }));
    }

    #[test]
    fn extract_usage_from_chunk_maps_token_stats() {
        let chunk = LlmChunk {
            id: "chunk-usage".to_string(),
            created: 0,
            model: "glm-test".to_string(),
            delta_text: None,
            delta_reasoning: None,
            finish_reason: None,
            usage: Some(LlmUsage {
                input_tokens: 12,
                output_tokens: 34,
                total_tokens: 46,
            }),
        };

        let usage = extract_usage_from_chunk(&chunk).expect("usage");
        assert_eq!(
            usage,
            Usage {
                input_tokens: 12,
                output_tokens: 34,
                total_tokens: 46,
            }
        );
    }

    fn tool_spec_echo() -> agent_core::tools::ToolSpec {
        agent_core::tools::ToolSpec {
            name: "echo".to_string(),
            description: "echo args".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            execution_policy: ToolExecutionPolicy::default(),
        }
    }
}
