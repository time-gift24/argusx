use crate::config::RetryPolicy;
use crate::error::LlmError;
use futures::future::BoxFuture;
use rand::Rng;
use std::time::Duration;
use tracing::warn;

/// Run an async operation with retry logic.
pub async fn run_with_retry<T>(
    policy: RetryPolicy,
    mut make_request: impl FnMut() -> BoxFuture<'static, Result<T, LlmError>>,
) -> Result<T, LlmError> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match make_request().await {
            Ok(result) => return Ok(result),
            Err(err) if should_retry(&err, &policy, attempt) => {
                let delay = backoff(policy.base_delay, attempt, &err, policy.max_delay);
                warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "Retrying request"
                );
                tokio::time::sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}

fn should_retry(err: &LlmError, policy: &RetryPolicy, attempt: u64) -> bool {
    if attempt >= policy.max_attempts {
        return false;
    }

    if !err.is_retryable() {
        return false;
    }

    // Check specific retry conditions
    match err {
        LlmError::RateLimit { .. } => policy.retry_on.retry_429,
        LlmError::ServerError { status, .. } => {
            policy.retry_on.retry_5xx && *status >= 500 && *status < 600
        }
        LlmError::NetworkError { .. } | LlmError::Timeout | LlmError::StreamIdleTimeout => {
            policy.retry_on.retry_network
        }
        _ => false,
    }
}

/// Calculate backoff duration with exponential increase and jitter.
pub fn backoff(
    base: Duration,
    attempt: u64,
    err: &LlmError,
    max_delay: Duration,
) -> Duration {
    // Prefer retry-after header if available
    if let Some(retry_after) = err.retry_after() {
        return retry_after.min(max_delay);
    }

    // Exponential backoff: base * 2^(attempt-1) * jitter
    let exp = 2u64.saturating_pow(attempt.saturating_sub(1) as u32);
    let jitter = rand::thread_rng().gen_range(0.9..1.1);
    let delay = Duration::from_millis(
        (base.as_millis() as f64 * exp as f64 * jitter) as u64
    );

    delay.min(max_delay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LlmError;
    use crate::config::RetryPolicy;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn backoff_increases_exponentially() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);
        let err = LlmError::ServerError {
            status: 500,
            message: "test".to_string(),
        };

        // Attempt 1: 1 * 1 * jitter ≈ 1s
        let d1 = backoff(base, 1, &err, max);
        assert!(d1 >= Duration::from_millis(900));
        assert!(d1 <= Duration::from_millis(1100));

        // Attempt 2: 1 * 2 * jitter ≈ 2s
        let d2 = backoff(base, 2, &err, max);
        assert!(d2 >= Duration::from_millis(1800));
        assert!(d2 <= Duration::from_millis(2200));

        // Attempt 3: 1 * 4 * jitter ≈ 4s
        let d3 = backoff(base, 3, &err, max);
        assert!(d3 >= Duration::from_millis(3600));
        assert!(d3 <= Duration::from_millis(4400));
    }

    #[test]
    fn backoff_respects_retry_after() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);
        let err = LlmError::RateLimit {
            message: "test".to_string(),
            retry_after: Some(Duration::from_secs(10)),
        };

        let d = backoff(base, 1, &err, max);
        assert_eq!(d, Duration::from_secs(10));
    }

    #[test]
    fn backoff_respects_max_delay() {
        let base = Duration::from_secs(10);
        let max = Duration::from_secs(30);
        let err = LlmError::ServerError {
            status: 500,
            message: "test".to_string(),
        };

        // 10 * 4 = 40, but capped at 30
        let d = backoff(base, 3, &err, max);
        assert!(d <= max);
    }

    #[tokio::test]
    async fn run_with_retry_succeeds_after_failures() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = run_with_retry(RetryPolicy::default(), move || {
            let attempts = attempts_clone.clone();
            Box::pin(async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(LlmError::ServerError {
                        status: 500,
                        message: "temporary".to_string(),
                    })
                } else {
                    Ok("success")
                }
            })
        })
        .await;

        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn run_with_retry_fails_after_max_attempts() {
        let policy = RetryPolicy::default().max_attempts(2);
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<(), _> = run_with_retry(policy, move || {
            let attempts = attempts_clone.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(LlmError::ServerError {
                    status: 500,
                    message: "always fails".to_string(),
                })
            })
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn run_with_retry_fails_immediately_for_non_retryable() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<(), _> = run_with_retry(RetryPolicy::default(), move || {
            let attempts = attempts_clone.clone();
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(LlmError::AuthError {
                    message: "invalid key".to_string(),
                })
            })
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1); // Only one attempt
    }
}
