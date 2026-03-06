use std::{
    io,
    path::{Path, PathBuf},
    time::Instant,
};

use serde::Serialize;
use tokio::io::AsyncWriteExt;

pub struct SseRecorder {
    file_path: PathBuf,
    file: tokio::fs::File,
    write_timing_sidecar: bool,
    timings: Vec<FrameTiming>,
    last_write_at: Option<Instant>,
}

impl SseRecorder {
    pub async fn create(path: impl Into<PathBuf>, write_timing_sidecar: bool) -> io::Result<Self> {
        let file_path = path.into();
        if let Some(parent) = file_path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            tokio::fs::create_dir_all(parent).await?;
        }

        let file = tokio::fs::File::create(&file_path).await?;
        Ok(Self {
            file_path,
            file,
            write_timing_sidecar,
            timings: Vec::new(),
            last_write_at: None,
        })
    }

    pub async fn write_frame(&mut self, frame: &str) -> io::Result<()> {
        self.file.write_all(frame.as_bytes()).await?;
        let now = Instant::now();
        let delay_ms = self
            .last_write_at
            .map(|last| now.duration_since(last).as_millis() as u64)
            .unwrap_or(0);
        self.last_write_at = Some(now);
        self.timings.push(FrameTiming {
            frame_index: self.timings.len(),
            delay_ms,
        });
        Ok(())
    }

    pub async fn finish(&mut self) -> io::Result<()> {
        self.file.flush().await?;
        if self.write_timing_sidecar {
            let body = serde_json::to_string(&self.timings)
                .map_err(|err| io::Error::other(format!("serialize timing metadata: {err}")))?;
            tokio::fs::write(sidecar_path(&self.file_path), body).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct FrameTiming {
    frame_index: usize,
    delay_ms: u64,
}

fn sidecar_path(path: &Path) -> PathBuf {
    path.with_extension("sse.meta.json")
}
