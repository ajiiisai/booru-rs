//! Fetch pages, stream posts, or collect a limited set of posts.
//!
//! Run with `nix develop -c cargo run --example pagination`.

use booru_rs::prelude::*;
use booru_rs::safebooru::Client as Safebooru;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Safebooru::new()?;
    let mut pages = client.search().tag("cat").limit(10).pages().max_pages(3);
    let mut page_number = 0;

    while let Some(page) = pages.next().await {
        let page = page?;
        page_number += 1;
        println!("Page {page_number}: {} posts", page.posts.len());
    }

    let mut posts = client.search().tag("dog").limit(25).posts().max_posts(50);
    while let Some(post) = posts.next().await {
        println!("Post #{}", post?.id);
    }

    let posts = client
        .search()
        .tag("bird")
        .limit(100)
        .posts()
        .max_posts(150)
        .collect()
        .await?;
    println!("Collected {} posts", posts.len());
    Ok(())
}
