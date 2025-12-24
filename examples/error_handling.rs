//! Error handling example.
//!
//! Run with: cargo run --example error_handling

use booru_rs::prelude::*;

#[tokio::main]
async fn main() {
    println!("=== Tag Limit Error ===\n");

    // Danbooru only allows 2 tags for anonymous users
    let result = DanbooruClient::builder()
        .tag("cat_ears")
        .and_then(|b| b.tag("blue_eyes"))
        .and_then(|b| b.tag("1girl")); // This will fail!

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(BooruError::TagLimitExceeded {
            client,
            max,
            actual,
        }) => {
            println!("✓ Caught TagLimitExceeded error:");
            println!("  Client: {}", client);
            println!("  Max allowed: {}", max);
            println!("  Attempted: {}", actual);
        }
        Err(e) => println!("Unexpected error: {}", e),
    }

    println!("\n=== Post Not Found ===\n");

    // Try to get a post that doesn't exist
    let result = DanbooruClient::builder()
        .build()
        .get_by_id(999_999_999)
        .await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(BooruError::PostNotFound(id)) => {
            println!("✓ Caught PostNotFound error:");
            println!("  Post ID: {}", id);
        }
        Err(e) => println!("Other error (API might return 404 differently): {}", e),
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
    let result = SafebooruClient::builder().tag("flower").map(|b| b.limit(3));

    match result {
        Ok(builder) => match builder.build().get().await {
            Ok(posts) => {
                println!("Got {} posts", posts.len());
                println!("First post: #{}", posts[0].id);
            }
            Err(e) => eprintln!("Request failed: {}", e),
        },
        Err(e) => eprintln!("Builder error: {}", e),
    }

    println!("\nExample completed!");
}
