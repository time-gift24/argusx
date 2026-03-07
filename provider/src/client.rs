use crate::{Dialect, Error, ErrorKind, Mapper, ProviderConfig, Request, StreamError};
use argus_core::{Error as CoreError, ResponseContract, ResponseEvent, ResponseStream, Usage};
use bytes::Bytes;
use eventsource_stream::{Event as SseMessage, EventStreamError, Eventsource};
use futures::StreamExt;
use futures::stream::{self, BoxStream};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Error as ReqwestError, Response};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::Instrument;

type MessageStream = BoxStream<'static, Result<SseMessage, EventStreamError<ReqwestError>>>;

pub struct ProviderClient {
    http: reqwest::Client,
    config: ProviderConfig,
}

impl ProviderClient {
    pub fn new(config: ProviderConfig) -> Result<Self, Error> {
        if config.base_url.trim().is_empty() {
            return Err(Error::Config("base_url is required".into()));
        }
        if config.api_key.trim().is_empty() {
            return Err(Error::Config("api_key is required".into()));
        }

        Ok(Self {
            http: reqwest::Client::new(),
            config,
        })
    }

    pub fn stream(&self, request: Request) -> Result<ResponseStream, Error> {
        self.stream_dialect(self.config.dialect, request)
    }

    fn stream_dialect(&self, dialect: Dialect, request: Request) -> Result<ResponseStream, Error> {
        let url = self.config.chat_completions_url();
        let http = self.http.clone();
        let api_key = self.config.api_key.clone();
        let headers = self.config.headers.clone();
        let request = request.normalized_for_send();
        let (tx, rx) = mpsc::channel(32);
        let provider = format!("{:?}", dialect);
        let span = tracing::Span::current();

        let producer = tokio::spawn(
            async move {
                // Emit llm_request event at the start
                tracing::info!(
                    event_name = "llm_request",
                    provider = provider.as_str(),
                );

                let response = match http
                    .post(url)
                    .header(reqwest::header::AUTHORIZATION, format!("Bearer {api_key}"))
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .header(reqwest::header::ACCEPT, "text/event-stream")
                    .headers(to_header_map(&headers))
                    .json(&request)
                    .send()
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        send_terminal_error(
                            &tx,
                            &mut ResponseContract::new(),
                            StreamError {
                                kind: ErrorKind::Transport,
                                message: err.to_string(),
                            },
                        )
                        .await;
                        return;
                    }
                };

                let mut contract = ResponseContract::new();
                let mut sse = match into_sse_message_stream(response).await {
                    Ok(sse) => sse,
                    Err(err) => {
                        send_terminal_error(&tx, &mut contract, err).await;
                        return;
                    }
                };
                let mut mapper = Mapper::new(dialect);
                let mut final_usage: Option<Usage> = None;

                while let Some(item) = sse.next().await {
                    match item {
                        Ok(message) => {
                            if message.data == "[DONE]" {
                                match mapper.on_done() {
                                    Ok(events) => {
                                        // Extract usage from Done event
                                        for event in &events {
                                            if let ResponseEvent::Done { usage, .. } = event {
                                                final_usage = usage.clone();
                                            }
                                        }

                                        // Emit llm_response_completed event
                                        emit_completion_event(&final_usage);

                                        if emit_events(&tx, &mut contract, events).await.is_err() {
                                            return;
                                        }
                                    }
                                    Err(err) => {
                                        send_terminal_error(
                                            &tx,
                                            &mut contract,
                                            StreamError {
                                                kind: classify_mapper_error(&err),
                                                message: err.to_string(),
                                            },
                                        )
                                        .await;
                                    }
                                }
                                return;
                            }

                            match mapper.feed(&message.data) {
                                Ok(events) => {
                                    // Track usage from streaming events
                                    for event in &events {
                                        if let ResponseEvent::Done { usage, .. } = event {
                                            final_usage = usage.clone();
                                        }
                                    }

                                    if emit_events(&tx, &mut contract, events).await.is_err() {
                                        return;
                                    }
                                }
                                Err(err) => {
                                    send_terminal_error(
                                        &tx,
                                        &mut contract,
                                        StreamError {
                                            kind: classify_mapper_error(&err),
                                            message: err.to_string(),
                                        },
                                    )
                                    .await;
                                    return;
                                }
                            }
                        }
                        Err(err) => {
                            send_terminal_error(
                                &tx,
                                &mut contract,
                                StreamError {
                                    kind: classify_eventsource_error(&err),
                                    message: err.to_string(),
                                },
                            )
                            .await;
                            return;
                        }
                    }
                }

                send_terminal_error(
                    &tx,
                    &mut contract,
                    StreamError {
                        kind: ErrorKind::Protocol,
                        message: "stream ended before [DONE]".into(),
                    },
                )
                .await;
            }
            .instrument(span),
        );

        Ok(ResponseStream::from_parts(rx, producer.abort_handle()))
    }
}

