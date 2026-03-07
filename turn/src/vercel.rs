use argus_core::ToolCall;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{PermissionDecision, ToolOutcome, TurnEvent, TurnFinishReason};

pub fn map_events(events: Vec<TurnEvent>) -> Vec<String> {
    let mut encoder = UiMessageStreamEncoder::default();
    let mut lines = Vec::new();

    for event in &events {
        encoder.push_event(event, &mut lines);
    }

    if encoder.finished {
        lines.push("data: [DONE]\n\n".into());
    }

    lines
}

#[derive(Default)]
struct UiMessageStreamEncoder {
    started: bool,
    step_open: bool,
    finished: bool,
    active_text_id: Option<String>,
    active_reasoning_id: Option<String>,
    next_text_id: u32,
    next_reasoning_id: u32,
}

impl UiMessageStreamEncoder {
    fn push_event(&mut self, event: &TurnEvent, lines: &mut Vec<String>) {
        match event {
            TurnEvent::TurnStarted => self.ensure_step_started(lines),
            TurnEvent::LlmTextDelta { text } => {
                self.ensure_step_started(lines);
                let text_id = self.ensure_text_part(lines);
                lines.push(sse(json!({
                    "type": "text-delta",
                    "id": text_id,
                    "delta": text,
                })));
            }
            TurnEvent::LlmReasoningDelta { text } => {
                self.ensure_step_started(lines);
                let reasoning_id = self.ensure_reasoning_part(lines);
                lines.push(sse(json!({
                    "type": "reasoning-delta",
                    "id": reasoning_id,
                    "delta": text,
                })));
            }
            TurnEvent::ToolCallPrepared { call } => {
                self.ensure_step_started(lines);
                lines.push(sse(json!({
                    "type": "tool-input-available",
                    "toolCallId": tool_call_id(call),
                    "toolName": tool_name(call),
                    "input": tool_input(call),
                })));
            }
            TurnEvent::ToolCallCompleted { call_id, result } => {
                self.ensure_step_started(lines);
                lines.push(sse(tool_output_chunk(call_id, result)));
            }
            TurnEvent::ToolCallPermissionRequested { request } => {
                self.ensure_step_started(lines);
                lines.push(sse(json!({
                    "type": "tool-approval-request",
                    "approvalId": request.request_id,
                    "toolCallId": request.tool_call_id,
                })));
            }
            TurnEvent::ToolCallPermissionResolved {
                request_id,
                decision,
            } => {
                self.ensure_step_started(lines);
                lines.push(sse(json!({
                    "type": "data-turn-control",
                    "data": {
                        "kind": "permission-resolved",
                        "requestId": request_id,
                        "decision": match decision {
                            PermissionDecision::Allow => "allow",
                            PermissionDecision::Deny => "deny",
                        },
                    },
                    "transient": true,
                })));
            }
            TurnEvent::StepFinished { .. } => {
                self.close_active_parts(lines);
                if self.step_open {
                    lines.push(sse(json!({ "type": "finish-step" })));
                    self.step_open = false;
                }
            }
            TurnEvent::TurnFinished { reason } => {
                self.ensure_started(lines);
                self.close_active_parts(lines);
                if self.step_open {
                    lines.push(sse(json!({ "type": "finish-step" })));
                    self.step_open = false;
                }
                if matches!(reason, TurnFinishReason::Cancelled) {
                    lines.push(sse(json!({
                        "type": "data-turn-control",
                        "data": {
                            "kind": "turn-finished",
                            "status": "cancelled",
                        },
                        "transient": true,
                    })));
                }
                lines.push(sse(json!({
                    "type": "finish",
                    "finishReason": match reason {
                        TurnFinishReason::Completed => "stop",
                        TurnFinishReason::Cancelled => "other",
                        TurnFinishReason::Failed => "error",
                        TurnFinishReason::MaxStepsExceeded => "error",
                        TurnFinishReason::ModelLengthLimit => "error",
                        TurnFinishReason::ModelProtocolError => "error",
                        TurnFinishReason::LlmTimeout => "error",
                    },
                })));
                self.finished = true;
            }
        }
    }

