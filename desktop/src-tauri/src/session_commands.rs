use std::sync::Arc;

use argus_core::ToolCall;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use session::manager::{SessionEvent, SessionManager, TurnDependencies};
use tauri::{AppHandle, Emitter, State};
use turn::{PermissionDecision, TurnError, TurnEvent, TurnObserver};
use uuid::Uuid;

use crate::{
    chat::{
        AllowListedToolAuthorizer, HydratedChatTurn, HydratedChatTurnStatus, HydratedToolCall,
        HydratedToolCallStatus, ProviderModelRunner, ScheduledToolRunner,
        plan::snapshot_from_output,
    },
    provider_settings::ProviderSettingsService,
};

pub type SharedSessionManager = Arc<SessionManager>;

pub struct DesktopSessionState {
    pub manager: SharedSessionManager,
    provider_settings: Arc<ProviderSettingsService>,
    tool_runner: Arc<ScheduledToolRunner>,
    tool_authorizer: Arc<AllowListedToolAuthorizer>,
    turn_manager: Arc<crate::chat::TurnManager>,
}

impl DesktopSessionState {
    pub fn new(manager: SessionManager) -> Result<Self, TurnError> {
        let provider_settings = ProviderSettingsService::from_default_location()
            .map_err(|err| TurnError::Runtime(err.to_string()))?;
        Ok(Self {
            manager: Arc::new(manager),
            provider_settings: Arc::new(provider_settings),
            tool_runner: Arc::new(ScheduledToolRunner::from_current_dir()?),
            tool_authorizer: Arc::new(AllowListedToolAuthorizer),
            turn_manager: Arc::new(crate::chat::TurnManager::new()),
        })
    }

    pub fn provider_settings(&self) -> Arc<ProviderSettingsService> {
        Arc::clone(&self.provider_settings)
    }

    pub fn turn_manager(&self) -> Arc<crate::chat::TurnManager> {
        Arc::clone(&self.turn_manager)
    }

    pub fn build_turn_dependencies(
        &self,
        observer: Arc<dyn TurnObserver>,
    ) -> Result<TurnDependencies, TurnError> {
        let model: Arc<dyn turn::ModelRunner> = Arc::new(ProviderModelRunner::from_provider_settings(
            Some(self.provider_settings.as_ref()),
        )?);
        let tool_runner: Arc<dyn turn::ToolRunner> = self.tool_runner.clone();
        let authorizer: Arc<dyn turn::ToolAuthorizer> = self.tool_authorizer.clone();

        Ok(TurnDependencies {
            model,
            tool_runner,
            authorizer,
            observer,
        })
    }

    pub async fn ensure_active_chat_thread(&self) -> Result<Uuid, TurnError> {
        if let Some(thread_id) = self.manager.active_thread_id() {
            return Ok(thread_id);
        }

        let threads = self
            .manager
            .list_threads()
            .await
            .map_err(|err| TurnError::Runtime(err.to_string()))?;
        if let Some(thread_id) = choose_active_chat_thread(None, &threads) {
            self.manager
                .switch_thread(thread_id)
                .await
                .map_err(|err| TurnError::Runtime(err.to_string()))?;
            return Ok(thread_id);
        }

        self.manager
            .create_thread(None)
            .await
            .map_err(|err| TurnError::Runtime(err.to_string()))
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

        while let Some(event) = recv_session_event(&mut rx).await {
            let payload = session_event_to_payload(event);
            let _ = app.emit("thread-event", payload);
        }
    });
}

async fn recv_session_event(
    rx: &mut tokio::sync::broadcast::Receiver<SessionEvent>,
) -> Option<SessionEvent> {
    loop {
        match rx.recv().await {
            Ok(event) => return Some(event),
            // UI bridge 落后时只跳过旧事件，不能把整条事件桥直接停掉。
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "session event bridge lagged behind producer");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
        }
    }
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
        .build_turn_dependencies(Arc::new(NoopTurnObserver))
        .map_err(|err| err.to_string())?;
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

pub(crate) fn choose_active_chat_thread(
    active_thread_id: Option<Uuid>,
    threads: &[session::ThreadRecord],
) -> Option<Uuid> {
    active_thread_id.or_else(|| {
        threads
            .iter()
            .find(|thread| matches!(thread.lifecycle, session::ThreadLifecycle::Open))
            .map(|thread| thread.id)
    })
}

