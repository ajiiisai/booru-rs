//! ### Usage
//! ```
//! use booru_rs::{DanbooruClient, Sort, DanbooruRating};
//!
//! #[tokio::main]
//! async fn main() {
//!     let posts = DanbooruClient::builder()
//!         .rating(DanbooruRating::General)
//!         .sort(Sort::Score)
//!         .build::<DanbooruClient>()
//!         .get()
//!         .await
//!         .expect("There was an error. (•-•)");
//!
//!     match posts.first() {
//!         Some(post) => println!("{:?}", post),
//!         None => panic!("Well... \"No posts found?\""),
//!     }
//! }
//! ```

pub mod client;
pub mod model;

// Conveience
pub use client::{
    danbooru::DanbooruClient, gelbooru::GelbooruClient, generic::Rating, generic::Sort,
};
pub use model::{danbooru::DanbooruRating, gelbooru::GelbooruRating};
