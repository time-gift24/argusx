mod support;

use std::{sync::Arc, time::Duration};

use argus_core::{Builtin, BuiltinToolCall, FinishReason, ResponseEvent, ToolCall, Usage};
use async_trait::async_trait;
use serde_json::json;
use tokio::{
    sync::{Mutex, Notify, mpsc, oneshot},
    task,
    time::timeout,
};
use tool::{ToolContext, ToolResult};
use turn::{
    LlmStepRequest, ModelRunner, ToolOutcome, ToolRunner, TurnController, TurnDriver, TurnError,
    TurnEvent, TurnFinishReason, TurnObserver, TurnSeed,
};

fn builtin_call(sequence: u32, call_id: &str) -> ToolCall {
    ToolCall::Builtin(BuiltinToolCall {
        sequence,
        call_id: call_id.into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    })
}

#[tokio::test]
async fn cancelling_before_next_model_invocation_stops_at_step_boundary() {
    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "cancel me".into(),
    };

    let first_step = vec![
        ResponseEvent::ToolDone(builtin_call(0, "call-1")),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let observer = Arc::new(CancelOnStepFinishedObserver::default());

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::multi_step_model(vec![first_step])),
        Arc::new(support::delayed_tool_runner([(
            "call-1",
            0,
            ToolResult::ok(json!({"source":"fast"})),
        )])),
        Arc::new(support::FakeAuthorizer::default()),
        observer.clone(),
    );
    observer.install_handle(handle.controller()).await;

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        events.push(event);
    }

    task.await.unwrap().unwrap();

    assert!(
        events
            .iter()
            .any(|event| matches!(event, TurnEvent::StepFinished { .. }))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Cancelled
        }
    )));
}

#[tokio::test]
async fn cancelling_during_model_start_finishes_without_waiting_for_stream_creation() {
    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "cancel before stream".into(),
    };
    let model = Arc::new(BlockingStartModelRunner::default());

    let (handle, task) = TurnDriver::spawn(
        context,
        model.clone(),
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    handle.cancel().await.unwrap();

    let cancelled = timeout(Duration::from_millis(50), async {
        while let Some(event) = handle.next_event().await {
            if matches!(
                event,
                TurnEvent::TurnFinished {
                    reason: TurnFinishReason::Cancelled
                }
            ) {
                return true;
            }
        }
        false
    })
    .await;

    model.release();

    assert!(matches!(cancelled, Ok(true)));
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn cancelling_during_tool_execution_keeps_results_that_finish_after_cancel() {
    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "cancel me".into(),
    };

    let first_step = vec![
        ResponseEvent::ToolDone(builtin_call(0, "call-1")),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let (started_tx, started_rx) = oneshot::channel();

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::multi_step_model(vec![first_step])),
        Arc::new(CompletesAfterCancelToolRunner::new(started_tx)),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    while let Some(event) = handle.next_event().await {
        if matches!(event, TurnEvent::ToolCallPrepared { .. }) {
            break;
        }
    }
    started_rx.await.unwrap();
    handle.cancel().await.unwrap();

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        events.push(event);
    }

    task.await.unwrap().unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::ToolCallCompleted {
            call_id,
            result: ToolOutcome::Success(value),
        } if call_id.as_ref() == "call-1" && value["source"] == "completed-after-cancel"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Cancelled
        }
    )));
}

#[derive(Default)]
struct BlockingStartModelRunner {
    release: Notify,
}

impl BlockingStartModelRunner {
    fn release(&self) {
        self.release.notify_waiters();
    }
}

#[async_trait]
impl ModelRunner for BlockingStartModelRunner {
    async fn start(
        &self,
        _request: LlmStepRequest,
    ) -> Result<argus_core::ResponseStream, TurnError> {
        self.release.notified().await;

        let (tx, rx) = mpsc::channel(1);
        let producer = task::spawn(async move {
            tx.send(ResponseEvent::Done {
                reason: FinishReason::Stop,
                usage: Some(Usage::zero()),
            })
            .await
            .unwrap();
        });

        Ok(argus_core::ResponseStream::from_parts(
            rx,
            producer.abort_handle(),
        ))
    }
}

struct CompletesAfterCancelToolRunner {
    started_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl CompletesAfterCancelToolRunner {
    fn new(started_tx: oneshot::Sender<()>) -> Self {
        Self {
            started_tx: Mutex::new(Some(started_tx)),
        }
    }
}

#[async_trait]
impl ToolRunner for CompletesAfterCancelToolRunner {
    async fn execute(&self, _call: ToolCall, ctx: ToolContext) -> Result<ToolResult, TurnError> {
        if let Some(started_tx) = self.started_tx.lock().await.take() {
            started_tx.send(()).unwrap();
        }
        ctx.cancel_token.cancelled().await;
        tokio::task::yield_now().await;
        Ok(ToolResult::ok(json!({"source":"completed-after-cancel"})))
    }
}

#[derive(Default)]
struct CancelOnStepFinishedObserver {
    handle: Mutex<Option<TurnController>>,
}

impl CancelOnStepFinishedObserver {
    async fn install_handle(&self, handle: TurnController) {
        *self.handle.lock().await = Some(handle);
    }
}

#[async_trait]
impl TurnObserver for CancelOnStepFinishedObserver {
    async fn on_event(&self, event: &TurnEvent) -> Result<(), TurnError> {
        if matches!(event, TurnEvent::StepFinished { .. })
            && let Some(handle) = self.handle.lock().await.clone()
        {
            handle.cancel().await?;
        }
        Ok(())
    }
}
