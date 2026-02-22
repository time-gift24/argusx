use agent_core::{
    InputEnvelope, RunStreamEvent, RuntimeEvent, ToolCallStatus, ToolResult, TranscriptItem,
    TurnStats, UiThreadEvent,
};

use crate::effect::Effect;
use crate::state::{Lifecycle, ModelState, TurnEngineConfig, TurnState};
use crate::transition::Transition;

pub fn reduce(state: TurnState, event: RuntimeEvent, config: &TurnEngineConfig) -> Transition {
    let mut tr = Transition::new(state);

    if tr.state.mark_seen(event.id()) {
        return tr;
    }

    if matches!(tr.state.lifecycle, Lifecycle::Done | Lifecycle::Failed) {
        return tr;
    }

    match event {
        RuntimeEvent::TurnStarted { turn_id, input, .. } => {
            if turn_id != tr.state.meta.turn_id {
                tr.add_run_event(RunStreamEvent::ProtocolWarning {
                    turn_id: tr.state.meta.turn_id.clone(),
                    message: format!(
                        "turn id mismatch: expected {}, got {turn_id}",
                        tr.state.meta.turn_id
                    ),
                });
                return tr;
            }

            tr.add_item(TranscriptItem::user_message(input.clone()));
            tr.state.enqueue_input(input);
            tr.add_run_event(RunStreamEvent::TurnStart {
                turn_id: tr.state.meta.turn_id.clone(),
            });
            start_model_from_pending(&mut tr, 0);
        }
        RuntimeEvent::ModelTextDelta { epoch, delta, .. } => {
            if !is_active_epoch(&tr.state, epoch) {
                return tr;
            }
            tr.state.output_buffer.push_str(&delta);
            tr.add_ui_event(UiThreadEvent::MessageDelta {
                turn_id: tr.state.meta.turn_id.clone(),
                delta,
            });
        }
        RuntimeEvent::ModelReasoningDelta { epoch, delta, .. } => {
            if !is_active_epoch(&tr.state, epoch) {
                return tr;
            }
            tr.state.reasoning_buffer.push_str(&delta);
            tr.add_ui_event(UiThreadEvent::ReasoningDelta {
                turn_id: tr.state.meta.turn_id.clone(),
                delta,
            });
        }
        RuntimeEvent::ModelToolCall { epoch, call, .. } => {
            if !is_active_epoch(&tr.state, epoch) {
                return tr;
            }
            let call_id = call.call_id.clone();
            if tr.state.inflight_tools.contains_key(&call_id) {
                tr.add_run_event(RunStreamEvent::ProtocolWarning {
                    turn_id: tr.state.meta.turn_id.clone(),
                    message: format!("duplicate tool call id: {call_id}"),
                });
                return tr;
            }

            tr.state
                .inflight_tools
                .insert(call_id.clone(), call.clone());
            tr.add_item(TranscriptItem::tool_call(epoch, call.clone()));
            tr.add_run_event(RunStreamEvent::ToolExecutionPlanned {
                turn_id: tr.state.meta.turn_id.clone(),
                call_id: call_id.clone(),
                tool_name: call.tool_name.clone(),
            });
            tr.add_ui_event(UiThreadEvent::ToolCallRequested {
                turn_id: tr.state.meta.turn_id.clone(),
                call_id,
                tool_name: call.tool_name.clone(),
            });
            tr.add_effect(Effect::ExecuteTool { epoch, call });
        }
        RuntimeEvent::ToolDispatched { epoch, call_id, .. } => {
            if !is_active_epoch(&tr.state, epoch) {
                return tr;
            }
            if let Some(call) = tr.state.inflight_tools.get(&call_id) {
                tr.add_run_event(RunStreamEvent::ToolExecutionStart {
                    turn_id: tr.state.meta.turn_id.clone(),
                    call_id: call_id.clone(),
                    tool_name: call.tool_name.clone(),
                });
                tr.add_ui_event(UiThreadEvent::ToolCallProgress {
                    turn_id: tr.state.meta.turn_id.clone(),
                    call_id,
                    status: ToolCallStatus::Running,
                });
            }
        }
        RuntimeEvent::ToolResultOk { epoch, result, .. } => {
            on_tool_result(&mut tr, epoch, result, false);
        }
        RuntimeEvent::ToolResultErr { epoch, result, .. } => {
            on_tool_result(&mut tr, epoch, result, true);
        }
        RuntimeEvent::InputInjected { input, .. } => {
            tr.add_item(TranscriptItem::user_message(input.clone()));
            tr.add_run_event(RunStreamEvent::InputInjected {
                turn_id: tr.state.meta.turn_id.clone(),
                input_id: input.id.clone(),
            });
            tr.state.enqueue_input(input);
            if tr.state.model_state == ModelState::Completed && tr.state.inflight_tools.is_empty() {
                let next_epoch = tr.state.epoch.saturating_add(1);
                start_model_from_pending(&mut tr, next_epoch);
            }
        }
        RuntimeEvent::ModelCompleted { epoch, usage, .. } => {
            if !is_active_epoch(&tr.state, epoch) {
                return tr;
            }
            if let Some(usage) = usage.as_ref() {
                tr.state.usage.merge(usage);
            }
            tr.state.model_state = ModelState::Completed;
            tr.state.retry_attempt = 0;

            tr.add_run_event(RunStreamEvent::ModelCompleted {
                turn_id: tr.state.meta.turn_id.clone(),
                usage,
            });

            if tr.state.inflight_tools.is_empty() && !tr.state.pending_inputs.is_empty() {
                let next_epoch = tr.state.epoch.saturating_add(1);
                start_model_from_pending(&mut tr, next_epoch);
            }
        }
        RuntimeEvent::RetryTimerFired { next_epoch, .. } => {
            if tr.state.lifecycle != Lifecycle::Backoff {
                return tr;
            }
            tr.state.lifecycle = Lifecycle::Active;
            tr.state.model_state = ModelState::Streaming;
            tr.state.epoch = next_epoch;

            let inputs = if tr.state.pending_inputs.is_empty() {
                tr.state.last_request_inputs.clone()
            } else {
                tr.state.drain_pending_inputs()
            };

            if inputs.is_empty() {
                let message = "retry fired but no input available".to_string();
                fail_turn(&mut tr, message, false);
                return tr;
            }

            tr.state.last_request_inputs = inputs.clone();
            tr.add_effect(Effect::StartModel {
                epoch: next_epoch,
                transcript: tr.state.transcript.clone(),
                inputs,
            });
        }
        RuntimeEvent::TransientError {
            epoch,
            message,
            retry_after_ms,
            ..
        } => {
            if epoch != tr.state.epoch {
                return tr;
            }
            tr.state.model_state = ModelState::Error;
            let attempt = tr.state.retry_attempt.saturating_add(1);
            let can_retry = attempt <= config.retry_policy.max_retries;
            tr.add_run_event(RunStreamEvent::TransientError {
                turn_id: tr.state.meta.turn_id.clone(),
                message: message.clone(),
                can_retry,
            });

            if can_retry {
                tr.state.retry_attempt = attempt;
                tr.state.lifecycle = Lifecycle::Backoff;
                let delay_ms = retry_after_ms.unwrap_or_else(|| backoff_ms(config, attempt));
                let next_epoch = tr.state.epoch.saturating_add(1);

                tr.add_run_event(RunStreamEvent::Retrying {
                    turn_id: tr.state.meta.turn_id.clone(),
                    attempt,
                    next_epoch,
                    delay_ms,
                });
                tr.add_ui_event(UiThreadEvent::Warning {
                    turn_id: tr.state.meta.turn_id.clone(),
                    message,
                });
                tr.add_effect(Effect::ScheduleRetry {
                    delay_ms,
                    next_epoch,
                });
            } else {
                fail_turn(&mut tr, message, false);
            }
        }
        RuntimeEvent::FatalError { message, .. } => {
            fail_turn(&mut tr, message, false);
        }
        RuntimeEvent::CancelRequested { reason, .. } => {
            let message = reason.unwrap_or_else(|| "turn cancelled".to_string());
            fail_turn(&mut tr, message, true);
            tr.add_effect(Effect::CancelInflightTools);
        }
    }

    maybe_finalize(&mut tr);
    tr
}

