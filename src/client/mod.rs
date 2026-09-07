//! Client implementations for various booru sites.
//!
//! This module provides shared HTTP execution helpers used by the
//! provider clients.
//!
//! # Available Clients
//!
//! - `danbooru::Client` for danbooru.donmai.us, 2 tag limit
//! - `gelbooru::Client` for gelbooru.com, unlimited tags
//! - `safebooru::Client` for safebooru.org, unlimited tags, SFW only
//! - `rule34::Client` for api.rule34.xxx, unlimited tags
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "danbooru")]
//! use booru_rs::danbooru::Client;
//!
//! # #[cfg(feature = "danbooru")]
//! # async fn example() -> booru_rs::error::Result<()> {
//! let client = Client::new()?;
//! let posts = client
//!     .search()
//!     .tag("cat_ears")
//!     .limit(10)
//!     .send()
//!     .await?;
//!
//! // Get a specific post by ID
//! let post = client.post(12345).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Custom HTTP Client
//!
//! By default, all clients share a connection-pooled HTTP client.
//!
//! ```no_run
//! # #[cfg(feature = "safebooru")]
//! use booru_rs::safebooru::Client;
//!
//! # #[cfg(feature = "safebooru")]
//! # async fn example() -> booru_rs::error::Result<()> {
//! let custom_client = reqwest::Client::builder()
//!     .timeout(std::time::Duration::from_secs(60))
//!     .build()
//!     .unwrap();
//!
//! let client = Client::builder().http_client(custom_client).build()?;
//! let posts = client.search().tag("nature").send().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::LazyLock;
use std::time::Duration;

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
use crate::error::BooruError;
use crate::error::Result;
use crate::model::Post;
use crate::ratelimit::RateLimiter;
use crate::retry::RetryConfig;
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
use crate::retry::is_retryable;
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
use reqwest::header::HeaderMap;

#[cfg(any(feature = "danbooru", feature = "gelbooru", feature = "rule34"))]
#[derive(Clone, Default)]
pub(crate) struct Secret(String);

#[cfg(any(feature = "danbooru", feature = "gelbooru", feature = "rule34"))]
impl Secret {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

#[cfg(any(feature = "danbooru", feature = "gelbooru", feature = "rule34"))]
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[cfg(feature = "danbooru")]
pub mod danbooru;
#[cfg(feature = "gelbooru")]
pub mod gelbooru;
pub mod generic;
#[cfg(feature = "rule34")]
pub mod rule34;
#[cfg(feature = "safebooru")]
pub mod safebooru;
pub mod stream;

/// Result of one page fetched through the provider operation interface.
#[derive(Debug, Clone)]
pub struct PageResult<P, C> {
    /// Posts returned by the provider, in wire order.
    pub posts: Vec<P>,
    /// Continuation for the next page, if one exists.
    pub next: Option<C>,
}

/// Operation interface for generic provider callers and external adapters.
///
/// Provider clients retain their fluent, provider-specific inherent methods.
/// Implementations of this trait expose the small common seam needed by
/// generic pagination code without requiring access to client internals.
///
/// The returned futures are `Send` so generic streams stay `Send` and remain
/// usable inside spawned tasks.
pub trait Client {
    /// Owned query accepted by this provider.
    type Query: Clone;
    /// Provider-specific post type.
    type Post: Post;
    /// Provider-specific continuation for a subsequent page.
    type Continuation: Clone;

    /// Fetches one page and returns its continuation.
    fn page(
        &self,
        query: Self::Query,
        continuation: Option<Self::Continuation>,
    ) -> impl std::future::Future<Output = Result<PageResult<Self::Post, Self::Continuation>>> + Send;

