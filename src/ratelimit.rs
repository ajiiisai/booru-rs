//! Rate limiting for API requests.
//!
//! This module provides rate limiting functionality to prevent overwhelming
//! booru APIs and getting blocked.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::ratelimit::RateLimiter;
//! use std::time::Duration;
//!
//! # async fn example() -> booru_rs::error::Result<()> {
//! // Create a limiter allowing 2 requests per second
//! let limiter = RateLimiter::new(2, Duration::from_secs(1))?;
//!
//! // This will wait if necessary to respect the rate limit
//! limiter.acquire().await;
//! // ... make request ...
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::error::{BooruError, Result};

/// A token bucket rate limiter for controlling API request rates.
///
/// This limiter uses a token bucket algorithm where tokens are replenished
/// at a fixed rate. Each request consumes one token.
///
/// # Thread Safety
///
/// `RateLimiter` is `Send`, `Sync`, and `Clone`, making it safe to share
/// across tasks and threads.
///
/// # Example
///
/// ```no_run
/// use booru_rs::ratelimit::RateLimiter;
/// use std::time::Duration;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// // Allow 5 requests per 2 seconds
/// let limiter = RateLimiter::new(5, Duration::from_secs(2))?;
///
/// for _ in 0..10 {
///     limiter.acquire().await;
///     println!("Making request...");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    config: RateLimiterConfig,
}

#[derive(Clone, Copy)]
struct RateLimiterConfig {
    /// Maximum tokens in the bucket.
    capacity: u32,
    /// How long it takes to refill the entire bucket.
    refill_interval: Duration,
}

struct RateLimiterState {
    /// Current number of available tokens.
    tokens: f64,
    /// When we last updated the token count.
    last_update: Instant,
}

impl RateLimiter {
    /// Creates a new rate limiter.
    ///
    /// # Arguments
    ///
    /// * `requests` - Maximum number of requests allowed per interval
    /// * `per_interval` - The time window for the request limit
    ///
    /// # Example
    ///
    /// ```
    /// use booru_rs::ratelimit::RateLimiter;
    /// use std::time::Duration;
    ///
    /// // 10 requests per second
    /// let limiter = RateLimiter::new(10, Duration::from_secs(1)).expect("valid rate limit");
    ///
    /// // 100 requests per minute
    /// let limiter = RateLimiter::new(100, Duration::from_secs(60)).expect("valid rate limit");
    /// ```
    #[must_use = "handle the construction error or use the limiter"]
    pub fn new(requests: u32, per_interval: Duration) -> Result<Self> {
        if requests == 0 {
            return Err(BooruError::InvalidRateLimitConfig(
                "requests must be greater than zero".to_string(),
            ));
        }
        if per_interval.is_zero() {
            return Err(BooruError::InvalidRateLimitConfig(
                "refill interval must be greater than zero".to_string(),
            ));
        }

        Ok(Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                tokens: requests as f64,
                last_update: Instant::now(),
            })),
            config: RateLimiterConfig {
                capacity: requests,
                refill_interval: per_interval,
            },
        })
    }

    /// Creates a rate limiter suitable for most booru APIs.
    ///
    /// This uses conservative defaults (2 requests/second) that should
    /// work with any booru without getting blocked.
    #[must_use = "handle the construction error or use the limiter"]
    pub fn default_booru() -> Result<Self> {
        Self::new(2, Duration::from_secs(1))
    }

    /// Acquires a token, waiting if necessary.
    ///
    /// This method will block (asynchronously) until a token is available,
    /// ensuring that the rate limit is respected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::ratelimit::RateLimiter;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let limiter = RateLimiter::new(1, Duration::from_secs(1))?;
    ///
    /// // First request goes through immediately
    /// limiter.acquire().await;
    ///
    /// // Second request waits ~1 second
    /// limiter.acquire().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn acquire(&self) {
        loop {
            let wait_time = {
                let mut state = self.state.lock().await;
                self.refill_tokens(&mut state);

                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    return;
                }

                // Calculate how long until we have 1 token
                let tokens_needed = 1.0 - state.tokens;
                let refill_rate =
                    self.config.capacity as f64 / self.config.refill_interval.as_secs_f64();
                Duration::from_secs_f64(tokens_needed / refill_rate)
            };

            tokio::time::sleep(wait_time).await;
        }
    }

    /// Tries to acquire a token without waiting.
    ///
    /// Returns `true` if a token was acquired, `false` if rate limit exceeded.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::ratelimit::RateLimiter;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let limiter = RateLimiter::new(1, Duration::from_secs(1))?;
    ///
    /// if limiter.try_acquire().await {
    ///     println!("Request allowed");
    /// } else {
    ///     println!("Rate limited, try again later");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn try_acquire(&self) -> bool {
        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Returns the current number of available tokens.
    pub async fn available(&self) -> u32 {
        let mut state = self.state.lock().await;
        self.refill_tokens(&mut state);
        state.tokens as u32
    }

    fn refill_tokens(&self, state: &mut RateLimiterState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_update);

        if elapsed > Duration::ZERO {
            let refill_rate =
                self.config.capacity as f64 / self.config.refill_interval.as_secs_f64();
            let new_tokens = elapsed.as_secs_f64() * refill_rate;

            state.tokens = (state.tokens + new_tokens).min(self.config.capacity as f64);
            state.last_update = now;
        }
    }
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter")
            .field("capacity", &self.config.capacity)
            .field("refill_interval", &self.config.refill_interval)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_try_acquire() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1)).unwrap();

        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn clones_share_token_state() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1)).unwrap();
        let clone = limiter.clone();

        assert!(limiter.try_acquire().await);
        assert!(clone.try_acquire().await);
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_available() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1)).unwrap();

        assert_eq!(limiter.available().await, 5);
        limiter.acquire().await;
        assert_eq!(limiter.available().await, 4);
    }

    #[tokio::test]
    async fn test_refill() {
        let limiter = RateLimiter::new(10, Duration::from_millis(100)).unwrap();

        // Drain all tokens
        for _ in 0..10 {
            limiter.try_acquire().await;
        }
        assert_eq!(limiter.available().await, 0);

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(limiter.available().await >= 10);
    }

    #[test]
    fn rejects_zero_requests() {
        let result = RateLimiter::new(0, Duration::from_secs(1));

        assert!(matches!(
            result,
            Err(BooruError::InvalidRateLimitConfig(message))
                if message == "requests must be greater than zero"
        ));
    }

    #[test]
    fn rejects_zero_refill_interval() {
        let result = RateLimiter::new(1, Duration::ZERO);

        assert!(matches!(
            result,
            Err(BooruError::InvalidRateLimitConfig(message))
                if message == "refill interval must be greater than zero"
        ));
    }
}
