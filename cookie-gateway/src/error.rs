// Error types for cookie-gateway
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CookieGatewayError {
    #[error("Domain not whitelisted: {domain}")]
    DomainNotWhitelisted { domain: String },

    #[error("User has not opted in")]
    UserNotOptedIn,

    #[error("Invalid URL format: {url}")]
    InvalidUrl { url: String },

    #[error("Invalid domain: {domain}")]
    InvalidDomain { domain: String },

    #[error("No cookies found for domain: {domain}")]
    NoCookiesFound { domain: String },

    #[error("No active extension client is connected")]
    ExtensionClientUnavailable,

    #[error("Extension command timed out for domain '{domain}' after {timeout_ms}ms")]
    ExtensionCommandTimeout { domain: String, timeout_ms: u64 },

    #[error("Extension command failed: {message}")]
    ExtensionCommandFailed { message: String },

    #[error("HTTP request failed: {message}")]
    HttpRequestFailed { message: String },

    #[error("Failed to parse response: {message}")]
    ResponseParseError { message: String },
}

impl IntoResponse for CookieGatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            CookieGatewayError::DomainNotWhitelisted { domain } => (
                StatusCode::FORBIDDEN,
                format!("Domain '{domain}' is not whitelisted"),
            ),
            CookieGatewayError::UserNotOptedIn => (
                StatusCode::FORBIDDEN,
                "User has not opted in to cookie capture".to_string(),
            ),
            CookieGatewayError::InvalidUrl { url } => {
                (StatusCode::BAD_REQUEST, format!("Invalid URL: {url}"))
            }
            CookieGatewayError::InvalidDomain { domain } => {
                (StatusCode::BAD_REQUEST, format!("Invalid domain: {domain}"))
            }
            CookieGatewayError::NoCookiesFound { domain } => (
                StatusCode::NOT_FOUND,
                format!("No cookies found for domain: {domain}"),
            ),
            CookieGatewayError::ExtensionClientUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "No active extension client is connected".to_string(),
            ),
            CookieGatewayError::ExtensionCommandTimeout { domain, timeout_ms } => (
                StatusCode::GATEWAY_TIMEOUT,
                format!("Extension command timed out for domain '{domain}' after {timeout_ms}ms"),
            ),
            CookieGatewayError::ExtensionCommandFailed { message } => (
                StatusCode::BAD_GATEWAY,
                format!("Extension command failed: {message}"),
            ),
            CookieGatewayError::HttpRequestFailed { message } => (
                StatusCode::BAD_GATEWAY,
                format!("HTTP request failed: {message}"),
            ),
            CookieGatewayError::ResponseParseError { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {message}"),
            ),
        };

        let body = Json(serde_json::json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
