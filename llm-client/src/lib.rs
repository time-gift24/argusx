//! Robust LLM HTTP client with retry and SSE support.
//!
//! This crate provides:
//! - Exponential backoff retry with jitter
//! - Detailed error types (retryable vs non-retryable)
//! - SSE streaming with idle timeout detection
//! - Configurable timeouts

pub mod config;
pub mod error;
pub mod retry;
pub mod sse;
pub mod providers;

pub use config::{RetryPolicy, RetryOn, TimeoutConfig};
pub use error::LlmError;
pub use retry::run_with_retry;