pub(crate) fn hydrate_chat_turn(turn: &session::TurnRecord) -> HydratedChatTurn {
    let mut assistant_text = String::new();
    let mut tool_calls: Vec<HydratedToolCall> = Vec::new();
    let mut latest_plan = None;

    for message in &turn.transcript {
        match message {
            session::PersistedMessage::AssistantText { content } => assistant_text.push_str(content),
            session::PersistedMessage::AssistantToolCalls { content, calls } => {
                if let Some(content) = content {
                    assistant_text.push_str(content);
                }

                for call in calls {
                    tool_calls.push(HydratedToolCall {
                        call_id: call.call_id.clone(),
                        name: call.tool_name.clone(),
                        arguments_json: call.arguments.clone(),
                        output_summary: None,
                        error_summary: None,
                        status: HydratedToolCallStatus::Running,
                    });
                }
            }
            session::PersistedMessage::ToolResult {
                call_id,
                tool_name,
                content,
                is_error,
            } => {
                let existing_index = tool_calls
                    .iter()
                    .position(|tool_call| tool_call.call_id == *call_id)
                    .unwrap_or_else(|| {
                        tool_calls.push(HydratedToolCall {
                            call_id: call_id.clone(),
                            name: tool_name.clone(),
                            arguments_json: "{}".into(),
                            output_summary: None,
                            error_summary: None,
                            status: HydratedToolCallStatus::Running,
                        });
                        tool_calls.len() - 1
                    });

                let tool_call = &mut tool_calls[existing_index];
                tool_call.status = if *is_error {
                    HydratedToolCallStatus::Failed
                } else {
                    HydratedToolCallStatus::Success
                };
                if *is_error {
                    tool_call.error_summary = Some(content.clone());
                } else {
                    tool_call.output_summary = Some(content.clone());
                }

                if !*is_error && tool_name == "update_plan" {
                    if let Ok(output) = serde_json::from_str::<Value>(content) {
                        latest_plan = snapshot_from_output(call_id, &output);
                    }
                }
            }
            session::PersistedMessage::User { .. } | session::PersistedMessage::SystemNote { .. } => {}
        }
    }

    if let Some(final_output) = &turn.final_output {
        assistant_text = final_output.clone();
    }

    let (status, error) = hydrate_turn_status(turn);

    HydratedChatTurn {
        turn_id: turn.id.to_string(),
        prompt: turn.user_input.clone(),
        assistant_text,
        reasoning_text: String::new(),
        status,
        error,
        latest_plan,
        tool_calls,
    }
}

fn hydrate_turn_status(turn: &session::TurnRecord) -> (HydratedChatTurnStatus, Option<String>) {
    match turn.status {
        session::TurnStatus::Completed => (HydratedChatTurnStatus::Completed, None),
        session::TurnStatus::Cancelled => (HydratedChatTurnStatus::Cancelled, None),
        session::TurnStatus::Failed => (
            HydratedChatTurnStatus::Failed,
            Some(hydrated_turn_error(turn, "Turn failed.")),
        ),
        session::TurnStatus::Interrupted
        | session::TurnStatus::Running
        | session::TurnStatus::WaitingPermission => (
            HydratedChatTurnStatus::Failed,
            Some("Turn interrupted.".into()),
        ),
    }
}

fn hydrated_turn_error(turn: &session::TurnRecord, fallback: &str) -> String {
    turn.finish_reason
        .as_ref()
        .map(|reason| format!("Turn ended with {}.", reason.to_ascii_lowercase()))
        .unwrap_or_else(|| fallback.to_string())
}

struct NoopTurnObserver;

