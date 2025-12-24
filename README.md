![ci-badge][] [![crates.io version]][crates.io link] [![docs.rs]][docs.rs link]

# booru-rs

An async Rust client for various booru image board APIs.

## Features

- **Type-safe API** — Compile-time checks ensure you use the correct rating types for each booru
- **Async/await** — Built on tokio and reqwest for efficient async I/O
- **Connection pooling** — Shared HTTP client with automatic connection reuse
- **Proper error handling** — No panics, all errors are returned as `Result` types
- **Common `Post` trait** — Write generic code that works with any booru site
- **Async streams** — Paginate through results with async iterators
- **Image downloads** — Download images with progress tracking and concurrent downloads
- **Automatic retries** — Transient failures are retried with exponential backoff
- **Rate limiting** — Protect against API throttling
- **Response caching** — Reduce redundant API calls
- **Tag validation** — Catch common mistakes before making requests

## Supported Sites

| Site | Client | Tag Limit | Auth Required |
|------|--------|-----------|---------------|
| [Danbooru](https://danbooru.donmai.us) | `DanbooruClient` | 2 | No |
| [Gelbooru](https://gelbooru.com) | `GelbooruClient` | Unlimited | **Yes** |
| [Safebooru](https://safebooru.org) | `SafebooruClient` | Unlimited | No |
| [Rule34](https://rule34.xxx) | `Rule34Client` | Unlimited | **Yes** |

> **Note:** Gelbooru and Rule34 require API credentials. See [Authentication](#gelbooru-authentication) below.

**Planned:**
- [ ] Konachan
- [ ] 3DBooru

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
booru-rs = "0.3"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Selective Features

By default, all booru clients are included. You can enable only the ones you need:

```toml
[dependencies]
# Only Danbooru support
booru-rs = { version = "0.3", default-features = false, features = ["danbooru"] }

# Danbooru + Safebooru
booru-rs = { version = "0.3", default-features = false, features = ["danbooru", "safebooru"] }
```

Available features: `danbooru`, `gelbooru`, `safebooru`, `rule34`

## Quick Start

Use the `prelude` for convenient imports:

```rust
use booru_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino")?
        .tag("2girls")?
        .rating(GelbooruRating::General)
        .sort(Sort::Random)  // Or use .random() shorthand
        .limit(5)
        .blacklist_tag(GelbooruRating::Explicit)
        .build()
        .get()
        .await?;

    for post in &posts {
        println!("Post #{}: {}", post.id, post.file_url);
    }

    Ok(())
}
```

## API Examples

### Basic Usage

```rust
use booru_rs::prelude::*;

// Danbooru (limited to 2 tags)
let posts = DanbooruClient::builder()
    .tag("cat_ears")?
    .rating(DanbooruRating::General)
    .limit(10)
    .build()
    .get()
    .await?;

// Get a specific post by ID
let post = DanbooruClient::builder()
    .build()
    .get_by_id(12345)
    .await?;
```

### Multiple Tags at Once

```rust
use booru_rs::prelude::*;

// Gelbooru has no tag limit
let posts = GelbooruClient::builder()
    .tags(["cat_ears", "blue_eyes", "1girl"])?
    .blacklist_tags(["ugly", "low_quality"])
    .sort(Sort::Score)
    .build()
    .get()
    .await?;
```

### Generic Code with the `Post` Trait

```rust
use booru_rs::prelude::*;
use booru_rs::Post;

fn print_post_info(post: &impl Post) {
    println!("#{}: {}x{}", post.id(), post.width(), post.height());
    if let Some(url) = post.file_url() {
        println!("  URL: {}", url);
    }
}
```

### Pagination

```rust
use booru_rs::prelude::*;

// Get page 5 of results
let posts = SafebooruClient::builder()
    .tag("landscape")?
    .page(5)
    .limit(100)
    .build()
    .get()
    .await?;
```

### Async Pagination Stream

```rust
use booru_rs::prelude::*;

// Stream through all results automatically
let mut stream = SafebooruClient::builder()
    .tag("landscape")?
    .limit(100)
    .into_post_stream()
    .max_posts(500);  // Stop after 500 posts

while let Some(post) = stream.next().await {
    println!("Post #{}", post?.id);
}
```

### Rate Limiting

```rust
use booru_rs::prelude::*;
use std::time::Duration;

// 2 requests per second
let limiter = RateLimiter::new(2, Duration::from_secs(1));

for tag in ["cat", "dog", "bird"] {
    limiter.acquire().await;  // Waits if needed
    let posts = SafebooruClient::builder()
        .tag(tag)?
        .build()
        .get()
        .await?;
}
```

### Response Caching

```rust
use booru_rs::prelude::*;

let cache = Cache::new();

// First call hits the API
let posts = SafebooruClient::builder()
    .tag("nature")?
    .build()
    .get()
    .await?;
cache.insert("nature_search".to_string(), &posts).await;

// Later, check cache first
if let Some(cached) = cache.get::<Vec<SafebooruPost>>(&"nature_search".to_string()).await {
    println!("Cache hit! {} posts", cached.len());
}
```

### Tag Validation

```rust
use booru_rs::validation::{validate_tag, validate_tag_strict};

// Get warnings about potential issues
let result = validate_tag("cat ears");  // Space instead of underscore
if result.has_warnings() {
    println!("Suggested: {}", result.tag());  // "cat_ears"
}

// Or get normalized tag directly
let tag = validate_tag_strict("  cat ears  ")?;  // Returns "cat_ears"
```

### Custom HTTP Client

```rust
use booru_rs::prelude::*;
use booru_rs::client::ClientBuilder;

let custom_client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(60))
    .build()?;

// Use with_client to create a builder with custom HTTP client
let posts = ClientBuilder::<SafebooruClient>::with_client(custom_client)
    .tag("nature")?
    .build()
    .get()
    .await?;
```

> **Note:** Most users won't need a custom HTTP client. The default shared client
> provides connection pooling and sensible timeouts.

### Downloading Images

```rust
use booru_rs::prelude::*;
use std::path::Path;

let posts = SafebooruClient::builder()
    .tag("landscape")?
    .limit(10)
    .build()
    .get()
    .await?;

// Download posts directly
let downloader = Downloader::new();
for post in &posts {
    let result = downloader.download_post(post, Path::new("./downloads")).await?;
    println!("Downloaded: {}", result.path.display());
}

// Download with progress tracking
for post in &posts {
    let result = downloader
        .download_post_with_progress(post, Path::new("./downloads"), |progress| {
            println!("{}/{} bytes (post #{})", 
                progress.downloaded, 
                progress.total.unwrap_or(0),
                progress.post_id);
        })
        .await?;
}

// Concurrent downloads (4 at a time)
let results = downloader.download_posts(&posts, Path::new("./downloads"), 4).await;

// Custom options
let downloader = Downloader::new()
    .options(DownloadOptions::default().overwrite().filename("{id}_{md5}.{ext}"));
```

### Gelbooru Authentication

Gelbooru requires API credentials for all API requests. To get your credentials:

1. Create an account at [gelbooru.com](https://gelbooru.com)
2. Go to **My Account** → **Options** → **API Access Credentials**
3. Copy your **API Key** and **User ID**

```rust
use booru_rs::prelude::*;

let posts = GelbooruClient::builder()
    .set_credentials("your_api_key", "your_user_id")
    .tag("landscape")?
    .build()
    .get()
    .await?;
```

You can also load credentials from environment variables:

```rust
use booru_rs::prelude::*;

let api_key = std::env::var("GELBOORU_API_KEY").expect("GELBOORU_API_KEY not set");
let user_id = std::env::var("GELBOORU_USER_ID").expect("GELBOORU_USER_ID not set");

let posts = GelbooruClient::builder()
    .set_credentials(api_key, user_id)
    .tag("cat_ears")?
    .build()
    .get()
    .await?;
```

### Rule34 Authentication

Rule34 also requires API credentials:

1. Create an account at [rule34.xxx](https://rule34.xxx)
2. Go to **My Account** → **Options** → **API Access Credentials**
3. Copy your **API Key** and **User ID**

```rust
use booru_rs::prelude::*;

let posts = Rule34Client::builder()
    .set_credentials("your_api_key", "your_user_id")
    .tag("landscape")?
    .build()
    .get()
    .await?;
```

## Error Handling

All fallible operations return `Result<T, BooruError>`:

```rust
use booru_rs::prelude::*;

match DanbooruClient::builder().tag("a")?.tag("b")?.tag("c") {
    Ok(_) => unreachable!(),
    Err(BooruError::TagLimitExceeded { client, max, actual }) => {
        println!("{} only allows {} tags, tried to add {}", client, max, actual);
    }
    Err(e) => println!("Other error: {}", e),
}
```

## Minimum Supported Rust Version

This crate requires Rust 1.92 or later (2024 edition).

## License

Licensed under the [MIT License](LICENSE-MIT).

[ci-badge]: https://img.shields.io/github/actions/workflow/status/ajiiisai/booru-rs/ci.yml?branch=main
[crates.io link]: https://crates.io/crates/booru-rs
[crates.io version]: https://img.shields.io/crates/v/booru-rs.svg?style=flat-square
[docs.rs]: https://img.shields.io/docsrs/booru-rs
[docs.rs link]: https://docs.rs/booru-rs
