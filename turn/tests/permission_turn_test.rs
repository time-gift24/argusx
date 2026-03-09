mod support;

use std::sync::Arc;

use argus_core::{Builtin, BuiltinToolCall, FinishReason, ResponseEvent, ToolCall, Usage};
use serde_json::json;
use tool::ToolResult;
use turn::{PermissionDecision, TurnDriver, TurnEvent, TurnFinishReason, TurnSeed};

fn builtin_call(sequence: u32, call_id: &str) -> ToolCall {
    ToolCall::Builtin(BuiltinToolCall {
        sequence,
        call_id: call_id.into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    })
}

#[tokio::test]
async fn turn_waits_for_permission_and_resumes_after_allow() {
    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "read file".into(),
        system_prompt: None,
    };

    let first_step = vec![
        ResponseEvent::ToolDone(builtin_call(0, "call-1")),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let second_step = vec![
        ResponseEvent::ContentDelta("approved".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ];

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::multi_step_model(vec![first_step, second_step])),
        Arc::new(support::delayed_tool_runner([(
            "call-1",
            0,
            ToolResult::ok(json!({"source":"approved"})),
        )])),
        Arc::new(support::permission_authorizer([(
            "call-1",
            support::ask_permission("call-1", "perm-1"),
        )])),
        Arc::new(support::FakeObserver),
    );

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        if let TurnEvent::ToolCallPermissionRequested { request } = &event {
            handle
                .resolve_permission(request.request_id.clone(), PermissionDecision::Allow)
                .await
                .unwrap();
        }
        events.push(event);
    }

    task.await.unwrap().unwrap();

    assert!(
        events
            .iter()
            .any(|event| matches!(event, TurnEvent::ToolCallPermissionRequested { .. }))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::ToolCallPermissionResolved { request_id, decision }
            if request_id.as_ref() == "perm-1" && matches!(decision, PermissionDecision::Allow)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::ToolCallCompleted { call_id, .. } if call_id.as_ref() == "call-1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Completed
        }
    )));
}