fn is_active_epoch(state: &TurnState, epoch: u64) -> bool {
    state.lifecycle == Lifecycle::Active
        && state.model_state == ModelState::Streaming
        && epoch == state.epoch
}

fn start_model_from_pending(tr: &mut Transition, next_epoch: u64) {
    let inputs = tr.state.drain_pending_inputs();
    if inputs.is_empty() {
        return;
    }

    tr.state.epoch = next_epoch;
    tr.state.lifecycle = Lifecycle::Active;
    tr.state.model_state = ModelState::Streaming;
    tr.state.last_request_inputs = inputs.clone();
    tr.add_effect(Effect::StartModel {
        epoch: next_epoch,
        transcript: tr.state.transcript.clone(),
        inputs,
    });
}

fn on_tool_result(tr: &mut Transition, epoch: u64, result: ToolResult, is_error: bool) {
    if epoch != tr.state.epoch {
        return;
    }
    let call_id = result.call_id.clone();
    if tr.state.inflight_tools.remove(&call_id).is_none() {
        tr.add_run_event(RunStreamEvent::ProtocolWarning {
            turn_id: tr.state.meta.turn_id.clone(),
            message: format!("tool result without inflight call: {call_id}"),
        });
        return;
    }

    tr.add_item(TranscriptItem::tool_result(epoch, result.clone()));

    if is_error {
        tr.add_run_event(RunStreamEvent::ToolExecutionError {
            turn_id: tr.state.meta.turn_id.clone(),
            result: result.clone(),
        });
        tr.add_ui_event(UiThreadEvent::ToolCallProgress {
            turn_id: tr.state.meta.turn_id.clone(),
            call_id: call_id.clone(),
            status: ToolCallStatus::Failed,
        });
    } else {
        tr.add_run_event(RunStreamEvent::ToolExecutionDone {
            turn_id: tr.state.meta.turn_id.clone(),
            result: result.clone(),
        });
        tr.add_ui_event(UiThreadEvent::ToolCallProgress {
            turn_id: tr.state.meta.turn_id.clone(),
            call_id: call_id.clone(),
            status: ToolCallStatus::Completed,
        });
    }

    tr.add_ui_event(UiThreadEvent::ToolCallCompleted {
        turn_id: tr.state.meta.turn_id.clone(),
        result: result.clone(),
    });

    tr.state
        .enqueue_input(InputEnvelope::tool_json(result.output));

    if tr.state.model_state == ModelState::Completed && tr.state.inflight_tools.is_empty() {
        let next_epoch = tr.state.epoch.saturating_add(1);
        start_model_from_pending(tr, next_epoch);
    }
}

