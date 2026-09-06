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
//! use booru_rs::danbooru::Client;
//!
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
//! use booru_rs::safebooru::Client;
//!
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

use crate::error::{BooruError, Result};

#[cfg(feature = "danbooru")]
pub mod danbooru;
#[cfg(feature = "gelbooru")]
pub mod gelbooru;
pub mod generic;
#[cfg(feature = "rule34")]
pub mod rule34;
#[cfg(feature = "safebooru")]
pub mod safebooru;

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

pub(crate) fn validate_endpoint(url: &str) -> Result<String> {
    let trimmed = url.trim_end_matches('/');
    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| BooruError::InvalidUrl(url.to_string()))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(BooruError::InvalidUrl(url.to_string()));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn dapi_url(endpoint: &str) -> String {
    format!("{endpoint}/index.php")
}

pub(crate) fn dapi_credentials<'a>(
    key: &'a Option<String>,
    user: &'a Option<String>,
) -> Option<(&'a String, &'a String)> {
    match (key, user) {
        (Some(key), Some(user)) => Some((key, user)),
        _ => None,
    }
}

pub(crate) fn dapi_query(
    params: &[(&'static str, String)],
    credentials: Option<(&String, &String)>,
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
        query.push(("api_key".to_string(), key.clone()));
        query.push(("user_id".to_string(), user.clone()));
    }
    query
}

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

/// Rejects an API response with an unsuccessful status before decoding.
///
/// Failures become [`BooruError::HttpStatus`] with a bounded body excerpt.
/// Callers map endpoint-specific statuses such as missing posts first.
pub(crate) async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(BooruError::http_status(status, &body))
}
