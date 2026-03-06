mod support;

use std::sync::Arc;

use argus_core::{Builtin, BuiltinToolCall, FinishReason, ResponseEvent, ToolCall, Usage};
use serde_json::json;
use tool::ToolResult;
use turn::{StepFinishReason, ToolOutcome, TurnContext, TurnDriver, TurnEvent, TurnFinishReason};

fn builtin_call(sequence: u32, call_id: &str) -> ToolCall {
    ToolCall::Builtin(BuiltinToolCall {
        sequence,
        call_id: call_id.into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    })
}

#[tokio::test]
async fn tool_batch_emits_each_result_immediately_then_finishes_step_once() {
    let context = TurnContext {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        user_message: "read files".into(),
    };

    let first_step = vec![
        ResponseEvent::ToolDone(builtin_call(0, "call-1")),
        ResponseEvent::ToolDone(builtin_call(1, "call-2")),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let second_step = vec![
        ResponseEvent::ContentDelta("done".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ];

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::multi_step_model(vec![first_step, second_step])),
        Arc::new(support::delayed_tool_runner([
            ("call-1", 40, ToolResult::ok(json!({"source":"slow"}))),
            ("call-2", 5, ToolResult::ok(json!({"source":"fast"}))),
        ])),
        Arc::new(support::FakeAuthorizer),
        Arc::new(support::FakeObserver),
    );

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        events.push(event);
    }

    task.await.unwrap().unwrap();

    let completed: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            TurnEvent::ToolCallCompleted { call_id, result } => Some((call_id.as_str(), result)),
            _ => None,
        })
        .collect();

    assert_eq!(completed.len(), 2);
    assert_eq!(completed[0].0, "call-2");
    assert!(matches!(completed[0].1, ToolOutcome::Success(value) if value["source"] == "fast"));
    assert_eq!(completed[1].0, "call-1");

    let step_finishes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            TurnEvent::StepFinished { step_index, reason } => Some((*step_index, reason)),
            _ => None,
        })
        .collect();
    assert_eq!(step_finishes.len(), 1);
    assert_eq!(step_finishes[0].0, 0);
    assert!(matches!(step_finishes[0].1, StepFinishReason::ToolCalls));

    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Completed
        }
    )));
}
