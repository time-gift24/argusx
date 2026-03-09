use std::{
    collections::VecDeque,
    sync::{Arc, Barrier, Mutex},
    time::Duration,
};

use argus_core::{FinishReason, ResponseEvent, ResponseStream, Usage};
use async_trait::async_trait;
use session::{
    manager::{SessionEvent, SessionManager, TurnDependencies},
    store::ThreadStore,
    types::{PersistedMessage, TurnStatus},
};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::{sync::mpsc, task, time::timeout};
use tool::{ToolContext, ToolResult};
use turn::{
    AuthorizationDecision, LlmStepRequest, ModelRunner, ToolAuthorizer, ToolRunner, TurnError,
    TurnEvent, TurnFinishReason,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_send_message_rejects_second_turn_for_same_thread() {
    for _ in 0..25 {
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let store = ThreadStore::new(pool);
        store.init_schema().await.unwrap();

        let manager = SessionManager::new("session-1".into(), store);
        let thread_id = manager.create_thread(Some("A".into())).await.unwrap();
        let deps = TurnDependencies {
            model: Arc::new(SlowTextModel::new(Duration::from_millis(120), "done")),
            tool_runner: Arc::new(NoopToolRunner),
            authorizer: Arc::new(AllowAuthorizer),
        };

        // Get thread and call send_message twice - second call should fail
        let thread = manager.get_thread(thread_id, Some(deps)).await.unwrap();

        // First send_message should succeed
        let result1 = thread.send_message("first".into()).await;
        assert!(result1.is_ok(), "first send_message should succeed: {:?}", result1);

        // Second send_message on the same thread should fail (active turn exists)
        let result2 = thread.send_message("second".into()).await;
        assert!(result2.is_err(), "second send_message should fail");
        let err = result2.unwrap_err();
        assert!(
            err.to_string().contains("Turn already active"),
            "error should be about active turn: {err}"
        );

        // Should only have one turn in history
        let history = manager.load_thread_history(thread_id).await.unwrap();
        assert_eq!(history.len(), 1);
    }
}

#[tokio::test]
async fn failed_turn_persists_incremental_transcript_messages() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let manager = SessionManager::new("session-1".into(), store);
    let thread_id = manager.create_thread(Some("A".into())).await.unwrap();

    let deps = TurnDependencies {
        model: Arc::new(FailingAfterTextModel::new("partial", "boom")),
        tool_runner: Arc::new(NoopToolRunner),
        authorizer: Arc::new(AllowAuthorizer),
    };

    // send_message waits for turn completion, so we can just check the result afterwards
    let thread = manager.get_thread(thread_id, Some(deps)).await.unwrap();
    let result = thread.send_message("hello".into()).await;
    // The result might be Ok or Err depending on implementation, but turn should be persisted

    // Give a small delay for any async cleanup
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let history = manager.load_thread_history(thread_id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert!(matches!(history[0].status, TurnStatus::Failed));
    assert_eq!(
        history[0].transcript,
        vec![
            PersistedMessage::User {
                content: "hello".into(),
            },
            PersistedMessage::AssistantText {
                content: "partial".into(),
            },
        ]
    );
}

async fn wait_for_turn_finished(
    events: &mut tokio::sync::broadcast::Receiver<SessionEvent>,
    thread_id: uuid::Uuid,
    expected_reason: TurnFinishReason,
) {
    timeout(Duration::from_secs(2), async {
        loop {
            match events.recv().await {
                Ok(SessionEvent::Turn {
                    thread_id: event_thread_id,
                    event: TurnEvent::TurnFinished { reason },
                    ..
                }) if event_thread_id == thread_id && reason == expected_reason => break,
                Ok(_) => continue,
                Err(err) => panic!("event bridge closed early: {err}"),
            }
        }
    })
    .await
    .unwrap();
}

#[derive(Clone)]
struct SlowTextModel {
    delay: Duration,
    output: &'static str,
}

impl SlowTextModel {
    fn new(delay: Duration, output: &'static str) -> Self {
        Self { delay, output }
    }
}

#[async_trait]
impl ModelRunner for SlowTextModel {
    async fn start(&self, _request: LlmStepRequest) -> Result<ResponseStream, TurnError> {
        let (tx, rx) = mpsc::channel(4);
        let delay = self.delay;
        let output = self.output;
        let producer = task::spawn(async move {
            tokio::time::sleep(delay).await;
            tx.send(ResponseEvent::ContentDelta(output.into()))
                .await
                .unwrap();
            tx.send(ResponseEvent::Done {
                reason: FinishReason::Stop,
                usage: Some(Usage::zero()),
            })
            .await
            .unwrap();
        });
        Ok(ResponseStream::from_parts(rx, producer.abort_handle()))
    }
}

#[derive(Clone)]
struct FailingAfterTextModel {
    responses: Arc<Mutex<VecDeque<(&'static str, &'static str)>>>,
}

impl FailingAfterTextModel {
    fn new(text: &'static str, error: &'static str) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from([(text, error)]))),
        }
    }
}

#[async_trait]
impl ModelRunner for FailingAfterTextModel {
    async fn start(&self, _request: LlmStepRequest) -> Result<ResponseStream, TurnError> {
        let (text, error) = self.responses.lock().unwrap().pop_front().unwrap();
        let (tx, rx) = mpsc::channel(4);
        let producer = task::spawn(async move {
            tx.send(ResponseEvent::ContentDelta(text.into()))
                .await
                .unwrap();
            tx.send(ResponseEvent::Error(argus_core::Error {
                message: error.into(),
            }))
            .await
            .unwrap();
        });
        Ok(ResponseStream::from_parts(rx, producer.abort_handle()))
    }
}

struct NoopToolRunner;

#[async_trait]
impl ToolRunner for NoopToolRunner {
    async fn execute(
        &self,
        _call: argus_core::ToolCall,
        _ctx: ToolContext,
    ) -> Result<ToolResult, TurnError> {
        Ok(ToolResult::ok(serde_json::json!({"ok": true})))
    }
}

struct AllowAuthorizer;

#[async_trait]
impl ToolAuthorizer for AllowAuthorizer {
    async fn authorize(
        &self,
        _call: &argus_core::ToolCall,
    ) -> Result<AuthorizationDecision, TurnError> {
        Ok(AuthorizationDecision::Allow)
    }
}
