use futures::future::BoxFuture;
use serde::Serialize;
use std::{
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;
use tokio::{sync::mpsc, task::JoinHandle};

pub struct SseRecorder {
    command_tx: Option<mpsc::UnboundedSender<RecorderCommand>>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

impl SseRecorder {
    pub async fn create(path: impl Into<PathBuf>, write_timing_sidecar: bool) -> io::Result<Self> {
        let session = FileRecorderSession::create(path.into(), write_timing_sidecar).await?;
        Ok(Self::spawn_with_session(session))
    }

    pub async fn write_frame(&mut self, frame: &str) -> io::Result<()> {
        self.send_command(RecorderCommand::Frame {
            frame: frame.to_owned(),
            observed_at: Instant::now(),
        })
        .await
    }

    pub async fn finish(&mut self) -> io::Result<()> {
        if let Some(tx) = self.command_tx.take() {
            match tx.send(RecorderCommand::Finish) {
                Ok(()) => {}
                Err(_) => {
                    return match self.await_worker().await {
                        Ok(()) => Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "recorder worker stopped before finish",
                        )),
                        Err(err) => Err(err),
                    };
                }
            }
        }
        self.await_worker().await
    }

    #[cfg(test)]
    fn spawn_for_test<S>(session: S) -> Self
    where
        S: RecorderSession,
    {
        Self::spawn_with_session(session)
    }

    fn spawn_with_session<S>(session: S) -> Self
    where
        S: RecorderSession,
    {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let worker = tokio::spawn(run_recorder(session, command_rx));
        Self {
            command_tx: Some(command_tx),
            worker: Some(worker),
        }
    }

    async fn send_command(&mut self, command: RecorderCommand) -> io::Result<()> {
        let Some(tx) = self.command_tx.as_ref() else {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "recorder already finished",
            ));
        };

        if tx.send(command).is_err() {
            self.command_tx = None;
            return match self.await_worker().await {
                Ok(()) => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "recorder worker stopped",
                )),
                Err(err) => Err(err),
            };
        }

        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            self.command_tx = None;
            self.await_worker().await?;
        }

        Ok(())
    }

    async fn await_worker(&mut self) -> io::Result<()> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };

        worker
            .await
            .map_err(|err| io::Error::other(format!("recorder worker join failed: {err}")))?
    }
}

impl Drop for SseRecorder {
    fn drop(&mut self) {
        self.command_tx.take();
        self.worker.take();
    }
}

#[derive(Debug, Serialize)]
struct FrameTiming {
    frame_index: usize,
    delay_ms: u64,
}

enum RecorderCommand {
    Frame { frame: String, observed_at: Instant },
    Finish,
}

trait RecorderSession: Send + 'static {
    fn write_frame<'a>(
        &'a mut self,
        frame: &'a str,
        observed_at: Instant,
    ) -> BoxFuture<'a, io::Result<()>>;
    fn finish<'a>(&'a mut self) -> BoxFuture<'a, io::Result<()>>;
    fn cleanup<'a>(&'a mut self) -> BoxFuture<'a, ()>;
}

struct FileRecorderSession {
    file_path: PathBuf,
    temp_path: PathBuf,
    file: tokio::fs::File,
    timings: Vec<FrameTiming>,
    last_frame_at: Option<Instant>,
    sidecar: Option<SidecarPaths>,
}

struct SidecarPaths {
    final_path: PathBuf,
    temp_path: PathBuf,
    promoted: bool,
}

impl FileRecorderSession {
    async fn create(file_path: PathBuf, write_timing_sidecar: bool) -> io::Result<Self> {
        if let Some(parent) = file_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            tokio::fs::create_dir_all(parent).await?;
        }

        let temp_path = temporary_path(&file_path);
        let file = tokio::fs::File::create(&temp_path).await?;
        let sidecar = write_timing_sidecar.then(|| {
            let final_path = sidecar_path(&file_path);
            SidecarPaths {
                temp_path: temporary_path(&final_path),
                final_path,
                promoted: false,
            }
        });

        Ok(Self {
            file_path,
            temp_path,
            file,
            timings: Vec::new(),
            last_frame_at: None,
            sidecar,
        })
    }
}

