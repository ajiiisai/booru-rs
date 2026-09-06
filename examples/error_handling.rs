//! Error handling example.
//!
//! Run with: cargo run --example error_handling

use booru_rs::danbooru::Client as Danbooru;
use booru_rs::prelude::*;
use booru_rs::safebooru::Client as Safebooru;

#[tokio::main]
async fn main() {
    println!("=== Tag Limit Error ===\n");

    // Danbooru only allows 2 tags for anonymous users
    let client = Danbooru::new().expect("client must build");
    let result = client
        .search()
        .tag("cat_ears")
        .tag("blue_eyes")
        .tag("1girl") // This will fail!
        .send()
        .await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(error) => match error.source_error() {
            BooruError::TagLimitExceeded {
                client,
                max,
                actual,
            } => {
                println!("Caught TagLimitExceeded error:");
                println!("  Client: {client}");
                println!("  Max allowed: {max}");
                println!("  Attempted: {actual}");
            }
            _ => println!("Unexpected error: {error}"),
        },
    }

    println!("\n=== Post Not Found ===\n");

    // Try to get a post that doesn't exist
    let result = client.post(999_999_999).await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(error) => match error.source_error() {
            BooruError::PostNotFound(id) => {
                println!("Caught PostNotFound error:");
                println!("  Post ID: {id}");
                if let Some(context) = error.context() {
                    println!("  Provider: {}", context.provider);
                    println!("  Operation: {}", context.operation);
                }
            }
            _ => println!("Other error: {error}"),
        },
    }

    println!("\n=== Error Inspection Methods ===\n");

    // Create a parse error for demonstration
    let error = BooruError::Parse(serde_json::from_str::<()>("invalid").unwrap_err());

    println!("Error: {}", error);
    println!("  is_network_error: {}", error.is_network_error());
    println!("  is_parse_error: {}", error.is_parse_error());
    println!("  is_not_found: {}", error.is_not_found());

    println!("\n=== Using Result Combinators ===\n");

    // Functional error handling with Result
    let result = Safebooru::builder().endpoint("::::");

    match result {
        Ok(builder) => match builder.build() {
            Ok(client) => match client.search().tag("flower").limit(3).send().await {
                Ok(posts) => {
                    println!("Got {} posts", posts.len());
                    println!("First post: #{}", posts[0].id);
                }
                Err(e) => eprintln!("Request failed: {}", e),
            },
            Err(e) => eprintln!("Builder error: {}", e),
        },
        Err(e) => eprintln!("Endpoint error: {}", e),
    }

    println!("\nExample completed!");
}
