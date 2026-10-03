//! Search Gelbooru with API credentials from your account settings.
//!
//! Set credentials in Bash:
//!
//! ```sh
//! export GELBOORU_API_KEY="your_api_key"
//! export GELBOORU_USER_ID="your_user_id"
//! ```
//!
//! Or use PowerShell:
//!
//! ```powershell
//! $env:GELBOORU_API_KEY = "your_api_key"
//! $env:GELBOORU_USER_ID = "your_user_id"
//! ```
//!
//! Run with `nix develop -c cargo run --example gelbooru` in Bash,
//! or `cargo run --example gelbooru` in PowerShell with Rust installed.

use booru_rs::gelbooru::Client as Gelbooru;
use booru_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let (Ok(api_key), Ok(user_id)) = (
        std::env::var("GELBOORU_API_KEY"),
        std::env::var("GELBOORU_USER_ID"),
    ) else {
        eprintln!(
            "Set GELBOORU_API_KEY and GELBOORU_USER_ID. See examples/gelbooru.rs for commands."
        );
        return Ok(());
    };

    let client = Gelbooru::builder()
        .set_credentials(api_key, user_id)
        .build()?;
    let posts = client
        .search()
        .tags(["cat_ears", "blue_eyes", "1girl", "solo"])
        .rating(GelbooruRating::General)
        .sort(Sort::Score)
        .limit(5)
        .send()
        .await?;

    println!("Search: {} posts", posts.len());
    for post in &posts {
        println!(
            "#{}: {}",
            post.id,
            post.file_url.as_deref().unwrap_or("(no URL)")
        );
    }

    let posts = client
        .search()
        .tag("landscape")
        .rating(GelbooruRating::General)
        .random()
        .limit(3)
        .send()
        .await?;

    println!("Random search: {} posts", posts.len());
    for post in &posts {
        println!(
            "#{}: {}",
            post.id,
            post.file_url.as_deref().unwrap_or("(no URL)")
        );
    }
    Ok(())
}