impl RecorderSession for FileRecorderSession {
    fn write_frame<'a>(
        &'a mut self,
        frame: &'a str,
        observed_at: Instant,
    ) -> BoxFuture<'a, io::Result<()>> {
        Box::pin(async move {
            self.file.write_all(frame.as_bytes()).await?;
            let delay_ms = self
                .last_frame_at
                .map(|last| observed_at.duration_since(last).as_millis() as u64)
                .unwrap_or(0);
            self.last_frame_at = Some(observed_at);
            self.timings.push(FrameTiming {
                frame_index: self.timings.len(),
                delay_ms,
            });
            Ok(())
        })
    }

    fn finish<'a>(&'a mut self) -> BoxFuture<'a, io::Result<()>> {
        Box::pin(async move {
            self.file.flush().await?;
            if let Some(sidecar) = self.sidecar.as_mut() {
                let body = serde_json::to_string(&self.timings)
                    .map_err(|err| io::Error::other(format!("serialize timing metadata: {err}")))?;
                tokio::fs::write(&sidecar.temp_path, body).await?;
                tokio::fs::rename(&sidecar.temp_path, &sidecar.final_path).await?;
                sidecar.promoted = true;
            }
            tokio::fs::rename(&self.temp_path, &self.file_path).await?;
            Ok(())
        })
    }

    fn cleanup<'a>(&'a mut self) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let _ = tokio::fs::remove_file(&self.temp_path).await;
            if let Some(sidecar) = self.sidecar.as_ref() {
                let cleanup_path = if sidecar.promoted {
                    &sidecar.final_path
                } else {
                    &sidecar.temp_path
                };
                let _ = tokio::fs::remove_file(cleanup_path).await;
            }
        })
    }
}

async fn run_recorder<S>(
    mut session: S,
    mut command_rx: mpsc::UnboundedReceiver<RecorderCommand>,
) -> io::Result<()>
where
    S: RecorderSession,
{
    let mut finish_requested = false;

    while let Some(command) = command_rx.recv().await {
        match command {
            RecorderCommand::Frame { frame, observed_at } => {
                if let Err(err) = session.write_frame(&frame, observed_at).await {
                    session.cleanup().await;
                    return Err(err);
                }
            }
            RecorderCommand::Finish => {
                finish_requested = true;
                break;
            }
        }
    }

    if !finish_requested {
        session.cleanup().await;
        return Ok(());
    }

    if let Err(err) = session.finish().await {
        session.cleanup().await;
        return Err(err);
    }

    Ok(())
}

fn sidecar_path(path: &Path) -> PathBuf {
    path.with_extension("sse.meta.json")
}

fn temporary_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .map(|name| format!("{}.argusx-{nonce}.tmp", name.to_string_lossy()))
        .unwrap_or_else(|| format!("recording.argusx-{nonce}.tmp"));
    path.with_file_name(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[derive(Clone)]
    struct MockSession {
        events: Arc<Mutex<Vec<&'static str>>>,
        write_delay: Duration,
        fail_on_finish: bool,
    }

    impl MockSession {
        fn with_write_delay(write_delay: Duration) -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                write_delay,
                fail_on_finish: false,
            }
        }

        fn fail_on_finish() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                write_delay: Duration::ZERO,
                fail_on_finish: true,
            }
        }

        fn events(&self) -> Vec<&'static str> {
            self.events.lock().unwrap().clone()
        }
    }

    impl RecorderSession for MockSession {
        fn write_frame<'a>(
            &'a mut self,
            _frame: &'a str,
            _observed_at: Instant,
        ) -> futures::future::BoxFuture<'a, io::Result<()>> {
            Box::pin(async move {
                self.events.lock().unwrap().push("write-start");
                if !self.write_delay.is_zero() {
                    tokio::time::sleep(self.write_delay).await;
                }
                self.events.lock().unwrap().push("write-done");
                Ok(())
            })
        }

        fn finish<'a>(&'a mut self) -> futures::future::BoxFuture<'a, io::Result<()>> {
            Box::pin(async move {
                self.events.lock().unwrap().push("finish");
                if self.fail_on_finish {
                    Err(io::Error::other("finish failed"))
                } else {
                    Ok(())
                }
            })
        }

        fn cleanup<'a>(&'a mut self) -> futures::future::BoxFuture<'a, ()> {
            Box::pin(async move {
                self.events.lock().unwrap().push("cleanup");
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn write_frame_returns_before_background_flush_completes() {
        let session = MockSession::with_write_delay(Duration::from_secs(5));
        let mut recorder = SseRecorder::spawn_for_test(session.clone());

        let write = recorder.write_frame("data: {\"id\":\"1\"}\n\n");
        futures::pin_mut!(write);
        assert!(write.now_or_never().is_some_and(|result| result.is_ok()));

        tokio::task::yield_now().await;
        assert_eq!(session.events(), vec!["write-start"]);
    }

    #[tokio::test]
    async fn finish_failure_triggers_cleanup_of_incomplete_recording() {
        let session = MockSession::fail_on_finish();
        let mut recorder = SseRecorder::spawn_for_test(session.clone());

        recorder
            .write_frame("data: {\"id\":\"1\"}\n\n")
            .await
            .unwrap();
        tokio::task::yield_now().await;

        let err = recorder.finish().await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert_eq!(
            session.events(),
            vec!["write-start", "write-done", "finish", "cleanup"]
        );
    }
}
