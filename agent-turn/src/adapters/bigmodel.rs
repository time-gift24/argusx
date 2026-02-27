use std::sync::Arc;

use agent_core::{
    new_id, AgentError, InputEnvelope, InputPart, InputSource, LanguageModel, ModelEventStream,
    ModelOutputEvent, ModelRequest, NoteLevel, ToolCall, TranscriptItem, TransientError, Usage,
};
use async_trait::async_trait;
use bigmodel_api::{
    ChatRequest, ChatResponseChunk, Content, FunctionDefinition, FunctionTool, Message, Role,
    Tool as BigModelTool, Usage as BigModelUsage,
};
use futures::StreamExt;
use llm_client::providers::BigModelHttpClient;
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
    client: Arc<BigModelHttpClient>,
    config: BigModelAdapterConfig,
}

impl BigModelModelAdapter {
    pub fn new(client: Arc<BigModelHttpClient>) -> Self {
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
            let mut stream = client.chat_stream(request);
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
    chunk: ChatResponseChunk,
    tx: &mpsc::UnboundedSender<Result<ModelOutputEvent, AgentError>>,
) {
    for choice in chunk.choices {
        let delta = choice.delta;

        if let Some(reasoning_delta) = delta.reasoning_content {
            if !reasoning_delta.is_empty() {
                let _ = tx.send(Ok(ModelOutputEvent::ReasoningDelta {
                    delta: reasoning_delta,
                }));
            }
        }

        if let Some(tool_calls) = delta.tool_calls {
            for tool_call in tool_calls {
                let Some(function) = tool_call.function else {
                    continue;
                };
                let Some(tool_name) = function.name else {
                    continue;
                };
                let arguments = function
                    .arguments
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .unwrap_or_else(|| serde_json::json!({}));
                let call = ToolCall {
                    call_id: tool_call.id.unwrap_or_else(new_id),
                    tool_name,
                    arguments,
                };
                let _ = tx.send(Ok(ModelOutputEvent::ToolCall { call }));
            }
        }

        if let Some(content_delta) = delta.content {
            if !content_delta.is_empty() {
                let _ = tx.send(Ok(ModelOutputEvent::TextDelta {
                    delta: content_delta,
                }));
            }
        }
    }
}

fn extract_usage_from_chunk(chunk: &ChatResponseChunk) -> Option<Usage> {
    chunk.usage.as_ref().map(bigmodel_usage_to_usage)
}

fn bigmodel_usage_to_usage(usage: &BigModelUsage) -> Usage {
    Usage {
        input_tokens: non_negative_u64(usage.prompt_tokens),
        output_tokens: non_negative_u64(usage.completion_tokens),
        total_tokens: non_negative_u64(usage.total_tokens),
    }
}

fn non_negative_u64(value: i32) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn convert_model_request(request: ModelRequest, cfg: &BigModelAdapterConfig) -> ChatRequest {
    let ModelRequest {
        transcript,
        inputs,
        tools,
        ..
    } = request;
    let mut messages = Vec::new();

    if let Some(prompt) = cfg.system_prompt.as_ref() {
        messages.push(Message::system(prompt.clone()));
    }

    for item in transcript {
        if let Some(message) = transcript_item_to_message(item) {
            messages.push(message);
        }
    }

    for input in inputs {
        messages.push(input_envelope_to_message(input));
    }

    let tools = tools
        .into_iter()
        .map(core_tool_spec_to_bigmodel_tool)
        .collect::<Vec<_>>();

    let mut chat_request = ChatRequest::new(cfg.model.clone(), messages).stream();
    if !tools.is_empty() {
        chat_request = chat_request.tools(tools);
    }
    chat_request.max_tokens = cfg.max_tokens;
    chat_request.temperature = cfg.temperature;
    chat_request.top_p = cfg.top_p;
    chat_request
}

fn core_tool_spec_to_bigmodel_tool(spec: agent_core::tools::ToolSpec) -> BigModelTool {
    BigModelTool::Function(FunctionTool {
        function: FunctionDefinition {
            name: spec.name,
            description: spec.description,
            parameters: spec.input_schema,
        },
    })
}

fn transcript_item_to_message(item: TranscriptItem) -> Option<Message> {
    match item {
        TranscriptItem::UserMessage { input, .. } => Some(input_envelope_to_message(input)),
        TranscriptItem::AssistantMessage { text, .. } => Some(Message::assistant(text)),
        TranscriptItem::Reasoning { text, .. } => Some(Message {
            role: Role::Assistant,
            content: Content::Text(String::new()),
            reasoning_content: Some(text),
        }),
        TranscriptItem::ToolCall { call, .. } => Some(Message::assistant(tool_call_as_text(&call))),
        TranscriptItem::ToolResult { result, .. } => Some(Message {
            role: Role::Tool,
            content: Content::Text(result.output.to_string()),
            reasoning_content: None,
        }),
        TranscriptItem::SystemNote { level, message, .. } => Some(Message::system(format!(
            "{} {}",
            note_prefix(level),
            message
        ))),
    }
}

fn input_envelope_to_message(input: InputEnvelope) -> Message {
    let text = format_input_parts(input.parts);
    let role = match input.source {
        InputSource::User => Role::User,
        InputSource::Tool => Role::Tool,
        InputSource::System => Role::System,
    };

    Message {
        role,
        content: Content::Text(text),
        reasoning_content: None,
    }
}

fn tool_call_as_text(call: &ToolCall) -> String {
    format!(
        "[tool_call] id={} name={} args={}",
        call.call_id, call.tool_name, call.arguments
    )
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
        LlmError::RateLimit { message, retry_after } => {
            AgentError::Transient(TransientError::RateLimit {
                message,
                retry_after_ms: retry_after.map(|d| d.as_millis() as u64),
            })
        }
        LlmError::NetworkError { message } => {
            AgentError::Transient(TransientError::Network {
                message,
                retry_after_ms: None,
            })
        }
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
    use bigmodel_api::{ChoiceChunk, Delta, DeltaToolCall, DeltaToolFunction};

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
        assert!(matches!(&converted.messages[0].role, Role::System));
        assert_eq!(message_text(&converted.messages[0]), "be helpful");
        assert!(matches!(&converted.messages[1].role, Role::Assistant));
        assert_eq!(message_text(&converted.messages[1]), "previous");
        assert!(matches!(&converted.messages[2].role, Role::User));
        assert_eq!(message_text(&converted.messages[2]), "hello");
    }

