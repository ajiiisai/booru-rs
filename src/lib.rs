//! # booru-rs
//!
//! An async Rust client for various booru image board APIs.
//!
//! This library provides a unified interface for querying multiple booru sites
//! including Danbooru, Gelbooru, Safebooru, and Rule34.
//!
//! ## Features
//!
//! - **Type-safe API**: Compile-time checks ensure you use the correct rating types for each booru
//! - **Async/await**: Built on tokio and reqwest for efficient async I/O
//! - **Connection pooling**: Shared HTTP client with automatic connection reuse
//! - **Proper error handling**: No panics, all errors are returned as `Result` types
//! - **Common trait**: Use the [`Post`] trait for generic code across booru sites
//! - **Async streams**: Paginate through results with async iterators
//! - **Image downloads**: Download images with progress tracking and concurrent downloads
//! - **Automatic retries**: Transient failures are retried with exponential backoff
//! - **Rate limiting**: Protect against API throttling
//! - **Response caching**: Reduce redundant API calls
//! - **Tag validation**: Catch common mistakes before making requests
//! - **Tag autocomplete**: Get tag suggestions as users type
//!
//! ## Quick Start
//!
//! The easiest way to get started is with the [`prelude`]:
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
//!         .limit(10)
//!         .send()
//!         .await?;
//!
//!     for post in posts {
//!         println!("Post {}: {:?}", post.id, post.file_url);
//!     }
//!
//!     Ok(())
//! }
//! # #[cfg(not(feature = "danbooru"))]
//! # fn main() {}
//! ```
//!
//! ## Supported Sites
//!
//! | Site | Client | Tag Limit | Auth Required |
//! |------|--------|-----------|---------------|
//! | [Danbooru](https://danbooru.donmai.us) | `danbooru::Client` | 2 | No |
//! | [Gelbooru](https://gelbooru.com) | `gelbooru::Client` | Unlimited | Yes |
//! | [Safebooru](https://safebooru.org) | `safebooru::Client` | Unlimited | No |
//! | [Rule34](https://rule34.xxx) | `rule34::Client` | Unlimited | Yes |
//!
//! ## Pagination with Async Streams
//!
//! Iterate through all results with `posts()`:
//!
//! ```no_run
//! # #[cfg(feature = "safebooru")]
//! use booru_rs::safebooru::Client;
//!
//! # #[cfg(feature = "safebooru")]
//! # async fn example() -> booru_rs::error::Result<()> {
//! let client = Client::new()?;
//! let mut stream = client
//!     .search()
//!     .tag("landscape")
//!     .limit(100)
//!     .posts()
//!     .max_posts(500);
//!
//! while let Some(post) = stream.next().await {
//!     println!("Post #{}", post?.id);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Generic Code with the Post Trait
//!
//! Use the [`Post`] trait to write code that works with any booru:
//!
//! ```no_run
//! use booru_rs::prelude::*;
//! use booru_rs::model::Post;
//!
//! fn print_post(post: &impl Post) {
//!     let height = post.height()
//!         .map(|height| height.to_string())
//!         .unwrap_or_else(|| "unknown".into());
//!     println!("#{}: {}x{}", post.id(), post.width(), height);
//! }
//! ```

pub mod autocomplete;
pub mod cache;
pub mod client;
#[cfg(feature = "download")]
pub mod download;
pub mod error;
pub mod model;
pub mod prelude;
pub mod ratelimit;
pub mod retry;
pub mod validation;

// Re-export core types at crate root for convenience
pub use autocomplete::TagSuggestion;
pub use cache::{CacheKey, CacheOperation};
pub use client::RequestPolicy;
pub use client::generic::Sort;
pub use error::{BooruError, ErrorContext, Operation, Provider, Result};
pub use model::Post;

/// Danbooru client and model types.
#[cfg(feature = "danbooru")]
pub mod danbooru {
    pub use crate::client::danbooru::*;
    pub use crate::model::danbooru::*;
}

/// Gelbooru client and model types.
#[cfg(feature = "gelbooru")]
pub mod gelbooru {
    pub use crate::client::gelbooru::*;
    pub use crate::model::gelbooru::*;
}

/// Rule34 client and model types.
#[cfg(feature = "rule34")]
pub mod rule34 {
    pub use crate::client::rule34::*;
    pub use crate::model::rule34::*;
}

/// Safebooru client and model types.
#[cfg(feature = "safebooru")]
pub mod safebooru {
    pub use crate::client::safebooru::*;
    pub use crate::model::safebooru::*;
}
