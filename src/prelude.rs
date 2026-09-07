//! Convenient re-exports for common usage patterns.
//!
//! This module provides a single import for the most commonly used types,
//! making it easier to get started with the library.
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "danbooru")]
//! use booru_rs::danbooru::Client;
//!
//! # #[cfg(feature = "danbooru")]
//! #[tokio::main]
//! async fn main() -> booru_rs::error::Result<()> {
//!     let client = Client::new()?;
//!     let posts = client
//!         .search()
//!         .tag("cat_ears")
//!         .rating(booru_rs::model::danbooru::DanbooruRating::General)
//!         .sort(booru_rs::client::generic::Sort::Score)
//!         .limit(10)
//!         .send()
//!         .await?;
//!
//!     println!("Found {} posts", posts.len());
//!     Ok(())
//! }
//! # #[cfg(not(feature = "danbooru"))]
//! # fn main() {}
//! ```

// Core traits and types
pub use crate::client::RequestPolicy;
pub use crate::client::generic::Sort;
pub use crate::client::{Builder, Client};
pub use crate::error::{BooruError, Result};

// Autocomplete
pub use crate::autocomplete::TagSuggestion;

// Retry configuration
pub use crate::retry::RetryConfig;

// Rate limiting
pub use crate::ratelimit::RateLimiter;

// Caching
pub use crate::cache::{Cache, CacheConfig, CacheError, CacheKey, CacheOperation};

// Tag validation
pub use crate::validation::{TagValidation, TagWarning, validate_tag};

// Download utilities
#[cfg(feature = "download")]
pub use crate::download::{DownloadOptions, DownloadProgress, DownloadResult, Downloader};

// Danbooru
#[cfg(feature = "danbooru")]
pub use crate::model::danbooru::{DanbooruPost, DanbooruPostRating, DanbooruRating};

// Gelbooru
#[cfg(feature = "gelbooru")]
pub use crate::model::gelbooru::{GelbooruPost, GelbooruPostRating, GelbooruRating};

// Rule34
#[cfg(feature = "rule34")]
pub use crate::model::rule34::{Rule34Post, Rule34Rating};

// Safebooru
#[cfg(feature = "safebooru")]
pub use crate::model::safebooru::{SafebooruPost, SafebooruRating};
