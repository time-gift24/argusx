// Adapted from reqwest-eventsource v0.6.0 (MIT OR Apache-2.0).
// Local modifications:
// - Default retry policy is `Never` for LLM streaming safety.

use crate::error::Error;
use std::time::Duration;

/// Describes how an `EventSource` should retry on receiving an `Error`.
pub trait RetryPolicy {
    /// Return a retry delay or `None` to stop retrying.
    fn retry(&self, error: &Error, last_retry: Option<(usize, Duration)>) -> Option<Duration>;

    /// Update reconnection time using `retry:` field from SSE event.
    fn set_reconnection_time(&mut self, duration: Duration);
}

/// Exponential backoff retry policy.
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    pub start: Duration,
    pub factor: f64,
    pub max_duration: Option<Duration>,
    pub max_retries: Option<usize>,
}

impl ExponentialBackoff {
    pub const fn new(
        start: Duration,
        factor: f64,
        max_duration: Option<Duration>,
        max_retries: Option<usize>,
    ) -> Self {
        Self {
            start,
            factor,
            max_duration,
            max_retries,
        }
    }
}

impl RetryPolicy for ExponentialBackoff {
    fn retry(&self, _error: &Error, last_retry: Option<(usize, Duration)>) -> Option<Duration> {
        if let Some((retry_num, last_duration)) = last_retry {
            if self.max_retries.is_none() || retry_num < self.max_retries.unwrap() {
                let duration = last_duration.mul_f64(self.factor);
                if let Some(max_duration) = self.max_duration {
                    Some(duration.min(max_duration))
                } else {
                    Some(duration)
                }
            } else {
                None
            }
        } else {
            Some(self.start)
        }
    }

    fn set_reconnection_time(&mut self, duration: Duration) {
        self.start = duration;
        if let Some(max_duration) = self.max_duration {
            self.max_duration = Some(max_duration.max(duration));
        }
    }
}

/// Constant-delay retry policy.
#[derive(Debug, Clone)]
pub struct Constant {
    pub delay: Duration,
    pub max_retries: Option<usize>,
}

impl Constant {
    pub const fn new(delay: Duration, max_retries: Option<usize>) -> Self {
        Self { delay, max_retries }
    }
}

impl RetryPolicy for Constant {
    fn retry(&self, _error: &Error, last_retry: Option<(usize, Duration)>) -> Option<Duration> {
        if let Some((retry_num, _)) = last_retry {
            if self.max_retries.is_none() || retry_num < self.max_retries.unwrap() {
                Some(self.delay)
            } else {
                None
            }
        } else {
            Some(self.delay)
        }
    }

    fn set_reconnection_time(&mut self, duration: Duration) {
        self.delay = duration;
    }
}

/// Never retry.
#[derive(Debug, Clone, Copy, Default)]
pub struct Never;

impl RetryPolicy for Never {
    fn retry(&self, _error: &Error, _last_retry: Option<(usize, Duration)>) -> Option<Duration> {
        None
    }

    fn set_reconnection_time(&mut self, _duration: Duration) {}
}

/// Default retry policy for llm-client SSE.
pub const DEFAULT_RETRY: Never = Never;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff_respects_max_retries() {
        let p = ExponentialBackoff::new(
            Duration::from_millis(100),
            2.0,
            Some(Duration::from_secs(10)),
            Some(2),
        );

        assert_eq!(
            p.retry(&Error::StreamEnded, None),
            Some(Duration::from_millis(100))
        );
        assert_eq!(
            p.retry(&Error::StreamEnded, Some((1, Duration::from_millis(100)))),
            Some(Duration::from_millis(200))
        );
        assert_eq!(
            p.retry(&Error::StreamEnded, Some((2, Duration::from_millis(200)))),
            None
        );
    }

    #[test]
    fn constant_retry_respects_max_retries() {
        let p = Constant::new(Duration::from_millis(50), Some(1));

        assert_eq!(
            p.retry(&Error::StreamEnded, None),
            Some(Duration::from_millis(50))
        );
        assert_eq!(
            p.retry(&Error::StreamEnded, Some((1, Duration::from_millis(50)))),
            None
        );
    }
}