    fn ensure_started(&mut self, lines: &mut Vec<String>) {
        if !self.started {
            lines.push(sse(json!({ "type": "start" })));
            self.started = true;
        }
    }

    fn ensure_step_started(&mut self, lines: &mut Vec<String>) {
        self.ensure_started(lines);
        if !self.step_open {
            lines.push(sse(json!({ "type": "start-step" })));
            self.step_open = true;
        }
    }

    fn ensure_text_part(&mut self, lines: &mut Vec<String>) -> String {
        if let Some(text_id) = &self.active_text_id {
            return text_id.clone();
        }

        self.next_text_id += 1;
        let text_id = format!("text-{}", self.next_text_id);
        lines.push(sse(json!({
            "type": "text-start",
            "id": text_id,
        })));
        self.active_text_id = Some(text_id.clone());
        text_id
    }

    fn ensure_reasoning_part(&mut self, lines: &mut Vec<String>) -> String {
        if let Some(reasoning_id) = &self.active_reasoning_id {
            return reasoning_id.clone();
        }

        self.next_reasoning_id += 1;
        let reasoning_id = format!("reasoning-{}", self.next_reasoning_id);
        lines.push(sse(json!({
            "type": "reasoning-start",
            "id": reasoning_id,
        })));
        self.active_reasoning_id = Some(reasoning_id.clone());
        reasoning_id
    }

    fn close_active_parts(&mut self, lines: &mut Vec<String>) {
        if let Some(text_id) = self.active_text_id.take() {
            lines.push(sse(json!({
                "type": "text-end",
                "id": text_id,
            })));
        }

        if let Some(reasoning_id) = self.active_reasoning_id.take() {
            lines.push(sse(json!({
                "type": "reasoning-end",
                "id": reasoning_id,
            })));
        }
    }
}

fn tool_call_id(call: &ToolCall) -> String {
    match call {
        ToolCall::FunctionCall { call_id, .. } => call_id.clone(),
        ToolCall::Builtin(call) => call.call_id.clone(),
        ToolCall::Mcp(call) => call.id.clone(),
    }
}

fn tool_name(call: &ToolCall) -> String {
    match call {
        ToolCall::FunctionCall { name, .. } => name.clone(),
        ToolCall::Builtin(call) => call.builtin.canonical_name().to_string(),
        ToolCall::Mcp(call) => call.name.clone().unwrap_or_default(),
    }
}

fn tool_input(call: &ToolCall) -> Value {
    match call {
        ToolCall::FunctionCall { arguments_json, .. } => parse_args(arguments_json),
        ToolCall::Builtin(call) => parse_args(&call.arguments_json),
        ToolCall::Mcp(call) => call
            .arguments_json
            .as_deref()
            .map(parse_args)
            .unwrap_or_else(|| json!({})),
    }
}

fn tool_output_chunk(call_id: &str, result: &ToolOutcome) -> Value {
    match result {
        ToolOutcome::Success(output) => json!({
            "type": "tool-output-available",
            "toolCallId": call_id,
            "output": output,
        }),
        ToolOutcome::Denied => json!({
            "type": "tool-output-denied",
            "toolCallId": call_id,
        }),
        ToolOutcome::Failed { message, .. } => json!({
            "type": "tool-output-error",
            "toolCallId": call_id,
            "errorText": message,
        }),
        ToolOutcome::TimedOut => json!({
            "type": "tool-output-error",
            "toolCallId": call_id,
            "errorText": "timed_out",
        }),
        ToolOutcome::Cancelled => json!({
            "type": "tool-output-error",
            "toolCallId": call_id,
            "errorText": "cancelled",
        }),
    }
}

fn parse_args(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({ "raw": raw }))
}

fn sse(payload: impl Serialize) -> String {
    format!(
        "data: {}\n\n",
        serde_json::to_string(&payload).expect("ui message stream payload should serialize")
    )
}
