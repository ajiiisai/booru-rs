//! Gelbooru example with API authentication.
//!
//! Gelbooru requires API credentials. Set these environment variables:
//! - GELBOORU_API_KEY: Your API key from account settings
//! - GELBOORU_USER_ID: Your user ID from account settings
//!
//! Run with: cargo run --example gelbooru

use booru_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Get credentials from environment
    let api_key = match std::env::var("GELBOORU_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: GELBOORU_API_KEY environment variable not set");
            eprintln!();
            eprintln!("To use this example:");
            eprintln!("  1. Create an account at https://gelbooru.com");
            eprintln!("  2. Go to My Account → Options → API Access Credentials");
            eprintln!("  3. Set environment variables:");
            eprintln!("     $env:GELBOORU_API_KEY = \"your_api_key\"");
            eprintln!("     $env:GELBOORU_USER_ID = \"your_user_id\"");
            eprintln!("  4. Run this example again");
            return Ok(());
        }
    };

    let user_id = match std::env::var("GELBOORU_USER_ID") {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Error: GELBOORU_USER_ID environment variable not set");
            return Ok(());
        }
    };

    println!("=== Gelbooru Example ===\n");

    // Gelbooru has no tag limit
    let posts = GelbooruClient::builder()
        .set_credentials(&api_key, &user_id)
        .tag("cat_ears")?
        .tag("blue_eyes")?
        .tag("1girl")?
        .tag("solo")?
        .rating(GelbooruRating::General)
        .sort(Sort::Score)
        .limit(5)
        .build()
        .get()
        .await?;

    println!("Found {} posts:", posts.len());
    for post in &posts {
        println!(
            "  #{}: {}x{} - score:{} - {}",
            post.id, post.width, post.height, post.score, post.file_url
        );
    }

    println!("\n=== Random Posts ===\n");

    let random_posts = GelbooruClient::builder()
        .set_credentials(&api_key, &user_id)
        .tag("landscape")?
        .rating(GelbooruRating::General)
        .random()
        .limit(3)
        .build()
        .get()
        .await?;

    println!("Random posts:");
    for post in &random_posts {
        println!("  #{}: {}", post.id, post.file_url);
    }

    Ok(())
}
