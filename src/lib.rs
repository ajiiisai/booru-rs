//! # booru-rs
//!
//! An async Rust client for Danbooru, Gelbooru, Safebooru, Rule34, and Konachan.
//! Each site has a reusable client, typed query filters, and a post model.
//!
//! ## Search posts
//!
//! The application supplies the Tokio runtime. Reuse the client across requests:
//!
//! ```no_run
//! # #[cfg(feature = "danbooru")]
//! use booru_rs::danbooru::{Client, DanbooruRating};
//!
//! # #[cfg(feature = "danbooru")]
//! #[tokio::main]
//! async fn main() -> booru_rs::Result<()> {
//! 	let client = Client::new()?;
//! 	let posts = client
//! 		.search()
//! 		.tag("cat_ears")
//! 		.rating(DanbooruRating::General)
//! 		.limit(10)
//! 		.send()
//! 		.await?;
//!
//! 	for post in posts {
//! 		if let Some(url) = post.file_url {
//! 			println!("{}: {url}", post.id);
//! 		}
//! 	}
//! 	Ok(())
//! }
//! # #[cfg(not(feature = "danbooru"))]
//! # fn main() {}
//! ```
//!
//! ## Sites and features
//!
//! All five site features belong to the default set:
//!
//! | Feature | Client | API credentials |
//! | --- | --- | --- |
//! | `danbooru` | `danbooru::Client` | Optional |
//! | `gelbooru` | `gelbooru::Client` | Required for post requests |
//! | `safebooru` | `safebooru::Client` | Not required |
//! | `rule34` | `rule34::Client` | Required for post requests |
//! | `konachan` | `konachan::Client` | Not required |
//!
//! Gelbooru and Rule34 accept an API key and user ID through `ClientBuilder::set_credentials`.
//! Danbooru accepts an API key and username through the same method.
//! Danbooru permits two query tags. The sort filter counts toward this limit, but typed rating filters do not.
//! Other clients impose no local tag limit. Site limits still apply.
//!
//! The `download` feature enables the image downloader.
//! The `live-tests` feature enables optional tests against site APIs.
//!
//! ## API modules
//!
//! - [`client`]: queries, builders, pagination streams, and request policies.
//!   Clients use no retries or rate limiter by default.
//! - [`model`]: provider models and the shared [`Post`] and [`Rating`] types.
//!   Media URLs and response ratings can be absent.
//! - [`autocomplete`]: tag suggestions for clients that implement [`Autocomplete`].
//! - [`cache`]: an in-memory cache that your application manages.
//! - [`retry`] and [`ratelimit`]: optional request policies.
//! - `download`: image downloads, progress callbacks, and optional MD5 checks.
//! - [`error`]: [`BooruError`], provider context, and error category helpers.
//! - [`prelude`]: common imports for application code.
//!
//! See the [README](https://github.com/ajiiisai/booru-rs#readme) for installation and
//! [examples](https://github.com/ajiiisai/booru-rs/tree/main/examples) for complete programs.

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
pub use cache::{CacheError, CacheKey, CacheOperation};
pub use client::generic::Sort;
pub use client::{Autocomplete, Builder, Client, Continuation, PageResult, Query, RequestPolicy};
pub use error::{BooruError, ErrorContext, Operation, Provider, Result};
pub use model::{Post, Rating};

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

/// Konachan client and model types.
#[cfg(feature = "konachan")]
pub mod konachan {
    pub use crate::client::konachan::*;
    pub use crate::model::konachan::*;
}
