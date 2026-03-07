//! Regression tests for transcript-driven turn semantics.
//!
//! All tests here validate behaviour that the design document calls
//! "semantic closure": tool results must feed back into the next LLM step,
//! the turn must have a hard termination boundary, and finish reasons must
//! be preserved faithfully.

mod support;

use std::{sync::Arc, time::Duration};

use argus_core::{Builtin, BuiltinToolCall, FinishReason, ResponseEvent, ToolCall, Usage};
use serde_json::json;
use tool::ToolResult;
use turn::{
    FinalStepPolicy, LlmStepRequest, TurnContext, TurnDriver, TurnEvent, TurnFinishReason,
    TurnMessage, TurnOptions,
};

fn context() -> TurnContext {
    TurnContext {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        user_message: "do tools".into(),
    }
}

fn builtin_call(sequence: u32, call_id: &str) -> ToolCall {
    ToolCall::Builtin(BuiltinToolCall {
        sequence,
        call_id: call_id.into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    })
}

async fn collect_events(handle: turn::TurnHandle) -> Vec<TurnEvent> {
    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        events.push(event);
    }
    events
}

// ---------------------------------------------------------------------------
// Test 1: second step receives prior assistant tool calls and tool result
// ---------------------------------------------------------------------------

#[tokio::test]
async fn second_step_receives_tool_calls_and_result_in_messages() {
    let call = builtin_call(0, "call-1");

    let first_step = vec![
        ResponseEvent::ToolDone(call.clone()),
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

    let model = Arc::new(support::FakeModelRunner::new(vec![first_step, second_step]));
    let model_ref = Arc::clone(&model);

    let (handle, task) = TurnDriver::spawn(
        context(),
        model,
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    collect_events(handle).await;
    task.await.unwrap().unwrap();

    let requests = model_ref.received_requests().await;
    assert_eq!(requests.len(), 2, "expected exactly two model invocations");

    let second = &requests[1];
    // messages must be: User, AssistantToolCalls, ToolResult
    assert_eq!(
        second.messages.len(),
        3,
        "second step must have User + AssistantToolCalls + ToolResult, got: {:?}",
        second.messages
    );
    assert!(
        matches!(&second.messages[0], TurnMessage::User { content } if content == "do tools"),
        "first message must be the user turn"
    );
    assert!(
        matches!(&second.messages[1], TurnMessage::AssistantToolCalls { calls, .. } if calls.len() == 1),
        "second message must carry the assistant tool call"
    );
    assert!(
        matches!(
            &second.messages[2],
            TurnMessage::ToolResult { call_id, is_error, .. }
                if call_id == "call-1" && !is_error
        ),
        "third message must be a successful tool result"
    );
}

// ---------------------------------------------------------------------------
// Test 2: denied tool appears in next step as an error ToolResult
// ---------------------------------------------------------------------------

#[tokio::test]
async fn denied_tool_appears_as_error_tool_result_in_next_step() {
    use turn::AuthorizationDecision;

    let call = builtin_call(0, "call-denied");

    let first_step = vec![
        ResponseEvent::ToolDone(call.clone()),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let second_step = vec![
        ResponseEvent::ContentDelta("ok".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ];

    let model = Arc::new(support::FakeModelRunner::new(vec![first_step, second_step]));
    let model_ref = Arc::clone(&model);

    let authorizer = support::permission_authorizer([("call-denied", AuthorizationDecision::Deny)]);

    let (handle, task) = TurnDriver::spawn(
        context(),
        model,
        Arc::new(support::instant_tool_runner()),
        Arc::new(authorizer),
        Arc::new(support::FakeObserver),
    );

    collect_events(handle).await;
    task.await.unwrap().unwrap();

    let requests = model_ref.received_requests().await;
    assert_eq!(requests.len(), 2);

    let second = &requests[1];
    assert_eq!(second.messages.len(), 3);
    assert!(
        matches!(
            &second.messages[2],
            TurnMessage::ToolResult { call_id, is_error, .. }
                if call_id == "call-denied" && *is_error
        ),
        "denied tool must appear as is_error=true ToolResult, got: {:?}",
        second.messages[2]
    );
}

// ---------------------------------------------------------------------------
// Test 3: timed-out tool appears in next step as an error ToolResult
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timed_out_tool_appears_as_error_tool_result_in_next_step() {
    let call = builtin_call(0, "call-slow");

    let first_step = vec![
        ResponseEvent::ToolDone(call.clone()),
        ResponseEvent::Done {
            reason: FinishReason::ToolCalls,
            usage: Some(Usage::zero()),
        },
    ];
    let second_step = vec![
        ResponseEvent::ContentDelta("ok".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ];

    let model = Arc::new(support::FakeModelRunner::new(vec![first_step, second_step]));
    let model_ref = Arc::clone(&model);

    let options = TurnOptions {
        tool_timeout: Duration::from_millis(10),
        ..TurnOptions::default()
    };

    let (handle, task) = TurnDriver::spawn_with_options(
        context(),
        options,
        model,
        Arc::new(support::delayed_tool_runner([(
            "call-slow",
            200,
            ToolResult::ok(json!({})),
        )])),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    collect_events(handle).await;
    task.await.unwrap().unwrap();

    let requests = model_ref.received_requests().await;
    assert_eq!(requests.len(), 2);

    let second = &requests[1];
    assert_eq!(second.messages.len(), 3);
    assert!(
        matches!(
            &second.messages[2],
            TurnMessage::ToolResult { call_id, is_error, .. }
                if call_id == "call-slow" && *is_error
        ),
        "timed-out tool must appear as is_error=true ToolResult"
    );
}

// ---------------------------------------------------------------------------
// Test 4: FinishReason::Length maps to TurnFinishReason::ModelLengthLimit
// ---------------------------------------------------------------------------

#[tokio::test]
async fn length_finish_reason_maps_to_model_length_limit() {
    let step = vec![
        ResponseEvent::ContentDelta("truncated output".into()),
        ResponseEvent::Done {
            reason: FinishReason::Length,
            usage: Some(Usage::zero()),
        },
    ];

    let (handle, task) = TurnDriver::spawn(
        context(),
        Arc::new(support::FakeModelRunner::new(vec![step])),
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let events = collect_events(handle).await;
    task.await.unwrap().unwrap();

    assert!(
        events.iter().any(|e| matches!(
            e,
            TurnEvent::TurnFinished {
                reason: TurnFinishReason::ModelLengthLimit
            }
        )),
        "Length finish reason must produce ModelLengthLimit, got: {:?}",
        events.last()
    );
}

// ---------------------------------------------------------------------------
// Test 5: Unknown finish reason maps to TurnFinishReason::ModelProtocolError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_finish_reason_maps_to_model_protocol_error() {
    let step = vec![ResponseEvent::Done {
        reason: FinishReason::Unknown("content_filter".into()),
        usage: Some(Usage::zero()),
    }];

    let (handle, task) = TurnDriver::spawn(
        context(),
        Arc::new(support::FakeModelRunner::new(vec![step])),
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let events = collect_events(handle).await;
    task.await.unwrap().unwrap();

    assert!(
        events.iter().any(|e| matches!(
            e,
            TurnEvent::TurnFinished {
                reason: TurnFinishReason::ModelProtocolError
            }
        )),
        "Unknown finish reason must produce ModelProtocolError"
    );
}

// ---------------------------------------------------------------------------
// Test 6: max_steps with ForceText — model always wants tools but turn completes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn max_steps_force_text_makes_turn_complete_despite_greedy_model() {
    // Each tool-call step returns one tool call.
    // The forced-text step (step 8, allow_tools=false) returns Stop.
    let mut steps: Vec<Vec<ResponseEvent>> = (0..8)
        .map(|i| {
            vec![
                ResponseEvent::ToolDone(builtin_call(0, &format!("call-{i}"))),
                ResponseEvent::Done {
                    reason: FinishReason::ToolCalls,
                    usage: Some(Usage::zero()),
                },
            ]
        })
        .collect();
    // The final forced-text step
    steps.push(vec![
        ResponseEvent::ContentDelta("I am done".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ]);

    let options = TurnOptions {
        max_steps: 8,
        final_step_policy: FinalStepPolicy::ForceText,
        ..TurnOptions::default()
    };

    let model = Arc::new(support::FakeModelRunner::new(steps));
    let model_ref = Arc::clone(&model);

    let (handle, task) = TurnDriver::spawn_with_options(
        context(),
        options,
        model,
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let events = collect_events(handle).await;
    task.await.unwrap().unwrap();

    assert!(
        events.iter().any(|e| matches!(
            e,
            TurnEvent::TurnFinished {
                reason: TurnFinishReason::Completed
            }
        )),
        "Turn must complete (not fail) after ForceText final step"
    );

    let requests = model_ref.received_requests().await;
    assert_eq!(
        requests.len(),
        9,
        "expected 8 tool-call steps + 1 forced-text step"
    );

    let last_request = requests.last().unwrap();
    assert!(
        !last_request.allow_tools,
        "final step must have allow_tools=false"
    );
}

// ---------------------------------------------------------------------------
// Test 7: max_steps with Fail — turn finishes with MaxStepsExceeded
// ---------------------------------------------------------------------------

#[tokio::test]
async fn max_steps_fail_policy_produces_max_steps_exceeded() {
    // All steps return tool calls — the turn should fail at max_steps.
    let steps: Vec<Vec<ResponseEvent>> = (0..5)
        .map(|i| {
            vec![
                ResponseEvent::ToolDone(builtin_call(0, &format!("call-{i}"))),
                ResponseEvent::Done {
                    reason: FinishReason::ToolCalls,
                    usage: Some(Usage::zero()),
                },
            ]
        })
        .collect();

    let options = TurnOptions {
        max_steps: 3,
        final_step_policy: FinalStepPolicy::Fail,
        ..TurnOptions::default()
    };

    let (handle, task) = TurnDriver::spawn_with_options(
        context(),
        options,
        Arc::new(support::FakeModelRunner::new(steps)),
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let events = collect_events(handle).await;
    task.await.unwrap().unwrap();

    assert!(
        events.iter().any(|e| matches!(
            e,
            TurnEvent::TurnFinished {
                reason: TurnFinishReason::MaxStepsExceeded
            }
        )),
        "Fail policy must produce MaxStepsExceeded, got: {:?}",
        events.last()
    );
}

// ---------------------------------------------------------------------------
// Test 8: stream idle timeout produces LlmTimeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_idle_timeout_produces_llm_timeout() {
    use argus_core::ResponseStream;
    use async_trait::async_trait;
    use tokio::{sync::mpsc, task};
    use turn::ModelRunner;

    struct HangingModel;

    #[async_trait]
    impl ModelRunner for HangingModel {
        async fn start(&self, _request: LlmStepRequest) -> Result<ResponseStream, turn::TurnError> {
            let (tx, rx) = mpsc::channel::<ResponseEvent>(4);
            // Send one delta so the stream starts, then hang forever.
            tx.send(ResponseEvent::ContentDelta("partial".into()))
                .await
                .unwrap();
            let handle = task::spawn(async move {
                let _tx = tx; // keep sender alive so channel never closes
                tokio::time::sleep(Duration::from_secs(3600)).await;
            });
            Ok(ResponseStream::from_parts(rx, handle.abort_handle()))
        }
    }

    let options = TurnOptions {
        stream_idle_timeout: Duration::from_millis(80),
        ..TurnOptions::default()
    };

    let (handle, task) = TurnDriver::spawn_with_options(
        context(),
        options,
        Arc::new(HangingModel),
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let events = collect_events(handle).await;
    task.await.unwrap().unwrap();

    assert!(
        events.iter().any(|e| matches!(
            e,
            TurnEvent::TurnFinished {
                reason: TurnFinishReason::LlmTimeout
            }
        )),
        "Stream idle timeout must produce LlmTimeout"
    );
}