fn maybe_finalize(tr: &mut Transition) {
    if !tr.state.can_finish() {
        return;
    }

    if !tr.state.reasoning_buffer.is_empty() {
        tr.add_item(TranscriptItem::reasoning(tr.state.reasoning_buffer.clone()));
    }

    if !tr.state.output_buffer.is_empty() {
        tr.add_item(TranscriptItem::assistant_message(
            tr.state.output_buffer.clone(),
        ));
    }

    tr.state.done_emitted = true;
    tr.state.lifecycle = Lifecycle::Done;

    let stats = TurnStats {
        tool_calls_count: tr.state.tool_calls_count(),
        total_input_tokens: tr.state.usage.input_tokens,
        total_output_tokens: tr.state.usage.output_tokens,
    };

    let final_message =
        (!tr.state.output_buffer.is_empty()).then_some(tr.state.output_buffer.clone());

    tr.add_run_event(RunStreamEvent::TurnDone {
        turn_id: tr.state.meta.turn_id.clone(),
        epoch: tr.state.epoch,
        final_message: final_message.clone(),
        usage: tr.state.usage.clone(),
        stats: stats.clone(),
    });

    tr.add_ui_event(UiThreadEvent::Done {
        turn_id: tr.state.meta.turn_id.clone(),
        summary: final_message,
        stats,
    });

    tr.add_effect(Effect::PersistCheckpoint);
}

fn fail_turn(tr: &mut Transition, message: String, cancelled: bool) {
    tr.state.lifecycle = Lifecycle::Failed;
    tr.state.model_state = ModelState::Error;
    let stats = TurnStats {
        tool_calls_count: tr.state.tool_calls_count(),
        total_input_tokens: tr.state.usage.input_tokens,
        total_output_tokens: tr.state.usage.output_tokens,
    };
    tr.add_item(TranscriptItem::system_note(
        agent_core::NoteLevel::Error,
        message.clone(),
    ));
    tr.add_run_event(RunStreamEvent::TurnFailed {
        turn_id: tr.state.meta.turn_id.clone(),
        epoch: tr.state.epoch,
        message: message.clone(),
        usage: tr.state.usage.clone(),
        stats,
        cancelled,
    });
    tr.add_ui_event(UiThreadEvent::Error {
        turn_id: tr.state.meta.turn_id.clone(),
        message,
    });
    tr.add_effect(Effect::PersistCheckpoint);
}

fn backoff_ms(config: &TurnEngineConfig, attempt: u32) -> u64 {
    let capped = attempt.saturating_sub(1).min(10);
    let multiplier = 1_u64.checked_shl(capped).unwrap_or(u64::MAX);
    config.retry_policy.base_delay_ms.saturating_mul(multiplier)
}

#[cfg(test)]
mod tests {
    use agent_core::{RuntimeEvent, Usage};

    use super::*;
    use crate::test_helpers::*;

    fn base_state() -> TurnState {
        StateBuilder::new("s1", "t1").build()
    }

    fn cfg() -> TurnEngineConfig {
        test_config()
    }

    #[test]
    fn replay_is_deterministic() {
        let run_once = || {
            let mut state = base_state();
            let mut runs = Vec::new();

            let events = vec![
                EventBuilder::turn_started("t1", user_input("hi")).build(),
                EventBuilder::model_text_delta("hello")
                    .with_epoch(0)
                    .build(),
                EventBuilder::model_completed()
                    .with_epoch(0)
                    .with_usage(Usage {
                        input_tokens: 1,
                        output_tokens: 1,
                        total_tokens: 2,
                    })
                    .build(),
            ];

            for ev in events {
                let tr = reduce(state, ev, &cfg());
                runs.extend(tr.run_events.clone());
                state = tr.state;
            }
            (
                state.lifecycle,
                state.done_emitted,
                state.transcript.len(),
                runs,
            )
        };

        let a = run_once();
        let b = run_once();
        assert_eq!(a, b);
    }

    #[test]
    fn completed_does_not_finish_when_tool_inflight() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("hi")).build(),
            &cfg(),
        )
        .state;

        let state = reduce(
            state,
            EventBuilder::model_tool_call("c1", "echo", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &cfg(),
        )
        .state;

        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg(),
        )
        .state;

        assert_eq!(state.model_state, ModelState::Completed);
        assert!(!state.done_emitted);
    }

    #[test]
    fn live_injection_after_completed_restarts_model() {
        let state = base_state();

        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("first")).build(),
            &cfg(),
        )
        .state;

        let state = reduce(
            state,
            EventBuilder::input_injected(user_input("second")).build(),
            &cfg(),
        )
        .state;

        let tr = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg(),
        );

        assert_eq!(tr.state.epoch, 1);
        assert_eq!(tr.state.model_state, ModelState::Streaming);
        assert!(tr
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { epoch: 1, .. })));
    }

    #[test]
    fn transient_error_schedules_retry_and_bumps_epoch() {
        let state = base_state();
        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("hi")).build(),
            &cfg(),
        )
        .state;

        let backoff = reduce(
            state,
            EventBuilder::transient_error("timeout")
                .with_epoch(0)
                .with_retry_after_ms(1)
                .build(),
            &cfg(),
        );

        assert_eq!(backoff.state.lifecycle, Lifecycle::Backoff);
        assert!(backoff
            .effects
            .iter()
            .any(|e| matches!(e, Effect::ScheduleRetry { next_epoch: 1, .. })));

        let retry = reduce(
            backoff.state,
            RuntimeEvent::RetryTimerFired {
                event_id: "e3".into(),
                next_epoch: 1,
            },
            &cfg(),
        );

        assert_eq!(retry.state.lifecycle, Lifecycle::Active);
        assert_eq!(retry.state.epoch, 1);
        assert!(retry
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { epoch: 1, .. })));
    }

    #[test]
    fn turn_done_is_emitted_once() {
        let state = base_state();
        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("hi")).build(),
            &cfg(),
        )
        .state;

        let state = reduce(
            state,
            EventBuilder::model_text_delta("done").with_epoch(0).build(),
            &cfg(),
        )
        .state;

        let first = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg(),
        );

        assert!(first
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnDone { .. })));

        let second = reduce(
            first.state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg(),
        );

        assert!(!second
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnDone { .. })));
    }
}

