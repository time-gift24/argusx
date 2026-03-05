// Adapted from reqwest-eventsource v0.6.0 (MIT OR Apache-2.0).
// Local modifications:
// - Uses in-tree SSE parser/event stream (self-maintained).
// - Added `EventSource::from_response` to preserve existing HTTP error mapping path in providers.
// - Default retry policy changed to `Never` for LLM streaming safety.
// - Content-Type check is lenient by default (warn + continue).
// - Fixed retry counter increment logic (`n + 1`).

use crate::sse::error::{CannotCloneRequestError, Error};
use crate::sse::message_event::MessageEvent;
use crate::sse::retry::{RetryPolicy, DEFAULT_RETRY};
use crate::sse::traits::Eventsource;
use core::pin::Pin;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use pin_project_lite::pin_project;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Error as ReqwestError, IntoUrl, RequestBuilder, Response};
use std::time::Duration;
use tokio::time::{sleep, Sleep};

use tracing::warn;

/// Ready state of an `EventSource` stream.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum ReadyState {
    Connecting = 0,
    Open = 1,
    Closed = 2,
}

type ResponseFuture = BoxFuture<'static, Result<Response, ReqwestError>>;
type MessageStream = BoxStream<
    'static,
    Result<MessageEvent, crate::sse::event_stream::EventStreamError<ReqwestError>>,
>;
type BoxedRetry = Box<dyn RetryPolicy + Send + Unpin + 'static>;

pin_project! {
/// SSE event source stream.
#[project = EventSourceProjection]
pub struct EventSource {
    builder: Option<RequestBuilder>,
    #[pin]
    next_response: Option<ResponseFuture>,
    #[pin]
    cur_stream: Option<MessageStream>,
    #[pin]
    delay: Option<Pin<Box<Sleep>>>,
    is_closed: bool,
    retry_policy: BoxedRetry,
    last_event_id: String,
    last_retry: Option<(usize, Duration)>,
    enforce_content_type: bool,
    open_pending: bool,
}
}

impl EventSource {
    /// Build from a request builder (request is executed lazily through polling).
    pub fn new(builder: RequestBuilder) -> Result<Self, CannotCloneRequestError> {
        let builder = builder.header(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("text/event-stream"),
        );

        let response_future = Box::pin(builder.try_clone().ok_or(CannotCloneRequestError)?.send());

        Ok(Self {
            builder: Some(builder),
            next_response: Some(response_future),
            cur_stream: None,
            delay: None,
            is_closed: false,
            retry_policy: Box::new(DEFAULT_RETRY),
            last_event_id: String::new(),
            last_retry: None,
            enforce_content_type: false,
            open_pending: false,
        })
    }

    /// Build from an already successful HTTP response.
    pub fn from_response(response: Response) -> Result<Self, Error> {
        Self::from_response_with_options(response, false)
    }

    /// Build from response with explicit content-type enforcement setting.
    pub fn from_response_with_options(
        response: Response,
        enforce_content_type: bool,
    ) -> Result<Self, Error> {
        let mut this = Self {
            builder: None,
            next_response: None,
            cur_stream: None,
            delay: None,
            is_closed: false,
            retry_policy: Box::new(DEFAULT_RETRY),
            last_event_id: String::new(),
            last_retry: None,
            enforce_content_type,
            open_pending: true,
        };

        let response = check_response(response, enforce_content_type)?;
        this.handle_response(response);
        Ok(this)
    }

    /// Convenience GET constructor.
    pub fn get<T: IntoUrl>(url: T) -> Self {
        Self::new(reqwest::Client::new().get(url)).expect("cloneable request")
    }

    /// Close stream and stop reconnect attempts.
    pub fn close(&mut self) {
        self.is_closed = true;
    }

    /// Replace retry policy.
    pub fn set_retry_policy(&mut self, policy: BoxedRetry) {
        self.retry_policy = policy;
    }

    /// Enforce strict `text/event-stream` content-type check.
    pub fn set_enforce_content_type(&mut self, enforce: bool) {
        self.enforce_content_type = enforce;
    }

    /// Last observed SSE event ID.
    pub fn last_event_id(&self) -> &str {
        &self.last_event_id
    }

    /// Current ready state.
    pub fn ready_state(&self) -> ReadyState {
        if self.is_closed {
            ReadyState::Closed
        } else if self.delay.is_some() || self.next_response.is_some() {
            ReadyState::Connecting
        } else if self.cur_stream.is_some() || self.open_pending {
            ReadyState::Open
        } else {
            ReadyState::Connecting
        }
    }

    fn handle_response(&mut self, res: Response) {
        self.last_retry.take();
        let mut stream = res.bytes_stream().eventsource();
        stream.set_last_event_id(self.last_event_id.clone());
        stream.set_flush_on_eof(true);
        self.cur_stream = Some(stream.boxed());
    }
}

fn parse_content_type(value: Option<&HeaderValue>) -> Option<String> {
    value.and_then(|v| v.to_str().ok()).map(ToString::to_string)
}

fn is_event_stream_content_type(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .map(str::trim)
        .map(|mime| mime.eq_ignore_ascii_case("text/event-stream"))
        .unwrap_or(false)
}

