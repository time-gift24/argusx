use std::collections::HashMap;
use std::pin::Pin;

use bigmodel_api::{
    ChatRequest, ChatResponse, ChatResponseChunk, FunctionDefinition, FunctionTool, Message, Role,
    Tool as BigModelTool,
};
use futures::{Stream, StreamExt};
use llm_client::sse::{parse_sse_stream_result, SseEvent};
use llm_client::{
    run_with_retry, LlmChunk, LlmChunkStream, LlmError, LlmRequest, LlmResponse, LlmRole,
    LlmToolCall, LlmUsage, ProviderAdapter, RetryPolicy, TimeoutConfig,
};
use tokio::time::timeout;

use crate::openai_compat::{send_chat_completions_request, ChatCompletionsConfig};

#[derive(Debug, Clone)]
pub struct BigModelConfig {
    pub base_url: String,
    pub api_key: String,
    pub headers: HashMap<String, String>,
}

impl BigModelConfig {
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

    fn transport_config(&self) -> Result<ChatCompletionsConfig, LlmError> {
        ChatCompletionsConfig::new(
            self.base_url.clone(),
            self.api_key.clone(),
            self.headers.clone(),
        )
    }
}

pub struct BigModelHttpClient {
    http: reqwest::Client,
    http_stream: reqwest::Client,
    config: BigModelConfig,
    retry: RetryPolicy,
    timeout: TimeoutConfig,
}

impl BigModelHttpClient {
    pub fn new(config: BigModelConfig) -> Self {
        Self::with_options(config, RetryPolicy::default(), TimeoutConfig::default())
    }

    pub fn with_options(
        config: BigModelConfig,
        retry: RetryPolicy,
        timeout: TimeoutConfig,
    ) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(timeout.connect_timeout)
            .timeout(timeout.request_timeout)
            .build()
            .expect("failed to build HTTP client");

        let http_stream = reqwest::Client::builder()
            .connect_timeout(timeout.connect_timeout)
            .build()
            .expect("failed to build streaming HTTP client");

        Self {
            http,
            http_stream,
            config,
            retry,
            timeout,
        }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let http = self.http.clone();
        let transport = self.config.transport_config()?;
        let retry = self.retry.clone();

        run_with_retry(retry, move || {
            let http = http.clone();
            let transport = transport.clone();
            let request = request.clone();
            Box::pin(async move {
                let response =
                    send_chat_completions_request(&http, &transport, &request, false).await?;
                response.json().await.map_err(|err| LlmError::ParseError {
                    message: err.to_string(),
                })
            })
        })
        .await
    }

    pub fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponseChunk, LlmError>> + Send>> {
        let http = self.http_stream.clone();
        let transport = self.config.transport_config();
        let retry = self.retry.clone();
        let idle_timeout = self.timeout.stream_idle_timeout;

        Box::pin(async_stream::try_stream! {
            let transport = transport?;
            let response = run_with_retry(retry, || {
                let http = http.clone();
                let transport = transport.clone();
                let request = request.clone();
                Box::pin(async move {
                    send_chat_completions_request(&http, &transport, &request, true).await
                })
            })
            .await?;

            let byte_stream = response
                .bytes_stream()
                .map(|item| item.map_err(LlmError::from));
            let mut sse_events = std::pin::pin!(parse_sse_stream_result(byte_stream));

            loop {
                match timeout(idle_timeout, sse_events.next()).await {
                    Ok(Some(Ok(SseEvent::Data(json)))) => {
                        let chunk = parse_chunk(&json)?;
                        yield chunk;
                    }
                    Ok(Some(Ok(SseEvent::Done))) => return,
                    Ok(Some(Ok(SseEvent::Error(message)))) => {
                        Err(LlmError::StreamError { message })?;
                    }
                    Ok(Some(Err(err))) => {
                        Err(err)?;
                    }
                    Ok(None) => break,
                    Err(_) => Err(LlmError::StreamIdleTimeout)?,
                }
            }
        })
    }
}