// =============================================================================
// Level 1: Single Event Tests
// =============================================================================
//
// This module tests each event type in isolation, verifying:
// - Guard conditions (when event should be processed vs ignored)
// - State mutations (what fields change)
// - Output events (RunStreamEvent, UiThreadEvent)
// - Effects (side effects triggered)
//
// ## Test Structure
//
// Each test follows the Given-When-Then pattern:
// - **Given**: Initial state setup
// - **When**: Event is dispatched
// - **Then**: Expected state changes and outputs

#[cfg(test)]
mod single_event_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::state::{Lifecycle, ModelState};
    use crate::test_helpers::*;
    use agent_core::{RunStreamEvent, ToolCallStatus, UiThreadEvent};

    // -------------------------------------------------------------------------
    // TurnStarted Tests
    // -------------------------------------------------------------------------

    /// Test: TurnStarted - Normal start
    ///
    /// Given: Initial state with lifecycle=Active, model_state=NotStarted
    /// When:  TurnStarted event with matching turn_id
    /// Then:
    ///   - lifecycle stays Active
    ///   - model_state becomes Streaming
    ///   - pending_inputs has 1 item
    ///   - transcript has 1 item (UserMessage)
    ///   - emits TurnStart run event
    ///   - emits StartModel effect
    #[test]
    fn turn_started_normal_start() {
        let state = StateBuilder::new("s1", "t1").build();

        let result = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("hello")).build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Active);
        assert_eq!(result.state.model_state, ModelState::Streaming);
        // Note: pending_inputs is drained by start_model_from_pending
        assert_eq!(result.state.pending_inputs.len(), 0);
        assert_eq!(result.state.transcript.len(), 1);

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnStart { .. })));

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { .. })));
    }

    /// Test: TurnStarted - Mismatched turn_id is ignored
    ///
    /// Given: Initial state with turn_id="t1"
    /// When:  TurnStarted event with turn_id="t2" (different)
    /// Then:
    ///   - State unchanged
    ///   - emits ProtocolWarning
    #[test]
    fn turn_started_mismatched_turn_id() {
        let state = StateBuilder::new("s1", "t1").build();

        let result = reduce(
            state,
            EventBuilder::turn_started("t2", user_input("hello")).build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Active);
        assert_eq!(result.state.model_state, ModelState::NotStarted);
        assert!(result.state.pending_inputs.is_empty());

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::ProtocolWarning { .. })));
    }

    // -------------------------------------------------------------------------
    // ModelTextDelta Tests
    // -------------------------------------------------------------------------

    /// Test: ModelTextDelta - Normal text append
    ///
    /// Given: State with lifecycle=Active, model_state=Streaming, epoch=0
    /// When:  ModelTextDelta{epoch=0, delta="hello"}
    /// Then:
    ///   - output_buffer = "hello"
    ///   - emits MessageDelta UI event
    #[test]
    fn model_text_delta_normal_append() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_text_delta("hello")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert_eq!(result.state.output_buffer, "hello");

        assert!(result
            .ui_events
            .iter()
            .any(|e| matches!(e, UiThreadEvent::MessageDelta { .. })));
    }

    /// Test: ModelTextDelta - Guard fails in non-Streaming state
    ///
    /// Given: State with model_state=NotStarted
    /// When:  ModelTextDelta{epoch=0, delta="hello"}
    /// Then:
    ///   - output_buffer unchanged
    ///   - No UI events emitted
    #[test]
    fn model_text_delta_ignored_in_not_started() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::NotStarted)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_text_delta("hello")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert!(result.state.output_buffer.is_empty());
    }

    /// Test: ModelTextDelta - Guard fails with wrong epoch
    ///
    /// Given: State with epoch=1
    /// When:  ModelTextDelta{epoch=0, delta="hello"}
    /// Then:
    ///   - output_buffer unchanged (epoch mismatch)
    #[test]
    fn model_text_delta_ignored_wrong_epoch() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_epoch(1)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_text_delta("hello")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert!(result.state.output_buffer.is_empty());
    }

    // -------------------------------------------------------------------------
    // ModelToolCall Tests
    // -------------------------------------------------------------------------

    /// Test: ModelToolCall - Normal tool call
    ///
    /// Given: Active streaming state
    /// When:  ModelToolCall{call_id="c1", name="echo", args={}}
    /// Then:
    ///   - inflight_tools has c1
    ///   - transcript has ToolCall item
    ///   - emits ToolExecutionPlanned
    ///   - emits ExecuteTool effect
    #[test]
    fn model_tool_call_normal() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_tool_call("c1", "echo", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert!(result.state.inflight_tools.contains_key("c1"));

        assert!(!result.state.transcript.is_empty());

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::ToolExecutionPlanned { .. })));

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::ExecuteTool { .. })));
    }

    /// Test: ModelToolCall - Duplicate call_id is ignored
    ///
    /// Given: State with inflight_tools containing call_id="c1"
    /// When:  ModelToolCall with call_id="c1" (duplicate)
    /// Then:
    ///   - State unchanged (no duplicate added)
    ///   - emits ProtocolWarning
    #[test]
    fn model_tool_call_duplicate_id() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_inflight_tool("c1", tool_call("c1", "echo", serde_json::json!({})))
            .build();

        let result = reduce(
            state,
            EventBuilder::model_tool_call("c1", "echo", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::ProtocolWarning { .. })));
    }

    // -------------------------------------------------------------------------
    // ToolDispatched Tests
    // -------------------------------------------------------------------------

    /// Test: ToolDispatched - Normal dispatch notification
    ///
    /// Given: State with inflight_tools containing call_id="c1"
    /// When:  ToolDispatched{call_id="c1"}
    /// Then:
    ///   - emits ToolExecutionStart
    ///   - emits ToolCallProgress (Running)
    #[test]
    fn tool_dispatched_normal() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_inflight_tool("c1", tool_call("c1", "echo", serde_json::json!({})))
            .build();

        let result = reduce(
            state,
            EventBuilder::tool_dispatched("c1").with_epoch(0).build(),
            &test_config(),
        );

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::ToolExecutionStart { .. })));

        assert!(result.ui_events.iter().any(|e| matches!(
            e,
            UiThreadEvent::ToolCallProgress {
                status: ToolCallStatus::Running,
                ..
            }
        )));
    }

    /// Test: ToolDispatched - Unknown call_id is ignored
    ///
    /// Given: State with no inflight tools
    /// When:  ToolDispatched{call_id="unknown"}
    /// Then:
    ///   - No events emitted
    #[test]
    fn tool_dispatched_unknown_call_id() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::tool_dispatched("unknown")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert!(result.run_events.is_empty());
    }

    // -------------------------------------------------------------------------
    // ToolResultOk Tests
    // -------------------------------------------------------------------------

    /// Test: ToolResultOk - Normal result processing
    ///
    /// Given: State with inflight_tools containing call_id="c1"
    /// When:  ToolResultOk{call_id="c1", result="ok"}
    /// Then:
    ///   - inflight_tools removes c1
    ///   - pending_inputs has 1 item (tool result as input)
    ///   - emits ToolExecutionDone
    ///   - emits ToolCallCompleted UI event
    #[test]
    fn tool_result_ok_normal() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_inflight_tool("c1", tool_call("c1", "echo", serde_json::json!({})))
            .build();

        let result = reduce(
            state,
            EventBuilder::tool_result_ok("c1", serde_json::json!("result")).build(),
            &test_config(),
        );

        assert!(!result.state.inflight_tools.contains_key("c1"));

        assert!(!result.state.pending_inputs.is_empty());

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::ToolExecutionDone { .. })));
    }

    /// Test: ToolResultOk - Unknown call_id is ignored
    ///
    /// Given: State with no inflight tools
    /// When:  ToolResultOk for unknown call_id
    /// Then:
    ///   - State unchanged
    #[test]
    fn tool_result_ok_unknown_call_id() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::tool_result_ok("unknown", serde_json::json!("result")).build(),
            &test_config(),
        );

        assert!(result.state.inflight_tools.is_empty());
    }

    // -------------------------------------------------------------------------
    // InputInjected Tests
    // -------------------------------------------------------------------------

    /// Test: InputInjected - Normal input injection
    ///
    /// Given: Active state
    /// When:  InputInjected with user input
    /// Then:
    ///   - pending_inputs has 1 item
    ///   - transcript has UserMessage
    ///   - emits InputInjected run event
    #[test]
    fn input_injected_normal() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::input_injected(user_input("injected")).build(),
            &test_config(),
        );

        assert_eq!(result.state.pending_inputs.len(), 1);

        assert!(!result.state.transcript.is_empty());

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::InputInjected { .. })));
    }

    /// Test: InputInjected - Restarts model when completed and no inflight tools
    ///
    /// Given: State with model_state=Completed, inflight_tools empty
    /// When:  InputInjected
    /// Then:
    ///   - epoch becomes 1
    ///   - model_state becomes Streaming
    ///   - emits StartModel effect with epoch=1
    #[test]
    fn input_injected_restarts_model_when_completed() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Completed)
            .build();

        let result = reduce(
            state,
            EventBuilder::input_injected(user_input("injected")).build(),
            &test_config(),
        );

        assert_eq!(result.state.epoch, 1);
        assert_eq!(result.state.model_state, ModelState::Streaming);

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { epoch: 1, .. })));
    }

    // -------------------------------------------------------------------------
    // ModelCompleted Tests
    // -------------------------------------------------------------------------

    /// Test: ModelCompleted - Normal completion
    ///
    /// Given: Active streaming state, no inflight tools
    /// When:  ModelCompleted
    /// Then:
    ///   - model_state = Completed
    ///   - emits ModelCompleted run event
    #[test]
    fn model_completed_normal() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &test_config(),
        );

        assert_eq!(result.state.model_state, ModelState::Completed);

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::ModelCompleted { .. })));
    }

    /// Test: ModelCompleted - Restarts if pending inputs and no inflight tools
    ///
    /// Given: State with pending_inputs, no inflight tools
    /// When:  ModelCompleted
    /// Then:
    ///   - model_state = Streaming
    ///   - epoch becomes 1
    ///   - emits StartModel effect
    #[test]
    fn model_completed_restarts_with_pending_inputs() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_pending_input(user_input("pending"))
            .build();

        let result = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &test_config(),
        );

        assert_eq!(result.state.epoch, 1);
        assert_eq!(result.state.model_state, ModelState::Streaming);

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { epoch: 1, .. })));
    }

    /// Test: ModelCompleted - Guard fails with wrong epoch
    ///
    /// Given: State with epoch=1
    /// When:  ModelCompleted{epoch=0}
    /// Then:
    ///   - model_state unchanged
    #[test]
    fn model_completed_ignored_wrong_epoch() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_epoch(1)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &test_config(),
        );

        assert_eq!(result.state.model_state, ModelState::Streaming);
    }

    // -------------------------------------------------------------------------
    // RetryTimerFired Tests
    // -------------------------------------------------------------------------

    /// Test: RetryTimerFired - Normal retry
    ///
    /// Given: State with lifecycle=Backoff
    /// When:  RetryTimerFired{next_epoch=1}
    /// Then:
    ///   - lifecycle = Active
    ///   - model_state = Streaming
    ///   - epoch = 1
    ///   - emits StartModel effect
    #[test]
    fn retry_timer_fired_normal() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Backoff)
            .with_model_state(ModelState::Error)
            .with_last_request_inputs(vec![user_input("test")])
            .build();

        let result = reduce(
            state,
            EventBuilder::retry_timer_fired(1).build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Active);
        assert_eq!(result.state.model_state, ModelState::Streaming);
        assert_eq!(result.state.epoch, 1);

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartModel { epoch: 1, .. })));
    }

    /// Test: RetryTimerFired - Guard fails if not Backoff
    ///
    /// Given: State with lifecycle=Active
    /// When:  RetryTimerFired
    /// Then:
    ///   - State unchanged
    #[test]
    fn retry_timer_fired_ignored_when_active() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::retry_timer_fired(1).build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Active);
    }

    // -------------------------------------------------------------------------
    // TransientError Tests
    // -------------------------------------------------------------------------

    /// Test: TransientError - Normal retry scheduling
    ///
    /// Given: Active state with retry budget available
    /// When:  TransientError{message="timeout"}
    /// Then:
    ///   - lifecycle = Backoff
    ///   - model_state = Error
    ///   - retry_attempt = 1
    ///   - emits ScheduleRetry effect
    #[test]
    fn transient_error_schedules_retry() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::transient_error("timeout")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Backoff);
        assert_eq!(result.state.model_state, ModelState::Error);
        assert_eq!(result.state.retry_attempt, 1);

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::ScheduleRetry { .. })));
    }

    /// Test: TransientError - Max retries exceeded fails the turn
    ///
    /// Given: State with retry_attempt >= max_retries
    /// When:  TransientError
    /// Then:
    ///   - lifecycle = Failed
    ///   - emits TurnFailed
    #[test]
    fn transient_error_fails_when_exhausted() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_retry_attempt(3)
            .build();

        let result = reduce(
            state,
            EventBuilder::transient_error("timeout")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Failed);

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnFailed { .. })));
    }

    /// Test: TransientError - Ignored if epoch mismatch
    ///
    /// Given: State with epoch=1
    /// When:  TransientError{epoch=0}
    /// Then:
    ///   - State unchanged
    #[test]
    fn transient_error_ignored_wrong_epoch() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_epoch(1)
            .build();

        let result = reduce(
            state,
            EventBuilder::transient_error("timeout")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Active);
    }

    // -------------------------------------------------------------------------
    // FatalError Tests
    // -------------------------------------------------------------------------

    /// Test: FatalError - Always fails the turn
    ///
    /// Given: Any state (Active, Backoff, etc.)
    /// When:  FatalError{message="something went wrong"}
    /// Then:
    ///   - lifecycle = Failed
    ///   - model_state = Error
    ///   - emits TurnFailed
    #[test]
    fn fatal_error_always_fails() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .build();

        let result = reduce(
            state,
            EventBuilder::fatal_error("something went wrong").build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Failed);
        assert_eq!(result.state.model_state, ModelState::Error);

        assert!(result
            .run_events
            .iter()
            .any(|e| matches!(e, RunStreamEvent::TurnFailed { .. })));
    }

    // -------------------------------------------------------------------------
    // CancelRequested Tests
    // -------------------------------------------------------------------------

    /// Test: CancelRequested - Always fails the turn
    ///
    /// Given: Active state
    /// When:  CancelRequested
    /// Then:
    ///   - lifecycle = Failed
    ///   - emits TurnFailed
    ///   - emits CancelInflightTools effect
    #[test]
    fn cancel_requested_fails_turn() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Active)
            .with_model_state(ModelState::Streaming)
            .with_inflight_tool("c1", tool_call("c1", "echo", serde_json::json!({})))
            .build();

        let result = reduce(
            state,
            EventBuilder::cancel_requested().build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Failed);

        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::CancelInflightTools)));
    }

    // -------------------------------------------------------------------------
    // Terminal State Protection Tests
    // -------------------------------------------------------------------------

    /// Test: Done state ignores all events
    ///
    /// Given: State with lifecycle=Done
    /// When:  Any event (ModelTextDelta, ToolResultOk, etc.)
    /// Then:
    ///   - State unchanged
    ///   - No events emitted
    #[test]
    fn done_state_ignores_events() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Done)
            .with_model_state(ModelState::Completed)
            .with_done_emitted(true)
            .build();

        let events = vec![
            EventBuilder::model_text_delta("hello")
                .with_epoch(0)
                .build(),
            EventBuilder::tool_result_ok("c1", serde_json::json!("result")).build(),
            EventBuilder::input_injected(user_input("test")).build(),
        ];

        for event in events {
            let result = reduce(state.clone(), event, &test_config());
            assert_eq!(result.state.lifecycle, Lifecycle::Done);
        }
    }

    /// Test: Failed state ignores all events
    ///
    /// Given: State with lifecycle=Failed
    /// When:  Any event
    /// Then:
    ///   - State unchanged
    #[test]
    fn failed_state_ignores_events() {
        let state = StateBuilder::new("s1", "t1")
            .with_lifecycle(Lifecycle::Failed)
            .with_model_state(ModelState::Error)
            .build();

        let result = reduce(
            state,
            EventBuilder::model_text_delta("hello")
                .with_epoch(0)
                .build(),
            &test_config(),
        );

        assert_eq!(result.state.lifecycle, Lifecycle::Failed);
    }

    // -------------------------------------------------------------------------
    // Idempotency Tests
    // -------------------------------------------------------------------------

    /// Test: Duplicate event IDs are ignored
    ///
    /// Given: State with seen_event_ids containing "e1"
    /// When:  Event with event_id="e1"
    /// Then:
    ///   - State unchanged (event was already processed)
    #[test]
    fn duplicate_event_id_ignored() {
        let state = StateBuilder::new("s1", "t1").with_seen_event("e1").build();

        let result = reduce(
            state,
            RuntimeEvent::ModelTextDelta {
                event_id: "e1".into(),
                epoch: 0,
                delta: "hello".into(),
            },
            &test_config(),
        );

        assert!(result.state.output_buffer.is_empty());
    }
}

