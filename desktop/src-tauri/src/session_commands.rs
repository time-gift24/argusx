use std::sync::Arc;

use argus_core::ToolCall;
use serde::Serialize;
use serde_json::{json, Value};
use session::manager::{SessionEvent, SessionManager, TurnDependencies};
use tauri::{AppHandle, Emitter, State};
use turn::{PermissionDecision, TurnEvent};
use uuid::Uuid;

pub type SharedSessionManager = Arc<SessionManager>;

pub struct DesktopSessionState {
    pub manager: SharedSessionManager,
    pub turn_dependencies: Option<TurnDependencies>,
}

impl DesktopSessionState {
    pub fn new(manager: SessionManager) -> Self {
        Self {
            manager: Arc::new(manager),
            turn_dependencies: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadEventPayload {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub kind: String,
    pub data: Value,
}

pub fn spawn_session_event_bridge(app: AppHandle, manager: SharedSessionManager) {
    tauri::async_runtime::spawn(async move {
        let mut rx = manager.subscribe();

        while let Ok(event) = rx.recv().await {
            let payload = session_event_to_payload(event);
            let _ = app.emit("thread-event", payload);
        }
    });
}

#[tauri::command]
pub async fn create_thread(
    state: State<'_, DesktopSessionState>,
    title: Option<String>,
) -> Result<String, String> {
    state
        .manager
        .create_thread(title)
        .await
        .map(|thread_id| thread_id.to_string())
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_threads(
    state: State<'_, DesktopSessionState>,
) -> Result<Vec<session::ThreadRecord>, String> {
    state
        .manager
        .list_threads()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn switch_thread(
    state: State<'_, DesktopSessionState>,
    thread_id: String,
) -> Result<(), String> {
    let thread_id = parse_uuid(&thread_id)?;
    state
        .manager
        .switch_thread(thread_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, DesktopSessionState>,
    thread_id: String,
    content: String,
) -> Result<(), String> {
    let deps = state
        .turn_dependencies
        .clone()
        .ok_or_else(|| "turn dependencies are not configured in desktop runtime".to_string())?;
    let thread_id = parse_uuid(&thread_id)?;
    state
        .manager
        .send_message(thread_id, content, deps)
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn resolve_thread_permission(
    state: State<'_, DesktopSessionState>,
    thread_id: String,
    request_id: String,
    decision: String,
) -> Result<(), String> {
    let thread_id = parse_uuid(&thread_id)?;
    let decision = match decision.as_str() {
        "allow" => PermissionDecision::Allow,
        "deny" => PermissionDecision::Deny,
        other => return Err(format!("unsupported permission decision: {other}")),
    };

    state
        .manager
        .resolve_permission(thread_id, request_id, decision)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn cancel_thread_turn(
    state: State<'_, DesktopSessionState>,
    thread_id: String,
) -> Result<(), String> {
    let thread_id = parse_uuid(&thread_id)?;
    state
        .manager
        .cancel_turn(thread_id)
        .await
        .map_err(|err| err.to_string())
}

fn parse_uuid(raw: &str) -> Result<Uuid, String> {
    Uuid::parse_str(raw).map_err(|err| format!("invalid uuid `{raw}`: {err}"))
}

fn session_event_to_payload(event: SessionEvent) -> ThreadEventPayload {
    match event {
        SessionEvent::Thread { thread_id, event } => ThreadEventPayload {
            thread_id: thread_id.to_string(),
            turn_id: None,
            kind: match event {
                session::ThreadEvent::ThreadCreated => "thread-created",
                session::ThreadEvent::ThreadActivated => "thread-activated",
                session::ThreadEvent::ThreadUpdated => "thread-updated",
                session::ThreadEvent::ThreadArchived => "thread-archived",
                session::ThreadEvent::TurnEventForwarded => "turn-event-forwarded",
            }
            .into(),
            data: json!({}),
        },
        SessionEvent::Turn {
            thread_id,
            turn_id,
            event,
        } => turn_event_payload(thread_id, turn_id, event),
    }
}

fn turn_event_payload(thread_id: Uuid, turn_id: Uuid, event: TurnEvent) -> ThreadEventPayload {
    let (kind, data) = match event {
        TurnEvent::TurnStarted => ("turn-started", json!({})),
        TurnEvent::LlmTextDelta { text } => ("llm-text-delta", json!({ "text": text.as_ref() })),
        TurnEvent::LlmReasoningDelta { text } => {
            ("llm-reasoning-delta", json!({ "text": text.as_ref() }))
        }
        TurnEvent::ToolCallPrepared { call } => (
            "tool-call-prepared",
            json!({
                "callId": tool_call_id(call.as_ref()),
                "toolName": tool_name(call.as_ref()),
            }),
        ),
        TurnEvent::ToolCallCompleted { call_id, result } => (
            "tool-call-completed",
            json!({
                "callId": call_id.as_ref(),
                "result": format!("{:?}", result),
            }),
        ),
        TurnEvent::ToolCallPermissionRequested { request } => (
            "tool-call-permission-requested",
            json!({
                "requestId": request.request_id,
                "toolCallId": request.tool_call_id,
            }),
        ),
        TurnEvent::ToolCallPermissionResolved {
            request_id,
            decision,
        } => (
            "tool-call-permission-resolved",
            json!({
                "requestId": request_id.as_ref(),
                "decision": match decision {
                    PermissionDecision::Allow => "allow",
                    PermissionDecision::Deny => "deny",
                },
            }),
        ),
        TurnEvent::StepFinished { step_index, reason } => (
            "step-finished",
            json!({ "stepIndex": step_index, "reason": format!("{:?}", reason) }),
        ),
        TurnEvent::TurnFinished { reason } => (
            "turn-finished",
            json!({ "reason": format!("{:?}", reason) }),
        ),
    };

    ThreadEventPayload {
        thread_id: thread_id.to_string(),
        turn_id: Some(turn_id.to_string()),
        kind: kind.into(),
        data,
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