async fn emit_events(
    tx: &mpsc::Sender<ResponseEvent>,
    contract: &mut ResponseContract,
    events: Vec<ResponseEvent>,
) -> Result<(), StreamError> {
    for event in events {
        contract.accept(&event).map_err(|_| StreamError {
            kind: ErrorKind::Protocol,
            message: "event after terminal".into(),
        })?;
        tx.send(event).await.map_err(|_| StreamError {
            kind: ErrorKind::Cancelled,
            message: "response stream receiver dropped".into(),
        })?;
    }

    Ok(())
}

async fn send_terminal_error(
    tx: &mpsc::Sender<ResponseEvent>,
    contract: &mut ResponseContract,
    err: StreamError,
) {
    let event = ResponseEvent::Error(CoreError {
        message: format!("{:?}: {}", err.kind, err.message),
    });

    if contract.accept(&event).is_ok() {
        let _ = tx.send(event).await;
    }
}

fn to_header_map(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (key, value) in headers {
        if key.trim().is_empty() {
            continue;
        }

        let Ok(name) = HeaderName::from_bytes(key.trim().as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(value) else {
            continue;
        };
        map.insert(name, value);
    }
    map
}

async fn into_sse_message_stream(response: Response) -> Result<MessageStream, StreamError> {
    let response = check_response(response).await?;
    let eof_flush = stream::once(async { Ok::<Bytes, ReqwestError>(Bytes::from_static(b"\n\n")) });
    Ok(response
        .bytes_stream()
        .chain(eof_flush)
        .eventsource()
        .boxed())
}

async fn check_response(response: Response) -> Result<Response, StreamError> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default().trim().to_owned();
        return Err(StreamError {
            kind: ErrorKind::HttpStatus,
            message: if body.is_empty() {
                format!("unexpected HTTP status {status}")
            } else {
                format!("unexpected HTTP status {status}: {body}")
            },
        });
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let valid_content_type = content_type
        .as_deref()
        .map(is_event_stream_content_type)
        .unwrap_or(false);

    if !valid_content_type {
        return Err(StreamError {
            kind: ErrorKind::HttpStatus,
            message: format!("invalid content-type for SSE: {content_type:?}"),
        });
    }

    Ok(response)
}

fn is_event_stream_content_type(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .map(str::trim)
        .map(|mime| mime.eq_ignore_ascii_case("text/event-stream"))
        .unwrap_or(false)
}

fn classify_eventsource_error(err: &EventStreamError<ReqwestError>) -> ErrorKind {
    match err {
        EventStreamError::Utf8(_) | EventStreamError::Parser(_) => ErrorKind::Parse,
        EventStreamError::Transport(_) => ErrorKind::Transport,
    }
}

fn classify_mapper_error(err: &Error) -> ErrorKind {
    match err {
        Error::Openai(crate::dialect::openai::mapper::Error::Parse(_))
        | Error::Zai(crate::dialect::zai::mapper::Error::Parse(_)) => ErrorKind::Parse,
        Error::Openai(crate::dialect::openai::mapper::Error::Protocol(_))
        | Error::Zai(crate::dialect::zai::mapper::Error::Protocol(_)) => ErrorKind::Protocol,
        Error::Config(_) => ErrorKind::Protocol,
    }
}

fn emit_completion_event(usage: &Option<Usage>) {
    let billing_key = uuid::Uuid::new_v4().to_string();
    match usage {
        Some(u) => {
            tracing::info!(
                event_name = "llm_response_completed",
                event_priority = "high",
                input_tokens = u.input_tokens,
                output_tokens = u.output_tokens,
                total_tokens = u.total_tokens,
                billing_dedupe_key = billing_key.as_str()
            );
        }
        None => {
            tracing::info!(
                event_name = "llm_response_completed",
                event_priority = "high",
                billing_dedupe_key = billing_key.as_str()
            );
        }
    }
}
