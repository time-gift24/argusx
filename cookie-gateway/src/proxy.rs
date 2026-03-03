// HTTP proxy module for cookie-gateway
use axum::{
    body::Body,
    extract::State,
    http::{Request, header},
    Json,
};
use hyper_util::client::legacy::Client;
use hyper_tls::HttpsConnector;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use crate::gateway::GatewayState;
use crate::error::CookieGatewayError;

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
    State(state): State<GatewayState>,
    Json(req): Json<ProxyRequest>,
) -> Result<Json<ProxyResponse>, CookieGatewayError> {
    let store = state.store.clone();

    // Parse URL to extract domain
    let url = match url::Url::parse(&req.url) {
        Ok(u) => u,
        Err(_) => {
            return Err(CookieGatewayError::InvalidUrl { url: req.url.clone() });
        }
    };

    let domain = match url.host_str() {
        Some(h) => h,
        None => {
            return Err(CookieGatewayError::InvalidUrl { url: req.url.clone() });
        }
    };

    // Check whitelist
    if !store.is_whitelisted(domain) {
        return Err(CookieGatewayError::DomainNotWhitelisted { domain: domain.to_string() });
    }

    // Check opt-in
    if !store.is_opted_in().await {
        return Err(CookieGatewayError::UserNotOptedIn);
    }

    // Get cookies for domain
    let cookies = store.get_cookies(domain).await;

    // Build HTTP client with TLS
    let https = HttpsConnector::new();
    let client: Client<_, Body> = Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(https);

    // Build request with empty body
    let mut request = Request::builder()
        .method("GET")
        .uri(req.url.as_str())
        .body(Body::empty())
        .map_err(|e| CookieGatewayError::HttpRequestFailed { message: e.to_string() })?;

    // Add cookies if available
    if let Some(cookies) = cookies {
        let cookie_header: String = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");
        request.headers_mut().insert(
            header::COOKIE,
            cookie_header.parse().unwrap(),
        );
    }

    request.headers_mut().insert(header::ACCEPT, "application/json".parse().unwrap());

    // Execute request
    let response = client.request(request).await
        .map_err(|e| CookieGatewayError::HttpRequestFailed { message: e.to_string() })?;

    // Extract response details
    let status = response.status().as_u16();
    let body_bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .map_err(|e| CookieGatewayError::ResponseParseError { message: e.to_string() })?
        .to_bytes();

    let body = String::from_utf8_lossy(&body_bytes).to_string();

    Ok(Json(ProxyResponse { status, body }))
}
