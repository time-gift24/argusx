use std::{sync::Arc, time::Duration};

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

#[tokio::test]
async fn switching_active_thread_does_not_cancel_running_turn() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let manager = SessionManager::new("session-1".into(), store);
    let thread_a = manager.create_thread(Some("A".into())).await.unwrap();
    let thread_b = manager.create_thread(Some("B".into())).await.unwrap();
    let mut events = manager.subscribe();

    let deps = TurnDependencies {
        model: Arc::new(SlowTextModel::new(Duration::from_millis(120), "done")),
        tool_runner: Arc::new(NoopToolRunner),
        authorizer: Arc::new(AllowAuthorizer),
        observer: Arc::new(NoopObserver),
    };

    manager
        .send_message(thread_a, "hello".into(), deps)
        .await
        .unwrap();
    manager.switch_thread(thread_b).await.unwrap();

    assert_eq!(manager.active_thread_id(), Some(thread_b));

    timeout(Duration::from_secs(2), async {
        loop {
            match events.recv().await {
                Ok(SessionEvent::Turn {
                    thread_id,
                    event:
                        TurnEvent::TurnFinished {
                            reason: TurnFinishReason::Completed,
                        },
                    ..
                }) if thread_id == thread_a => break,
                Ok(_) => continue,
                Err(err) => panic!("event bridge closed early: {err}"),
            }
        }
    })
    .await
    .unwrap();

    let history = manager.load_thread_history(thread_a).await.unwrap();
    assert_eq!(history.len(), 1);
    assert!(matches!(history[0].status, TurnStatus::Completed));
    assert_eq!(history[0].final_output.as_deref(), Some("done"));
    assert!(matches!(
        history[0].transcript.last(),
        Some(PersistedMessage::AssistantText { content }) if content == "done"
    ));
    assert_eq!(manager.active_thread_id(), Some(thread_b));
}

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
