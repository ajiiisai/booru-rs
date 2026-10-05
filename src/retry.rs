//! # Retry failed operations
//!
//! Provider clients use no retries by default.
//! Enable retries with the provider builder's `retry_config()` method or [`crate::RequestPolicy::with_retry_config`].
//!
//! [`RetryConfig::default()`] permits three retries after the first attempt.
//! The delay grows with exponential backoff, up to `max_delay`.
//! Provider clients use a valid `Retry-After` response header instead of the computed delay.
//! That header can specify a delay above `max_delay`.
//!
//! Use [`is_retryable`] to classify an error.
//! Use [`with_retry`] for an application operation that returns [`crate::Result`].
//! This helper uses the configured backoff and has no access to HTTP response headers.

use std::future::Future;
use std::time::Duration;

use crate::error::{BooruError, Result};

/// Default retry configuration.
pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 100;
pub const DEFAULT_MAX_DELAY_MS: u64 = 5000;

/// Configuration for retry behavior.
///
/// Use constructors and `with_*` methods instead of struct literals.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_retries: u32,
    /// Initial delay before the first retry.
    pub initial_delay: Duration,
    /// Maximum computed backoff delay. Provider clients can use a longer
    /// delay from a `Retry-After` response header.
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

    /// Validates the retry configuration.
    pub fn validate(&self) -> Result<()> {
        if !self.backoff_factor.is_finite() || self.backoff_factor < 0.0 {
            return Err(BooruError::InvalidRetryConfig(
                "backoff factor must be finite and non-negative".to_string(),
            ));
        }
        Ok(())
    }

    /// Calculates the delay for a given attempt number.
    pub(crate) fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_factor.powi(attempt.saturating_sub(1) as i32);
        let delay = Duration::from_millis(delay_ms as u64);

        delay.min(self.max_delay)
    }
}

/// Returns whether an error permits a retry.
///
/// Retryable errors are:
///
/// - Request timeouts, connection errors, body errors, request errors, or HTTP 5xx statuses.
/// - [`BooruError::RateLimited`].
/// - [`BooruError::HttpStatus`] with status 429 or 500 through 599.
///
/// Provider context delegates to the underlying error.
/// Parse errors, authentication errors, missing posts, and invalid queries
/// do not permit a retry.
pub fn is_retryable(error: &BooruError) -> bool {
    match error {
        BooruError::Context { source, .. } => is_retryable(source),
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
            e.is_request() || e.is_body() || (e.is_decode() && !error.is_parse_error())
        }
        // Don't retry parse errors, auth errors, or not found
        BooruError::Parse(_) => false,
        BooruError::TagLimitExceeded { .. } => false,
        BooruError::PostNotFound(_) => false,
        BooruError::EmptyResponse => false,
        BooruError::InvalidUrl(_) => false,
        BooruError::Unauthorized(_) => false,
        BooruError::InvalidTag { .. } => false,
        BooruError::InvalidQuery(_) => false,
        BooruError::RateLimited => true, // Rate limit errors can be retried after waiting
        BooruError::HttpStatus { status, .. } => *status == 429 || (500..600).contains(status),
        BooruError::Io(_) => false, // I/O errors are generally not retryable
        BooruError::InvalidConcurrency => false,
        BooruError::InvalidFilename(_) => false,
        BooruError::MissingMediaUrl(_) => false,
        BooruError::UnexpectedDownloadContentType(_) => false,
        BooruError::Md5Mismatch { .. } => false,
        BooruError::DownloadTaskFailed(_) => false,
        BooruError::DestinationConflict(_) => false,
        BooruError::InvalidRetryConfig(_) => false,
        BooruError::InvalidRateLimitConfig(_) => false,
    }
}

/// Repeats an operation after a retryable failure, up to `max_retries` times.
///
/// The first attempt does not count as a retry.
/// Each retry uses the delay from `config`. This helper does not inspect `Retry-After` headers.
///
/// # Example
///
/// ```
/// use booru_rs::{BooruError, retry::{RetryConfig, with_retry}};
/// use std::{cell::Cell, time::Duration};
///
/// #[tokio::main]
/// async fn main() -> booru_rs::Result<()> {
///     let attempts = Cell::new(0);
///     let config = RetryConfig::new(1).with_initial_delay(Duration::ZERO);
///     let value = with_retry(config, || async {
///         attempts.set(attempts.get() + 1);
///         if attempts.get() == 1 {
///             Err(BooruError::RateLimited)
///         } else {
///             Ok(42)
///         }
///     }).await?;
///
///     assert_eq!(value, 42);
///     assert_eq!(attempts.get(), 2);
///     Ok(())
/// }
/// ```
pub async fn with_retry<F, Fut, T>(config: RetryConfig, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    config.validate()?;

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

    #[tokio::test]
    async fn invalid_backoff_is_rejected_before_operation() {
        let mut calls = 0;
        let config = RetryConfig::default().with_backoff_factor(f64::NAN);

        let result = with_retry(config, || {
            calls += 1;
            async { Ok::<_, BooruError>(()) }
        })
        .await;

        assert!(matches!(result, Err(BooruError::InvalidRetryConfig(_))));
        assert_eq!(calls, 0);
    }

    #[tokio::test]
    async fn cancellation_interrupts_retry_backoff() {
        let config = RetryConfig::new(1)
            .with_initial_delay(Duration::from_secs(60))
            .with_max_delay(Duration::from_secs(60));
        let task = tokio::spawn(with_retry(config, || async {
            Err::<(), _>(BooruError::RateLimited)
        }));

        tokio::time::sleep(Duration::from_millis(10)).await;
        task.abort();

        assert!(
            task.await
                .expect_err("retry task must be cancelled")
                .is_cancelled()
        );
    }
}
