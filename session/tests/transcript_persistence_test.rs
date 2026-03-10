use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use argus_core::{FinishReason, ResponseEvent, ResponseStream, Usage};
use async_trait::async_trait;
use session::{
    manager::SessionManager,
    store::ThreadStore,
    types::{PersistedMessage, TurnStatus},
    TurnDependencies,
};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::{sync::mpsc, task};
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

    let thread = manager.get_thread(thread_id, Some(deps.clone())).await.unwrap();
    thread.send_message("hello".into()).await.unwrap();

    // Wait for turn to complete
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let history = manager.load_thread_history(thread_id).await.unwrap();
        if !history.is_empty() && history[0].status == TurnStatus::Completed {
            break;
        }
    }

    let thread = manager.get_thread(thread_id, Some(deps)).await.unwrap();
    thread.send_message("again".into()).await.unwrap();

    // Wait for second turn to complete
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let history = manager.load_thread_history(thread_id).await.unwrap();
        if history.len() == 2 && history[1].status == TurnStatus::Completed {
            break;
        }
    }

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
