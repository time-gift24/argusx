use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use agent_core::{
    new_id, AgentError, InputEnvelope, InputPart, InputSource, LanguageModel, ModelEventStream,
    ModelOutputEvent, ModelRequest, RunEventStream, Runtime, SessionMeta, ToolCall, TurnRequest,
    UiEventStream, Usage,
};
use agent_session::{SessionConfig, SessionRuntime};
use agent_tool::AgentToolRuntime;
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, RwLock};

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub color: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartAgentTurnPayload {
    session_id: String,
    input: String,
    model: Option<String>,
    attachments: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartAgentTurnResponse {
    turn_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelAgentTurnPayload {
    turn_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentStreamEnvelope {
    session_id: String,
    turn_id: String,
    source: String,
    seq: u64,
    ts: i64,
    event: serde_json::Value,
}

#[derive(Clone)]
struct DesktopModel;

#[async_trait]
impl LanguageModel for DesktopModel {
    fn model_name(&self) -> &str {
        "desktop-agent-model"
    }

    async fn stream(&self, request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        let user_text = latest_user_text(&request.inputs);
        let tool_output = latest_tool_output(&request.inputs);

        let response_stream = stream! {
            yield Ok(ModelOutputEvent::ReasoningDelta {
                delta: "Analyzing request...\n".to_string(),
            });
            tokio::time::sleep(Duration::from_millis(80)).await;

            if let Some(tool_output) = tool_output {
                yield Ok(ModelOutputEvent::ReasoningDelta {
                    delta: "Synthesizing tool output...\n".to_string(),
                });
                tokio::time::sleep(Duration::from_millis(80)).await;
                yield Ok(ModelOutputEvent::TextDelta {
                    delta: summarize_tool_output(&tool_output),
                });
            } else if let Some(command) = extract_tool_command(user_text.as_deref()) {
                yield Ok(ModelOutputEvent::ReasoningDelta {
                    delta: format!("Planning shell execution: `{command}`\n"),
                });
                tokio::time::sleep(Duration::from_millis(80)).await;
                yield Ok(ModelOutputEvent::ToolCall {
                    call: ToolCall::new("shell", json!({ "command": command })),
                });
            } else {
                let message = user_text.unwrap_or_else(|| "Hello".to_string());
                yield Ok(ModelOutputEvent::ReasoningDelta {
                    delta: "No tool required, generating answer directly.\n".to_string(),
                });
                tokio::time::sleep(Duration::from_millis(80)).await;
                yield Ok(ModelOutputEvent::TextDelta {
                    delta: format!("Received: {message}"),
                });
            }

            let input_len = request
                .inputs
                .iter()
                .flat_map(|input| input.parts.iter())
                .map(|part| match part {
                    InputPart::Text { text } => text.len() as u64,
                    InputPart::Json { value } => value.to_string().len() as u64,
                })
                .sum::<u64>();
            let usage = Usage {
                input_tokens: (input_len / 4).max(1),
                output_tokens: 32,
                total_tokens: (input_len / 4).max(1) + 32,
            };
            yield Ok(ModelOutputEvent::Completed { usage: Some(usage) });
        };

        Ok(Box::pin(response_stream))
    }
}

type RuntimeHandle = SessionRuntime<DesktopModel, AgentToolRuntime>;

struct AppState {
    runtime: Arc<RuntimeHandle>,
    frontend_to_backend_session: Arc<RwLock<HashMap<String, String>>>,
}

#[tauri::command]
async fn create_chat_session(title: Option<String>) -> Result<ChatSession, String> {
    let now = chrono::Utc::now().timestamp_millis();
    Ok(ChatSession {
        id: format!("s-{}", uuid::Uuid::new_v4()),
        title: title.unwrap_or_else(|| "New Chat".to_string()),
        color: "blue".to_string(),
        created_at: now,
        updated_at: now,
        status: "active".to_string(),
    })
}

#[tauri::command]
async fn list_chat_sessions() -> Result<Vec<ChatSession>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn delete_chat_session(_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn get_chat_messages(_session_id: String) -> Result<Vec<ChatMessage>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn start_agent_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: StartAgentTurnPayload,
) -> Result<StartAgentTurnResponse, String> {
    let _selected_model = payload.model.unwrap_or_else(|| "default".to_string());
    let _attachments_count = payload.attachments.as_ref().map_or(0, Vec::len);

    let backend_session_id = ensure_backend_session_id(&state, &payload.session_id).await?;
    let turn_id = new_id();
    let request = TurnRequest {
        meta: SessionMeta::new(backend_session_id, turn_id.clone()),
        initial_input: InputEnvelope::user_text(payload.input),
        transcript: Vec::new(),
    };

    let streams = state
        .runtime
        .run_turn(request)
        .await
        .map_err(|err| format!("failed to run turn: {err}"))?;

    spawn_stream_forwarders(app, payload.session_id, turn_id.clone(), streams.run, streams.ui);

    Ok(StartAgentTurnResponse { turn_id })
}

#[tauri::command]
async fn cancel_agent_turn(
    state: State<'_, AppState>,
    payload: CancelAgentTurnPayload,
) -> Result<(), String> {
    state
        .runtime
        .cancel_turn(&payload.turn_id, Some("cancelled from desktop ui".to_string()))
        .await
        .map_err(|err| format!("failed to cancel turn {}: {err}", payload.turn_id))
}

fn spawn_stream_forwarders(
    app: AppHandle,
    session_id: String,
    turn_id: String,
    run_stream: RunEventStream,
    ui_stream: UiEventStream,
) {
    // Use mpsc channel to merge run/ui streams into a single emitter
    // This ensures sequential seq allocation and emission, preventing out-of-order events
    let (tx, rx) = mpsc::channel::<(String, serde_json::Value)>(128);

    // Spawn the merged emitter task
    let app_clone = app.clone();
    let session_id_clone = session_id.clone();
    let turn_id_clone = turn_id.clone();
    let seq = Arc::new(AtomicU64::new(0));
    let seq_clone = Arc::clone(&seq);

    tokio::spawn(async move {
        let mut rx = rx;
        while let Some((source, event)) = rx.recv().await {
            let envelope = AgentStreamEnvelope {
                session_id: session_id_clone.clone(),
                turn_id: turn_id_clone.clone(),
                source,
                seq: seq_clone.fetch_add(1, Ordering::SeqCst) + 1,
                ts: chrono::Utc::now().timestamp_millis(),
                event,
            };
            if let Err(err) = app_clone.emit("agent:stream", envelope) {
                eprintln!("failed to emit agent stream event: {err}");
            }
        }
    });

    // Forward run stream events to the merged channel
    let tx_run = tx.clone();
    tokio::spawn(async move {
        let mut stream = run_stream;
        while let Some(event) = stream.next().await {
            let event_json = serde_json::to_value(&event).unwrap_or_else(
                |err| json!({ "type": "serialization_error", "message": err.to_string() }),
            );
            let _ = tx_run.send(("run".to_string(), event_json)).await;
        }
    });

    // Forward ui stream events to the merged channel
    let tx_ui = tx;
    tokio::spawn(async move {
        let mut stream = ui_stream;
        while let Some(event) = stream.next().await {
            let event_json = serde_json::to_value(&event).unwrap_or_else(
                |err| json!({ "type": "serialization_error", "message": err.to_string() }),
            );
            let _ = tx_ui.send(("ui".to_string(), event_json)).await;
        }
    });
}

async fn ensure_backend_session_id(
    state: &AppState,
    frontend_session_id: &str,
) -> Result<String, String> {
    if let Some(existing) = state
        .frontend_to_backend_session
        .read()
        .await
        .get(frontend_session_id)
        .cloned()
    {
        return Ok(existing);
    }

    let backend_session_id = state
        .runtime
        .create_session(
            None,
            Some(format!("Desktop Session {}", frontend_session_id)),
        )
        .await
        .map_err(|err| format!("failed to create backend session: {err}"))?;

    state
        .frontend_to_backend_session
        .write()
        .await
        .insert(frontend_session_id.to_string(), backend_session_id.clone());

    Ok(backend_session_id)
}

fn latest_user_text(inputs: &[InputEnvelope]) -> Option<String> {
    for input in inputs.iter().rev() {
        if input.source != InputSource::User {
            continue;
        }
        for part in input.parts.iter().rev() {
            if let InputPart::Text { text } = part {
                return Some(text.clone());
            }
        }
    }
    None
}

fn latest_tool_output(inputs: &[InputEnvelope]) -> Option<serde_json::Value> {
    for input in inputs.iter().rev() {
        if input.source != InputSource::Tool {
            continue;
        }
        for part in input.parts.iter().rev() {
            if let InputPart::Json { value } = part {
                return Some(value.clone());
            }
        }
    }
    None
}

fn extract_tool_command(text: Option<&str>) -> Option<String> {
    let text = text?.trim();
    if let Some(rest) = text.strip_prefix("/shell ") {
        let command = rest.trim();
        if !command.is_empty() {
            return Some(command.to_string());
        }
    }
    if let Some(rest) = text.strip_prefix("/tool ") {
        let command = rest.trim();
        if !command.is_empty() {
            return Some(command.to_string());
        }
    }
    None
}

fn summarize_tool_output(output: &serde_json::Value) -> String {
    let stdout = output
        .get("stdout")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("");
    let stderr = output
        .get("stderr")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("");
    let exit_code = output
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .map_or("unknown".to_string(), |code| code.to_string());

    if !stdout.is_empty() && stderr.is_empty() {
        return format!("Tool finished with exit code {exit_code}.\n\n{stdout}");
    }
    if stdout.is_empty() && !stderr.is_empty() {
        return format!("Tool finished with exit code {exit_code}.\n\nstderr:\n{stderr}");
    }
    if !stdout.is_empty() && !stderr.is_empty() {
        return format!(
            "Tool finished with exit code {exit_code}.\n\nstdout:\n{stdout}\n\nstderr:\n{stderr}"
        );
    }
    format!(
        "Tool finished with exit code {exit_code}.\n\n{}",
        output
            .to_string()
            .chars()
            .take(1000)
            .collect::<String>()
    )
}

fn build_runtime_state(base_path: PathBuf) -> AppState {
    let tools = tauri::async_runtime::block_on(AgentToolRuntime::default_with_builtins());
    let runtime = SessionRuntime::with_config(
        base_path,
        Arc::new(DesktopModel),
        Arc::new(tools),
        SessionConfig {
            max_parallel_tools: 4,
        },
    );

    AppState {
        runtime: Arc::new(runtime),
        frontend_to_backend_session: Arc::new(RwLock::new(HashMap::new())),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .setup(|app| {
            let base_path = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("argusx-desktop-agent"))
                .join("sessions");
            std::fs::create_dir_all(&base_path)?;
            app.manage(build_runtime_state(base_path));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            create_chat_session,
            list_chat_sessions,
            delete_chat_session,
            get_chat_messages,
            start_agent_turn,
            cancel_agent_turn,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
