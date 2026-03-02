use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use agent_core::{
    new_id, InputEnvelope, RunEventStream, Runtime, SessionMeta, TurnRequest, UiEventStream,
};
use agent_session::{SessionConfig, SessionFilter, SessionRuntime, SqliteSessionStore};
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
use persistence::{
    open_and_bootstrap, ChatMessageQuery, ChatMessageRange, ChatRepo, RuntimeConfigRepo,
    RuntimeConfigRepoError,
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
    chat_repo: Arc<ChatRepo>,
    runtime_config_repo: Arc<RuntimeConfigRepo>,
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
async fn create_chat_session(
    state: State<'_, AppState>,
    title: Option<String>,
) -> Result<ChatSession, String> {
    let backend_session_id = state
        .runtime
        .create_session(None, title)
        .await
        .map_err(|err| format!("failed to create session: {err}"))?;

    let Some(info) = state
        .runtime
        .get_session(&backend_session_id)
        .await
        .map_err(|err| format!("failed to load session: {err}"))?
    else {
        return Err("failed to load just-created session".to_string());
    };

    state
        .frontend_to_backend_session
        .write()
        .await
        .insert(backend_session_id.clone(), backend_session_id.clone());

    Ok(chat_session_from_info(info))
}

#[tauri::command]
async fn list_chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSession>, String> {
    let sessions = state
        .runtime
        .list_sessions(SessionFilter::default())
        .await
        .map_err(|err| format!("failed to list sessions: {err}"))?;

    let mut mapping = state.frontend_to_backend_session.write().await;
    for session in &sessions {
        mapping.insert(session.session_id.clone(), session.session_id.clone());
    }

    Ok(sessions.into_iter().map(chat_session_from_info).collect())
}

#[tauri::command]
async fn delete_chat_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let backend_session_id = state
        .frontend_to_backend_session
        .read()
        .await
        .get(&id)
        .cloned()
        .unwrap_or(id.clone());

    state
        .runtime
        .delete_session(&backend_session_id)
        .await
        .map_err(|err| format!("failed to delete session: {err}"))?;

    let mut mapping = state.frontend_to_backend_session.write().await;
    mapping.remove(&id);
    if backend_session_id != id {
        mapping.retain(|_, value| value != &backend_session_id);
    }
    Ok(())
}

#[tauri::command]
async fn get_chat_messages(
    state: State<'_, AppState>,
    session_id: String,
    range: Option<String>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<ChatMessage>, String> {
    let query = ChatMessageQuery {
        range: match range.as_deref() {
            Some("all") => ChatMessageRange::All,
            _ => ChatMessageRange::Last24Hours,
        },
        cursor,
        limit: limit.unwrap_or(300).clamp(1, 2_000),
    };

    let messages = state
        .chat_repo
        .list_messages(&session_id, query)
        .map_err(|err| format!("failed to query chat messages: {err}"))?;

    Ok(messages
        .into_iter()
        .map(|message| ChatMessage {
            id: message.id,
            session_id: message.session_id,
            role: message.role,
            content: message.content,
            created_at: message.created_at,
        })
        .collect())
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
    let persisted = state
        .runtime_config_repo
        .save(&normalized)
        .map_err(map_runtime_config_repo_error)?;
    let client = build_llm_client_from_runtime_config(&persisted)?;
    state.model_adapter.set_client(Arc::new(client));
    *state.llm_runtime_config.write().await = persisted.clone();
    Ok(persisted)
}

#[tauri::command]
async fn clear_llm_runtime_config(state: State<'_, AppState>) -> Result<LlmRuntimeConfig, String> {
    state
        .runtime_config_repo
        .clear()
        .map_err(map_runtime_config_repo_error)?;

    let normalized = normalize_runtime_config(LlmRuntimeConfig::default());
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

    let frontend_session_id_owned = frontend_session_id.to_string();
    if state
        .runtime
        .get_session(&frontend_session_id_owned)
        .await
        .map_err(|err| format!("failed to check session existence: {err}"))?
        .is_some()
    {
        state.frontend_to_backend_session.write().await.insert(
            frontend_session_id.to_string(),
            frontend_session_id.to_string(),
        );
        return Ok(frontend_session_id.to_string());
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
    let _ =
        open_and_bootstrap(&base_path).map_err(|err| format!("bootstrap schema failed: {err}"))?;
    let runtime_config_repo =
        RuntimeConfigRepo::new(base_path.clone()).map_err(map_runtime_config_repo_error)?;
    let chat_repo = ChatRepo::new(base_path.clone())
        .map_err(|err| format!("chat repo bootstrap failed: {err}"))?;
    let runtime_cfg = match runtime_config_repo.load() {
        Ok(Some(config)) => config,
        Ok(None) => normalize_runtime_config(LlmRuntimeConfig::default()),
        Err(RuntimeConfigRepoError::FingerprintMismatch) => {
            normalize_runtime_config(LlmRuntimeConfig::default())
        }
        Err(err) => return Err(map_runtime_config_repo_error(err)),
    };
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
    let sqlite_store = Arc::new(
        SqliteSessionStore::new(base_path)
            .map_err(|err| format!("failed to initialize sqlite session store: {err}"))?,
    );
    let runtime = SessionRuntime::with_store_and_config(
        sqlite_store,
        Arc::clone(&model_adapter),
        Arc::new(tools),
        SessionConfig {
            max_parallel_tools: 4,
        },
    );

    Ok(AppState {
        runtime: Arc::new(runtime),
        model_adapter,
        chat_repo: Arc::new(chat_repo),
        runtime_config_repo: Arc::new(runtime_config_repo),
        llm_runtime_config: Arc::new(RwLock::new(runtime_cfg)),
        frontend_to_backend_session: Arc::new(RwLock::new(HashMap::new())),
    })
}

fn map_runtime_config_repo_error(err: RuntimeConfigRepoError) -> String {
    match err {
        RuntimeConfigRepoError::FingerprintMismatch => {
            "stored config bound to different machine fingerprint".to_string()
        }
        other => format!("runtime config persistence error: {other}"),
    }
}

fn chat_session_from_info(info: agent_core::SessionInfo) -> ChatSession {
    ChatSession {
        id: info.session_id,
        title: info.title,
        color: "blue".to_string(),
        created_at: info.created_at,
        updated_at: info.updated_at,
        status: match info.status {
            agent_core::SessionStatus::Active => "active".to_string(),
            agent_core::SessionStatus::Idle => "idle".to_string(),
            agent_core::SessionStatus::Archived => "archived".to_string(),
        },
    }
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
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("argusx-desktop-agent"));
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("desktop.sqlite3");
            let state = build_runtime_state(db_path).map_err(std::io::Error::other)?;
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
            clear_llm_runtime_config,
            list_available_models,
            start_agent_turn,
            cancel_agent_turn,
            restore_turn_checkpoint,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
