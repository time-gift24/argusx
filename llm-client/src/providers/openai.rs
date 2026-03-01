use crate::error::LlmError;
use crate::{LlmChunk, LlmChunkStream, LlmMessage, LlmRequest, LlmResponse, LlmRole, LlmToolCall, LlmUsage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub api_key: String,
    pub headers: HashMap<String, String>,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            headers: HashMap::new(),
        }
    }
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    type_field: &'static str,
    function: OpenAiFunction,
}

#[derive(Serialize)]
struct OpenAiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    id: String,
    created: i64,
    model: String,
    #[serde(default)]
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCallResp>>,
}

#[derive(Deserialize)]
struct OpenAiToolCallResp {
    id: Option<String>,
    function: Option<OpenAiToolFunctionResp>,
}

#[derive(Deserialize)]
struct OpenAiToolFunctionResp {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

pub(crate) struct OpenAiAdapter {
    http: reqwest::Client,
    config: OpenAiConfig,
}

impl OpenAiAdapter {
    pub(crate) fn new(config: OpenAiConfig) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build OpenAI HTTP client");

        Self { http, config }
    }

    async fn chat_inner(&self, req: LlmRequest, stream_mode: bool) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        let payload = OpenAiRequest {
            model: req.model,
            messages: req
                .messages
                .into_iter()
                .map(|m| OpenAiMessage {
                    role: role_to_openai(&m),
                    content: m.content,
                })
                .collect(),
            stream: stream_mode,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            top_p: req.top_p,
            tools: req.tools.map(|tools| {
                tools
                    .into_iter()
                    .map(|tool| OpenAiTool {
                        type_field: "function",
                        function: OpenAiFunction {
                            name: tool.name,
                            description: tool.description,
                            parameters: tool.parameters,
                        },
                    })
                    .collect()
            }),
        };

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .headers(to_header_map(&self.config.headers))
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::from_http_status(status.as_u16(), body, &headers));
        }

        let body: OpenAiResponse = response.json().await.map_err(|err| LlmError::ParseError {
            message: err.to_string(),
        })?;

        let choice = body.choices.into_iter().next();
        let output_text = choice
            .as_ref()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();
        let finish_reason = choice.as_ref().and_then(|c| c.finish_reason.clone());
        let tool_calls = choice
            .as_ref()
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|calls| {
                calls
                    .iter()
                    .map(|call| {
                        serde_json::json!({
                            "id": call.id,
                            "name": call.function.as_ref().and_then(|f| f.name.clone()),
                            "arguments": call.function.as_ref().and_then(|f| f.arguments.clone()),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(LlmResponse {
            id: body.id,
            created: body.created,
            model: body.model,
            output_text,
            finish_reason,
            request_id: None,
            usage: body.usage.map(|u| LlmUsage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            extensions: serde_json::json!({ "tool_calls": tool_calls }),
        })
    }
}

fn role_to_openai(message: &LlmMessage) -> &'static str {
    match message.role {
        LlmRole::System => "system",
        LlmRole::User => "user",
        LlmRole::Assistant => "assistant",
        LlmRole::Tool => "tool",
    }
}

fn to_header_map(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
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

#[async_trait::async_trait]
impl crate::ProviderAdapter for OpenAiAdapter {
    fn id(&self) -> &str {
        "openai"
    }

    async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.chat_inner(req, false).await
    }

    fn chat_stream(&self, req: LlmRequest) -> LlmChunkStream {
        let client = self.http.clone();
        let cfg = self.config.clone();
        Box::pin(async_stream::stream! {
            let adapter = OpenAiAdapter { http: client, config: cfg };
            match adapter.chat_inner(req, false).await {
                Ok(resp) => {
                    let tool_calls = resp
                        .extensions
                        .get("tool_calls")
                        .and_then(|value| serde_json::from_value::<Vec<serde_json::Value>>(value.clone()).ok())
                        .map(|calls| {
                            calls
                                .into_iter()
                                .map(|call| LlmToolCall {
                                    call_id: call.get("id").and_then(|v| v.as_str()).map(ToString::to_string),
                                    tool_name: call.get("name").and_then(|v| v.as_str()).map(ToString::to_string),
                                    arguments: call.get("arguments").map(|v| {
                                        if let Some(s) = v.as_str() {
                                            s.to_string()
                                        } else {
                                            v.to_string()
                                        }
                                    }),
                                })
                                .collect::<Vec<_>>()
                        })
                        .filter(|calls| !calls.is_empty());

                    yield Ok(LlmChunk {
                        id: resp.id,
                        created: resp.created,
                        model: resp.model,
                        delta_text: (!resp.output_text.is_empty()).then_some(resp.output_text),
                        delta_reasoning: None,
                        delta_tool_calls: tool_calls,
                        finish_reason: resp.finish_reason,
                        usage: resp.usage,
                    });
                }
                Err(err) => {
                    yield Err(err);
                }
            }
        })
    }
}
