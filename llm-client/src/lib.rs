//! Robust LLM HTTP client with retry and SSE support.
//!
//! This crate provides:
//! - Exponential backoff retry with jitter
//! - Detailed error types (retryable vs non-retryable)
//! - SSE streaming with idle timeout detection
//! - Configurable timeouts
//!
//! # Example
//!
//! ```no_run
//! use llm_client::providers::{BigModelConfig, BigModelHttpClient};
//! use llm_client::{RetryPolicy, TimeoutConfig};
//! use bigmodel_api::{ChatRequest, Message};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = BigModelConfig {
//!         api_key: "your-api-key".to_string(),
//!         ..Default::default()
//!     };
//!
//!     let client = BigModelHttpClient::new(config);
//!
//!     let request = ChatRequest::new("glm-5", vec![
//!         Message::user("Hello!")
//!     ]);
//!
//!     let response = client.chat(request).await.unwrap();
//!     println!("{:?}", response);
//! }
//! ```

pub mod config;
pub mod error;
pub mod retry;
pub mod sse;
pub mod providers;

pub use config::{RetryPolicy, RetryOn, TimeoutConfig};
pub use error::LlmError;
pub use retry::run_with_retry;
