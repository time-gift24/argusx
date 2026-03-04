use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::process::Command;

use agent_core::{
    new_id, InputEnvelope, RunEventStream, Runtime, SessionMeta, TurnRequest, TurnStatus,
    UiEventStream,
};
use agent_session::{SessionConfig, SessionFilter, SessionRuntime, SqliteSessionStore};
use agent_tool::AgentToolRuntime;
use agent_turn::adapters::bigmodel::BigModelAdapterConfig;
use agent_turn::BigModelModelAdapter;
use cookie_gateway::{CookieGateway, CookieStore};
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
mod system_prompt;
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatTurnSummary {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub final_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
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
struct UpdateChatSessionPayload {
    id: String,
    title: String,
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
    runtime_config_bootstrap_error: Arc<RwLock<Option<String>>>,
    frontend_to_backend_session: Arc<RwLock<HashMap<String, String>>>,
    cookie_gateway: Arc<CookieGateway>,
}

const SQLITE_DB_PATH_ENV: &str = "ARGUSX_DESKTOP_DB_PATH";
const SQLITE_DB_FILE_NAME: &str = "desktop.sqlite3";
const SQLITE_DB_TEMP_DIR: &str = "argusx-desktop-agent";

struct UnconfiguredAdapter;

#[async_trait::async_trait]
impl ProviderAdapter for UnconfiguredAdapter {
    fn id(&self) -> &str {
        "unconfigured"
    }

    async fn chat(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::InvalidRequest {
            message: "未配置提供商".to_string(),
        })
    }

    fn chat_stream(&self, _req: LlmRequest) -> LlmChunkStream {
        Box::pin(async_stream::stream! {
            yield Err(LlmError::InvalidRequest {
                message: "未配置提供商".to_string(),
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
        .map_err(|err| format!("创建会话失败: {err}"))?;

    let Some(info) = state
        .runtime
        .get_session(&backend_session_id)
        .await
        .map_err(|err| format!("加载会话失败: {err}"))?
    else {
        return Err("加载刚创建的会话失败".to_string());
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
        .list_sessions(SessionFilter {
            limit: None,
            ..SessionFilter::default()
        })
        .await
        .map_err(|err| format!("failed to list sessions: {err}"))?;

    let mut mapping = state.frontend_to_backend_session.write().await;
    for session in &sessions {
        mapping.insert(session.session_id.clone(), session.session_id.clone());
    }

    Ok(sessions.into_iter().map(chat_session_from_info).collect())
}

#[tauri::command]
async fn update_chat_session(
    state: State<'_, AppState>,
    payload: UpdateChatSessionPayload,
) -> Result<ChatSession, String> {
    let title = payload.title.trim();
    if title.is_empty() {
        return Err("会话标题不能为空".to_string());
    }

    let backend_session_id = state
        .frontend_to_backend_session
        .read()
        .await
        .get(&payload.id)
        .cloned()
        .unwrap_or(payload.id.clone());

    let info = state
        .runtime
        .rename_session(&backend_session_id, title.to_string())
        .await
        .map_err(|err| format!("更新会话失败: {err}"))?;

    state
        .frontend_to_backend_session
        .write()
        .await
        .insert(payload.id, backend_session_id);

    Ok(chat_session_from_info(info))
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
        .map_err(|err| format!("删除会话失败: {err}"))?;

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
    let backend_session_id = state
        .frontend_to_backend_session
        .read()
        .await
        .get(&session_id)
        .cloned()
        .unwrap_or(session_id.clone());

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
        .list_messages(&backend_session_id, query)
        .map_err(|err| format!("查询聊天消息失败: {err}"))?;

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
async fn get_chat_turn_summaries(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatTurnSummary>, String> {
    let backend_session_id = state
        .frontend_to_backend_session
        .read()
        .await
        .get(&session_id)
        .cloned()
        .unwrap_or(session_id.clone());

    let summaries = state
        .runtime
        .list_turn_summaries(&backend_session_id)
        .await
        .map_err(|err| format!("查询轮次摘要失败: {err}"))?;

    Ok(summaries
        .into_iter()
        .map(|summary| ChatTurnSummary {
            id: summary.turn_id,
            session_id: backend_session_id.clone(),
            status: map_turn_status(summary.status),
            final_message: summary.final_message,
            created_at: summary.started_at,
            updated_at: summary.ended_at.unwrap_or(summary.started_at),
        })
        .collect())
}

#[tauri::command]
async fn get_llm_runtime_config(state: State<'_, AppState>) -> Result<LlmRuntimeConfig, String> {
    if let Some(error) = state.runtime_config_bootstrap_error.read().await.clone() {
        return Err(error);
    }
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
    *state.runtime_config_bootstrap_error.write().await = None;
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
    *state.runtime_config_bootstrap_error.write().await = None;
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
        .map_err(|err| format!("运行轮次失败: {err}"))?;

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
        .map_err(|err| format!("取消轮次 {} 失败: {err}", payload.turn_id))
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
                "在任何会话中未找到轮次 {}",
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
        .map_err(|err| format!("恢复检查点 {} 失败: {err}", payload.turn_id))?;

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

#[tauri::command]
async fn get_cookie_opt_in(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.cookie_gateway.store().is_opted_in().await)
}

#[tauri::command]
async fn set_cookie_opt_in(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.cookie_gateway.store().set_opt_in(enabled).await;
    Ok(())
}

#[tauri::command]
async fn open_extension_folder(app: AppHandle) -> Result<(), String> {
    let extension_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("capabilities/default/extensions");

    #[cfg(target_os = "macos")]
    Command::new("open")
        .arg(&extension_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    Command::new("explorer")
        .arg(&extension_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open")
        .arg(&extension_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn build_runtime_state(base_path: PathBuf) -> Result<AppState, String> {
    let _ =
        open_and_bootstrap(&base_path).map_err(|err| format!("bootstrap schema failed: {err}"))?;
    let runtime_config_repo =
        RuntimeConfigRepo::new(base_path.clone()).map_err(map_runtime_config_repo_error)?;
    let chat_repo = ChatRepo::new(base_path.clone())
        .map_err(|err| format!("chat repo bootstrap failed: {err}"))?;
    let (runtime_cfg, runtime_config_bootstrap_error) = match runtime_config_repo.load() {
        Ok(Some(config)) => (config, None),
        Ok(None) => (normalize_runtime_config(LlmRuntimeConfig::default()), None),
        Err(RuntimeConfigRepoError::FingerprintMismatch) => {
            runtime_config_repo
                .clear()
                .map_err(map_runtime_config_repo_error)?;
            (normalize_runtime_config(LlmRuntimeConfig::default()), None)
        }
        Err(RuntimeConfigRepoError::HostFingerprint(_)) => (
            normalize_runtime_config(LlmRuntimeConfig::default()),
            Some(fingerprint_unavailable_user_message()),
        ),
        Err(err) => return Err(map_runtime_config_repo_error(err)),
    };
    let llm_client = build_llm_client_from_runtime_config(&runtime_cfg)?;

    let mut model_config = BigModelAdapterConfig::default();
    if let Ok(model) = std::env::var("BIGMODEL_MODEL") {
        if !model.trim().is_empty() {
            model_config.model = model;
        }
    }

    // Resolve system prompt: env override or default autonomy prompt
    let prompt_override = std::env::var("ARGUSX_SYSTEM_PROMPT").ok();
    model_config.system_prompt = Some(system_prompt::resolve_desktop_system_prompt(
        prompt_override.as_deref(),
    ));

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

    // Initialize cookie gateway
    let cookie_store = CookieStore::new();
    let cookie_gateway = Arc::new(CookieGateway::new(cookie_store));

    Ok(AppState {
        runtime: Arc::new(runtime),
        model_adapter,
        chat_repo: Arc::new(chat_repo),
        runtime_config_repo: Arc::new(runtime_config_repo),
        llm_runtime_config: Arc::new(RwLock::new(runtime_cfg)),
        runtime_config_bootstrap_error: Arc::new(RwLock::new(runtime_config_bootstrap_error)),
        frontend_to_backend_session: Arc::new(RwLock::new(HashMap::new())),
        cookie_gateway,
    })
}

fn resolve_sqlite_db_path(app_data_dir: Option<PathBuf>) -> PathBuf {
    let env_override = std::env::var(SQLITE_DB_PATH_ENV).ok();
    resolve_sqlite_db_path_with_override(env_override.as_deref(), app_data_dir)
}

fn resolve_startup_db_path(app_data_dir: Option<PathBuf>) -> PathBuf {
    resolve_sqlite_db_path(app_data_dir)
}

fn resolve_sqlite_db_path_with_override(
    env_override: Option<&str>,
    app_data_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = env_override.map(str::trim).filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }

    let base_dir = app_data_dir
        .unwrap_or_else(|| std::env::temp_dir().join(SQLITE_DB_TEMP_DIR));
    base_dir.join(SQLITE_DB_FILE_NAME)
}

fn fingerprint_mismatch_user_message() -> String {
    "存储的运行时凭据绑定到不同的机器指纹。清除存储的凭据并重新输入API密钥。".to_string()
}

fn fingerprint_unavailable_user_message() -> String {
    "无法加载加密的运行时凭据，因为此机器上无法获取主机指纹。".to_string()
}

fn map_runtime_config_repo_error(err: RuntimeConfigRepoError) -> String {
    match err {
        RuntimeConfigRepoError::FingerprintMismatch => fingerprint_mismatch_user_message(),
        RuntimeConfigRepoError::HostFingerprint(_) => fingerprint_unavailable_user_message(),
        other => format!("runtime config persistence error: {other}"),
    }
}

fn map_turn_status(status: TurnStatus) -> String {
    match status {
        TurnStatus::Running => "running",
        TurnStatus::Done => "done",
        TurnStatus::Failed => "failed",
        TurnStatus::Cancelled => "cancelled",
    }
    .to_string()
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
            let db_path = resolve_startup_db_path(app.path().app_data_dir().ok());
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let state = build_runtime_state(db_path).map_err(std::io::Error::other)?;

            // Start cookie gateway server
            let gateway = state.cookie_gateway.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = gateway.start().await {
                    eprintln!("Cookie gateway error: {}", e);
                }
            });

            app.manage(state);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            create_chat_session,
            list_chat_sessions,
            update_chat_session,
            delete_chat_session,
            get_chat_messages,
            get_chat_turn_summaries,
            get_llm_runtime_config,
            set_llm_runtime_config,
            clear_llm_runtime_config,
            list_available_models,
            start_agent_turn,
            cancel_agent_turn,
            restore_turn_checkpoint,
            get_cookie_opt_in,
            set_cookie_opt_in,
            open_extension_folder,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_runtime_config() -> LlmRuntimeConfig {
        LlmRuntimeConfig {
            default_provider: Some(ProviderId::Openai),
            providers: crate::llm_runtime_config::ProviderConfigs {
                openai: crate::llm_runtime_config::ProviderRuntimeConfig {
                    api_key: "sk-openai-test".to_string(),
                    base_url: "https://openai.provider.test/v1".to_string(),
                    models: vec!["gpt-4o".to_string()],
                    headers: vec![],
                },
                ..crate::llm_runtime_config::ProviderConfigs::default()
            },
        }
    }

    #[test]
    fn fingerprint_message_is_actionable() {
        let message = fingerprint_mismatch_user_message();
        assert!(message.contains("指纹"));
        assert!(message.contains("清除存储的凭据"));
    }

    #[test]
    fn build_runtime_state_clears_mismatched_runtime_credentials() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("desktop.db");
        let repo = RuntimeConfigRepo::new(db_path.clone()).expect("create repo");
        let config = sample_runtime_config();
        repo.save_with_fingerprint(&config, "fp-legacy")
            .expect("save legacy config");

        let state = build_runtime_state(db_path.clone()).expect("build runtime state");
        let bootstrap_error = tauri::async_runtime::block_on(async {
            state.runtime_config_bootstrap_error.read().await.clone()
        });
        assert!(
            bootstrap_error.is_none(),
            "fingerprint mismatch should auto-recover instead of blocking runtime config reads"
        );

        let loaded_with_legacy_key = repo
            .load_with_fingerprint("fp-legacy")
            .expect("load with legacy fingerprint");
        assert!(
            loaded_with_legacy_key.is_none(),
            "auto-recovery should clear stale encrypted credentials"
        );
    }

    #[test]
    fn map_runtime_error_uses_actionable_fingerprint_message() {
        assert_eq!(
            map_runtime_config_repo_error(RuntimeConfigRepoError::FingerprintMismatch),
            fingerprint_mismatch_user_message()
        );
    }

    #[test]
    fn fingerprint_unavailable_message_is_actionable() {
        let message = fingerprint_unavailable_user_message();
        assert!(message.contains("主机指纹"));
        assert!(message.contains("无法获取"));
    }

    #[test]
    fn resolve_sqlite_db_path_uses_env_override_when_present() {
        let app_data_dir = Some(PathBuf::from("/var/app/data"));
        let resolved = resolve_sqlite_db_path_with_override(
            Some("/tmp/argusx-custom.sqlite3"),
            app_data_dir,
        );
        assert_eq!(resolved, PathBuf::from("/tmp/argusx-custom.sqlite3"));
    }

    #[test]
    fn resolve_sqlite_db_path_uses_app_data_dir_by_default() {
        let app_data_dir = Some(PathBuf::from("/var/app/data"));
        let resolved = resolve_sqlite_db_path_with_override(None, app_data_dir);
        assert_eq!(
            resolved,
            PathBuf::from("/var/app/data").join(SQLITE_DB_FILE_NAME)
        );
    }

    #[test]
    fn resolve_sqlite_db_path_falls_back_to_temp_dir_when_no_app_data_dir() {
        let resolved = resolve_sqlite_db_path_with_override(None, None);
        assert_eq!(
            resolved,
            std::env::temp_dir()
                .join(SQLITE_DB_TEMP_DIR)
                .join(SQLITE_DB_FILE_NAME)
        );
    }

    #[test]
    fn startup_db_path_resolves_to_sqlite_file_not_sessions_directory() {
        let resolved = resolve_startup_db_path(Some(PathBuf::from("/var/app/data")));
        assert_eq!(
            resolved,
            PathBuf::from("/var/app/data").join(SQLITE_DB_FILE_NAME)
        );
        assert_ne!(
            resolved.file_name().and_then(|value| value.to_str()),
            Some("sessions")
        );
    }
}