    #[test]
    fn default_config_uses_supported_model_name() {
        let cfg = BigModelAdapterConfig::default();
        assert_eq!(cfg.model, "glm-5");
    }

    #[test]
    fn convert_request_includes_tools() {
        let req = ModelRequest {
            epoch: 0,
            transcript: vec![],
            inputs: vec![InputEnvelope::user_text("hi")],
            tools: vec![tool_spec_echo()],
        };

        let out = convert_model_request(req, &BigModelAdapterConfig::default());
        assert!(out.tools.is_some());
        assert_eq!(out.tools.expect("tools").len(), 1);
    }

    #[test]
    fn transcript_tool_result_maps_to_tool_message() {
        let item = TranscriptItem::ToolResult {
            id: new_id(),
            epoch: 2,
            result: ToolResult::ok("call-1", serde_json::json!({"ok": true})),
        };

        let message = transcript_item_to_message(item).expect("message");
        assert!(matches!(message.role, Role::Tool));
        assert_eq!(message_text(&message), "{\"ok\":true}");
    }

    #[test]
    fn stream_chunk_emits_text_and_reasoning_deltas() {
        let chunk = ChatResponseChunk {
            id: "chunk-1".to_string(),
            created: 0,
            model: "glm-test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: Some("hello".to_string()),
                    reasoning_content: Some("thinking".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
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
    fn stream_chunk_emits_tool_call_event() {
        let chunk = make_tool_call_chunk("c1", "shell", r#"{"command":"echo ok"}"#);
        let (tx, mut rx) = mpsc::unbounded_channel();
        emit_chunk(chunk, &tx);

        let item = rx.try_recv().expect("event").expect("ok");
        assert!(matches!(item, ModelOutputEvent::ToolCall { .. }));
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
        let chunk = ChatResponseChunk {
            id: "chunk-usage".to_string(),
            created: 0,
            model: "glm-test".to_string(),
            choices: vec![],
            usage: Some(bigmodel_api::Usage {
                prompt_tokens: 12,
                completion_tokens: 34,
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

    fn message_text(message: &Message) -> &str {
        match &message.content {
            Content::Text(text) => text,
            _ => "",
        }
    }

    fn make_tool_call_chunk(call_id: &str, name: &str, arguments: &str) -> ChatResponseChunk {
        ChatResponseChunk {
            id: "chunk-tool".to_string(),
            created: 0,
            model: "glm-test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![DeltaToolCall {
                        id: Some(call_id.to_string()),
                        type_field: Some("function".to_string()),
                        function: Some(DeltaToolFunction {
                            name: Some(name.to_string()),
                            arguments: Some(arguments.to_string()),
                        }),
                        index: Some(0),
                    }]),
                },
                finish_reason: None,
            }],
            usage: None,
        }
    }
}
