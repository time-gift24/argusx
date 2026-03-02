// HTTP gateway module for cookie-gateway
use axum::{Router, routing::get, Json};
use std::sync::Arc;
use crate::CookieStore;

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

pub fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}