    /// Fetches one post by ID.
    fn post(&self, id: u32) -> impl std::future::Future<Output = Result<Self::Post>> + Send;
}

/// Shared builder interface for generic provider callers.
///
/// Each provider keeps its inherent `ClientBuilder` with the same fluent
/// methods. Implementations of this trait expose the common subset so generic
/// code can configure any provider without naming it. Credential methods stay
/// provider specific and are not part of this trait.
///
/// # Example
///
/// ```no_run
/// use booru_rs::client::Builder;
///
/// fn build<B: Builder>(builder: B) -> booru_rs::Result<B::Client> {
///     builder.build()
/// }
/// ```
pub trait Builder: Default + Sized {
    /// Provider client produced by [`Builder::build`].
    type Client: Client;

    /// Sets a custom API endpoint after validating it.
    ///
    /// # Errors
    ///
    /// Returns [`BooruError::InvalidUrl`] for blank, unparsable, or
    /// non-HTTP(S) endpoints.
    fn endpoint(self, url: impl Into<String>) -> Result<Self>;

    /// Uses a custom HTTP client instead of the shared one.
    fn http_client(self, client: reqwest::Client) -> Self;

    /// Sets the request retry and rate-limit policy.
    fn request_policy(self, policy: RequestPolicy) -> Self;

    /// Sets the retry configuration for requests made by this client.
    fn retry_config(self, config: RetryConfig) -> Result<Self>;

    /// Sets the rate limiter for requests made by this client.
    fn rate_limiter(self, limiter: RateLimiter) -> Self;

    /// Builds the configured client.
    fn build(self) -> Result<Self::Client>;
}

#[cfg(feature = "danbooru")]
impl Builder for danbooru::ClientBuilder {
    type Client = danbooru::Client;

    fn endpoint(self, url: impl Into<String>) -> Result<Self> {
        self.endpoint(url)
    }

    fn http_client(self, client: reqwest::Client) -> Self {
        self.http_client(client)
    }

    fn request_policy(self, policy: RequestPolicy) -> Self {
        self.request_policy(policy)
    }

    fn retry_config(self, config: RetryConfig) -> Result<Self> {
        self.retry_config(config)
    }

    fn rate_limiter(self, limiter: RateLimiter) -> Self {
        self.rate_limiter(limiter)
    }

    fn build(self) -> Result<Self::Client> {
        self.build()
    }
}

#[cfg(feature = "gelbooru")]
impl Builder for gelbooru::ClientBuilder {
    type Client = gelbooru::Client;

    fn endpoint(self, url: impl Into<String>) -> Result<Self> {
        self.endpoint(url)
    }

    fn http_client(self, client: reqwest::Client) -> Self {
        self.http_client(client)
    }

    fn request_policy(self, policy: RequestPolicy) -> Self {
        self.request_policy(policy)
    }

    fn retry_config(self, config: RetryConfig) -> Result<Self> {
        self.retry_config(config)
    }

    fn rate_limiter(self, limiter: RateLimiter) -> Self {
        self.rate_limiter(limiter)
    }

    fn build(self) -> Result<Self::Client> {
        self.build()
    }
}

#[cfg(feature = "rule34")]
impl Builder for rule34::ClientBuilder {
    type Client = rule34::Client;

    fn endpoint(self, url: impl Into<String>) -> Result<Self> {
        self.endpoint(url)
    }

    fn http_client(self, client: reqwest::Client) -> Self {
        self.http_client(client)
    }

    fn request_policy(self, policy: RequestPolicy) -> Self {
        self.request_policy(policy)
    }

    fn retry_config(self, config: RetryConfig) -> Result<Self> {
        self.retry_config(config)
    }

    fn rate_limiter(self, limiter: RateLimiter) -> Self {
        self.rate_limiter(limiter)
    }

    fn build(self) -> Result<Self::Client> {
        self.build()
    }
}

#[cfg(feature = "safebooru")]
impl Builder for safebooru::ClientBuilder {
    type Client = safebooru::Client;

    fn endpoint(self, url: impl Into<String>) -> Result<Self> {
        self.endpoint(url)
    }

    fn http_client(self, client: reqwest::Client) -> Self {
        self.http_client(client)
    }

    fn request_policy(self, policy: RequestPolicy) -> Self {
        self.request_policy(policy)
    }

