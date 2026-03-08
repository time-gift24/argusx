use std::sync::Arc;

use tauri::Emitter;
use tokio::task::JoinHandle;
use turn::{
    ModelRunner, ToolAuthorizer, ToolRunner, TurnContext, TurnDriver, TurnError, TurnHandle,
};
use uuid::Uuid;

use crate::chat::{
    AppState, StartTurnInput, StartTurnResult, TauriTurnObserver, TurnTargetKind,
    observer::turn_failed_event,
};

#[tauri::command]
pub async fn start_turn(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input: StartTurnInput,
) -> Result<StartTurnResult, String> {
    if !matches!(input.target_kind, TurnTargetKind::Agent) {
        return Err("workflow turns are not implemented yet".to_string());
    }

    let turn_id = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();
    let model_runner: Arc<dyn ModelRunner> = state.model_runner().map_err(stringify)?;
    let tool_runner: Arc<dyn ToolRunner> = state.tool_runner();
    let tool_authorizer: Arc<dyn ToolAuthorizer> = state.tool_authorizer();
    let turn_manager = state.turn_manager();
    let observer = Arc::new(TauriTurnObserver::new(
        app.clone(),
        turn_id.clone(),
        input.target_kind,
        input.target_id.clone(),
    ));
    let observer_for_driver: Arc<dyn turn::TurnObserver> = observer.clone();

    let (handle, task) = TurnDriver::spawn(
        TurnContext {
            session_id,
            turn_id: turn_id.clone(),
            user_message: input.prompt,
        },
        model_runner,
        tool_runner,
        tool_authorizer,
        observer_for_driver,
    );

    turn_manager.insert(turn_id.clone(), handle.controller()).await;

    tauri::async_runtime::spawn(drive_turn(
        app,
        turn_id.clone(),
        turn_manager,
        observer,
        handle,
        task,
    ));

    Ok(StartTurnResult { turn_id })
}

#[tauri::command]
pub async fn cancel_turn(
    state: tauri::State<'_, AppState>,
    turn_id: String,
) -> Result<(), String> {
    let controller = state
        .turn_manager()
        .take(&turn_id)
        .await
        .ok_or_else(|| format!("turn `{turn_id}` not found"))?;

    controller.cancel().await.map_err(stringify)
}

async fn drive_turn(
    app: tauri::AppHandle,
    turn_id: String,
    turn_manager: Arc<crate::chat::TurnManager>,
    observer: Arc<TauriTurnObserver>,
    handle: TurnHandle,
    task: JoinHandle<Result<(), TurnError>>,
) {
    while handle.next_event().await.is_some() {}

    let result = match task.await {
        Ok(inner) => inner,
        Err(err) => Err(TurnError::Runtime(err.to_string())),
    };

    let _ = turn_manager.take(&turn_id).await;

    if let Err(err) = result {
        if !observer.saw_failed_finish() {
            let _ = app.emit("turn-event", turn_failed_event(&turn_id, &err.to_string()));
        }
    }
}

fn stringify(err: impl std::fmt::Display) -> String {
    err.to_string()
}
