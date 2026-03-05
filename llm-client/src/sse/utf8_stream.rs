// Adapted from eventsource-stream v0.2.3 (MIT OR Apache-2.0).
// Local modifications:
// - Kept behavior compatible with upstream (partial UTF-8 chunks may emit empty strings).

use core::pin::Pin;
use futures::task::{Context, Poll};
use futures::Stream;
use pin_project_lite::pin_project;
use std::string::FromUtf8Error;

pin_project! {
pub struct Utf8Stream<S> {
    #[pin]
    stream: S,
    buffer: Vec<u8>,
    terminated: bool,
}
}

impl<S> Utf8Stream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
            terminated: false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Utf8StreamError<E> {
    Utf8(FromUtf8Error),
    Transport(E),
}

impl<E> From<FromUtf8Error> for Utf8StreamError<E> {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl<S, B, E> Stream for Utf8Stream<S>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    type Item = Result<String, Utf8StreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if *this.terminated {
            return Poll::Ready(None);
        }

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                this.buffer.extend_from_slice(bytes.as_ref());
                let bytes = std::mem::take(this.buffer);
                match String::from_utf8(bytes) {
                    Ok(string) => Poll::Ready(Some(Ok(string))),
                    Err(err) => {
                        let valid_size = err.utf8_error().valid_up_to();
                        let mut bytes = err.into_bytes();
                        let rem = bytes.split_off(valid_size);
                        *this.buffer = rem;
                        // SAFETY: `valid_size` splits at UTF-8 boundary.
                        Poll::Ready(Some(Ok(unsafe { String::from_utf8_unchecked(bytes) })))
                    }
                }
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(Utf8StreamError::Transport(err)))),
            Poll::Ready(None) => {
                *this.terminated = true;
                if this.buffer.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(
                        String::from_utf8(std::mem::take(this.buffer))
                            .map_err(Utf8StreamError::Utf8),
                    ))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
