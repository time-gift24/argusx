// Error types for cookie-gateway
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CookieGatewayError {
    #[error("Domain not whitelisted: {0}")]
    DomainNotWhitelisted { domain: String },

    #[error("User has not opted in")]
    UserNotOptedIn,

    #[error("Invalid URL format")]
    InvalidUrl { url: String },

    #[error("No cookies found for domain")]
    NoCookiesFound { domain: String },

    #[error("HTTP request failed")]
    HttpRequestFailed { message: String },

    #[error("Failed to parse response")]
    ResponseParseError { message: String },
}

impl std::fmt::Display for CookieGatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookieGatewayError::DomainNotWhitelisted { domain } => {
                write!(f, "Domain '{}' is not whitelisted", domain)
            }
            CookieGatewayError::UserNotOptedIn => {
                write!(f, "User has not opted in to cookie capture")
            }
            CookieGatewayError::InvalidUrl { url } => {
                write!(f, "Invalid URL: {}", url)
            }
            CookieGatewayError::NoCookiesFound { domain } => {
                write!(f, "No cookies found for domain: {}", domain)
            }
            CookieGatewayError::HttpRequestFailed { message } => {
                write!(f, "HTTP request failed: {}", message)
            }
            CookieGatewayError::ResponseParseError { message } => {
                write!(f, "Failed to parse response: {}", message)
            }
        }
    }
}
