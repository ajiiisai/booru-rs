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
//!
//! ## Quick Start
//!
//! The easiest way to get started is with the [`prelude`]:
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
//!     for post in posts {
//!         println!("Post {}: {:?}", post.id, post.file_url);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Supported Sites
//!
//! | Site | Client | Tag Limit | Auth Required |
//! |------|--------|-----------|---------------|
//! | [Danbooru](https://danbooru.donmai.us) | [`DanbooruClient`] | 2 | No |
//! | [Gelbooru](https://gelbooru.com) | [`GelbooruClient`] | Unlimited | Yes |
//! | [Safebooru](https://safebooru.org) | [`SafebooruClient`] | Unlimited | No |
//! | [Rule34](https://rule34.xxx) | [`Rule34Client`] | Unlimited | Yes |
//!
//! ## Pagination with Async Streams
//!
//! Use [`stream::PostStream`] to iterate through all results:
//!
//! ```no_run
//! use booru_rs::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let mut stream = SafebooruClient::builder()
//!     .tag("landscape")?
//!     .limit(100)
//!     .into_post_stream()
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
//!     println!("#{}: {}x{}", post.id(), post.width(), post.height());
//! }
//! ```

pub mod cache;
pub mod client;
pub mod download;
pub mod error;
pub mod model;
pub mod prelude;
pub mod ratelimit;
pub mod retry;
pub mod stream;
pub mod validation;

// Re-export core types at crate root for convenience
pub use client::Client;
pub use client::ClientBuilder;
#[cfg(feature = "danbooru")]
pub use client::DanbooruClient;
#[cfg(feature = "gelbooru")]
pub use client::GelbooruClient;
#[cfg(feature = "rule34")]
pub use client::Rule34Client;
#[cfg(feature = "safebooru")]
pub use client::SafebooruClient;
pub use client::generic::Sort;
pub use error::{BooruError, Result};
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
