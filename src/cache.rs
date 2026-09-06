//! Response caching for booru API requests.
//!
//! This module provides an in-memory cache for API responses to reduce
//! redundant network requests and improve performance.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::cache::{Cache, CacheConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), booru_rs::cache::CacheError> {
//! // Create a cache with 5-minute TTL and 1000 max entries
//! let cache: Cache<String> = Cache::with_config(CacheConfig {
//!     ttl: Duration::from_secs(300),
//!     max_entries: 1000,
//! });
//!
//! // Check cache before making request
//! let key = "danbooru:cat_ears:limit=10".to_string();
//! if let Some(cached) = cache.get::<Vec<u32>>(&key).await? {
//!     println!("Cache hit!");
//! } else {
//!     // Make request and cache result
//!     let result = vec![1, 2, 3];
//!     cache.insert(key, &result).await?;
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for the cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Time-to-live for cache entries.
    pub ttl: Duration,
    /// Maximum number of entries in the cache.
    pub max_entries: usize,
}

/// Errors returned while serializing or decoding cached values.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CacheError {
    /// A value could not be serialized for storage.
    #[error("failed to serialize cache value: {0}")]
    Serialize(#[source] serde_json::Error),
    /// Stored bytes could not be decoded as the requested type.
    #[error("failed to deserialize cache value: {0}")]
    Deserialize(#[source] serde_json::Error),
}

/// The operation represented by a cache entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CacheOperation {
    /// A single-post lookup.
    Post,
    /// A post search.
    Search,
    /// A tag autocomplete request.
    Autocomplete,
    /// A provider-specific operation.
    Custom(String),
}

/// A structured cache key scoped to one provider request.
///
/// Query terms retain their input order. Authentication identity is stored as
/// a one-way fingerprint so raw credentials never appear in the key or its
/// debug output.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    endpoint: String,
    operation: CacheOperation,
    auth_fingerprint: Option<u64>,
    query: Vec<String>,
    continuation: Option<String>,
}

impl CacheKey {
    /// Creates a key for a provider request.
    #[must_use]
    pub fn new<I, S>(
        endpoint: impl Into<String>,
        operation: CacheOperation,
        auth_identity: Option<&str>,
        query: I,
        continuation: Option<String>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            endpoint: endpoint.into(),
            operation,
            auth_fingerprint: auth_identity.map(auth_fingerprint),
            query: query.into_iter().map(Into::into).collect(),
            continuation,
        }
    }

    /// Returns the configured endpoint.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the operation represented by this key.
    #[must_use]
    pub fn operation(&self) -> &CacheOperation {
        &self.operation
    }

    /// Returns the query terms in their original order.
    #[must_use]
    pub fn query(&self) -> &[String] {
        &self.query
    }

    /// Returns the continuation token, if present.
    #[must_use]
    pub fn continuation(&self) -> Option<&str> {
        self.continuation.as_deref()
    }
}

impl std::fmt::Debug for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheKey")
            .field("endpoint", &self.endpoint)
            .field("operation", &self.operation)
            .field("auth_fingerprint", &self.auth_fingerprint)
            .field("query", &self.query)
            .field("continuation", &self.continuation)
            .finish()
    }
}

fn auth_fingerprint(identity: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    identity.hash(&mut hasher);
    hasher.finish()
}

impl Default for CacheConfig {
    /// Default configuration: caching disabled.
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300),
            max_entries: 0,
        }
    }
}

impl CacheConfig {
    /// Creates a short-lived cache suitable for real-time data.
    #[must_use]
    pub fn short_lived() -> Self {
        Self {
            ttl: Duration::from_secs(60),
            max_entries: 100,
        }
    }

    /// Creates a long-lived cache suitable for static data.
    #[must_use]
    pub fn long_lived() -> Self {
        Self {
            ttl: Duration::from_secs(3600),
            max_entries: 1000,
        }
    }
}

/// A cache entry with expiration time.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Serialized data.
    data: Vec<u8>,
    /// When this entry expires.
    expires_at: Instant,
    /// When this entry was last accessed.
    last_accessed: Instant,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// An in-memory cache for API responses.
