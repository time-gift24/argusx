use crate::{
    Dialect, Error, ErrorKind, Mapper, ProviderConfig, ProviderStreamMode, ReplayReader, Request,
    SseRecorder, StreamError,
};
use argus_core::{Error as CoreError, ResponseContract, ResponseEvent, ResponseStream};
use bytes::Bytes;
use eventsource_stream::{Event as SseMessage, EventStreamError, Eventsource};
use futures::{Stream, StreamExt};
use futures::stream::{self, BoxStream};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Error as ReqwestError, Response};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{Instrument, info, info_span, warn};

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
        let model = request.model.clone();
        if let Some(ProviderStreamMode::Replay { file_path, timing }) =
            self.config.dev.as_ref().map(|dev| &dev.stream_mode)
        {
            return self.stream_replay(dialect, model, file_path.clone(), *timing);
        }

        self.stream_live(dialect, model, request)
    }

    fn stream_live(&self, dialect: Dialect, model: String, request: Request) -> Result<ResponseStream, Error> {
        let url = self.config.chat_completions_url();
        let http = self.http.clone();
        let api_key = self.config.api_key.clone();
        let headers = self.config.headers.clone();
        let record_target = self
            .config
            .dev
            .as_ref()
            .and_then(|dev| dev.record_live_sse.clone());
        let record_enabled = record_target.is_some();
        let request = request.normalized_for_send();
        let (tx, rx) = mpsc::channel(32);
        let span = info_span!(
            "provider.stream",
            dialect = ?dialect,
            mode = "live",
            model = %model,
            record_enabled
        );

        let producer = tokio::spawn(async move {
            info!("stream started");
            let mut recorder = match record_target {
                Some(target) => {
                    match SseRecorder::create(target.file_path, target.write_timing_sidecar)
                        .await
                    {
                        Ok(recorder) => Some(recorder),
                        Err(err) => {
                            warn!(error = %err, "failed to create recorder");
                            None
                        }
                    }
                }
                None => None,
            };
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
                        StreamError {
                            kind: ErrorKind::Transport,
                            message: err.to_string(),
                        },
                    )
                    .await;
                    return;
                }
            };

            let sse = match into_sse_message_stream(response).await {
                Ok(sse) => sse,
                Err(err) => {
                    send_terminal_error(&tx, err).await;
                    return;
                }
            };
            let mut payloads = sse
                .map(|item| {
                item.map(|message| message.data).map_err(|err| StreamError {
                    kind: classify_eventsource_error(&err),
                    message: err.to_string(),
                })
                })
                .boxed();
            drive_payload_stream(&tx, dialect, &mut payloads, &mut recorder).await;
        }
        .instrument(span));

        Ok(ResponseStream::from_parts(rx, producer.abort_handle()))
    }

    fn stream_replay(
        &self,
        dialect: Dialect,
        model: String,
        file_path: std::path::PathBuf,
        timing: crate::ReplayTiming,
    ) -> Result<ResponseStream, Error> {
        let prepared = crate::replay::prepare(file_path, timing)?;
        let (tx, rx) = mpsc::channel(32);
        let span = info_span!(
            "provider.stream",
            dialect = ?dialect,
            mode = "replay",
            model = %model,
            record_enabled = false
        );

        let producer = tokio::spawn(async move {
            info!("stream started");
            let replay = ReplayReader::from_prepared(prepared);
            let mut payloads = replay
                .map(|item| item.and_then(|frame| parse_replay_frame(&frame)))
                .boxed();
            let mut recorder = None;
            drive_payload_stream(&tx, dialect, &mut payloads, &mut recorder).await;
        }
        .instrument(span));

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
    err: StreamError,
) {
    let mut contract = ResponseContract::new();
    let event = ResponseEvent::Error(CoreError {
        message: format!("{:?}: {}", err.kind, err.message),
    });

    if contract.accept(&event).is_ok() {
        let _ = tx.send(event).await;
    }
}

async fn drive_payload_stream<S>(
    tx: &mpsc::Sender<ResponseEvent>,
    dialect: Dialect,
    payloads: &mut S,
    recorder: &mut Option<SseRecorder>,
) where
    S: Stream<Item = Result<String, StreamError>> + Unpin,
{
    let mut contract = ResponseContract::new();
    let mut mapper = Mapper::new(dialect);

    while let Some(item) = payloads.next().await {
        match item {
            Ok(payload) => {
                write_payload_to_recorder(recorder, &payload).await;
                if payload == "[DONE]" {
                    match mapper.on_done() {
                        Ok(events) => {
                            if emit_events(tx, &mut contract, events).await.is_err() {
                                finish_recorder(recorder).await;
                                return;
                            }
                        }
                        Err(err) => {
                            send_terminal_error(
                                tx,
                                StreamError {
                                    kind: classify_mapper_error(&err),
                                    message: err.to_string(),
                                },
                            )
                            .await;
                            finish_recorder(recorder).await;
                        }
                    }
                    info!("stream completed");
                    finish_recorder(recorder).await;
                    return;
                }

                match mapper.feed(&payload) {
                    Ok(events) => {
                        if emit_events(tx, &mut contract, events).await.is_err() {
                            finish_recorder(recorder).await;
                            return;
                        }
                    }
                    Err(err) => {
                        send_terminal_error(
                            tx,
                            StreamError {
                                kind: classify_mapper_error(&err),
                                message: err.to_string(),
                            },
                        )
                        .await;
                        finish_recorder(recorder).await;
                        return;
                    }
                }
            }
            Err(err) => {
                send_terminal_error(tx, err).await;
                finish_recorder(recorder).await;
                return;
            }
        }
    }

    send_terminal_error(
        tx,
        StreamError {
            kind: ErrorKind::Protocol,
            message: "stream ended before [DONE]".into(),
        },
    )
    .await;
    finish_recorder(recorder).await;
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

fn parse_replay_frame(frame: &str) -> Result<String, StreamError> {
    let data_lines: Vec<&str> = frame
        .lines()
        .filter_map(|line| {
            line.strip_prefix("data:")
                .map(|payload| payload.strip_prefix(' ').unwrap_or(payload))
        })
        .collect();

    if data_lines.is_empty() {
        return Err(StreamError {
            kind: ErrorKind::Parse,
            message: "replay frame missing data field".into(),
        });
    }

    Ok(data_lines.join("\n"))
}

async fn write_payload_to_recorder(recorder: &mut Option<SseRecorder>, payload: &str) {
    let failed = match recorder.as_mut() {
        Some(active) => active
            .write_frame(&format_payload_as_sse_frame(payload))
            .await
            .is_err(),
        None => false,
    };

    if failed {
        warn!("failed to write recorder frame");
        *recorder = None;
    }
}

async fn finish_recorder(recorder: &mut Option<SseRecorder>) {
    if let Some(active) = recorder.as_mut() {
        let _ = active.finish().await;
    }
    *recorder = None;
}

fn format_payload_as_sse_frame(payload: &str) -> String {
    let mut frame = String::new();
    for line in payload.split('\n') {
        frame.push_str("data: ");
        frame.push_str(line);
        frame.push('\n');
    }
    frame.push('\n');
    frame
}
