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
//! # async fn example() {
//! // Create a cache with 5-minute TTL and 1000 max entries
//! let cache = Cache::new(CacheConfig {
//!     ttl: Duration::from_secs(300),
//!     max_entries: 1000,
//! });
//!
//! // Check cache before making request
//! let key = "danbooru:cat_ears:limit=10";
//! if let Some(cached) = cache.get::<Vec<u32>>(key).await {
//!     println!("Cache hit!");
//! } else {
//!     // Make request and cache result
//!     let result = vec![1, 2, 3];
//!     cache.insert(key, &result).await;
//! }
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
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

impl Default for CacheConfig {
    /// Default configuration: 5 minute TTL, 500 max entries.
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300),
            max_entries: 500,
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
/// # async fn example() {
/// let cache = Cache::with_config(CacheConfig::default());
///
/// // Cache a search result
/// let posts = vec!["post1", "post2"];
/// cache.insert("my_search", &posts).await;
///
/// // Retrieve later
/// if let Some(cached): Option<Vec<String>> = cache.get("my_search").await {
///     println!("Got {} posts from cache", cached.len());
/// }
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
    /// The value must be serializable. If the cache is full, the least
    /// recently accessed entry will be evicted.
    pub async fn insert<V>(&self, key: K, value: &V)
    where
        V: Serialize,
    {
        let data = match serde_json::to_vec(value) {
            Ok(d) => d,
            Err(_) => return,
        };

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
    }

    /// Retrieves a value from the cache.
    ///
    /// Returns `None` if the key doesn't exist or the entry has expired.
    pub async fn get<V>(&self, key: &K) -> Option<V>
    where
        V: for<'de> Deserialize<'de>,
    {
        // First check with read lock
        {
            let entries = self.entries.read().await;
            if let Some(entry) = entries.get(key) {
                if entry.is_expired() {
                    drop(entries);
                    self.remove(key).await;
                    return None;
                }

                if let Ok(value) = serde_json::from_slice(&entry.data) {
                    // We need to update last_accessed, so we'll do that below
                    drop(entries);

                    // Update last_accessed
                    let mut entries = self.entries.write().await;
                    if let Some(entry) = entries.get_mut(key) {
                        entry.last_accessed = Instant::now();
                    }

                    return Some(value);
                }
            }
        }

        None
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
///
/// # Example
///
/// ```
/// use booru_rs::cache::cache_key;
///
/// let key = cache_key("danbooru", &["cat_ears", "rating:general"], 10, 0);
/// assert!(key.contains("danbooru"));
/// assert!(key.contains("cat_ears"));
/// ```
#[must_use]
pub fn cache_key(client: &str, tags: &[String], limit: u32, page: u32) -> String {
    let mut tags_sorted = tags.to_vec();
    tags_sorted.sort();
    format!(
        "{}:{}:limit={}:page={}",
        client,
        tags_sorted.join(","),
        limit,
        page
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_get() {
        let cache = Cache::<String>::new();
        let value = vec![1, 2, 3];

        cache.insert("test".to_string(), &value).await;

        let retrieved: Option<Vec<i32>> = cache.get(&"test".to_string()).await;
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_expiration() {
        let cache = Cache::<String>::with_config(CacheConfig {
            ttl: Duration::from_millis(50),
            max_entries: 100,
        });

        cache.insert("test".to_string(), &"value").await;

        // Should exist immediately
        assert!(cache.contains_key(&"test".to_string()).await);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        let result: Option<String> = cache.get(&"test".to_string()).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = Cache::<String>::with_config(CacheConfig {
            ttl: Duration::from_secs(60),
            max_entries: 2,
        });

        cache.insert("a".to_string(), &1).await;
        cache.insert("b".to_string(), &2).await;

        // Access "a" to make it more recently used
        let _: Option<i32> = cache.get(&"a".to_string()).await;

        // Insert "c", which should evict "b" (LRU)
        cache.insert("c".to_string(), &3).await;

        assert!(cache.contains_key(&"a".to_string()).await);
        assert!(!cache.contains_key(&"b".to_string()).await);
        assert!(cache.contains_key(&"c".to_string()).await);
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
}
