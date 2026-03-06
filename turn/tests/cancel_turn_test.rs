mod support;

use std::sync::Arc;

use argus_core::{Builtin, BuiltinToolCall, FinishReason, ResponseEvent, ToolCall, Usage};
use serde_json::json;
use tool::ToolResult;
use turn::{TurnContext, TurnDriver, TurnEvent, TurnFinishReason};

fn builtin_call(sequence: u32, call_id: &str) -> ToolCall {
    ToolCall::Builtin(BuiltinToolCall {
        sequence,
        call_id: call_id.into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    })
}

#[tokio::test]
async fn cancelling_during_tool_execution_marks_turn_cancelled_and_keeps_completed_results() {
    let context = TurnContext {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        user_message: "cancel me".into(),
    };

    let first_step = vec![
        ResponseEvent::ToolDone(builtin_call(0, "call-1")),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::multi_step_model(vec![first_step])),
        Arc::new(support::delayed_tool_runner([(
            "call-1",
            200,
            ToolResult::ok(json!({"source":"slow"})),
        )])),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        if matches!(event, TurnEvent::ToolCallPrepared { .. }) {
            handle.cancel().await.unwrap();
        }
        events.push(event);
    }

    task.await.unwrap().unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Cancelled
        }
    )));
}
