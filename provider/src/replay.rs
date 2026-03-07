use std::{
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use async_stream::try_stream;
use futures::{Stream, StreamExt, stream::BoxStream};
use serde::Deserialize;

use crate::{Error, ReplayTiming, StreamError};

pub struct ReplayReader {
    inner: BoxStream<'static, Result<String, StreamError>>,
}

impl ReplayReader {
    pub async fn open(path: impl Into<PathBuf>, timing: ReplayTiming) -> Result<Self, Error> {
        Ok(Self::from_prepared(prepare(path, timing)?))
    }
}

impl Stream for ReplayReader {
    type Item = Result<String, StreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[derive(Debug, Deserialize)]
struct FrameTiming {
    frame_index: usize,
    delay_ms: u64,
}

pub(crate) struct PreparedReplay {
    frames: Vec<PreparedReplayFrame>,
}

struct PreparedReplayFrame {
    text: String,
    delay_ms: u64,
}

pub(crate) fn prepare(
    path: impl Into<PathBuf>,
    timing: ReplayTiming,
) -> Result<PreparedReplay, Error> {
    let path = path.into();
    let body = fs::read_to_string(&path).map_err(|err| {
        Error::Config(format!(
            "failed to read replay file {}: {err}",
            path.display()
        ))
    })?;
    let frames = parse_frames(&body)?;
    let delays = match timing {
        ReplayTiming::Fast => vec![0; frames.len()],
        ReplayTiming::Recorded => match load_delays(&path, frames.len()) {
            Ok(delays) => delays,
            Err(err) => {
                tracing::warn!(error = %err, "failed to load timing metadata, falling back to fast mode");
                vec![0; frames.len()]
            }
        },
    };

    Ok(PreparedReplay {
        frames: frames
            .into_iter()
            .zip(delays)
            .map(|(text, delay_ms)| PreparedReplayFrame { text, delay_ms })
            .collect(),
    })
}

impl ReplayReader {
    pub(crate) fn from_prepared(prepared: PreparedReplay) -> Self {
        let inner = try_stream! {
            for frame in prepared.frames {
                if frame.delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(frame.delay_ms)).await;
                }
                yield frame.text;
            }
        }
        .boxed();

        Self { inner }
    }
}

fn parse_frames(body: &str) -> Result<Vec<String>, Error> {
    let normalized = body.replace("\r\n", "\n");
    let frames: Vec<String> = normalized
        .split("\n\n")
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| format!("{chunk}\n\n"))
        .collect();

    if frames.is_empty() {
        return Err(Error::Config("replay file contains no SSE frames".into()));
    }

    Ok(frames)
}

fn load_delays(path: &Path, frame_count: usize) -> Result<Vec<u64>, Error> {
    let sidecar_path = sidecar_path(path);
    let body = fs::read_to_string(&sidecar_path).map_err(|err| {
        Error::Config(format!(
            "failed to read replay timing metadata {}: {err}",
            sidecar_path.display()
        ))
    })?;
    let entries: Vec<FrameTiming> = serde_json::from_str(&body).map_err(|err| {
        Error::Config(format!(
            "failed to parse replay timing metadata {}: {err}",
            sidecar_path.display()
        ))
    })?;

    if entries.len() != frame_count {
        return Err(Error::Config(format!(
            "replay timing metadata frame count mismatch: expected {frame_count}, got {}",
            entries.len()
        )));
    }

    let mut delays = vec![None; frame_count];
    for entry in entries {
        if entry.frame_index >= frame_count {
            return Err(Error::Config(format!(
                "replay timing metadata frame index {} out of bounds for {frame_count} frames",
                entry.frame_index
            )));
        }

        if delays[entry.frame_index].replace(entry.delay_ms).is_some() {
            return Err(Error::Config(format!(
                "duplicate replay timing metadata for frame {}",
                entry.frame_index
            )));
        }
    }

    delays
        .into_iter()
        .enumerate()
        .map(|(index, delay)| {
            delay.ok_or_else(|| {
                Error::Config(format!("missing replay timing metadata for frame {index}"))
            })
        })
        .collect()
}

fn sidecar_path(path: &Path) -> PathBuf {
    path.with_extension("sse.meta.json")
}
