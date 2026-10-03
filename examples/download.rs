//! Download Safebooru images individually, with progress, and as a batch.
//!
//! Run with `nix develop -c cargo run --example download --features download`.
//! The downloader skips files that already exist in `downloads/`.

use booru_rs::prelude::*;
use booru_rs::safebooru::Client as Safebooru;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Safebooru::new()?;
    let posts = client
        .search()
        .tag("landscape")
        .rating(SafebooruRating::General)
        .limit(5)
        .send()
        .await?;
    let dest_dir = Path::new("downloads");
    let downloader = Downloader::new();

    let Some((first, remaining)) = posts.split_first() else {
        println!("No posts found");
        return Ok(());
    };
    let result = downloader.download_post(first, dest_dir).await?;
    print_result(&result);

    let Some((second, remaining)) = remaining.split_first() else {
        return Ok(());
    };
    let result = downloader
        .download_post_with_progress(second, dest_dir, |progress| {
            println!("Post {}: {} bytes", progress.post_id, progress.downloaded);
        })
        .await?;
    print_result(&result);

    let results = downloader.download_posts(remaining, dest_dir, 3).await;
    for (post, result) in remaining.iter().zip(results) {
        match result {
            Ok(result) => print_result(&result),
            Err(error) => eprintln!("Post {}: {error}", post.id),
        }
    }
    Ok(())
}

fn print_result(result: &DownloadResult) {
    let action = if result.skipped { "Skipped" } else { "Saved" };
    println!("{action} {}: {} bytes", result.path.display(), result.size);
}
