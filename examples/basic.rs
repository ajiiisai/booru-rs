//! Basic usage example for booru-rs.
//!
//! Run with: cargo run --example basic

use booru_rs::danbooru::Client as Danbooru;
use booru_rs::prelude::*;
use booru_rs::safebooru::Client as Safebooru;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Danbooru Example ===\n");

    // Danbooru has a 2-tag limit for anonymous users
    let client = Danbooru::new()?;
    let posts = client
        .search()
        .tag("cat_ears")
        .rating(DanbooruRating::General)
        .limit(5)
        .send()
        .await?;

    println!("Found {} posts from Danbooru:", posts.len());
    for post in &posts {
        println!(
            "  #{}: {}x{} - {}",
            post.id,
            post.image_width,
            post.image_height,
            post.file_url.as_deref().unwrap_or("(no url)")
        );
    }

    println!("\n=== Safebooru Example ===\n");

    // Safebooru has no tag limit and is SFW-only
    let client = Safebooru::new()?;
    let posts = client
        .search()
        .tag("landscape")
        .tag("scenery")
        .tag("sky")
        .sort(Sort::Score)
        .limit(5)
        .send()
        .await?;

    println!("Found {} posts from Safebooru:", posts.len());
    for post in &posts {
        println!(
            "  #{}: {}x{} - {}",
            post.id, post.width, post.height, post.image
        );
    }

    println!("\n=== Get Post by ID ===\n");

    // Fetch a specific post by ID
    let post = Danbooru::new()?.post(1).await?;

    println!("Danbooru Post #1:");
    println!("  Tags: {}", post.tag_string);
    println!("  Score: {}", post.score);
    println!("  Rating: {:?}", post.rating);

    Ok(())
}
