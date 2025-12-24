//! Example: Download images from Safebooru
//!
//! This example demonstrates how to use the download helper to fetch images.
//!
//! Run with: cargo run --example download

use booru_rs::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let dest_dir = Path::new("./downloads");

    // Fetch some posts
    let posts = SafebooruClient::builder()
        .tag("landscape")?
        .rating(SafebooruRating::General)
        .limit(3)
        .build()
        .get()
        .await?;

    println!("Found {} posts to download", posts.len());

    // Create a downloader
    let downloader = Downloader::new();

    // Download each post
    for post in &posts {
        match downloader.download_post(post, dest_dir).await {
            Ok(result) => {
                if result.skipped {
                    println!("Skipped (already exists): {}", result.path.display());
                } else {
                    println!(
                        "Downloaded: {} ({} bytes)",
                        result.path.display(),
                        result.size
                    );
                }
            }
            Err(e) => {
                eprintln!("Failed to download post {}: {}", post.id, e);
            }
        }
    }

    // Example with progress callback
    println!("\nDownloading with progress...");
    if let Some(post) = posts.first() {
        let result = downloader
            .download_post_with_progress(post, dest_dir, |progress| {
                if let Some(total) = progress.total {
                    let percent = (progress.downloaded as f64 / total as f64) * 100.0;
                    println!(
                        "  Post {}: {:.1}% ({}/{})",
                        progress.post_id, percent, progress.downloaded, total
                    );
                } else {
                    println!("  Post {}: {} bytes", progress.post_id, progress.downloaded);
                }
            })
            .await?;

        println!(
            "Completed: {} ({} bytes)",
            result.path.display(),
            result.size
        );
    }

    // Example with concurrent downloads
    println!("\nConcurrent download of {} posts...", posts.len());
    let results = downloader.download_posts(&posts, dest_dir, 3).await;

    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(r) => println!("  [{}] {} - {} bytes", i + 1, r.path.display(), r.size),
            Err(e) => eprintln!("  [{}] Failed: {}", i + 1, e),
        }
    }

    println!("\nDone!");
    Ok(())
}
