use std::time::Duration;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_attempts: u64,
    /// Base delay for exponential backoff.
    pub base_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
    /// Conditions under which to retry.
    pub retry_on: RetryOn,
}

/// Conditions for retrying requests.
#[derive(Debug, Clone, Copy)]
pub struct RetryOn {
    /// Retry on 429 (rate limit) errors.
    pub retry_429: bool,
    /// Retry on 5xx server errors.
    pub retry_5xx: bool,
    /// Retry on network errors.
    pub retry_network: bool,
}

/// Configuration for timeouts.
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for establishing connection.
    pub connect_timeout: Duration,
    /// Timeout for complete request (non-streaming).
    pub request_timeout: Duration,
    /// Timeout for no data received on stream.
    pub stream_idle_timeout: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(30),
            retry_on: RetryOn::default(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(120),
            stream_idle_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for RetryOn {
    fn default() -> Self {
        Self {
            retry_429: true,
            retry_5xx: true,
            retry_network: true,
        }
    }
}

impl RetryPolicy {
    /// Set maximum retry attempts.
    pub fn max_attempts(mut self, n: u64) -> Self {
        self.max_attempts = n;
        self
    }

    /// Set base delay for backoff.
    pub fn base_delay(mut self, d: Duration) -> Self {
        self.base_delay = d;
        self
    }

    /// Set maximum delay cap.
    pub fn max_delay(mut self, d: Duration) -> Self {
        self.max_delay = d;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_secs(2));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!(policy.retry_on.retry_429);
        assert!(policy.retry_on.retry_5xx);
        assert!(policy.retry_on.retry_network);
    }

    #[test]
    fn default_timeout_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(120));
        assert_eq!(config.stream_idle_timeout, Duration::from_secs(30));
    }

    #[test]
    fn retry_policy_builder() {
        let policy = RetryPolicy::default()
            .max_attempts(5)
            .base_delay(Duration::from_secs(1));

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.base_delay, Duration::from_secs(1));
    }
}
