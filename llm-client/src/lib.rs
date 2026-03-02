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
//! ```ignore
//! use llm_client::LlmClient;
//! use llm_client::ProviderAdapter;
//! use std::sync::Arc;
//!
//! # async fn build_client(adapter: Arc<dyn ProviderAdapter>) -> llm_client::LlmClient {
//!     let client = LlmClient::builder()
//!         .register_adapter(adapter)
//!         .default_adapter("my-provider")
//!         .build()
//!         .expect("failed to build client");
//!
//!     client
//! # }
//! ```

pub mod adapter;
pub mod client;
pub mod config;
pub mod error;
pub mod retry;
pub mod sse;
pub mod types;

pub use adapter::{AdapterId, ProviderAdapter};
pub use client::{LlmClient, LlmClientBuilder};

pub use config::{RetryOn, RetryPolicy, TimeoutConfig};
pub use error::LlmError;
pub use retry::run_with_retry;

pub use types::{
    LlmChunk, LlmChunkStream, LlmMessage, LlmRequest, LlmResponse, LlmRole, LlmTool,
    LlmToolCall, LlmUsage,
};
