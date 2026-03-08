use argus_core::ToolCall;
use async_trait::async_trait;
use serde_json::json;
use turn::TurnObserver;
use turn::{PermissionDecision, ToolOutcome, TurnEvent, TurnFinishReason};

use super::events::DesktopTurnEvent;

pub trait ChatEventSink: Send + Sync {
    fn emit(&self, event: &DesktopTurnEvent) -> Result<(), turn::TurnError>;
}

pub struct DesktopTurnObserver {
    conversation_id: String,
    turn_id: String,
    sink: std::sync::Arc<dyn ChatEventSink>,
}

impl DesktopTurnObserver {
    pub fn new(
        conversation_id: impl Into<String>,
        turn_id: impl Into<String>,
        sink: std::sync::Arc<dyn ChatEventSink>,
    ) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            turn_id: turn_id.into(),
            sink,
        }
    }
}

#[async_trait]
impl TurnObserver for DesktopTurnObserver {
    async fn on_event(&self, event: &TurnEvent) -> Result<(), turn::TurnError> {
        if let Some(event) = map_turn_event(&self.conversation_id, &self.turn_id, event) {
            self.sink.emit(&event)?;
        }

        Ok(())
    }
}

pub fn map_turn_event(
    conversation_id: &str,
    turn_id: &str,
    event: &TurnEvent,
) -> Option<DesktopTurnEvent> {
    let event = match event {
        TurnEvent::TurnStarted => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "turn-started",
            json!({}),
        ),
        TurnEvent::LlmTextDelta { text } => {
            DesktopTurnEvent::text_delta(conversation_id, turn_id, text.as_ref())
        }
        TurnEvent::LlmReasoningDelta { text } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "llm-reasoning-delta",
            json!({ "text": text.as_ref() }),
        ),
        TurnEvent::ToolCallPrepared { call } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "tool-call-prepared",
            json!({
                "callId": tool_call_id(call.as_ref()),
                "arguments": tool_arguments(call.as_ref()),
                "name": tool_name(call.as_ref()),
            }),
        ),
        TurnEvent::ToolCallCompleted { call_id, result } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "tool-call-completed",
            json!({
                "callId": call_id.as_ref(),
                "error": tool_error(result),
                "output": tool_output(result),
                "status": tool_status(result),
            }),
        ),
        TurnEvent::ToolCallPermissionRequested { request } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "tool-approval-requested",
            json!({
                "requestId": request.request_id,
                "toolCallId": request.tool_call_id,
            }),
        ),
        TurnEvent::ToolCallPermissionResolved {
            request_id,
            decision,
        } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "tool-approval-resolved",
            json!({
                "requestId": request_id.as_ref(),
                "decision": match decision {
                    PermissionDecision::Allow => "allow",
                    PermissionDecision::Deny => "deny",
                },
            }),
        ),
        TurnEvent::StepFinished { step_index, reason } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "step-finished",
            json!({
                "stepIndex": step_index,
                "reason": format!("{reason:?}"),
            }),
        ),
        TurnEvent::TurnFinished { reason } => DesktopTurnEvent::new(
            conversation_id,
            turn_id,
            "turn-finished",
            json!({
                "reason": turn_finish_reason(reason),
            }),
        ),
    };

    Some(event)
}

fn tool_status(result: &ToolOutcome) -> &'static str {
    match result {
        ToolOutcome::Success(_) => "success",
        ToolOutcome::Failed { .. } => "error",
        ToolOutcome::TimedOut => "timed_out",
        ToolOutcome::Denied => "denied",
        ToolOutcome::Cancelled => "cancelled",
    }
}

fn tool_output(result: &ToolOutcome) -> Option<serde_json::Value> {
    match result {
        ToolOutcome::Success(output) => Some(output.clone()),
        _ => None,
    }
}

fn tool_error(result: &ToolOutcome) -> Option<serde_json::Value> {
    match result {
        ToolOutcome::Failed { message, retryable } => Some(json!({
            "message": message,
            "retryable": retryable,
        })),
        ToolOutcome::TimedOut => Some(json!({
            "message": "tool execution timed out",
        })),
        ToolOutcome::Denied => Some(json!({
            "message": "tool execution was denied",
        })),
        ToolOutcome::Cancelled => Some(json!({
            "message": "tool execution was cancelled",
        })),
        ToolOutcome::Success(_) => None,
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

fn tool_arguments(call: &ToolCall) -> Option<&str> {
    match call {
        ToolCall::FunctionCall { arguments_json, .. } => Some(arguments_json),
        ToolCall::Builtin(call) => Some(&call.arguments_json),
        ToolCall::Mcp(call) => call.arguments_json.as_deref(),
    }
}
