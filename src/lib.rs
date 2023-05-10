//! ### Usage
//! ```
//! use booru_rs::{DanbooruClient, Sort, DanbooruRating};
//! use booru_rs::client::Client;
//!
//! #[tokio::main]
//! async fn main() {
//!     let posts = DanbooruClient::builder()
//!         .default_url("https://testbooru.donmai.us")
//!         .rating(DanbooruRating::General)
//!         .sort(Sort::Score)
//!         .build()
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
