//! Inspect validation errors, missing posts, and error context.
//!
//! Run with `nix develop -c cargo run --example error_handling`.

use booru_rs::danbooru::Client as Danbooru;
use booru_rs::prelude::*;
use booru_rs::safebooru::Client as Safebooru;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Danbooru::new()?;

    // The client rejects three tags before a request.
    let result = client
        .search()
        .tags(["cat_ears", "blue_eyes", "1girl"])
        .send()
        .await;
    if let Err(error) = result {
        match error.source_error() {
            BooruError::TagLimitExceeded { max, actual, .. } => {
                println!("Query has {actual} tags. The client permits {max}.");
            }
            _ => eprintln!("Search failed: {error}"),
        }
        if let Some(context) = error.context() {
            println!(
                "Provider: {}. Operation: {}.",
                context.provider, context.operation
            );
        }
    }

    match client.post(999_999_999).await {
        Ok(post) => println!("Post #{} exists", post.id),
        Err(error) if error.is_not_found() => println!("Post not found"),
        Err(error) => eprintln!("Post request failed: {error}"),
    }

    if let Err(error) = serde_json::from_str::<()>("invalid") {
        let error = BooruError::from(error);
        println!("Parse error: {}", error.is_parse_error());
    }

    if let Err(error) = Safebooru::builder().endpoint("::::") {
        println!("Invalid endpoint: {error}");
    }
    Ok(())
}
