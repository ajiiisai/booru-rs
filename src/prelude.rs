//! # Common imports
//!
//! The prelude exports shared traits, request policies, errors, and provider model types.
//! Provider model exports follow their site features. Download exports require the `download` feature.
//! Import the provider client separately:
//!
//! ```no_run
//! # #[cfg(feature = "danbooru")]
//! use booru_rs::danbooru::Client as Danbooru;
//! # #[cfg(feature = "danbooru")]
//! use booru_rs::prelude::*;
//!
//! # #[cfg(feature = "danbooru")]
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = Danbooru::new()?;
//!     let posts = client
//!         .search()
//!         .tag("cat_ears")
//!         .rating(DanbooruRating::General)
//!         .sort(Sort::Score)
//!         .limit(10)
//!         .send()
//!         .await?;
//!
//!     println!("{} posts", posts.len());
//!     Ok(())
//! }
//! # #[cfg(not(feature = "danbooru"))]
//! # fn main() {}
//! ```
//!
//! The alias `Danbooru` distinguishes the provider client from the shared `Client` trait.

// Core traits and types
pub use crate::client::RequestPolicy;
pub use crate::client::generic::Sort;
pub use crate::client::{Autocomplete, Builder, Client, Continuation, Query};
pub use crate::error::{BooruError, Result};
pub use crate::model::{Post, Rating};

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

// Konachan
#[cfg(feature = "konachan")]
pub use crate::model::konachan::{KonachanPost, KonachanRating};
