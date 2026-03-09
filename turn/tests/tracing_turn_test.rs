mod support;

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use argus_core::ResponseEvent;
use tracing_subscriber::fmt::MakeWriter;
use turn::{TurnDriver, TurnSeed};

#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for SharedBuf {
    type Writer = SharedBufGuard;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufGuard(self.0.clone())
    }
}

struct SharedBufGuard(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBufGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn turn_tracing_emits_step_and_finish_markers() {
    let logs = SharedBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "hello".into(),
    };

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::text_only_model(["hello"])),
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
    );

    while handle.next_event().await.is_some() {}
    task.await.unwrap().unwrap();

    let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(output.contains("turn.run"));
    assert!(output.contains("turn started"));
    assert!(output.contains("step finished"));
    assert!(output.contains("turn finished"));
}

#[tokio::test]
async fn turn_tracing_reports_failed_turn_completion() {
    let logs = SharedBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "hello".into(),
    };

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::multi_step_model(vec![vec![ResponseEvent::Error(
            "boom".into(),
        )]])),
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
    );

    while handle.next_event().await.is_some() {}
    assert!(task.await.unwrap().is_err());

    let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(output.contains("turn started"));
    assert!(output.contains("turn finished"));
    assert!(output.contains("Failed"));
}
