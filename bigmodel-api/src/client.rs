use futures::Stream;
use reqwest::Client;
use std::pin::Pin;

use crate::config::Config;
use crate::error::{BigModelError, Result};
use crate::models::{ChatRequest, ChatResponse, ChatResponseChunk};

pub struct BigModelClient {
    config: Config,
    http: Client,
}

impl BigModelClient {
    pub fn new(config: Config) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self { config, http }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let chat_response: ChatResponse = response.json().await?;
            Ok(chat_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(Self::handle_error(status.as_u16(), error_text))
        }
    }

    pub fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponseChunk>> + Send + '_>> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let api_key = self.config.api_key.clone();
        let http_client = self.http.clone();

        Box::pin(async_stream::try_stream! {
            let response = http_client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .json(&request)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                let mut stream = response.bytes_stream();

                use futures::stream::StreamExt;
                let mut buffer = String::new();

                while let Some(chunk) = stream.next().await {
                    let bytes: bytes::Bytes = chunk?;
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        buffer.push_str(&text);

                        // Process complete lines, keep incomplete ones in buffer
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim_end().to_string();
                            buffer = buffer[pos + 1..].to_string();

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    return;
                                }
                                match serde_json::from_str::<ChatResponseChunk>(data) {
                                    Ok(response) => yield response,
                                    Err(e) => {
                                        // Log or handle parse error appropriately
                                        eprintln!("Failed to parse SSE chunk: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                let error_text = response.text().await.unwrap_or_default();
                Err(Self::handle_error(status.as_u16(), error_text))?
            }
        })
    }

    fn handle_error(status: u16, body: String) -> BigModelError {
        match status {
            400 => BigModelError::InvalidRequest(body),
            401 | 403 => BigModelError::AuthenticationError(body),
            429 => BigModelError::RateLimitError(body),
            500..=599 => BigModelError::ServerError(body),
            _ => BigModelError::ServerError(format!("Unknown error: {} - {}", status, body)),
        }
    }
}
