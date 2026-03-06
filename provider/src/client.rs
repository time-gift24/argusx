use crate::{
    Dialect, Error, ErrorKind, Mapper, ProviderConfig, Request, StreamError,
};
use argus_core::{Error as CoreError, ResponseContract, ResponseEvent, ResponseStream};
use bytes::Bytes;
use eventsource_stream::{Event as SseMessage, EventStreamError, Eventsource};
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Error as ReqwestError, Response};
use std::collections::HashMap;
use tokio::sync::mpsc;

type MessageStream = BoxStream<'static, Result<SseMessage, EventStreamError<ReqwestError>>>;

pub struct ProviderClient {
    _http: reqwest::Client,
    _config: ProviderConfig,
}

impl ProviderClient {
    pub fn new(config: ProviderConfig) -> Result<Self, Error> {
        if config.base_url.trim().is_empty() {
            return Err(Error::Config("base_url is required".into()));
        }
        if config.api_key.trim().is_empty() {
            return Err(Error::Config("api_key is required".into()));
        }

        let _ = config.dialect;

        Ok(Self {
            _http: reqwest::Client::new(),
            _config: config,
        })
    }

    pub fn stream(&self, request: Request) -> Result<ResponseStream, Error> {
        self.stream_dialect(self._config.dialect, request)
    }

    fn stream_dialect(&self, dialect: Dialect, request: Request) -> Result<ResponseStream, Error> {
        let url = format!(
            "{}/chat/completions",
            self._config.base_url.trim_end_matches('/')
        );
        let http = self._http.clone();
        let api_key = self._config.api_key.clone();
        let headers = self._config.headers.clone();
        let request = request.normalized_for_send();
        let (tx, rx) = mpsc::channel(32);

        let producer = tokio::spawn(async move {
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
            let mut sse = match into_sse_message_stream(response) {
                Ok(sse) => sse,
                Err(err) => {
                    send_terminal_error(
                        &tx,
                        &mut contract,
                        err,
                    )
                    .await;
                    return;
                }
            };
            let mut mapper = Mapper::new(dialect);

            while let Some(item) = sse.next().await {
                match item {
                    Ok(message) => {
                        if message.data == "[DONE]" {
                            match mapper.on_done() {
                                Ok(events) => {
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
        });

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

fn into_sse_message_stream(response: Response) -> Result<MessageStream, StreamError> {
    let response = check_response(response)?;
    let eof_flush = stream::once(async { Ok::<Bytes, ReqwestError>(Bytes::from_static(b"\n\n")) });
    Ok(response.bytes_stream().chain(eof_flush).eventsource().boxed())
}

fn check_response(response: Response) -> Result<Response, StreamError> {
    if !response.status().is_success() {
        return Err(StreamError {
            kind: ErrorKind::HttpStatus,
            message: format!("unexpected HTTP status {}", response.status()),
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
