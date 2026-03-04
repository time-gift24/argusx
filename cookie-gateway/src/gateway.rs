// HTTP gateway module for cookie-gateway
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::command_bus::GatewayCommandBus;
use crate::error::CookieGatewayError;
use crate::proxy;
use crate::tool::{CookieCommandClient, CookieFetchOutput, CookieFetchTool};
use crate::CookieData;
use crate::CookieStore;

const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 10_000;

#[derive(Clone)]
pub struct GatewayState {
    pub store: Arc<CookieStore>,
    pub command_bus: Arc<GatewayCommandBus>,
    pub cookie_fetch_tool: Arc<CookieFetchTool>,
}

impl GatewayState {
    pub fn new() -> Self {
        Self::with_store(Arc::new(CookieStore::new()))
    }

    pub fn with_store(store: Arc<CookieStore>) -> Self {
        let command_bus = Arc::new(GatewayCommandBus::new());
        let command_client: Arc<dyn CookieCommandClient> = command_bus.clone();
        let cookie_fetch_tool = Arc::new(CookieFetchTool::new(
            store.clone(),
            command_client,
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        ));

        Self {
            store,
            command_bus,
            cookie_fetch_tool,
        }
    }
}

#[derive(Deserialize)]
pub struct UploadCookiesRequest {
    pub domain: String,
    pub cookies: Vec<CookieData>,
}

#[derive(Deserialize)]
pub struct GetCookiesQuery {
    pub domain: String,
}

#[derive(Deserialize)]
pub struct FetchCookiesRequest {
    pub domain: String,
    pub refresh_after_ms: u64,
}

#[derive(Serialize)]
pub struct GetCookiesResponse {
    pub cookies: Vec<CookieData>,
}

pub fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ws/client", get(ws_client))
        .route("/api/cookies", post(upload_cookies))
        .route("/api/cookies", get(get_cookies))
        .route("/api/cookies/fetch", post(fetch_cookies))
        .route("/api/proxy", post(proxy::proxy_request))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

async fn ws_client(ws: WebSocketUpgrade, State(state): State<GatewayState>) -> impl IntoResponse {
    let command_bus = state.command_bus.clone();
    ws.on_upgrade(move |socket| async move {
        command_bus.handle_websocket(socket).await;
    })
}

async fn upload_cookies(
    State(state): State<GatewayState>,
    Json(payload): Json<UploadCookiesRequest>,
) -> Result<Json<serde_json::Value>, impl IntoResponse> {
    let store = state.store.clone();

    // Validate whitelist
    if !store.is_whitelisted(&payload.domain) {
        return Err(CookieGatewayError::DomainNotWhitelisted {
            domain: payload.domain.clone(),
        });
    }

    // Check opt-in
    if !store.is_opted_in().await {
        return Err(CookieGatewayError::UserNotOptedIn);
    }

    // Store cookies
    let count = payload.cookies.len();
    store.store_cookies(&payload.domain, payload.cookies).await;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "domain": payload.domain,
        "count": count,
    })))
}

async fn get_cookies(
    Query(query): Query<GetCookiesQuery>,
    State(state): State<GatewayState>,
) -> Result<Json<GetCookiesResponse>, impl IntoResponse> {
    let store = state.store.clone();

    // Validate whitelist
    if !store.is_whitelisted(&query.domain) {
        return Err(CookieGatewayError::DomainNotWhitelisted {
            domain: query.domain.clone(),
        });
    }

    // Retrieve cookies
    let cookies = store.get_cookies(&query.domain).await.unwrap_or_default();

    Ok(Json(GetCookiesResponse { cookies }))
}

async fn fetch_cookies(
    State(state): State<GatewayState>,
    Json(payload): Json<FetchCookiesRequest>,
) -> Result<Json<CookieFetchOutput>, impl IntoResponse> {
    let domain = payload.domain.trim().to_string();
    if domain.is_empty() {
        return Err(CookieGatewayError::InvalidDomain {
            domain: payload.domain,
        });
    }

    // Validate whitelist
    if !state.store.is_whitelisted(&domain) {
        return Err(CookieGatewayError::DomainNotWhitelisted { domain });
    }

    // Check opt-in
    if !state.store.is_opted_in().await {
        return Err(CookieGatewayError::UserNotOptedIn);
    }

    let result = state
        .cookie_fetch_tool
        .fetch(&domain, Duration::from_millis(payload.refresh_after_ms))
        .await?;

    Ok(Json(result))
}