///
/// The cache stores serialized data and automatically expires entries
/// after a configurable TTL. It uses LRU eviction when the max entry
/// limit is reached.
///
/// # Thread Safety
///
/// `Cache` is `Send`, `Sync`, and `Clone`, making it safe to share
/// across tasks and threads.
///
/// # Example
///
/// ```no_run
/// use booru_rs::cache::{Cache, CacheConfig};
///
/// # async fn example() -> Result<(), booru_rs::cache::CacheError> {
/// let cache: Cache<String> = Cache::with_config(CacheConfig::long_lived());
///
/// // Cache a search result
/// let posts = vec!["post1".to_string(), "post2".to_string()];
/// cache.insert("my_search".to_string(), &posts).await?;
///
/// // Retrieve later
/// if let Some(cached) = cache.get::<Vec<String>>(&"my_search".to_string()).await? {
///     println!("Got {} posts from cache", cached.len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Cache<K = String>
where
    K: Eq + Hash + Clone + Send + Sync,
{
    entries: Arc<RwLock<HashMap<K, CacheEntry>>>,
    config: CacheConfig,
}

impl<K> Cache<K>
where
    K: Eq + Hash + Clone + Send + Sync,
{
    /// Creates a new cache with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Creates a new cache with the given configuration.
    #[must_use]
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Inserts a value into the cache.
    ///
    /// The value must be serializable. Serialization failures are returned as
    /// [`CacheError::Serialize`]. If the cache is full, the least recently
    /// accessed entry will be evicted.
    pub async fn insert<V>(&self, key: K, value: &V) -> Result<(), CacheError>
    where
        V: Serialize,
    {
        if self.config.max_entries == 0 {
            return Ok(());
        }

        let data = serde_json::to_vec(value).map_err(CacheError::Serialize)?;

        let entry = CacheEntry {
            data,
            expires_at: Instant::now() + self.config.ttl,
            last_accessed: Instant::now(),
        };

        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.config.max_entries && !entries.contains_key(&key) {
            self.evict_lru(&mut entries);
        }

        entries.insert(key, entry);
        Ok(())
    }

    /// Retrieves a value from the cache.
    ///
    /// Returns `Ok(None)` if the key doesn't exist or the entry has expired.
    /// A value that cannot be decoded as `V` is returned as
    /// [`CacheError::Deserialize`].
    pub async fn get<V>(&self, key: &K) -> Result<Option<V>, CacheError>
    where
        V: for<'de> Deserialize<'de>,
    {
        let data = {
            let entries = self.entries.read().await;
            let Some(entry) = entries.get(key) else {
                return Ok(None);
            };
            if entry.is_expired() {
                drop(entries);
                let mut entries = self.entries.write().await;
                if entries.get(key).is_some_and(CacheEntry::is_expired) {
                    entries.remove(key);
                }
                return Ok(None);
            }
            entry.data.clone()
        };

        let value = serde_json::from_slice(&data).map_err(CacheError::Deserialize)?;
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(key)
            && !entry.is_expired()
            && entry.data == data
        {
            entry.last_accessed = Instant::now();
        }
        Ok(Some(value))
    }

    /// Removes an entry from the cache.
    pub async fn remove(&self, key: &K) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Clears all entries from the cache.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Returns the number of entries in the cache.
    ///
    /// Note: This includes expired entries that haven't been cleaned up yet.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Returns true if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    /// Removes all expired entries from the cache.
    pub async fn cleanup_expired(&self) {
        let mut entries = self.entries.write().await;
        entries.retain(|_, entry| !entry.is_expired());
    }

    /// Checks if a key exists in the cache and is not expired.
    pub async fn contains_key(&self, key: &K) -> bool {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    fn evict_lru(&self, entries: &mut HashMap<K, CacheEntry>) {
        // Find the least recently used entry
        if let Some((key_to_remove, _)) = entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, e)| (k.clone(), e.last_accessed))
        {
            entries.remove(&key_to_remove);
        }
    }
}

impl<K> Default for Cache<K>
where
    K: Eq + Hash + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> std::fmt::Debug for Cache<K>
where
    K: Eq + Hash + Clone + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("config", &self.config)
            .finish()
    }
}

