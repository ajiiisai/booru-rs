//! Client implementations for various booru sites.
//!
//! This module provides the [`Client`] trait and [`ClientBuilder`] for constructing
//! and using booru API clients.
//!
//! # Available Clients
//!
//! - [`DanbooruClient`] — For [danbooru.donmai.us](https://danbooru.donmai.us) (2 tag limit)
//! - [`GelbooruClient`] — For [gelbooru.com](https://gelbooru.com) (unlimited tags)
//! - [`SafebooruClient`] — For [safebooru.org](https://safebooru.org) (unlimited tags, SFW only)
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
//! By default, all clients share a connection-pooled HTTP client. You can provide
//! your own client for custom configuration:
//!
//! ```no_run
//! use booru_rs::prelude::*;
//!
//! # fn example() -> Result<()> {
//! let custom_client = reqwest::Client::builder()
//!     .timeout(std::time::Duration::from_secs(60))
//!     .build()
//!     .unwrap();
//!
//! let client = GelbooruClient::builder()
//!     .tag("nature")?
//!     .build();
//!
//! // Or use with_client to create a builder with a custom HTTP client:
//! let client = <GelbooruClient as Client>::builder()
//!     .tag("nature")?
//!     .build();
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
///
/// This client is lazily initialized and reused across all requests
/// for better performance.
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

/// Builder for constructing booru API clients.
///
/// This builder allows you to configure various options before
/// creating a client to query a booru site.
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
///
/// This trait defines the interface that all booru clients must implement.
/// It provides compile-time type safety for client-specific features like
/// ratings and tag limits.
///
/// # Associated Types
///
/// - `Post`: The post type returned by this client
/// - `Rating`: The rating type specific to this booru site
///
/// # Associated Constants
///
/// - `URL`: The base URL for the API
/// - `SORT`: The prefix for sort/order tags
/// - `MAX_TAGS`: Optional limit on the number of tags per query
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
    /// Creates a new builder with default settings.
    ///
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

    /// Creates a new builder with a custom HTTP client.
    ///
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

    /// Sets a custom base URL for the API.
    ///
    /// This is primarily useful for testing with mock servers.
    #[must_use]
    pub fn with_custom_url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    /// Sets the API key and username for authenticated requests.
    ///
    /// Some booru sites require or benefit from authentication.
    #[must_use]
    pub fn set_credentials(mut self, key: impl Into<String>, user: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self.user = Some(user.into());
        self
    }

    /// Adds a tag to the search query.
    ///
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

    /// Adds a rating filter to the search query.
    ///
    /// The rating type is specific to each booru site, ensuring
    /// compile-time type safety.
    ///
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

    /// Sets the maximum number of posts to retrieve.
    ///
    /// Default is 100, which is also typically the maximum allowed by most APIs.
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

    /// Excludes posts with the specified tag.
    ///
    /// Multiple blacklist tags can be added by calling this method multiple times.
    #[must_use]
    pub fn blacklist_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(format!("-{}", tag.into()));
        self
    }

    /// Overrides the default API URL.
    ///
    /// Useful for testing or accessing mirror sites.
    #[must_use]
    pub fn default_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Sets the page number for pagination.
    ///
    /// Page numbering starts at 0.
    #[must_use]
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Adds multiple tags to the search query at once.
    ///
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

    /// Excludes multiple tags from the search query at once.
    ///
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

    /// Returns the current number of tags in the query.
    #[must_use]
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Returns `true` if the builder has any tags configured.
    #[must_use]
    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    /// Builds the client with the configured options.
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

// Re-exports for convenience
#[cfg(feature = "danbooru")]
pub use danbooru::DanbooruClient;
#[cfg(feature = "gelbooru")]
pub use gelbooru::GelbooruClient;
#[cfg(feature = "rule34")]
pub use rule34::Rule34Client;
#[cfg(feature = "safebooru")]
pub use safebooru::SafebooruClient;