#[async_trait]
impl TurnObserver for NoopTurnObserver {
    async fn on_event(&self, _event: &TurnEvent) -> Result<(), TurnError> {
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use session::{PersistedMessage, PersistedToolCall, PersistedToolKind, ThreadLifecycle, ThreadRecord, TurnRecord, TurnStatus};

    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn recv_session_event_recovers_after_lagged_error() {
        let thread_id = Uuid::new_v4();
        let (tx, mut rx) = broadcast::channel(2);

        tx.send(SessionEvent::Thread {
            thread_id,
            event: session::ThreadEvent::ThreadCreated,
        })
        .unwrap();
        tx.send(SessionEvent::Thread {
            thread_id,
            event: session::ThreadEvent::ThreadActivated,
        })
        .unwrap();
        tx.send(SessionEvent::Thread {
            thread_id,
            event: session::ThreadEvent::ThreadUpdated,
        })
        .unwrap();

        let event = recv_session_event(&mut rx).await;
        assert!(matches!(
            event,
            Some(SessionEvent::Thread {
                thread_id: observed_thread_id,
                event: session::ThreadEvent::ThreadActivated,
            }) if observed_thread_id == thread_id
        ));
    }

    #[test]
    fn choose_active_chat_thread_prefers_runtime_active_thread() {
        let active_thread_id = Uuid::new_v4();
        let other_thread_id = Uuid::new_v4();
        let now = Utc::now();

        let threads = vec![ThreadRecord {
            id: other_thread_id,
            session_id: "default-session".into(),
            agent_profile_id: None,
            is_subagent: false,
            title: Some("Other".into()),
            lifecycle: ThreadLifecycle::Open,
            created_at: now,
            updated_at: now,
            last_turn_number: 1,
        }];

        let selected = choose_active_chat_thread(Some(active_thread_id), &threads);

        assert_eq!(selected, Some(active_thread_id));
    }

    #[test]
    fn choose_active_chat_thread_falls_back_to_latest_open_thread() {
        let latest_open_id = Uuid::new_v4();
        let now = Utc::now();

        let threads = vec![
            ThreadRecord {
                id: latest_open_id,
                session_id: "default-session".into(),
                agent_profile_id: None,
                is_subagent: false,
                title: Some("Latest".into()),
                lifecycle: ThreadLifecycle::Open,
                created_at: now,
                updated_at: now,
                last_turn_number: 2,
            },
            ThreadRecord {
                id: Uuid::new_v4(),
                session_id: "default-session".into(),
                agent_profile_id: None,
                is_subagent: false,
                title: Some("Archived".into()),
                lifecycle: ThreadLifecycle::Archived,
                created_at: now,
                updated_at: now,
                last_turn_number: 7,
            },
        ];

        let selected = choose_active_chat_thread(None, &threads);

        assert_eq!(selected, Some(latest_open_id));
    }

    #[test]
    fn hydrate_chat_turn_restores_tool_calls_and_plan_snapshot() {
        let thread_id = Uuid::new_v4();
        let started_at = Utc::now();
        let turn = TurnRecord {
            id: Uuid::new_v4(),
            thread_id,
            turn_number: 1,
            user_input: "Review this plan".into(),
            status: TurnStatus::Completed,
            finish_reason: Some("Completed".into()),
            transcript: vec![
                PersistedMessage::User {
                    content: "Review this plan".into(),
                },
                PersistedMessage::AssistantToolCalls {
                    content: Some("Let me inspect the repo.".into()),
                    calls: vec![PersistedToolCall {
                        sequence: 0,
                        call_id: "call-update-plan".into(),
                        tool_name: "update_plan".into(),
                        arguments: r#"{"plan":[{"step":"Write failing test","status":"completed"}]}"#.into(),
                        kind: PersistedToolKind::Builtin,
                        server_label: None,
                    }],
                },
                PersistedMessage::ToolResult {
                    call_id: "call-update-plan".into(),
                    tool_name: "update_plan".into(),
                    content: serde_json::json!({
                        "plan": {
                            "title": "Execution Plan",
                            "description": "Already in progress",
                            "is_streaming": false,
                            "tasks": [
                                {
                                    "id": "task-1",
                                    "status": "completed",
                                    "title": "Write failing test"
                                }
                            ]
                        }
                    })
                    .to_string(),
                    is_error: false,
                },
                PersistedMessage::AssistantText {
                    content: "Implemented the bootstrap shim.".into(),
                },
            ],
            final_output: Some("Implemented the bootstrap shim.".into()),
            started_at,
            finished_at: Some(started_at),
        };

        let hydrated = hydrate_chat_turn(&turn);

        assert_eq!(hydrated.turn_id, turn.id.to_string());
        assert_eq!(hydrated.prompt, "Review this plan");
        assert_eq!(hydrated.assistant_text, "Implemented the bootstrap shim.");
        assert_eq!(hydrated.status, HydratedChatTurnStatus::Completed);
        assert_eq!(hydrated.tool_calls.len(), 1);
        assert_eq!(hydrated.tool_calls[0].call_id, "call-update-plan");
        assert_eq!(hydrated.tool_calls[0].status, HydratedToolCallStatus::Success);
        assert_eq!(
            hydrated.latest_plan.as_ref().map(|plan| plan.title.as_str()),
            Some("Execution Plan")
        );
    }
}
