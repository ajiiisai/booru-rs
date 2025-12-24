//! Basic usage example for booru-rs.
//!
//! Run with: cargo run --example basic

use booru_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Danbooru Example ===\n");

    // Danbooru has a 2-tag limit for anonymous users
    let posts = DanbooruClient::builder()
        .tag("cat_ears")?
        .rating(DanbooruRating::General)
        .limit(5)
        .build()
        .get()
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
    let posts = SafebooruClient::builder()
        .tag("landscape")?
        .tag("scenery")?
        .tag("sky")?
        .sort(Sort::Score)
        .limit(5)
        .build()
        .get()
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
    let post = DanbooruClient::builder().build().get_by_id(1).await?;

    println!("Danbooru Post #1:");
    println!("  Tags: {}", post.tag_string);
    println!("  Score: {}", post.score);
    println!("  Rating: {:?}", post.rating);

    Ok(())
}
