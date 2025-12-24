//! Convenient re-exports for common usage patterns.
//!
//! This module provides a single import for the most commonly used types,
//! making it easier to get started with the library.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let posts = DanbooruClient::builder()
//!         .tag("cat_ears")?
//!         .rating(DanbooruRating::General)
//!         .sort(Sort::Score)
//!         .limit(10)
//!         .build()
//!         .get()
//!         .await?;
//!
//!     println!("Found {} posts", posts.len());
//!     Ok(())
//! }
//! ```

// Core traits and types
pub use crate::client::Client;
pub use crate::client::ClientBuilder;
pub use crate::client::generic::Sort;
pub use crate::error::{BooruError, Result};

// Stream types for pagination
pub use crate::stream::{PageStream, PostStream};

// Retry configuration
pub use crate::retry::RetryConfig;

// Rate limiting
pub use crate::ratelimit::RateLimiter;

// Caching
pub use crate::cache::{Cache, CacheConfig};

// Tag validation
pub use crate::validation::{TagValidation, TagWarning, validate_tag};

// Download utilities
pub use crate::download::{DownloadOptions, DownloadProgress, DownloadResult, Downloader};

// Danbooru
#[cfg(feature = "danbooru")]
pub use crate::client::DanbooruClient;
#[cfg(feature = "danbooru")]
pub use crate::model::danbooru::{DanbooruPost, DanbooruRating};

// Gelbooru
#[cfg(feature = "gelbooru")]
pub use crate::client::GelbooruClient;
#[cfg(feature = "gelbooru")]
pub use crate::model::gelbooru::{GelbooruPost, GelbooruRating, GelbooruResponse};

// Rule34
#[cfg(feature = "rule34")]
pub use crate::client::Rule34Client;
#[cfg(feature = "rule34")]
pub use crate::model::rule34::{Rule34Post, Rule34Rating};

// Safebooru
#[cfg(feature = "safebooru")]
pub use crate::client::SafebooruClient;
#[cfg(feature = "safebooru")]
pub use crate::model::safebooru::{SafebooruPost, SafebooruRating};
