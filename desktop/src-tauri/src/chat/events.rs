use std::collections::HashMap;

use argus_core::ToolCall;
use serde::{Deserialize, Serialize};
use serde_json::json;
use turn::{StepFinishReason, ToolOutcome, TurnEvent, TurnFinishReason};

use crate::chat::plan::{snapshot_from_tool_outcome, DesktopPlanSnapshot};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TurnTargetKind {
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "workflow")]
    Workflow,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTurnInput {
    pub prompt: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTurnResult {
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopTurnEvent {
    pub turn_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Default)]
pub struct DesktopTurnEventMapper {
    prepared_tool_names: HashMap<(String, String), String>,
}

impl DesktopTurnEventMapper {
    pub fn map_event(&mut self, turn_id: &str, event: &TurnEvent) -> Vec<DesktopTurnEvent> {
        let mut events = Vec::with_capacity(2);

        if let Some(mapped) = map_turn_event(turn_id, event) {
            events.push(mapped);
        }

        match event {
            TurnEvent::ToolCallPrepared { call } => {
                self.prepared_tool_names.insert(
                    (turn_id.to_string(), tool_call_id(call.as_ref()).to_string()),
                    tool_name(call.as_ref()).to_string(),
                );
            }
            TurnEvent::ToolCallCompleted { call_id, result } => {
                let prepared_tool_name = self
                    .prepared_tool_names
                    .remove(&(turn_id.to_string(), call_id.to_string()));

                if prepared_tool_name.as_deref() == Some("update_plan") {
                    if let Some(mapped) = plan_updated_event(turn_id, call_id.as_ref(), result) {
                        events.push(mapped);
                    }
                }
            }
            TurnEvent::TurnFinished { .. } => {
                self.prepared_tool_names
                    .retain(|(observed_turn_id, _), _| observed_turn_id != turn_id);
            }
            TurnEvent::TurnStarted
            | TurnEvent::LlmTextDelta { .. }
            | TurnEvent::LlmReasoningDelta { .. }
            | TurnEvent::ToolCallPermissionRequested { .. }
            | TurnEvent::ToolCallPermissionResolved { .. }
            | TurnEvent::StepFinished { .. } => {}
        }

        events
    }
}

pub fn map_turn_event(turn_id: &str, event: &TurnEvent) -> Option<DesktopTurnEvent> {
    let payload = match event {
        TurnEvent::TurnStarted => DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "turn-started".to_string(),
            data: json!({}),
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HydratedChatTurnStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HydratedToolCallStatus {
    Running,
    Success,
    Failed,
    TimedOut,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HydratedToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments_json: String,
    pub output_summary: Option<String>,
    pub error_summary: Option<String>,
    pub status: HydratedToolCallStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HydratedChatTurn {
    pub turn_id: String,
    pub prompt: String,
    pub assistant_text: String,
    pub reasoning_text: String,
    pub status: HydratedChatTurnStatus,
    pub error: Option<String>,
    pub latest_plan: Option<DesktopPlanSnapshot>,
    pub tool_calls: Vec<HydratedToolCall>,
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