    fn retry_config(self, config: RetryConfig) -> Result<Self> {
        self.retry_config(config)
    }

    fn rate_limiter(self, limiter: RateLimiter) -> Self {
        self.rate_limiter(limiter)
    }

    fn build(self) -> Result<Self::Client> {
        self.build()
    }
}

/// Shared HTTP client with connection pooling and timeouts.
static SHARED_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
});

/// Returns a reference to the shared HTTP client.
#[inline]
pub fn shared_client() -> &'static reqwest::Client {
    &SHARED_CLIENT
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) fn validate_endpoint(url: &str) -> Result<String> {
    let trimmed = url.trim_end_matches('/');
    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| BooruError::InvalidUrl(url.to_string()))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(BooruError::InvalidUrl(url.to_string()));
    }
    Ok(trimmed.to_string())
}

#[cfg(any(feature = "gelbooru", feature = "rule34"))]
pub(crate) fn dapi_url(endpoint: &str) -> String {
    format!("{endpoint}/index.php")
}

#[cfg(any(feature = "gelbooru", feature = "rule34"))]
pub(crate) fn dapi_credentials<'a>(
    key: &'a Option<Secret>,
    user: &'a Option<Secret>,
) -> Option<(&'a Secret, &'a Secret)> {
    match (key, user) {
        (Some(key), Some(user)) => Some((key, user)),
        _ => None,
    }
}

