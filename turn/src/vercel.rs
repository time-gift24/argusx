use argus_core::ToolCall;
use serde_json::{Value, json};

use crate::{
    PermissionDecision, StepFinishReason, ToolOutcome, TurnEvent, TurnFinishReason,
};

pub fn map_events(events: Vec<TurnEvent>) -> Vec<String> {
    events.iter().filter_map(map_event).collect()
}

fn map_event(event: &TurnEvent) -> Option<String> {
    match event {
        TurnEvent::TurnStarted => None,
        TurnEvent::LlmTextDelta { text } => Some(format!("0:{}\n", to_json(text))),
        TurnEvent::LlmReasoningDelta { text } => Some(format!("4:{}\n", to_json(text))),
        TurnEvent::ToolCallPrepared { call } => Some(format!("7:{}\n", to_json(&vec![tool_call_payload(call)]))),
        TurnEvent::ToolCallCompleted { call_id, result } => Some(format!(
            "8:{}\n",
            to_json(&vec![json!({
                "toolCallId": call_id,
                "result": tool_result_payload(result),
            })])
        )),
        TurnEvent::ToolCallPermissionRequested { request } => Some(format!(
            "1:{}\n",
            to_json(&vec![json!({
                "type": "permission_required",
                "request_id": request.request_id,
                "tool_call_id": request.tool_call_id,
            })])
        )),
        TurnEvent::ToolCallPermissionResolved { request_id, decision } => Some(format!(
            "1:{}\n",
            to_json(&vec![json!({
                "type": "permission_resolved",
                "request_id": request_id,
                "decision": match decision {
                    PermissionDecision::Allow => "allow",
                    PermissionDecision::Deny => "deny",
                },
            })])
        )),
        TurnEvent::StepFinished { step_index, reason } => Some(format!(
            "e:{}\n",
            to_json(&json!({
                "stepIndex": step_index,
                "finishReason": match reason {
                    StepFinishReason::ToolCalls => "tool-calls",
                },
            }))
        )),
        TurnEvent::TurnFinished { reason } => Some(format!(
            "d:{}\n",
            to_json(&json!({
                "finishReason": match reason {
                    TurnFinishReason::Completed => "stop",
                    TurnFinishReason::Cancelled => "cancelled",
                    TurnFinishReason::Failed => "error",
                },
            }))
        )),
    }
}

fn tool_call_payload(call: &ToolCall) -> Value {
    match call {
        ToolCall::FunctionCall {
            call_id,
            name,
            arguments_json,
            ..
        } => json!({
            "toolCallId": call_id,
            "toolName": name,
            "args": parse_args(arguments_json),
        }),
        ToolCall::Builtin(call) => json!({
            "toolCallId": call.call_id,
            "toolName": call.builtin.canonical_name(),
            "args": parse_args(&call.arguments_json),
        }),
        ToolCall::Mcp(call) => json!({
            "toolCallId": call.id,
            "toolName": call.name.clone().unwrap_or_default(),
            "args": call
                .arguments_json
                .as_deref()
                .map(parse_args)
                .unwrap_or_else(|| json!({})),
        }),
    }
}

fn tool_result_payload(result: &ToolOutcome) -> Value {
    match result {
        ToolOutcome::Success(value) => value.clone(),
        ToolOutcome::Failed { message, retryable } => {
            json!({ "error": message, "retryable": retryable })
        }
        ToolOutcome::TimedOut => json!({ "error": "timed_out" }),
        ToolOutcome::Denied => json!({ "error": "denied" }),
        ToolOutcome::Cancelled => json!({ "error": "cancelled" }),
    }
}

fn parse_args(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({ "raw": raw }))
}

fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("vercel event payload should serialize")
}
