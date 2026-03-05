use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use llm_client::error::LlmError;
use llm_client::sse::{Event as SseEvent, EventSource};
use llm_client::{
    LlmChunk, LlmChunkStream, LlmRequest, LlmResponse, LlmRole, LlmTool, LlmToolCall, LlmUsage,
    ProviderAdapter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ChatCompletionsConfig {
    pub base_url: String,
    pub api_key: String,
    pub headers: HashMap<String, String>,
}

impl ChatCompletionsConfig {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self, LlmError> {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(LlmError::InvalidRequest {
                message: "base_url is required".to_string(),
            });
        }

        let api_key = api_key.into().trim().to_string();
        if api_key.is_empty() {
            return Err(LlmError::InvalidRequest {
                message: "api_key is required".to_string(),
            });
        }

        Ok(Self {
            base_url,
            api_key,
            headers,
        })
    }

    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

pub fn to_header_map(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (key, value) in headers {
        if key.trim().is_empty() {
            continue;
        }
        let Ok(name) = reqwest::header::HeaderName::from_bytes(key.trim().as_bytes()) else {
            continue;
        };
        let Ok(val) = reqwest::header::HeaderValue::from_str(value) else {
            continue;
        };
        map.insert(name, val);
    }
    map
}

pub async fn send_chat_completions_request<T: Serialize>(
    http: &reqwest::Client,
    config: &ChatCompletionsConfig,
    payload: &T,
    stream: bool,
) -> Result<reqwest::Response, LlmError> {
    let mut request = http
        .post(config.chat_completions_url())
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .headers(to_header_map(&config.headers))
        .json(payload);

    if stream {
        request = request.header("Accept", "text/event-stream");
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let headers = response.headers().clone();
        let body = response.text().await.unwrap_or_default();
        return Err(LlmError::from_http_status(status.as_u16(), body, &headers));
    }

    Ok(response)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionsMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionsTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsTool {
    #[serde(rename = "type")]
    pub type_field: String,
    pub function: ChatCompletionsFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsResponse {
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    pub model: String,
    #[serde(default)]
    pub choices: Vec<ChatCompletionsChoice>,
    #[serde(default)]
    pub usage: Option<ChatCompletionsUsage>,
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsChoice {
    #[serde(default)]
    pub index: Option<i32>,
    pub message: ChatCompletionsResponseMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsResponseMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ChatCompletionsToolCallResponse>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsToolCallResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub type_field: Option<String>,
    #[serde(default)]
    pub function: Option<ChatCompletionsToolFunctionResponse>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsToolFunctionResponse {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsChunk {
    pub id: String,
    #[serde(default)]
    pub created: Option<i64>,
    pub model: String,
    #[serde(default)]
    pub choices: Vec<ChatCompletionsChoiceChunk>,
    #[serde(default)]
    pub usage: Option<ChatCompletionsUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionsChoiceChunk {
    #[serde(default)]
    pub index: Option<i32>,
    #[serde(default)]
    pub delta: ChatCompletionsDelta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ChatCompletionsDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ChatCompletionsToolCallResponse>>,
}

pub fn to_chat_completions_request(req: LlmRequest, stream: bool) -> ChatCompletionsRequest {
    ChatCompletionsRequest {
        model: req.model,
        messages: req
            .messages
            .into_iter()
            .map(|message| ChatCompletionsMessage {
                role: role_to_provider(message.role).to_string(),
                content: message.content,
                reasoning_content: None,
            })
            .collect(),
        stream,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        top_p: req.top_p,
        tools: req.tools.map(to_completions_tools),
    }
}

pub fn response_to_llm(resp: ChatCompletionsResponse) -> LlmResponse {
    let created = resp.created.unwrap_or_else(now_unix_seconds);

    let first_choice = resp.choices.first();
    let output_text = first_choice
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_default();
    let finish_reason = first_choice.and_then(|choice| choice.finish_reason.clone());
    let tool_calls = first_choice
        .and_then(|choice| choice.message.tool_calls.as_ref())
        .map(|calls| map_tool_calls(calls))
        .unwrap_or_default();

    let mut extensions = serde_json::Value::Object(resp.extensions);
    if let Some(obj) = extensions.as_object_mut() {
        obj.insert("tool_calls".to_string(), Value::Array(tool_calls));
        if let Some(reasoning) =
            first_choice.and_then(|choice| choice.message.reasoning_content.clone())
        {
            obj.insert("reasoning_content".to_string(), Value::String(reasoning));
        }
    }

    LlmResponse {
        id: resp.id,
        request_id: resp.request_id,
        created,
        model: resp.model,
        output_text,
        finish_reason,
        usage: resp.usage.map(map_usage),
        extensions,
    }
}

pub fn chunk_to_llm(chunk: ChatCompletionsChunk) -> LlmChunk {
    let (delta_text, delta_reasoning, delta_tool_calls, finish_reason) =
        if let Some(choice) = chunk.choices.first() {
            (
                choice.delta.content.clone(),
                choice.delta.reasoning_content.clone(),
                choice
                    .delta
                    .tool_calls
                    .as_ref()
                    .map(|calls| map_tool_calls_to_llm(calls)),
                choice.finish_reason.clone(),
            )
        } else {
            (None, None, None, None)
        };

    LlmChunk {
        id: chunk.id,
        created: chunk.created.unwrap_or_else(now_unix_seconds),
        model: chunk.model,
        delta_text,
        delta_reasoning,
        delta_tool_calls,
        finish_reason,
        usage: chunk.usage.map(map_usage),
    }
}

pub struct OpenAiCompatAdapter {
    http: reqwest::Client,
    config: ChatCompletionsConfig,
}

impl OpenAiCompatAdapter {
    pub fn new(config: ChatCompletionsConfig) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build OpenAI compat HTTP client");
        Self { http, config }
    }

    async fn chat_inner(&self, req: LlmRequest) -> Result<ChatCompletionsResponse, LlmError> {
        let payload = to_chat_completions_request(req, false);
        let response =
            send_chat_completions_request(&self.http, &self.config, &payload, false).await?;
        response.json().await.map_err(|err| LlmError::ParseError {
            message: err.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl ProviderAdapter for OpenAiCompatAdapter {
    fn id(&self) -> &str {
        "openai"
    }

    async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let response = self.chat_inner(req).await?;
        Ok(response_to_llm(response))
    }

    fn chat_stream(&self, req: LlmRequest) -> LlmChunkStream {
        let client = self.http.clone();
        let cfg = self.config.clone();
        Box::pin(async_stream::stream! {
            let payload = to_chat_completions_request(req, true);
            let response = match send_chat_completions_request(&client, &cfg, &payload, true).await {
                Ok(response) => response,
                Err(err) => {
                    yield Err(err);
                    return;
                }
            };

            let mut sse_events = match EventSource::from_response(response) {
                Ok(es) => es,
                Err(err) => {
                    yield Err(LlmError::from(err));
                    return;
                }
            };

            while let Some(item) = sse_events.next().await {
                match item {
                    Ok(SseEvent::Open) => continue,
                    Ok(SseEvent::Message(message)) => {
                        if message.data == "[DONE]" {
                            break;
                        }
                        if message.event == "error" {
                            yield Err(LlmError::StreamError {
                                message: message.data,
                            });
                            return;
                        }
                        let json = message.data;
                        if let Ok(value) = serde_json::from_str::<Value>(&json) {
                            if let Some(err_msg) = value
                                .get("error")
                                .and_then(|err| err.get("message").or(Some(err)))
                                .map(Value::to_string)
                            {
                                yield Err(LlmError::StreamError {
                                    message: err_msg,
                                });
                                return;
                            }
                        }

                        match serde_json::from_str::<ChatCompletionsChunk>(&json) {
                            Ok(chunk) => yield Ok(chunk_to_llm(chunk)),
                            Err(err) => {
                                yield Err(LlmError::ParseError {
                                    message: format!(
                                        "failed to parse chat completions chunk: {}; payload={}",
                                        err,
                                        truncate_payload(&json, 300)
                                    ),
                                });
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        yield Err(LlmError::from(err));
                        return;
                    }
                }
            }
        })
    }
}

fn role_to_provider(role: LlmRole) -> &'static str {
    match role {
        LlmRole::System => "system",
        LlmRole::User => "user",
        LlmRole::Assistant => "assistant",
        LlmRole::Tool => "tool",
    }
}

fn to_completions_tools(tools: Vec<LlmTool>) -> Vec<ChatCompletionsTool> {
    tools
        .into_iter()
        .map(|tool| ChatCompletionsTool {
            type_field: "function".to_string(),
            function: ChatCompletionsFunction {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
            },
        })
        .collect()
}

fn map_tool_calls(calls: &[ChatCompletionsToolCallResponse]) -> Vec<Value> {
    calls
        .iter()
        .map(|call| {
            serde_json::json!({
                "id": call.id,
                "name": call.function.as_ref().and_then(|f| f.name.clone()),
                "arguments": call.function.as_ref().and_then(|f| f.arguments.clone()),
            })
        })
        .collect()
}

fn map_tool_calls_to_llm(calls: &[ChatCompletionsToolCallResponse]) -> Vec<LlmToolCall> {
    calls
        .iter()
        .map(|call| LlmToolCall {
            call_id: call.id.clone(),
            tool_name: call.function.as_ref().and_then(|f| f.name.clone()),
            arguments: call.function.as_ref().and_then(|f| f.arguments.clone()),
        })
        .collect()
}

fn map_usage(usage: ChatCompletionsUsage) -> LlmUsage {
    LlmUsage {
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
    }
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn truncate_payload(payload: &str, max_len: usize) -> String {
    let mut chars = payload.chars();
    let truncated: String = chars.by_ref().take(max_len).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}