#[cfg(any(feature = "gelbooru", feature = "rule34"))]
pub(crate) fn dapi_query(
    params: &[(&'static str, String)],
    credentials: Option<(&Secret, &Secret)>,
) -> Vec<(String, String)> {
    let mut query: Vec<(String, String)> = vec![
        ("page".to_string(), "dapi".to_string()),
        ("s".to_string(), "post".to_string()),
        ("q".to_string(), "index".to_string()),
        ("json".to_string(), "1".to_string()),
    ];
    for (key, value) in params {
        query.push(((*key).to_string(), value.clone()));
    }
    if let Some((key, user)) = credentials {
        query.push(("api_key".to_string(), key.expose().to_string()));
        query.push(("user_id".to_string(), user.expose().to_string()));
    }
    query
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) fn validate_tags(tags: &[String]) -> Result<()> {
    for tag in tags {
        if tag.is_empty() {
            return Err(BooruError::InvalidTag {
                tag: tag.clone(),
                reason: "tag must not be empty".to_string(),
            });
        }
        if tag.chars().any(char::is_whitespace) {
            return Err(BooruError::InvalidTag {
                tag: tag.clone(),
                reason: "tag must not contain whitespace".to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) fn validate_raw_queries(
    expressions: &[String],
    has_rating: bool,
    has_sort: bool,
) -> Result<()> {
    if expressions
        .iter()
        .any(|expression| expression.trim().is_empty())
    {
        return Err(BooruError::InvalidQuery(
            "raw query expressions must not be empty".to_string(),
        ));
    }
    let contains_rating = expressions.iter().any(|expression| {
        expression
            .split_whitespace()
            .any(|term| term.trim_start_matches('-').starts_with("rating:"))
    });
    if has_rating && contains_rating {
        return Err(BooruError::InvalidQuery(
            "raw rating filters cannot be combined with rating()".to_string(),
        ));
    }
    let contains_sort = expressions.iter().any(|expression| {
        expression.split_whitespace().any(|term| {
            let term = term.trim_start_matches('-');
            term.starts_with("sort:") || term.starts_with("order:")
        })
    });
    if has_sort && contains_sort {
        return Err(BooruError::InvalidQuery(
            "raw sort filters cannot be combined with sort()".to_string(),
        ));
    }
    Ok(())
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) fn validate_random_conflict(
    tags: &[String],
    expressions: &[String],
    has_sort: bool,
) -> Result<()> {
    let random_count = tags
        .iter()
        .filter(|tag| {
            let term = tag.trim();
            term == "order:random" || term == "sort:random"
        })
        .count();
    if random_count > 1 {
        return Err(BooruError::InvalidQuery(
            "random() must not be repeated".to_string(),
        ));
    }
    if random_count == 1 {
        if has_sort {
            return Err(BooruError::InvalidQuery(
                "random() cannot be combined with sort()".to_string(),
            ));
        }
        let raw_contains_sort = expressions.iter().any(|expression| {
            expression.split_whitespace().any(|term| {
                let term = term.trim_start_matches('-');
                term.starts_with("sort:") || term.starts_with("order:")
            })
        });
        if raw_contains_sort {
            return Err(BooruError::InvalidQuery(
                "random() cannot be combined with raw sort filters".to_string(),
            ));
        }
    }
    Ok(())
}

/// Rejects an API response with an unsuccessful status before decoding.
///
/// Failures become [`BooruError::HttpStatus`] with a bounded body excerpt.
/// Callers map endpoint-specific statuses such as missing posts first.
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(BooruError::http_status(status, &body))
}

/// Request policies shared by provider clients.
#[derive(Debug, Clone)]
pub struct RequestPolicy {
    retry: RetryConfig,
    rate_limiter: Option<RateLimiter>,
}

impl Default for RequestPolicy {
    /// Disabled policy: no retries and no rate limiter.
    ///
    /// Enable retries with
    /// `RequestPolicy::new().with_retry_config(RetryConfig::default())`.
    fn default() -> Self {
        Self {
            retry: RetryConfig::no_retry(),
            rate_limiter: None,
        }
    }
}

impl RequestPolicy {
    /// Creates a disabled policy with no retries and no rate limiter.
    ///
    /// Enable retries with
    /// `RequestPolicy::new().with_retry_config(RetryConfig::default())`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the retry configuration after validating it.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Result<Self> {
        config.validate()?;
        self.retry = config;
        Ok(self)
    }

    /// Sets a shared rate limiter for outbound requests.
    #[must_use]
    pub fn with_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }
}

/// Executes a request under the configured limiter and retry policy.
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) async fn execute_with_policy<F, Fut>(
    policy: &RequestPolicy,
    mut operation: F,
) -> Result<reqwest::Response>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response>>,
{
    policy.retry.validate()?;
    let mut attempt = 0;

    loop {
        if let Some(limiter) = &policy.rate_limiter {
            limiter.acquire().await;
        }

        let (result, retry_after) = match operation().await {
            Ok(response) => {
                let retry_after = parse_retry_after(response.headers());
                (ensure_success(response).await, retry_after)
            }
            Err(error) => (Err(error), None),
        };

        match result {
            Ok(response) => return Ok(response),
            Err(error) => {
                if attempt >= policy.retry.max_retries || !is_retryable(&error) {
                    return Err(error);
                }
                attempt += 1;
                let delay = retry_after
                    .unwrap_or_else(|| policy.retry.delay_for_attempt(attempt))
                    .min(policy.retry.max_delay);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    let seconds = value.trim().parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds))
}

#[cfg(test)]
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
mod tests {
    use super::{parse_retry_after, validate_random_conflict};
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
    use std::time::Duration;

    #[test]
    fn parses_numeric_retry_after() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("3"));

        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(3)));
    }

    #[test]
    fn ignores_non_numeric_retry_after() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("tomorrow"));

        assert_eq!(parse_retry_after(&headers), None);
    }

    #[test]
    fn random_rejects_sort_and_repeats() {
        assert!(validate_random_conflict(&["sort:random".to_string()], &[], false).is_ok());
        assert!(validate_random_conflict(&["order:random".to_string()], &[], false).is_ok());
        assert!(validate_random_conflict(&[], &[], false).is_ok());
        assert!(validate_random_conflict(&["sort:random".to_string()], &[], true).is_err());
        assert!(
            validate_random_conflict(
                &["sort:random".to_string()],
                &["sort:score".to_string()],
                false
            )
            .is_err()
        );
        assert!(
            validate_random_conflict(
                &["sort:random".to_string(), "sort:random".to_string()],
                &[],
                false
            )
            .is_err()
        );
    }
}
