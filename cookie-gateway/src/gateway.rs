// HTTP gateway module for cookie-gateway
use axum::{Router, routing::{get, post}, Json, extract::{State, Query}, response::IntoResponse};
use std::sync::Arc;
use crate::CookieStore;
use serde::{Deserialize, Serialize};
use crate::CookieData;
use crate::error::CookieGatewayError;
use crate::proxy;

#[derive(Clone)]
pub struct GatewayState {
    pub store: Arc<CookieStore>,
}

impl GatewayState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(CookieStore::new()),
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

#[derive(Serialize)]
pub struct GetCookiesResponse {
    pub cookies: Vec<CookieData>,
}

pub fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/cookies", post(upload_cookies))
        .route("/api/cookies", get(get_cookies))
        .route("/api/proxy", post(proxy::proxy_request))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

async fn upload_cookies(
    State(state): State<GatewayState>,
    Json(payload): Json<UploadCookiesRequest>,
) -> Result<Json<serde_json::Value>, impl IntoResponse> {
    let store = state.store.clone();

    // Validate whitelist
    if !store.is_whitelisted(&payload.domain) {
        return Err(CookieGatewayError::DomainNotWhitelisted { domain: payload.domain.clone() });
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
        return Err(CookieGatewayError::DomainNotWhitelisted { domain: query.domain.clone() });
    }

    // Retrieve cookies
    let cookies = store.get_cookies(&query.domain).await.unwrap_or_default();

    Ok(Json(GetCookiesResponse { cookies }))
}
