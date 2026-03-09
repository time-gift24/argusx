use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use argus_core::{FinishReason, ResponseEvent, ResponseStream, Usage};
use async_trait::async_trait;
use session::{
    manager::{SessionEvent, SessionManager, TurnDependencies},
    store::ThreadStore,
    types::PersistedMessage,
};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::{
    sync::mpsc,
    task,
    time::{Duration, timeout},
};
use tool::{ToolContext, ToolResult};
use turn::{
    AuthorizationDecision, LlmStepRequest, ModelRunner, ToolAuthorizer, ToolRunner, TurnError,
    TurnEvent, TurnFinishReason,
};

#[tokio::test]
async fn completed_turns_persist_only_incremental_transcript_messages() {
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
        model: Arc::new(QueuedTextModel::new(["first", "second"])),
        tool_runner: Arc::new(NoopToolRunner),
        authorizer: Arc::new(AllowAuthorizer),
    };

    manager
        .send_message(thread_id, "hello".into(), deps.clone())
        .await
        .unwrap();
    wait_for_turn_finished(&mut events, thread_id).await;

    manager
        .send_message(thread_id, "again".into(), deps)
        .await
        .unwrap();
    wait_for_turn_finished(&mut events, thread_id).await;

    let history = manager.load_thread_history(thread_id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].transcript.len(), 2);
    assert_eq!(history[1].transcript.len(), 2);
    assert_eq!(
        history[1].transcript,
        vec![
            PersistedMessage::User {
                content: "again".into(),
            },
            PersistedMessage::AssistantText {
                content: "second".into(),
            },
        ]
    );
}

async fn wait_for_turn_finished(
    events: &mut tokio::sync::broadcast::Receiver<SessionEvent>,
    thread_id: uuid::Uuid,
) {
    timeout(Duration::from_secs(2), async {
        loop {
            match events.recv().await {
                Ok(SessionEvent::Turn {
                    thread_id: event_thread_id,
                    event:
                        TurnEvent::TurnFinished {
                            reason: TurnFinishReason::Completed,
                        },
                    ..
                }) if event_thread_id == thread_id => break,
                Ok(_) => continue,
                Err(err) => panic!("event bridge closed early: {err}"),
            }
        }
    })
    .await
    .unwrap();
}

#[derive(Clone)]
struct QueuedTextModel {
    outputs: Arc<Mutex<VecDeque<&'static str>>>,
}

impl QueuedTextModel {
    fn new(outputs: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(outputs.into_iter().collect())),
        }
    }
}

#[async_trait]
impl ModelRunner for QueuedTextModel {
    async fn start(&self, _request: LlmStepRequest) -> Result<ResponseStream, TurnError> {
        let output = self.outputs.lock().unwrap().pop_front().unwrap();
        let (tx, rx) = mpsc::channel(4);
        let producer = task::spawn(async move {
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
