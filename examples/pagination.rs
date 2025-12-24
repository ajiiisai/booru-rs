//! Pagination example using async streams.
//!
//! Run with: cargo run --example pagination

use booru_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Page Stream Example ===\n");
    println!("Fetching pages of results...\n");

    // Use into_page_stream() to iterate over pages
    let mut page_stream = SafebooruClient::builder()
        .tag("cat")?
        .limit(10) // 10 posts per page
        .into_page_stream()
        .max_pages(3); // Only fetch 3 pages

    let mut total_posts = 0;
    let mut page_num = 0;

    while let Some(page_result) = page_stream.next().await {
        let posts = page_result?;
        if posts.is_empty() {
            break;
        }
        page_num += 1;
        total_posts += posts.len();
        println!(
            "Page {}: {} posts (IDs: {} - {})",
            page_num,
            posts.len(),
            posts.first().map(|p| p.id).unwrap_or(0),
            posts.last().map(|p| p.id).unwrap_or(0)
        );
    }

    println!("\nTotal posts from page stream: {}", total_posts);

    println!("\n=== Post Stream Example ===\n");
    println!("Fetching individual posts with auto-pagination...\n");

    // Use into_post_stream() to iterate over individual posts
    let mut post_stream = SafebooruClient::builder()
        .tag("dog")?
        .limit(25) // Fetch 25 at a time internally
        .into_post_stream()
        .max_posts(50); // But only yield 50 total

    let mut count = 0;
    while let Some(post_result) = post_stream.next().await {
        let post = post_result?;
        count += 1;
        if count <= 5 || count > 45 {
            println!(
                "  Post #{}: {} ({}x{})",
                count, post.id, post.width, post.height
            );
        } else if count == 6 {
            println!("  ... ({} more posts) ...", 50 - 10);
        }
    }

    println!("\nTotal posts from post stream: {}", count);

    println!("\n=== Collect All Posts ===\n");

    // Use collect() to get all posts at once
    let all_posts = SafebooruClient::builder()
        .tag("bird")?
        .limit(100)
        .into_post_stream()
        .max_posts(150)
        .collect()
        .await?;

    println!("Collected {} posts total", all_posts.len());

    Ok(())
}
