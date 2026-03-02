use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use agent_core::{
    new_id, InputEnvelope, RunEventStream, Runtime, SessionMeta, TurnRequest, UiEventStream,
};
use agent_session::{SessionConfig, SessionRuntime};
use agent_tool::AgentToolRuntime;
use agent_turn::adapters::bigmodel::BigModelAdapterConfig;
use agent_turn::BigModelModelAdapter;
use futures::StreamExt;
use llm_client::{LlmChunkStream, LlmClient, LlmError, LlmRequest, LlmResponse, ProviderAdapter};
use llm_provider::anthropic::{AnthropicAdapter, AnthropicConfig};
use llm_provider::bigmodel::{BigModelAdapter, BigModelConfig};
use llm_provider::openai_compat::{ChatCompletionsConfig, OpenAiCompatAdapter};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, RwLock};

mod llm_runtime_config;
mod persistence;
mod secure_config;
use llm_runtime_config::{
    list_available_models as derive_available_models, normalize_runtime_config,
    validate_turn_selection, AvailableModel, LlmRuntimeConfig, ProviderId,
};

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
    provider: ProviderId,
    model: String,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreTurnCheckpointPayload {
    session_id: String,
    turn_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreTurnCheckpointResponse {
    restored_turn_id: String,
    removed_turn_ids: Vec<String>,
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

type RuntimeHandle = SessionRuntime<BigModelModelAdapter, AgentToolRuntime>;

struct AppState {
    runtime: Arc<RuntimeHandle>,
    model_adapter: Arc<BigModelModelAdapter>,
    llm_runtime_config: Arc<RwLock<LlmRuntimeConfig>>,
    frontend_to_backend_session: Arc<RwLock<HashMap<String, String>>>,
}

struct UnconfiguredAdapter;

#[async_trait::async_trait]
impl ProviderAdapter for UnconfiguredAdapter {
    fn id(&self) -> &str {
        "unconfigured"
    }

    async fn chat(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::InvalidRequest {
            message: "No provider is configured".to_string(),
        })
    }

    fn chat_stream(&self, _req: LlmRequest) -> LlmChunkStream {
        Box::pin(async_stream::stream! {
            yield Err(LlmError::InvalidRequest {
                message: "No provider is configured".to_string(),
            });
        })
    }
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
async fn get_llm_runtime_config(state: State<'_, AppState>) -> Result<LlmRuntimeConfig, String> {
    Ok(state.llm_runtime_config.read().await.clone())
}

#[tauri::command]
async fn set_llm_runtime_config(
    state: State<'_, AppState>,
    payload: LlmRuntimeConfig,
) -> Result<LlmRuntimeConfig, String> {
    let normalized = normalize_runtime_config(payload);
    let client = build_llm_client_from_runtime_config(&normalized)?;
    state.model_adapter.set_client(Arc::new(client));
    *state.llm_runtime_config.write().await = normalized.clone();
    Ok(normalized)
}

#[tauri::command]
async fn list_available_models(state: State<'_, AppState>) -> Result<Vec<AvailableModel>, String> {
    let cfg = state.llm_runtime_config.read().await;
    Ok(derive_available_models(&cfg))
}

#[tauri::command]
async fn start_agent_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: StartAgentTurnPayload,
) -> Result<StartAgentTurnResponse, String> {
    let _attachments_count = payload.attachments.as_ref().map_or(0, Vec::len);

    {
        let cfg = state.llm_runtime_config.read().await;
        validate_turn_selection(&cfg, &payload.provider, &payload.model)?;
    }

    let backend_session_id = ensure_backend_session_id(&state, &payload.session_id).await?;
    let turn_id = new_id();
    let request = TurnRequest {
        meta: SessionMeta::new(backend_session_id, turn_id.clone()),
        provider: payload.provider.as_adapter_id().to_string(),
        model: payload.model.clone(),
        initial_input: InputEnvelope::user_text(payload.input),
        transcript: Vec::new(),
    };

    let streams = state
        .runtime
        .run_turn(request)
        .await
        .map_err(|err| format!("failed to run turn: {err}"))?;

    spawn_stream_forwarders(
        app,
        payload.session_id,
        turn_id.clone(),
        streams.run,
        streams.ui,
    );

    Ok(StartAgentTurnResponse { turn_id })
}

#[tauri::command]
async fn cancel_agent_turn(
    state: State<'_, AppState>,
    payload: CancelAgentTurnPayload,
) -> Result<(), String> {
    state
        .runtime
        .cancel_turn(
            &payload.turn_id,
            Some("cancelled from desktop ui".to_string()),
        )
        .await
        .map_err(|err| format!("failed to cancel turn {}: {err}", payload.turn_id))
}

#[tauri::command]
async fn restore_turn_checkpoint(
    state: State<'_, AppState>,
    payload: RestoreTurnCheckpointPayload,
) -> Result<RestoreTurnCheckpointResponse, String> {
    let mapped_backend_session_id = state
        .frontend_to_backend_session
        .read()
        .await
        .get(&payload.session_id)
        .cloned();

    let backend_session_id = if let Some(session_id) = mapped_backend_session_id {
        session_id
    } else {
        let Some(found_session_id) = state
            .runtime
            .find_session_id_by_turn_id(&payload.turn_id)
            .await
            .map_err(|err| format!("failed to resolve session by turn id: {err}"))?
        else {
            return Err(format!(
                "turn {} was not found in any session",
                payload.turn_id
            ));
        };
        state
            .frontend_to_backend_session
            .write()
            .await
            .insert(payload.session_id.clone(), found_session_id.clone());
        found_session_id
    };

    let result = state
        .runtime
        .restore_to_turn(&backend_session_id, &payload.turn_id)
        .await
        .map_err(|err| format!("failed to restore checkpoint {}: {err}", payload.turn_id))?;

    Ok(RestoreTurnCheckpointResponse {
        restored_turn_id: result.restored_turn_id,
        removed_turn_ids: result.removed_turn_ids,
    })
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

fn build_runtime_state(base_path: PathBuf) -> Result<AppState, String> {
    let runtime_cfg = normalize_runtime_config(LlmRuntimeConfig::default());
    let llm_client = build_llm_client_from_runtime_config(&runtime_cfg)?;

    let mut model_config = BigModelAdapterConfig::default();
    if let Ok(model) = std::env::var("BIGMODEL_MODEL") {
        if !model.trim().is_empty() {
            model_config.model = model;
        }
    }

    let tools = tauri::async_runtime::block_on(AgentToolRuntime::default_with_builtins());
    let model_adapter =
        Arc::new(BigModelModelAdapter::new(Arc::new(llm_client)).with_config(model_config));
    let runtime = SessionRuntime::with_config(
        base_path,
        Arc::clone(&model_adapter),
        Arc::new(tools),
        SessionConfig {
            max_parallel_tools: 4,
        },
    );

    Ok(AppState {
        runtime: Arc::new(runtime),
        model_adapter,
        llm_runtime_config: Arc::new(RwLock::new(runtime_cfg)),
        frontend_to_backend_session: Arc::new(RwLock::new(HashMap::new())),
    })
}

fn build_llm_client_from_runtime_config(cfg: &LlmRuntimeConfig) -> Result<LlmClient, String> {
    let mut builder = LlmClient::builder();
    let mut default_adapter = cfg
        .default_provider
        .as_ref()
        .map(ProviderId::as_adapter_id)
        .map(ToString::to_string);

    if cfg.providers.bigmodel.is_available() {
        let provider_cfg = BigModelConfig::new(
            cfg.providers.bigmodel.base_url.clone(),
            cfg.providers.bigmodel.api_key.clone(),
            cfg.providers.bigmodel.header_map(),
        )
        .map_err(|err| format!("failed to create bigmodel config: {err}"))?;
        builder = builder.register_adapter(Arc::new(BigModelAdapter::new(provider_cfg)));
        if default_adapter.is_none() {
            default_adapter = Some("bigmodel".to_string());
        }
    }

    if cfg.providers.openai.is_available() {
        let provider_cfg = ChatCompletionsConfig::new(
            cfg.providers.openai.base_url.clone(),
            cfg.providers.openai.api_key.clone(),
            cfg.providers.openai.header_map(),
        )
        .map_err(|err| format!("failed to create openai config: {err}"))?;
        builder = builder.register_adapter(Arc::new(OpenAiCompatAdapter::new(provider_cfg)));
        if default_adapter.is_none() {
            default_adapter = Some("openai".to_string());
        }
    }

    if cfg.providers.anthropic.is_available() {
        let provider_cfg = AnthropicConfig::new(
            cfg.providers.anthropic.base_url.clone(),
            cfg.providers.anthropic.api_key.clone(),
            cfg.providers.anthropic.header_map(),
        )
        .map_err(|err| format!("failed to create anthropic config: {err}"))?;
        builder = builder.register_adapter(Arc::new(AnthropicAdapter::new(provider_cfg)));
        if default_adapter.is_none() {
            default_adapter = Some("anthropic".to_string());
        }
    }

    if let Some(adapter) = default_adapter {
        builder = builder.default_adapter(adapter);
    } else {
        builder = builder
            .register_adapter(Arc::new(UnconfiguredAdapter))
            .default_adapter("unconfigured");
    }

    builder
        .build()
        .map_err(|err| format!("failed to build LLM client: {err}"))
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
            let state = build_runtime_state(base_path).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            create_chat_session,
            list_chat_sessions,
            delete_chat_session,
            get_chat_messages,
            get_llm_runtime_config,
            set_llm_runtime_config,
            list_available_models,
            start_agent_turn,
            cancel_agent_turn,
            restore_turn_checkpoint,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
