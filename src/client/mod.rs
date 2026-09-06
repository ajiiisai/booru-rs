//! Client implementations for various booru sites.
//!
//! This module provides the [`Client`] trait and [`ClientBuilder`] for constructing
//! and using booru API clients.
//!
//! # Available Clients
//!
//! - [`DanbooruClient`] for danbooru.donmai.us, 2 tag limit
//! - [`GelbooruClient`] for gelbooru.com, unlimited tags
//! - [`SafebooruClient`] for safebooru.org, unlimited tags, SFW only
//! - [`Rule34Client`] for api.rule34.xxx, unlimited tags
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! // Using the builder pattern
//! let posts = GelbooruClient::builder()
//!     .tags(["cat_ears", "blue_eyes"])?
//!     .rating(GelbooruRating::General)
//!     .sort(Sort::Score)
//!     .limit(10)
//!     .build()
//!     .get()
//!     .await?;
//!
//! // Get a specific post by ID
//! let post = DanbooruClient::builder()
//!     .build()
//!     .get_by_id(12345)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Custom HTTP Client
//!
//! By default, all clients share a connection-pooled HTTP client.
//!
//! ```no_run
//! use booru_rs::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let custom_client = reqwest::Client::builder()
//!     .timeout(std::time::Duration::from_secs(60))
//!     .build()
//!     .unwrap();
//!
//! // Use ClientBuilder::with_client to create a builder with custom HTTP client
//! let posts = ClientBuilder::<SafebooruClient>::with_client(custom_client)
//!     .tag("nature")?
//!     .build()
//!     .get()
//!     .await?;
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

/// Builder for constructing booru API clients.
///
/// # Example
///
/// ```no_run
/// use booru_rs::danbooru::{DanbooruClient, DanbooruRating};
/// use booru_rs::client::Client;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// let client = DanbooruClient::builder()
///     .tag("cat_ears")?
///     .rating(DanbooruRating::General)
///     .limit(10)
///     .build();
///
/// let posts = client.get().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ClientBuilder<T: Client> {
    pub(crate) client: reqwest::Client,
    pub(crate) key: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) limit: u32,
    pub(crate) url: String,
    pub(crate) page: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Client> Clone for ClientBuilder<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            key: self.key.clone(),
            user: self.user.clone(),
            tags: self.tags.clone(),
            limit: self.limit,
            url: self.url.clone(),
            page: self.page,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Core trait for booru API clients.
pub trait Client: From<ClientBuilder<Self>> + Sized + Send + Sync {
    /// The post type returned by this client.
    type Post: Send;

    /// The rating type for this booru site.
    type Rating: Into<String> + Send;

    /// Base URL for the booru API.
    const URL: &'static str;

    /// Prefix used for sorting tags (e.g., "order:" or "sort:").
    const SORT: &'static str;

    /// Maximum number of tags allowed per query, or `None` for unlimited.
    const MAX_TAGS: Option<usize>;

    /// Creates a new builder for this client.
    #[must_use]
    fn builder() -> ClientBuilder<Self> {
        ClientBuilder::new()
    }

    /// Retrieves a single post by its unique ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the post is not found.
    fn get_by_id(&self, id: u32) -> impl std::future::Future<Output = Result<Self::Post>> + Send;

    /// Retrieves posts matching the configured query.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the response cannot be parsed.
    fn get(&self) -> impl std::future::Future<Output = Result<Vec<Self::Post>>> + Send;
}

impl<T: Client> ClientBuilder<T> {
    /// Uses the shared HTTP client for connection pooling.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: SHARED_CLIENT.clone(),
            key: None,
            user: None,
            tags: Vec::new(),
            limit: 100,
            url: T::URL.to_string(),
            page: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Use this when you need custom HTTP configuration (e.g., proxy, custom TLS).
    #[must_use]
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            key: None,
            user: None,
            tags: Vec::new(),
            limit: 100,
            url: T::URL.to_string(),
            page: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets the API endpoint. Trailing slashes are removed, base paths are kept.
    ///
    /// # Errors
    ///
    /// Returns [`BooruError::InvalidUrl`] if the endpoint is blank, does not
    /// parse, or uses a scheme other than HTTP(S).
    pub fn endpoint(mut self, url: impl Into<String>) -> Result<Self> {
        self.url = validate_endpoint(&url.into())?;
        Ok(self)
    }

    /// Some booru sites require or benefit from authentication.
    #[must_use]
    pub fn set_credentials(mut self, key: impl Into<String>, user: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self.user = Some(user.into());
        self
    }

    /// # Errors
    ///
    /// Returns [`BooruError::TagLimitExceeded`] if adding this tag would exceed
    /// the client's maximum tag limit.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::danbooru::DanbooruClient;
    /// use booru_rs::client::Client;
    ///
    /// # fn example() -> booru_rs::error::Result<()> {
    /// let client = DanbooruClient::builder()
    ///     .tag("cat_ears")?
    ///     .tag("blue_eyes")?
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn tag(mut self, tag: impl Into<String>) -> Result<Self> {
        if let Some(max) = T::MAX_TAGS
            && self.tags.len() >= max
        {
            return Err(BooruError::TagLimitExceeded {
                client: std::any::type_name::<T>()
                    .rsplit("::")
                    .next()
                    .unwrap_or("Unknown"),
                max,
                actual: self.tags.len() + 1,
            });
        }
        self.tags.push(tag.into());
        Ok(self)
    }

    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::danbooru::{DanbooruClient, DanbooruRating};
    /// use booru_rs::client::Client;
    ///
    /// let client = DanbooruClient::builder()
    ///     .rating(DanbooruRating::General)
    ///     .build();
    /// ```
    #[must_use]
    pub fn rating(mut self, rating: T::Rating) -> Self {
        self.tags.push(format!("rating:{}", rating.into()));
        self
    }

    /// Default is 100.
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Enables random ordering of results.
    #[must_use]
    pub fn random(mut self) -> Self {
        self.tags.push(format!("{}random", T::SORT));
        self
    }

    /// Adds a sort order to the query.
    #[must_use]
    pub fn sort(mut self, order: generic::Sort) -> Self {
        self.tags.push(format!("{}{}", T::SORT, order));
        self
    }

    /// Multiple blacklist tags can be added by calling this method multiple times.
    #[must_use]
    pub fn blacklist_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(format!("-{}", tag.into()));
        self
    }

    /// Page numbering starts at 0.
    #[must_use]
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// # Errors
    ///
    /// Returns [`BooruError::TagLimitExceeded`] if adding these tags would exceed
    /// the client's maximum tag limit.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::prelude::*;
    ///
    /// # fn example() -> Result<()> {
    /// let client = GelbooruClient::builder()
    ///     .tags(["cat_ears", "blue_eyes", "1girl"])?
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn tags<I, S>(mut self, tags: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self = self.tag(tag)?;
        }
        Ok(self)
    }

    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::prelude::*;
    ///
    /// # fn example() -> Result<()> {
    /// let client = GelbooruClient::builder()
    ///     .tag("cat_ears")?
    ///     .blacklist_tags(["ugly", "low_quality"])
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self = self.blacklist_tag(tag);
        }
        self
    }

    #[must_use]
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    #[must_use]
    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    #[must_use]
    pub fn build(self) -> T {
        T::from(self)
    }
}

impl<T: Client> Default for ClientBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "danbooru")]
pub use danbooru::DanbooruClient;
#[cfg(feature = "gelbooru")]
pub use gelbooru::GelbooruClient;
#[cfg(feature = "rule34")]
pub use rule34::Rule34Client;
#[cfg(feature = "safebooru")]
pub use safebooru::SafebooruClient;
