mod support;

use std::{sync::Arc, time::Duration};

use argus_core::{Builtin, BuiltinToolCall, FinishReason, ResponseEvent, ToolCall, Usage};
use serde_json::json;
use tool::ToolResult;
use turn::{ToolOutcome, TurnContext, TurnDriver, TurnEvent, TurnFinishReason, TurnOptions};

fn builtin_call(sequence: u32, call_id: &str) -> ToolCall {
    ToolCall::Builtin(BuiltinToolCall {
        sequence,
        call_id: call_id.into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    })
}

#[tokio::test]
async fn slow_tool_times_out_but_turn_still_completes() {
    let context = TurnContext {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        user_message: "timeout".into(),
    };

    let first_step = vec![
        ResponseEvent::ToolDone(builtin_call(0, "call-1")),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let second_step = vec![
        ResponseEvent::ContentDelta("after timeout".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ];

    let (handle, task) = TurnDriver::spawn_with_options(
        context,
        TurnOptions {
            tool_timeout: Duration::from_millis(10),
        },
        Arc::new(support::multi_step_model(vec![first_step, second_step])),
        Arc::new(support::delayed_tool_runner([(
            "call-1",
            100,
            ToolResult::ok(json!({"source":"slow"})),
        )])),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        events.push(event);
    }

    task.await.unwrap().unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::ToolCallCompleted {
            result: ToolOutcome::TimedOut,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Completed
        }
    )));
}
