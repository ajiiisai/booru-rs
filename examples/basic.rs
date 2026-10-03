//! Search Danbooru and Safebooru, then fetch a post by ID.
//!
//! Run with `nix develop -c cargo run --example basic`.

use booru_rs::danbooru::Client as Danbooru;
use booru_rs::prelude::*;
use booru_rs::safebooru::Client as Safebooru;

#[tokio::main]
async fn main() -> Result<()> {
    let danbooru = Danbooru::new()?;
    let posts = danbooru
        .search()
        .tag("cat_ears")
        .rating(DanbooruRating::General)
        .limit(5)
        .send()
        .await?;

    println!("Danbooru: {} posts", posts.len());
    for post in &posts {
        println!(
            "#{}: {}",
            post.id,
            post.file_url.as_deref().unwrap_or("(no URL)")
        );
    }

    let safebooru = Safebooru::new()?;
    let posts = safebooru
        .search()
        .tags(["landscape", "scenery", "sky"])
        .sort(Sort::Score)
        .limit(5)
        .send()
        .await?;

    println!("Safebooru: {} posts", posts.len());
    for post in &posts {
        println!(
            "#{}: {}",
            post.id,
            post.file_url.as_deref().unwrap_or("(no URL)")
        );
    }

    let post = danbooru.post(1).await?;
    println!("Danbooru #{}: {}", post.id, post.tag_string);
    Ok(())
}
