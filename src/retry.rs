//! Retry logic with exponential backoff for transient failures.
//!
//! This module provides utilities for automatically retrying failed requests
//! with exponential backoff delays.

use std::future::Future;
use std::time::Duration;

use crate::error::{BooruError, Result};

/// Default retry configuration.
pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 100;
pub const DEFAULT_MAX_DELAY_MS: u64 = 5000;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_retries: u32,
    /// Initial delay before the first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplier applied to delay after each retry (for exponential backoff).
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_delay: Duration::from_millis(DEFAULT_INITIAL_DELAY_MS),
            max_delay: Duration::from_millis(DEFAULT_MAX_DELAY_MS),
            backoff_factor: 2.0,
        }
    }
}

impl RetryConfig {
    /// Creates a new retry configuration with the specified max retries.
    #[must_use]
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Disables retries.
    #[must_use]
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Sets the initial delay.
    #[must_use]
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Sets the maximum delay.
    #[must_use]
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Sets the backoff multiplier.
    #[must_use]
    pub fn with_backoff_factor(mut self, factor: f64) -> Self {
        self.backoff_factor = factor;
        self
    }

    /// Calculates the delay for a given attempt number.
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_factor.powi(attempt.saturating_sub(1) as i32);
        let delay = Duration::from_millis(delay_ms as u64);

        delay.min(self.max_delay)
    }
}

/// Determines if an error is retryable.
///
/// Only transient network errors should be retried. Parse errors,
/// authentication errors, and not-found errors are not retryable.
pub fn is_retryable(error: &BooruError) -> bool {
    match error {
        BooruError::Request(e) => {
            // Retry on timeout, connection errors, but not on HTTP 4xx errors
            if e.is_timeout() || e.is_connect() {
                return true;
            }
            // Check for server errors (5xx) which are retryable
            if let Some(status) = e.status() {
                return status.is_server_error();
            }
            // Retry on other transient request errors
            e.is_request()
        }
        // Don't retry parse errors, auth errors, or not found
        BooruError::Parse(_) => false,
        BooruError::TagLimitExceeded { .. } => false,
        BooruError::PostNotFound(_) => false,
        BooruError::EmptyResponse => false,
        BooruError::InvalidUrl(_) => false,
        BooruError::Unauthorized(_) => false,
        BooruError::InvalidTag { .. } => false,
        BooruError::RateLimited => true, // Rate limit errors can be retried after waiting
        BooruError::Io(_) => false,      // I/O errors are generally not retryable
    }
}

/// Executes an async operation with retry logic.
///
/// # Example
///
/// ```ignore
/// use booru_rs::retry::{with_retry, RetryConfig};
///
/// let result = with_retry(RetryConfig::default(), || async {
///     // Your fallible async operation here
///     Ok(())
/// }).await;
/// ```
pub async fn with_retry<F, Fut, T>(config: RetryConfig, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut last_error;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = e;

                // Check if we should retry
                if attempt >= config.max_retries || !is_retryable(&last_error) {
                    return Err(last_error);
                }

                attempt += 1;

                // Calculate delay with exponential backoff
                let delay = config.delay_for_attempt(attempt);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig::default();

        assert_eq!(config.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn test_delay_max_cap() {
        let config = RetryConfig::default().with_max_delay(Duration::from_millis(150));

        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(150)); // Capped
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(150)); // Capped
    }
}
