use crate::transport::sse::Error;
use bytes::Bytes;
use eventsource_stream::{Event as MessageEvent, Eventsource};
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use reqwest::header::HeaderValue;
use reqwest::{Error as ReqwestError, Response};
use std::pin::Pin;
use std::task::{Context, Poll};

type MessageStream =
    BoxStream<'static, Result<MessageEvent, eventsource_stream::EventStreamError<ReqwestError>>>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Event {
    Open,
    Message(MessageEvent),
}

pub struct EventSource {
    open_pending: bool,
    stream: MessageStream,
}

impl EventSource {
    pub fn from_response(response: Response) -> Result<Self, Error> {
        let response = check_response(response)?;
        let eof_flush =
            stream::once(async { Ok::<Bytes, ReqwestError>(Bytes::from_static(b"\n\n")) });
        let stream = response.bytes_stream().chain(eof_flush).eventsource().boxed();

        Ok(Self {
            open_pending: true,
            stream,
        })
    }
}

impl Stream for EventSource {
    type Item = Result<Event, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.open_pending {
            self.open_pending = false;
            return Poll::Ready(Some(Ok(Event::Open)));
        }

        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => Poll::Ready(Some(Ok(Event::Message(event)))),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn check_response(response: Response) -> Result<Response, Error> {
    if !response.status().is_success() {
        return Err(Error::InvalidStatusCode(response.status()));
    }

    let content_type = parse_content_type(response.headers().get(reqwest::header::CONTENT_TYPE));
    let valid_content_type = content_type
        .as_deref()
        .map(is_event_stream_content_type)
        .unwrap_or(false);

    if !valid_content_type {
        return Err(Error::InvalidContentType(content_type));
    }

    Ok(response)
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