/// Generates a cache key from request parameters.
///
/// This creates a consistent key format for caching booru API responses.
/// Query terms retain the order provided by the caller.
///
/// # Example
///
/// ```
/// use booru_rs::cache::cache_key;
///
/// let key = cache_key("danbooru", &["cat_ears".to_string(), "rating:general".to_string()], 10, 0);
/// assert!(key.contains("danbooru"));
/// assert!(key.contains("cat_ears"));
/// ```
#[must_use]
pub fn cache_key(client: &str, tags: &[String], limit: u32, page: u32) -> String {
    format!(
        "{}:{}:limit={}:page={}",
        client,
        tags.join(","),
        limit,
        page
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_get() {
        let cache = Cache::<String>::with_config(CacheConfig::long_lived());
        let value = vec![1, 2, 3];

        cache.insert("test".to_string(), &value).await.unwrap();

        let retrieved: Option<Vec<i32>> = cache.get(&"test".to_string()).await.unwrap();
        assert_eq!(retrieved, Some(value));
    }

    struct FailsSerialize;

    impl Serialize for FailsSerialize {
        fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom(
                "intentional serialization failure",
            ))
        }
    }

    #[tokio::test]
    async fn cache_reports_serialization_and_type_errors() {
        let cache = Cache::<String>::with_config(CacheConfig::long_lived());

        assert!(matches!(
            cache.insert("bad".to_string(), &FailsSerialize).await,
            Err(CacheError::Serialize(_))
        ));

        cache.insert("value".to_string(), &"text").await.unwrap();
        assert!(matches!(
            cache.get::<u32>(&"value".to_string()).await,
            Err(CacheError::Deserialize(_))
        ));
    }

    #[tokio::test]
    async fn test_expiration() {
        let cache = Cache::<String>::with_config(CacheConfig {
            ttl: Duration::from_millis(50),
            max_entries: 100,
        });

        cache.insert("test".to_string(), &"value").await.unwrap();

        // Should exist immediately
        assert!(cache.contains_key(&"test".to_string()).await);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        let result: Option<String> = cache.get(&"test".to_string()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = Cache::<String>::with_config(CacheConfig {
            ttl: Duration::from_secs(60),
            max_entries: 2,
        });

        cache.insert("a".to_string(), &1).await.unwrap();
        cache.insert("b".to_string(), &2).await.unwrap();

        // Access "a" to make it more recently used
        let _: Option<i32> = cache.get(&"a".to_string()).await.unwrap();

        // Insert "c", which should evict "b" (LRU)
        cache.insert("c".to_string(), &3).await.unwrap();

        assert!(cache.contains_key(&"a".to_string()).await);
        assert!(!cache.contains_key(&"b".to_string()).await);
        assert!(cache.contains_key(&"c".to_string()).await);
    }

    #[tokio::test]
    async fn zero_capacity_disables_cache() {
        let cache = Cache::<String>::with_config(CacheConfig {
            ttl: Duration::from_secs(60),
            max_entries: 0,
        });

        cache.insert("key".to_string(), &"value").await.unwrap();

        assert!(cache.is_empty().await);
        let value: Option<String> = cache.get(&"key".to_string()).await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn default_cache_is_disabled() {
        let cache = Cache::<String>::new();

        cache.insert("key".to_string(), &"value").await.unwrap();

        assert!(cache.is_empty().await);
    }

    #[test]
    fn test_cache_key() {
        let key = cache_key(
            "danbooru",
            &["blue_eyes".to_string(), "cat_ears".to_string()],
            10,
            0,
        );
        assert!(key.starts_with("danbooru:"));
        assert!(key.contains("limit=10"));
        assert!(key.contains("page=0"));
    }

    #[test]
    fn cache_key_preserves_query_order() {
        let first = cache_key(
            "danbooru",
            &["raw:order:score".to_string(), "raw:order:id".to_string()],
            10,
            0,
        );
        let second = cache_key(
            "danbooru",
            &["raw:order:id".to_string(), "raw:order:score".to_string()],
            10,
            0,
        );

        assert_ne!(first, second);
    }

    #[test]
    fn scoped_cache_key_preserves_query_order() {
        let first = CacheKey::new(
            "https://example.test",
            CacheOperation::Search,
            Some("user-1"),
            ["raw:a", "raw:b"],
            Some("page-2".to_string()),
        );
        let second = CacheKey::new(
            "https://example.test",
            CacheOperation::Search,
            Some("user-1"),
            ["raw:b", "raw:a"],
            Some("page-2".to_string()),
        );

        assert_ne!(first, second);
        assert_eq!(first.query(), &["raw:a", "raw:b"]);
        assert_eq!(first.continuation(), Some("page-2"));
    }

    #[test]
    fn scoped_cache_key_debug_omits_auth_identity() {
        let key = CacheKey::new(
            "https://example.test",
            CacheOperation::Post,
            Some("secret-api-key"),
            ["id=42"],
            None,
        );

        assert!(!format!("{key:?}").contains("secret-api-key"));
    }
}
