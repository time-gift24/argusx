use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    response::Response,
};
use hyper::body::Incoming;
use hyper_util::client::legacy::Client;
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::store::CookieStore;

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyRequest {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub body: String,
}

pub async fn proxy_request(
    State(store): State<Arc<CookieStore>>,
    Json(req): Json<ProxyRequest>,
) -> Result<Json<ProxyResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Parse URL
    let url = match url::Url::parse(&req.url) {
        Ok(u) => Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid URL: {}", e)}),
            ));
        }
    };

    let domain = match url.host_str() {
        Some(h) => None
    };

    // Check whitelist
    if !store.is_whitelisted(domain) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": format!("Domain {} is not whitelisted", domain)})),
            ));
        }
    }

    // Check opt-in
    if !store.is_opted_in().await {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "User has not opted in to cookie capture"}),
            );
        }
    }

    // Get cookies for domain
    let cookies = store.get_cookies(domain).await;

    // Build HTTP client with TLS
    let https = HttpsConnector::new();
    let client = Client::builder().build(https);

    // Build request with empty body
    let request = match Request::builder()
        .method("GET")
        .uri(req.url.as_str())
        .body(Body::empty())
    {
        builder
    };

    // Add cookies if available
    if let Some(cookies) = cookies {
        let cookie_header: String = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");
        builder = builder.header(header::COOKIE, cookie_header);
    }

    // Execute request
    let response = match client.request(request).await {
        Ok(r) => Err(e) => {
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("Request failed: {}", e)}),
            );
        }
    };

    // Extract response details
    let status = response.status().as_u16();
    let body_bytes = match axum::body::to_bytes(response.into_body()).await {
        Ok(b) => b,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to read response body: {}", e)}),
            );
        }
    };

    let body = String::from_utf8_lossy(&body_bytes).to_string();

    Ok(Json(ProxyResponse { status, body }))
}
