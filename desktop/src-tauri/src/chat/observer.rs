use std::sync::{
    atomic::{AtomicBool, Ordering},
};

use argus_core::ToolCall;
use async_trait::async_trait;
use serde_json::json;
use tauri::Emitter;
use turn::{StepFinishReason, ToolOutcome, TurnError, TurnEvent, TurnFinishReason, TurnObserver};

use crate::chat::{DesktopTurnEvent, TurnTargetKind};

pub struct TauriTurnObserver {
    app: tauri::AppHandle,
    turn_id: String,
    target_kind: TurnTargetKind,
    target_id: String,
    saw_failed_finish: AtomicBool,
}

impl TauriTurnObserver {
    pub fn new(
        app: tauri::AppHandle,
        turn_id: String,
        target_kind: TurnTargetKind,
        target_id: String,
    ) -> Self {
        Self {
            app,
            turn_id,
            target_kind,
            target_id,
            saw_failed_finish: AtomicBool::new(false),
        }
    }

    pub fn saw_failed_finish(&self) -> bool {
        self.saw_failed_finish.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl TurnObserver for TauriTurnObserver {
    async fn on_event(&self, event: &TurnEvent) -> Result<(), TurnError> {
        if matches!(
            event,
            TurnEvent::TurnFinished {
                reason: TurnFinishReason::Failed
            }
        ) {
            self.saw_failed_finish.store(true, Ordering::Relaxed);
        }

        if let Some(payload) = map_turn_event(
            &self.turn_id,
            self.target_kind,
            &self.target_id,
            event,
        ) {
            self.app
                .emit("turn-event", payload)
                .map_err(|err| TurnError::Runtime(err.to_string()))?;
        }

        Ok(())
    }
}

pub fn map_turn_event(
    turn_id: &str,
    target_kind: TurnTargetKind,
    target_id: &str,
    event: &TurnEvent,
) -> Option<DesktopTurnEvent> {
    let payload = match event {
        TurnEvent::TurnStarted => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "turn-started".to_string(),
            data: json!({
                "targetKind": target_kind,
                "targetId": target_id,
            }),
        },
        TurnEvent::LlmTextDelta { text } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "llm-text-delta".to_string(),
            data: json!({ "text": text }),
        },
        TurnEvent::LlmReasoningDelta { text } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "llm-reasoning-delta".to_string(),
            data: json!({ "text": text }),
        },
        TurnEvent::ToolCallPrepared { call } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "tool-call-prepared".to_string(),
            data: json!({
                "callId": tool_call_id(call.as_ref()),
                "name": tool_name(call.as_ref()),
                "argumentsJson": tool_arguments_json(call.as_ref()),
            }),
        },
        TurnEvent::ToolCallCompleted { call_id, result } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "tool-call-completed".to_string(),
            data: json!({
                "callId": call_id,
                "result": tool_outcome_value(result),
            }),
        },
        TurnEvent::ToolCallPermissionRequested { request } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "tool-call-permission-requested".to_string(),
            data: json!({
                "requestId": request.request_id,
                "toolCallId": request.tool_call_id,
            }),
        },
        TurnEvent::ToolCallPermissionResolved {
            request_id,
            decision,
        } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "tool-call-permission-resolved".to_string(),
            data: json!({
                "requestId": request_id,
                "decision": match decision {
                    turn::PermissionDecision::Allow => "allow",
                    turn::PermissionDecision::Deny => "deny",
                },
            }),
        },
        TurnEvent::StepFinished { step_index, reason } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "step-finished".to_string(),
            data: json!({
                "stepIndex": step_index,
                "reason": step_finish_reason(reason),
            }),
        },
        TurnEvent::TurnFinished { reason } => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "turn-finished".to_string(),
            data: json!({
                "reason": turn_finish_reason(reason),
            }),
        },
    };

    Some(payload)
}

pub fn turn_failed_event(turn_id: &str, message: &str) -> DesktopTurnEvent {
    DesktopTurnEvent {
        turn_id: turn_id.to_string(),
        event_type: "turn-failed".to_string(),
        data: json!({ "message": message }),
    }
}

fn tool_call_id(call: &ToolCall) -> &str {
    match call {
        ToolCall::FunctionCall { call_id, .. } => call_id,
        ToolCall::Builtin(call) => &call.call_id,
        ToolCall::Mcp(call) => &call.id,
    }
}

fn tool_name(call: &ToolCall) -> &str {
    match call {
        ToolCall::FunctionCall { name, .. } => name,
        ToolCall::Builtin(call) => call.builtin.canonical_name(),
        ToolCall::Mcp(call) => call.name.as_deref().unwrap_or_default(),
    }
}

fn tool_arguments_json(call: &ToolCall) -> &str {
    match call {
        ToolCall::FunctionCall { arguments_json, .. } => arguments_json,
        ToolCall::Builtin(call) => &call.arguments_json,
        ToolCall::Mcp(call) => call.arguments_json.as_deref().unwrap_or("{}"),
    }
}

fn tool_outcome_value(outcome: &ToolOutcome) -> serde_json::Value {
    match outcome {
        ToolOutcome::Success(output) => json!({
            "status": "success",
            "output": output,
        }),
        ToolOutcome::Failed { message, retryable } => json!({
            "status": "failed",
            "message": message,
            "retryable": retryable,
        }),
        ToolOutcome::TimedOut => json!({
            "status": "timed_out",
        }),
        ToolOutcome::Denied => json!({
            "status": "denied",
        }),
        ToolOutcome::Cancelled => json!({
            "status": "cancelled",
        }),
    }
}

fn step_finish_reason(reason: &StepFinishReason) -> &'static str {
    match reason {
        StepFinishReason::ToolCalls => "tool_calls",
    }
}

fn turn_finish_reason(reason: &TurnFinishReason) -> &'static str {
    match reason {
        TurnFinishReason::Completed => "completed",
        TurnFinishReason::Cancelled => "cancelled",
        TurnFinishReason::Failed => "failed",
        TurnFinishReason::MaxStepsExceeded => "max_steps_exceeded",
        TurnFinishReason::ModelLengthLimit => "model_length_limit",
        TurnFinishReason::ModelProtocolError => "model_protocol_error",
        TurnFinishReason::LlmTimeout => "llm_timeout",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_failed_event_uses_expected_shape() {
        let event = turn_failed_event("turn-1", "boom");

        assert_eq!(event.turn_id, "turn-1");
        assert_eq!(event.event_type, "turn-failed");
        assert_eq!(event.data["message"], "boom");
    }
}
