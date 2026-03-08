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
    TurnEvent, TurnFinishReason, TurnObserver,
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
            observer: Arc::new(NoopObserver),
        };
        let barrier = Arc::new(Barrier::new(3));

        let task_a = {
            let manager = manager.clone();
            let deps = deps.clone();
            let barrier = Arc::clone(&barrier);
            task::spawn(async move {
                barrier.wait();
                manager.send_message(thread_id, "first".into(), deps).await
            })
        };
        let task_b = {
            let manager = manager.clone();
            let deps = deps.clone();
            let barrier = Arc::clone(&barrier);
            task::spawn(async move {
                barrier.wait();
                manager.send_message(thread_id, "second".into(), deps).await
            })
        };

        barrier.wait();

        let result_a = timeout(Duration::from_secs(2), task_a)
            .await
            .unwrap()
            .unwrap();
        let result_b = timeout(Duration::from_secs(2), task_b)
            .await
            .unwrap()
            .unwrap();
        let history = manager.load_thread_history(thread_id).await.unwrap();

        let outcomes = [result_a, result_b];
        let success_count = outcomes.iter().filter(|result| result.is_ok()).count();
        let active_turn_errors = outcomes
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|err| err.to_string().contains("already has an active turn"))
            .count();

        assert_eq!(success_count, 1, "outcomes: {outcomes:?}");
        assert_eq!(active_turn_errors, 1, "outcomes: {outcomes:?}");
        assert_eq!(history.len(), 1, "outcomes: {outcomes:?}");
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
    let mut events = manager.subscribe();

    let deps = TurnDependencies {
        model: Arc::new(FailingAfterTextModel::new("partial", "boom")),
        tool_runner: Arc::new(NoopToolRunner),
        authorizer: Arc::new(AllowAuthorizer),
        observer: Arc::new(NoopObserver),
    };

    manager
        .send_message(thread_id, "hello".into(), deps)
        .await
        .unwrap();
    wait_for_turn_finished(&mut events, thread_id, TurnFinishReason::Failed).await;

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

struct NoopObserver;

#[async_trait]
impl TurnObserver for NoopObserver {
    async fn on_event(&self, _event: &TurnEvent) -> Result<(), TurnError> {
        Ok(())
    }
}
