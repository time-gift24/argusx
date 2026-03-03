// Error types for cookie-gateway
use thiserror::Error;
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};

#[derive(Debug, Error)]
pub enum CookieGatewayError {
    #[error("Domain not whitelisted: {domain}")]
    DomainNotWhitelisted { domain: String },

    #[error("User has not opted in")]
    UserNotOptedIn,

    #[error("Invalid URL format: {url}")]
    InvalidUrl { url: String },

    #[error("No cookies found for domain: {domain}")]
    NoCookiesFound { domain: String },

    #[error("HTTP request failed: {message}")]
    HttpRequestFailed { message: String },

    #[error("Failed to parse response: {message}")]
    ResponseParseError { message: String },
}

impl IntoResponse for CookieGatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            CookieGatewayError::DomainNotWhitelisted { domain } => {
                (StatusCode::FORBIDDEN, format!("Domain '{}' is not whitelisted", domain))
            }
            CookieGatewayError::UserNotOptedIn => {
                (StatusCode::FORBIDDEN, "User has not opted in to cookie capture".to_string())
            }
            CookieGatewayError::InvalidUrl { url } => {
                (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", url))
            }
            CookieGatewayError::NoCookiesFound { domain } => {
                (StatusCode::NOT_FOUND, format!("No cookies found for domain: {}", domain))
            }
            CookieGatewayError::HttpRequestFailed { message } => {
                (StatusCode::BAD_GATEWAY, format!("HTTP request failed: {}", message))
            }
            CookieGatewayError::ResponseParseError { message } => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse response: {}", message))
            }
        };

        let body = Json(serde_json::json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
