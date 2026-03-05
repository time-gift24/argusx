// Adapted from eventsource-stream v0.2.3 (MIT OR Apache-2.0).
// Local modifications:
// - Exported `MessageEvent` type from llm-client.
// - Added configurable EOF flush behavior (`set_flush_on_eof`) and enabled it by default.
// - On stream end, trailing partial line is finalized to preserve llm-client historical behavior.

use crate::sse::message_event::MessageEvent;
use crate::sse::parser::{is_bom, is_lf, line, RawEventLine};
use crate::sse::utf8_stream::{Utf8Stream, Utf8StreamError};
use core::fmt;
use core::pin::Pin;
use core::time::Duration;
use futures::task::{Context, Poll};
use futures::Stream;
use nom::error::Error as NomError;
use pin_project_lite::pin_project;
use std::string::{FromUtf8Error, ToString};

#[derive(Default, Debug)]
struct EventBuilder {
    event: MessageEvent,
    is_complete: bool,
}

impl EventBuilder {
    fn add(&mut self, line: RawEventLine<'_>) {
        match line {
            RawEventLine::Field(field, val) => {
                let val = val.unwrap_or("");
                match field {
                    "event" => {
                        self.event.event = val.to_string();
                    }
                    "data" => {
                        self.event.data.push_str(val);
                        self.event.data.push('\u{000A}');
                    }
                    "id" => {
                        if !val.contains('\u{0000}') {
                            self.event.id = val.to_string();
                        }
                    }
                    "retry" => {
                        if let Ok(val) = val.parse::<u64>() {
                            self.event.retry = Some(Duration::from_millis(val));
                        }
                    }
                    _ => {}
                }
            }
            RawEventLine::Comment => {}
            RawEventLine::Empty => self.is_complete = true,
        }
    }

    fn dispatch_if_complete(&mut self) -> Option<MessageEvent> {
        if !self.is_complete {
            return None;
        }
        self.dispatch_any()
    }

    fn dispatch_any(&mut self) -> Option<MessageEvent> {
        let builder = core::mem::take(self);
        let mut event = builder.event;
        self.event.id = event.id.clone();

        if event.data.is_empty() {
            return None;
        }

        if is_lf(event.data.chars().next_back().unwrap()) {
            event.data.pop();
        }

        if event.event.is_empty() {
            event.event = "message".to_string();
        }

        Some(event)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EventStreamState {
    NotStarted,
    Started,
    Terminated,
}

impl EventStreamState {
    fn is_terminated(self) -> bool {
        matches!(self, Self::Terminated)
    }

    fn is_started(self) -> bool {
        matches!(self, Self::Started)
    }
}

pin_project! {
/// A stream of parsed SSE `MessageEvent`s.
pub struct EventStream<S> {
    #[pin]
    stream: Utf8Stream<S>,
    buffer: String,
    builder: EventBuilder,
    state: EventStreamState,
    last_event_id: String,
    flush_on_eof: bool,
}
}

impl<S> EventStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream: Utf8Stream::new(stream),
            buffer: String::new(),
            builder: EventBuilder::default(),
            state: EventStreamState::NotStarted,
            last_event_id: String::new(),
            flush_on_eof: true,
        }
    }

    pub fn set_last_event_id(&mut self, id: impl Into<String>) {
        self.last_event_id = id.into();
    }

    pub fn set_flush_on_eof(&mut self, flush_on_eof: bool) {
        self.flush_on_eof = flush_on_eof;
    }
}

#[derive(Debug, PartialEq)]
pub enum EventStreamError<E> {
    Utf8(FromUtf8Error),
    Parser(NomError<String>),
    Transport(E),
}

impl<E> From<Utf8StreamError<E>> for EventStreamError<E> {
    fn from(err: Utf8StreamError<E>) -> Self {
        match err {
            Utf8StreamError::Utf8(err) => Self::Utf8(err),
            Utf8StreamError::Transport(err) => Self::Transport(err),
        }
    }
}

impl<E> From<NomError<&str>> for EventStreamError<E> {
    fn from(err: NomError<&str>) -> Self {
        EventStreamError::Parser(NomError::new(err.input.to_string(), err.code))
    }
}

impl<E> fmt::Display for EventStreamError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8(err) => f.write_fmt(format_args!("UTF8 error: {}", err)),
            Self::Parser(err) => f.write_fmt(format_args!("Parse error: {}", err)),
            Self::Transport(err) => f.write_fmt(format_args!("Transport error: {}", err)),
        }
    }
}

impl<E> std::error::Error for EventStreamError<E> where E: fmt::Display + fmt::Debug + Send + Sync {}

fn parse_event<E>(
    buffer: &mut String,
    builder: &mut EventBuilder,
) -> Result<Option<MessageEvent>, EventStreamError<E>> {
    if buffer.is_empty() {
        return Ok(None);
    }

    loop {
        match line(buffer.as_ref()) {
            Ok((rem, next_line)) => {
                builder.add(next_line);
                let consumed = buffer.len() - rem.len();
                let rem = buffer.split_off(consumed);
                *buffer = rem;
                if let Some(event) = builder.dispatch_if_complete() {
                    return Ok(Some(event));
                }
            }
            Err(nom::Err::Incomplete(_)) => return Ok(None),
            Err(nom::Err::Error(err)) | Err(nom::Err::Failure(err)) => return Err(err.into()),
        }
    }
}

impl<S, B, E> Stream for EventStream<S>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    type Item = Result<MessageEvent, EventStreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match parse_event(this.buffer, this.builder) {
            Ok(Some(event)) => {
                *this.last_event_id = event.id.clone();
                return Poll::Ready(Some(Ok(event)));
            }
            Err(err) => return Poll::Ready(Some(Err(err))),
            _ => {}
        }

        if this.state.is_terminated() {
            return Poll::Ready(None);
        }

        loop {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(string))) => {
                    if string.is_empty() {
                        continue;
                    }

                    let slice = if this.state.is_started() {
                        &string
                    } else {
                        *this.state = EventStreamState::Started;
                        if is_bom(string.chars().next().unwrap()) {
                            &string[1..]
                        } else {
                            &string
                        }
                    };

                    this.buffer.push_str(slice);

                    match parse_event(this.buffer, this.builder) {
                        Ok(Some(event)) => {
                            *this.last_event_id = event.id.clone();
                            return Poll::Ready(Some(Ok(event)));
                        }
                        Err(err) => return Poll::Ready(Some(Err(err))),
                        _ => {}
                    }
                }
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err.into()))),
                Poll::Ready(None) => {
                    *this.state = EventStreamState::Terminated;

                    // Try finalizing a trailing partial line at EOF.
                    if !this.buffer.is_empty()
                        && !this.buffer.ends_with('\n')
                        && !this.buffer.ends_with('\r')
                    {
                        this.buffer.push('\n');
                    }

                    match parse_event(this.buffer, this.builder) {
                        Ok(Some(event)) => {
                            *this.last_event_id = event.id.clone();
                            return Poll::Ready(Some(Ok(event)));
                        }
                        Err(err) => return Poll::Ready(Some(Err(err))),
                        _ => {}
                    }

                    if *this.flush_on_eof {
                        if let Some(event) = this.builder.dispatch_any() {
                            *this.last_event_id = event.id.clone();
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }

                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