// =============================================================================
// Level 2: Event Sequence Tests
// =============================================================================
//
// This module tests sequences of events to verify correct state transitions
// across multiple steps.
//
// ## Test Structure
//
// Each test follows the Given-When-Then pattern:
// - **Given**: Initial state setup
// - **When**: Event sequence [E1, E2, E3, ...]
// - **Then**: Expected intermediate and final states

#[cfg(test)]
mod sequence_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::state::{Lifecycle, ModelState};
    use crate::test_helpers::*;

    // -------------------------------------------------------------------------
    // Simple Conversation Flow
    // -------------------------------------------------------------------------

    /// Test: Simple text-only conversation flow
    ///
    /// Sequence:
    ///   1. TurnStarted -> Starts model, enqueues input
    ///   2. ModelTextDelta "hello" -> Appends to output
    ///   3. ModelCompleted -> Model finishes, turn completes
    ///
    /// Expected Final State:
    ///   - lifecycle = Done
    ///   - output_buffer = "hello"
    ///   - done_emitted = true
    #[test]
    fn sequence_simple_text_conversation() {
        // Given: Initial state
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        // When: Execute event sequence
        let final_state = ScenarioRunner::new(state, &cfg)
            .push(EventBuilder::turn_started("t1", user_input("hi")).build())
            .push(
                EventBuilder::model_text_delta("hello")
                    .with_epoch(0)
                    .build(),
            )
            .push(EventBuilder::model_completed().with_epoch(0).build())
            .run()
            .into_state();

        // Then: Final state
        assert_eq!(final_state.lifecycle, Lifecycle::Done);
        assert_eq!(final_state.output_buffer, "hello");
        assert!(final_state.done_emitted);
    }

    // -------------------------------------------------------------------------
    // Tool Call Flow
    // -------------------------------------------------------------------------

    /// Test: Tool call and result flow
    ///
    /// Sequence:
    ///   1. TurnStarted -> Starts model
    ///   2. ModelToolCall{c1} -> Tool call added to inflight
    ///   3. ToolDispatched{c1} -> Tool execution started
    ///   4. ToolResultOk{c1} -> Tool result received, model restarted
    ///   5. ModelTextDelta "result" -> Final text
    ///   6. ModelCompleted -> Turn completes
    ///
    /// Expected:
    ///   - Model restarts after tool result (epoch=1)
    ///   - Final state = Done
    #[test]
    fn sequence_tool_call_complete_flow() {
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        let result = ScenarioRunner::new(state, &cfg)
            .push(EventBuilder::turn_started("t1", user_input("use echo")).build())
            .push(
                EventBuilder::model_tool_call("c1", "echo", serde_json::json!({}))
                    .with_epoch(0)
                    .build(),
            )
            .push(EventBuilder::tool_dispatched("c1").with_epoch(0).build())
            // Model completes while tool is still running
            .push(EventBuilder::model_completed().with_epoch(0).build())
            // Tool result comes back, model restarts because model_state==Completed
            .push(EventBuilder::tool_result_ok("c1", serde_json::json!("tool result")).build())
            .push(
                EventBuilder::model_text_delta("result")
                    .with_epoch(1)
                    .build(),
            )
            .push(EventBuilder::model_completed().with_epoch(1).build())
            .run();

        // Then: Model restarted after tool result (because model had completed)
        assert_eq!(result.state().epoch, 1);

        // Then: Final state is Done
        assert_eq!(result.state().lifecycle, Lifecycle::Done);
    }

    /// Test: Multiple tools in parallel
    ///
    /// Sequence:
    ///   1. TurnStarted
    ///   2. ModelToolCall{c1}, ModelToolCall{c2}, ModelToolCall{c3} (parallel)
    ///   3. ModelCompleted (model finishes before tools)
    ///   4. ToolResultOk for each tool (in any order)
    ///   5. Model restarts after all tools complete
    ///
    /// Expected:
    ///   - Turn doesn't finish while tools are inflight
    ///   - Model restarts after all tools done
    #[test]
    fn sequence_multiple_tools_parallel() {
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        // Step 1-2: Start and add 3 tool calls
        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("run all")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_tool_call("c1", "tool1", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_tool_call("c2", "tool2", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_tool_call("c3", "tool3", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;

        // Then: All 3 tools in flight
        assert_eq!(state.inflight_tools.len(), 3);

        // Step 3: Model completes (should NOT finish - tools still running)
        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg,
        )
        .state;

        // Then: Model restarted because tools are still inflight, wait for tools
        // Note: Model completes but turn doesn't finalize because inflight_tools is not empty
        assert_eq!(state.model_state, ModelState::Completed);
        assert!(!state.done_emitted);

        // Step 4: Tool results come in
        let state = reduce(
            state,
            EventBuilder::tool_result_ok("c1", serde_json::json!("r1")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::tool_result_ok("c2", serde_json::json!("r2")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::tool_result_ok("c3", serde_json::json!("r3")).build(),
            &cfg,
        )
        .state;

        // Then: All tools complete, model should restart
        assert_eq!(state.epoch, 1);
    }

    // -------------------------------------------------------------------------
    // Error and Retry Flow
    // -------------------------------------------------------------------------

    /// Test: Transient error and retry flow
    ///
    /// Sequence:
    ///   1. TurnStarted
    ///   2. ModelTextDelta "hello"
    ///   3. TransientError -> Enters backoff
    ///   4. RetryTimerFired -> Retries with epoch+1
    ///   5. ModelTextDelta "world"
    ///   6. ModelCompleted -> Turn completes
    ///
    /// Expected:
    ///   - Retry increments epoch
    ///   - Final state = Done
    #[test]
    fn sequence_transient_error_retry() {
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("test")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_text_delta("hello")
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;

        // Transient error
        let backoff = reduce(
            state,
            EventBuilder::transient_error("timeout")
                .with_epoch(0)
                .build(),
            &cfg,
        );
        assert_eq!(backoff.state.lifecycle, Lifecycle::Backoff);

        // Retry timer fires
        let retry = reduce(
            backoff.state,
            EventBuilder::retry_timer_fired(1).build(),
            &cfg,
        );
        assert_eq!(retry.state.epoch, 1);
        assert_eq!(retry.state.lifecycle, Lifecycle::Active);

        // Continue conversation
        let state = reduce(
            retry.state,
            EventBuilder::model_text_delta("world")
                .with_epoch(1)
                .build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(1).build(),
            &cfg,
        )
        .state;

        // Then: Completed successfully after retry
        assert_eq!(state.lifecycle, Lifecycle::Done);
    }

    /// Test: Cancel requested during conversation
    ///
    /// Sequence:
    ///   1. TurnStarted
    ///   2. ModelToolCall{c1}
    ///   3. CancelRequested -> Turn fails
    ///
    /// Expected:
    ///   - lifecycle = Failed
    #[test]
    fn sequence_cancel_during_tool_call() {
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("test")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_tool_call("c1", "echo", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;

        // Cancel
        let result = reduce(state, EventBuilder::cancel_requested().build(), &cfg);

        // Then: Turn failed
        assert_eq!(result.state.lifecycle, Lifecycle::Failed);

        // Then: Cancel effect triggered
        assert!(result
            .effects
            .iter()
            .any(|e| matches!(e, Effect::CancelInflightTools)));
    }

    /// Test: Tool failure and recovery
    ///
    /// Sequence:
    ///   1. TurnStarted
    ///   2. ModelToolCall{c1}
    ///   3. ToolResultErr{c1} -> Tool failed
    ///   4. ModelTextDelta "error occurred"
    ///   5. ModelCompleted -> Turn completes despite tool error
    ///
    /// Expected:
    ///   - Tool error doesn't fail the turn
    ///   - Turn completes normally
    #[test]
    fn sequence_tool_failure_recovery() {
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("use tool")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_tool_call("c1", "failing_tool", serde_json::json!({}))
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::tool_result_err("c1", "tool failed").build(),
            &cfg,
        )
        .state;
        // After tool result, there's pending input (the tool result), so model restarts
        let state = reduce(
            state,
            EventBuilder::model_text_delta("error occurred")
                .with_epoch(0)
                .build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg,
        )
        .state;

        // Now epoch=1, model restarted because there was pending input
        assert_eq!(state.epoch, 1);

        // Complete the restarted model
        let state = reduce(
            state,
            EventBuilder::model_text_delta("done").with_epoch(1).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(1).build(),
            &cfg,
        )
        .state;

        // Then: Turn completed despite tool error
        assert_eq!(state.lifecycle, Lifecycle::Done);
    }

    // -------------------------------------------------------------------------
    // Input Injection Flow
    // -------------------------------------------------------------------------

    /// Test: Input injection during model completion
    ///
    /// Sequence:
    ///   1. TurnStarted{input="first"}
    ///   2. InputInjected{input="second"}
    ///   3. ModelCompleted -> Triggers restart with new input
    ///   4. ModelTextDelta "response"
    ///   5. ModelCompleted -> Turn completes
    ///
    /// Expected:
    ///   - epoch = 1 after restart
    ///   - Final state = Done
    #[test]
    fn sequence_input_injection_during_completion() {
        let state = StateBuilder::new("s1", "t1").build();
        let cfg = test_config();

        let state = reduce(
            state,
            EventBuilder::turn_started("t1", user_input("first")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::input_injected(user_input("second")).build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(0).build(),
            &cfg,
        )
        .state;

        // Then: Model restarted with new input
        assert_eq!(state.epoch, 1);
        assert_eq!(state.model_state, ModelState::Streaming);

        // Continue
        let state = reduce(
            state,
            EventBuilder::model_text_delta("response")
                .with_epoch(1)
                .build(),
            &cfg,
        )
        .state;
        let state = reduce(
            state,
            EventBuilder::model_completed().with_epoch(1).build(),
            &cfg,
        )
        .state;

        // Then: Completed
        assert_eq!(state.lifecycle, Lifecycle::Done);
    }
}
