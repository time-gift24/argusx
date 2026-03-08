use std::sync::Arc;

use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::{
    chat::{
        HydratedChatTurn, StartTurnInput, StartTurnResult, TauriTurnObserver, TurnTargetKind,
    },
    session_commands::{DesktopSessionState, hydrate_chat_turn},
};

#[tauri::command]
pub async fn load_active_chat_thread(
    state: State<'_, DesktopSessionState>,
) -> Result<Vec<HydratedChatTurn>, String> {
    let thread_id = state.ensure_active_chat_thread().await.map_err(stringify)?;
    let history = state
        .manager
        .load_thread_history(thread_id)
        .await
        .map_err(stringify)?;

    Ok(history.iter().map(hydrate_chat_turn).collect())
}

#[tauri::command]
pub async fn start_turn(
    app: tauri::AppHandle,
    state: State<'_, DesktopSessionState>,
    input: StartTurnInput,
) -> Result<StartTurnResult, String> {
    if !matches!(input.target_kind, TurnTargetKind::Agent) {
        return Err("workflow turns are not implemented yet".to_string());
    }

    let thread_id = state.ensure_active_chat_thread().await.map_err(stringify)?;
    let turn_id = Uuid::new_v4();
    let observer: Arc<dyn turn::TurnObserver> = Arc::new(TauriTurnObserver::new(
        app,
        turn_id.to_string(),
        input.target_kind,
        input.target_id,
    ));
    let deps = state.build_turn_dependencies(observer).map_err(stringify)?;

    state
        .manager
        .send_message_with_turn_id(thread_id, turn_id, input.prompt, deps)
        .await
        .map_err(stringify)?;
    state.turn_manager().insert(turn_id.to_string(), thread_id).await;

    Ok(StartTurnResult {
        turn_id: turn_id.to_string(),
    })
}

#[tauri::command]
pub async fn cancel_turn(
    state: State<'_, DesktopSessionState>,
    turn_id: String,
) -> Result<(), String> {
    let thread_id = state
        .turn_manager()
        .get(&turn_id)
        .await
        .ok_or_else(|| format!("turn `{turn_id}` not found"))?;

    state.manager.cancel_turn(thread_id).await.map_err(stringify)
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolveTurnPermissionDecision {
    Allow,
    Deny,
}

impl ResolveTurnPermissionDecision {
    fn into_turn_decision(self) -> turn::PermissionDecision {
        match self {
            Self::Allow => turn::PermissionDecision::Allow,
            Self::Deny => turn::PermissionDecision::Deny,
        }
    }
}

#[tauri::command]
pub async fn resolve_turn_permission(
    state: State<'_, DesktopSessionState>,
    turn_id: String,
    request_id: String,
    decision: ResolveTurnPermissionDecision,
) -> Result<(), String> {
    let thread_id = state
        .turn_manager()
        .get(&turn_id)
        .await
        .ok_or_else(|| format!("turn `{turn_id}` not found"))?;

    state
        .manager
        .resolve_permission(thread_id, request_id, decision.into_turn_decision())
        .await
        .map_err(stringify)
}

fn stringify(err: impl std::fmt::Display) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_turn_permission_decision_maps_deny() {
        assert!(matches!(
            ResolveTurnPermissionDecision::Deny.into_turn_decision(),
            turn::PermissionDecision::Deny
        ));
    }
}
