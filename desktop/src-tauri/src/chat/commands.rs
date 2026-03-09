use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::{
    chat::{
        submission::PermissionDecision,
        ChatController, HydratedChatTurn, StartTurnInput, StartTurnResult, TurnTargetKind,
    },
    session_commands::DesktopSessionState,
};

/// Build a ChatController from DesktopSessionState.
fn build_controller(state: &DesktopSessionState) -> ChatController<'_> {
    ChatController::new(
        Arc::clone(&state.manager),
        state.turn_manager(),
        state,
    )
}

#[tauri::command]
pub async fn load_active_chat_thread(
    state: State<'_, DesktopSessionState>,
) -> Result<Vec<HydratedChatTurn>, String> {
    let controller = build_controller(&state);
    let result = controller
        .load_active_thread()
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.turns)
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

    let controller = build_controller(&state);
    let prompt_text = input.prompt.clone();
    let prompt_input = crate::chat::submission::PromptInput {
        text: input.prompt,
        target_kind: input.target_kind,
        target_id: input.target_id,
    };

    let result = controller
        .start_prompt_turn_with_app(prompt_text, app, Some(prompt_input))
        .await
        .map_err(|e| e.to_string())?;

    Ok(StartTurnResult { turn_id: result.turn_id })
}

#[tauri::command]
pub async fn cancel_turn(
    state: State<'_, DesktopSessionState>,
    turn_id: String,
) -> Result<(), String> {
    let controller = build_controller(&state);
    controller
        .cancel_turn(turn_id)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolveTurnPermissionDecision {
    Allow,
    Deny,
}

impl ResolveTurnPermissionDecision {
    fn into_permission_decision(self) -> PermissionDecision {
        match self {
            Self::Allow => PermissionDecision::Allow,
            Self::Deny => PermissionDecision::Deny,
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
    let controller = build_controller(&state);
    controller
        .resolve_permission(turn_id, request_id, decision.into_permission_decision())
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_turn_permission_decision_maps_deny() {
        assert!(matches!(
            ResolveTurnPermissionDecision::Deny.into_permission_decision(),
            PermissionDecision::Deny
        ));
    }

    #[test]
    fn resolve_turn_permission_decision_maps_allow() {
        assert!(matches!(
            ResolveTurnPermissionDecision::Allow.into_permission_decision(),
            PermissionDecision::Allow
        ));
    }
}