fn check_response(response: Response, enforce_content_type: bool) -> Result<Response, Error> {
    if !response.status().is_success() {
        return Err(Error::InvalidStatusCode(response.status()));
    }

    let content_type = parse_content_type(response.headers().get(reqwest::header::CONTENT_TYPE));
    let valid_content_type = content_type
        .as_deref()
        .map(is_event_stream_content_type)
        .unwrap_or(false);

    if !valid_content_type {
        if enforce_content_type {
            return Err(Error::InvalidContentType(content_type));
        }

        warn!(
            content_type = ?content_type,
            "SSE response content-type is not text/event-stream; continuing in lenient mode"
        );
    }

    Ok(response)
}

fn next_retry_number(last_retry: Option<(usize, Duration)>) -> usize {
    last_retry.map(|(retry_num, _)| retry_num + 1).unwrap_or(1)
}

impl<'a> EventSourceProjection<'a> {
    fn clear_fetch(&mut self) {
        self.next_response.take();
        self.cur_stream.take();
    }

    fn retry_fetch(&mut self) -> Result<(), Error> {
        self.cur_stream.take();

        let Some(builder) = self.builder.as_ref() else {
            *self.is_closed = true;
            return Ok(());
        };

        let req = builder.try_clone().ok_or(CannotCloneRequestError)?.header(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_str(self.last_event_id)
                .map_err(|_| Error::InvalidLastEventId(self.last_event_id.clone()))?,
        );

        self.next_response.replace(Box::pin(req.send()));
        Ok(())
    }

    fn handle_event(&mut self, event: &MessageEvent) {
        *self.last_event_id = event.id.clone();
        if let Some(duration) = event.retry {
            self.retry_policy.set_reconnection_time(duration);
        }
    }

    fn handle_error(&mut self, error: &Error) {
        self.clear_fetch();

        if self.builder.is_none() {
            *self.is_closed = true;
            return;
        }

        if let Some(retry_delay) = self.retry_policy.retry(error, *self.last_retry) {
            let retry_num = next_retry_number(*self.last_retry);
            *self.last_retry = Some((retry_num, retry_delay));
            self.delay.replace(Box::pin(sleep(retry_delay)));
        } else {
            *self.is_closed = true;
        }
    }
}

/// Events emitted by `EventSource`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Event {
    Open,
    Message(MessageEvent),
}

impl From<MessageEvent> for Event {
    fn from(event: MessageEvent) -> Self {
        Self::Message(event)
    }
}

impl Stream for EventSource {
    type Item = Result<Event, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if *this.is_closed {
            return Poll::Ready(None);
        }

        if *this.open_pending {
            *this.open_pending = false;
            return Poll::Ready(Some(Ok(Event::Open)));
        }

        if let Some(delay) = this.delay.as_mut().as_pin_mut() {
            match delay.poll(cx) {
                Poll::Ready(_) => {
                    this.delay.take();
                    if let Err(err) = this.retry_fetch() {
                        *this.is_closed = true;
                        return Poll::Ready(Some(Err(err)));
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        if let Some(response_future) = this.next_response.as_mut().as_pin_mut() {
            match response_future.poll(cx) {
                Poll::Ready(Ok(res)) => {
                    this.next_response.take();
                    match check_response(res, *this.enforce_content_type) {
                        Ok(res) => {
                            this.last_retry.take();
                            let mut stream = res.bytes_stream().eventsource();
                            stream.set_last_event_id(this.last_event_id.clone());
                            stream.set_flush_on_eof(true);
                            this.cur_stream.replace(stream.boxed());
                            return Poll::Ready(Some(Ok(Event::Open)));
                        }
                        Err(err) => {
                            *this.is_closed = true;
                            return Poll::Ready(Some(Err(err)));
                        }
                    }
                }
                Poll::Ready(Err(err)) => {
                    let err = Error::Transport(err);
                    this.handle_error(&err);
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        if let Some(stream) = this.cur_stream.as_mut().as_pin_mut() {
            match stream.poll_next(cx) {
                Poll::Ready(Some(Err(err))) => {
                    let err = Error::from(err);
                    this.handle_error(&err);
                    Poll::Ready(Some(Err(err)))
                }
                Poll::Ready(Some(Ok(event))) => {
                    this.handle_event(&event);
                    Poll::Ready(Some(Ok(Event::Message(event))))
                }
                Poll::Ready(None) => {
                    let err = Error::StreamEnded;
                    this.handle_error(&err);
                    Poll::Ready(Some(Err(err)))
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            *this.is_closed = true;
            Poll::Ready(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_counter_is_incremental() {
        let one = next_retry_number(None);
        let two = next_retry_number(Some((one, Duration::from_millis(10))));
        let three = next_retry_number(Some((two, Duration::from_millis(20))));

        assert_eq!(one, 1);
        assert_eq!(two, 2);
        assert_eq!(three, 3);
    }

    #[test]
    fn content_type_matcher_accepts_charset_suffix() {
        assert!(is_event_stream_content_type(
            "text/event-stream; charset=utf-8"
        ));
        assert!(is_event_stream_content_type("text/event-stream"));
        assert!(!is_event_stream_content_type("application/json"));
    }
}
