use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

use argus_core::ToolCall;
use async_trait::async_trait;
use serde_json::json;
use tauri::Emitter;
use tokio::sync::Mutex;
use turn::{StepFinishReason, ToolOutcome, TurnError, TurnEvent, TurnFinishReason, TurnObserver};

use crate::chat::{DesktopTurnEvent, TurnTargetKind};

use super::plan::snapshot_from_tool_outcome;

pub struct TauriTurnObserver {
    app: tauri::AppHandle,
    turn_id: String,
    target_kind: TurnTargetKind,
    target_id: String,
    prepared_tool_names: Mutex<HashMap<String, String>>,
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
            prepared_tool_names: Mutex::new(HashMap::new()),
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
        let extra_plan_event = match event {
            TurnEvent::ToolCallPrepared { call } => {
                self.prepared_tool_names
                    .lock()
                    .await
                    .insert(
                        tool_call_id(call.as_ref()).to_string(),
                        tool_name(call.as_ref()).to_string(),
                    );
                None
            }
            TurnEvent::ToolCallCompleted { call_id, result } => {
                let prepared_tool_name = self.prepared_tool_names.lock().await.remove(call_id.as_ref());

                if prepared_tool_name.as_deref() == Some("update_plan") {
                    plan_updated_event(&self.turn_id, call_id.as_ref(), result)
                } else {
                    None
                }
            }
            _ => None,
        };

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

        if let Some(payload) = extra_plan_event {
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

pub fn plan_updated_event(
    turn_id: &str,
    call_id: &str,
    result: &ToolOutcome,
) -> Option<DesktopTurnEvent> {
    let snapshot = snapshot_from_tool_outcome(call_id, result)?;

    Some(DesktopTurnEvent {
        turn_id: turn_id.to_string(),
        event_type: "plan-updated".to_string(),
        data: serde_json::to_value(snapshot).ok()?,
    })
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
