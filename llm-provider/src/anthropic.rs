use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use llm_client::error::LlmError;
use llm_client::{
    LlmChunk, LlmChunkStream, LlmMessage, LlmRequest, LlmResponse, LlmRole, LlmUsage,
    ProviderAdapter,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub base_url: String,
    pub api_key: String,
    pub headers: HashMap<String, String>,
}

impl AnthropicConfig {
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
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    type_field: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

pub struct AnthropicAdapter {
    http: reqwest::Client,
    config: AnthropicConfig,
}

impl AnthropicAdapter {
    pub fn new(config: AnthropicConfig) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build Anthropic HTTP client");

        Self { http, config }
    }

    async fn chat_inner(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/messages", self.config.base_url);

        let mut system_messages = Vec::new();
        let mut messages = Vec::new();
        for msg in req.messages {
            if matches!(msg.role, LlmRole::System) {
                system_messages.push(msg.content);
                continue;
            }
            messages.push(AnthropicMessage {
                role: role_to_anthropic(&msg),
                content: msg.content,
            });
        }

        let payload = AnthropicRequest {
            model: req.model,
            max_tokens: req.max_tokens.unwrap_or(1024),
            messages,
            system: (!system_messages.is_empty()).then(|| system_messages.join("\n")),
            temperature: req.temperature,
            top_p: req.top_p,
        };

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
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

        let body: AnthropicResponse = response.json().await.map_err(|err| LlmError::ParseError {
            message: err.to_string(),
        })?;

        let output_text = body
            .content
            .iter()
            .filter_map(|part| {
                if part.type_field == "text" {
                    part.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(LlmResponse {
            id: body.id,
            request_id: None,
            created: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            model: body.model,
            output_text,
            finish_reason: body.stop_reason,
            usage: body.usage.map(|u| LlmUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.input_tokens.saturating_add(u.output_tokens),
            }),
            extensions: serde_json::json!({}),
        })
    }
}

fn role_to_anthropic(message: &LlmMessage) -> &'static str {
    match message.role {
        LlmRole::Assistant => "assistant",
        LlmRole::System => "user",
        LlmRole::Tool => "user",
        LlmRole::User => "user",
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
impl ProviderAdapter for AnthropicAdapter {
    fn id(&self) -> &str {
        "anthropic"
    }

    async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.chat_inner(req).await
    }

    fn chat_stream(&self, req: LlmRequest) -> LlmChunkStream {
        let client = self.http.clone();
        let cfg = self.config.clone();
        Box::pin(async_stream::stream! {
            let adapter = AnthropicAdapter { http: client, config: cfg };
            match adapter.chat_inner(req).await {
                Ok(resp) => {
                    yield Ok(LlmChunk {
                        id: resp.id,
                        created: resp.created,
                        model: resp.model,
                        delta_text: (!resp.output_text.is_empty()).then_some(resp.output_text),
                        delta_reasoning: None,
                        delta_tool_calls: None,
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
