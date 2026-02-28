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
//! use llm_client::LlmClient;
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = LlmClient::builder()
//!         .with_default_bigmodel_from_env()
//!         .expect("failed to create client")
//!         .build()
//!         .expect("failed to build client");
//!
//!     let request = llm_client::LlmRequest {
//!         model: "glm-5".to_string(),
//!         messages: vec![llm_client::LlmMessage {
//!             role: llm_client::LlmRole::User,
//!             content: "Hello!".to_string(),
//!         }],
//!         stream: false,
//!         max_tokens: None,
//!         temperature: None,
//!         top_p: None,
//!     };
//!
//!     let response = client.chat(request).await.unwrap();
//!     println!("{:?}", response);
//! }
//! ```

pub mod adapter;
pub mod client;
pub mod config;
pub mod error;
pub mod mapping;
pub(crate) mod providers;
pub mod retry;
pub mod sse;
pub mod types;

pub use adapter::{AdapterId, ProviderAdapter};
pub use client::{LlmClient, LlmClientBuilder};

pub use config::{RetryOn, RetryPolicy, TimeoutConfig};
pub use error::LlmError;
pub use retry::run_with_retry;

pub use types::{
    LlmChunk, LlmChunkStream, LlmMessage, LlmRequest, LlmResponse, LlmRole, LlmUsage,
};