fn parse_chunk(payload: &str) -> Result<ChatResponseChunk, LlmError> {
    serde_json::from_str(payload).map_err(|err| LlmError::ParseError {
        message: format!(
            "failed to parse SSE chunk: {}; payload={}",
            err,
            truncate_payload(payload, 200),
        ),
    })
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

pub struct BigModelAdapter {
    client: BigModelHttpClient,
}

impl BigModelAdapter {
    pub fn new(config: BigModelConfig) -> Self {
        Self {
            client: BigModelHttpClient::new(config),
        }
    }
}

#[async_trait::async_trait]
impl ProviderAdapter for BigModelAdapter {
    fn id(&self) -> &str {
        "bigmodel"
    }

    async fn chat(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let bigmodel_req = to_bigmodel_request(&req);
        let response = self.client.chat(bigmodel_req).await?;
        Ok(to_llm_response(&response))
    }

    fn chat_stream(&self, req: LlmRequest) -> LlmChunkStream {
        let stream_req = to_bigmodel_request(&req).stream();
        let stream = self.client.chat_stream(stream_req);
        Box::pin(stream.map(|item| item.map(to_llm_chunk)))
    }
}

pub fn to_bigmodel_request(req: &LlmRequest) -> ChatRequest {
    let messages: Vec<Message> = req
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                LlmRole::System => Role::System,
                LlmRole::User => Role::User,
                LlmRole::Assistant => Role::Assistant,
                LlmRole::Tool => Role::Tool,
            };
            Message {
                role,
                content: m.content.clone().into(),
                reasoning_content: None,
            }
        })
        .collect();

    let tools = req.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|t| {
                BigModelTool::Function(FunctionTool {
                    function: FunctionDefinition {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.parameters.clone(),
                    },
                })
            })
            .collect()
    });

    ChatRequest {
        model: req.model.clone(),
        messages,
        do_sample: None,
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        stream: req.stream,
        tool_stream: None,
        tools,
        tool_choice: None,
        stop: None,
        response_format: None,
        request_id: None,
        user_id: None,
        thinking: None,
    }
}

pub fn to_llm_response(resp: &ChatResponse) -> LlmResponse {
    let first_choice = resp.choices.first();
    let output_text = first_choice
        .and_then(|c| match &c.message.content {
            bigmodel_api::Content::Text(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let finish_reason = first_choice.map(|c| c.finish_reason.clone());
    let usage = resp.usage.as_ref().map(|u| LlmUsage {
        input_tokens: u.prompt_tokens.try_into().unwrap_or(0),
        output_tokens: u.completion_tokens.try_into().unwrap_or(0),
        total_tokens: u.total_tokens.try_into().unwrap_or(0),
    });

    let mut extensions = serde_json::json!({
        "web_search": resp.web_search,
        "video_result": resp.video_result,
        "content_filter": resp.content_filter,
    });

    if let Some(reasoning) = first_choice.and_then(|c| c.message.reasoning_content.clone()) {
        if let Some(obj) = extensions.as_object_mut() {
            obj.insert(
                "reasoning_content".to_string(),
                serde_json::Value::String(reasoning),
            );
        }
    }

    LlmResponse {
        id: resp.id.clone(),
        request_id: resp.request_id.clone(),
        created: resp.created,
        model: resp.model.clone(),
        output_text,
        finish_reason,
        usage,
        extensions,
    }
}

pub fn to_llm_chunk(chunk: ChatResponseChunk) -> LlmChunk {
    let delta = chunk.choices.first().map(|c| &c.delta);

    let delta_text = delta.and_then(|d| d.content.clone());
    let delta_reasoning = delta.and_then(|d| d.reasoning_content.clone());
    let finish_reason = chunk.choices.first().and_then(|c| c.finish_reason.clone());

    let delta_tool_calls = delta.and_then(|d| {
        d.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|tc| LlmToolCall {
                    call_id: tc.id.clone(),
                    tool_name: tc.function.as_ref().and_then(|f| f.name.clone()),
                    arguments: tc.function.as_ref().and_then(|f| f.arguments.clone()),
                })
                .collect()
        })
    });

    let usage = chunk.usage.as_ref().map(|u| LlmUsage {
        input_tokens: u.prompt_tokens.try_into().unwrap_or(0),
        output_tokens: u.completion_tokens.try_into().unwrap_or(0),
        total_tokens: u.total_tokens.try_into().unwrap_or(0),
    });

    LlmChunk {
        id: chunk.id,
        created: chunk.created,
        model: chunk.model,
        delta_text,
        delta_reasoning,
        delta_tool_calls,
        finish_reason,
        usage,
    }
}
