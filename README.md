![ci-badge][] [![crates.io version]][crates.io link] [![docs.rs]][docs.rs link]

# booru-rs

An async Rust client for Danbooru, Gelbooru, Safebooru, and Rule34.

## Features

- Provider clients with typed ratings and provider-specific models
- Fluent searches with preflight validation
- Owned queries that you can save and run again
- Single-post requests, autocomplete, page streams, and post streams
- Shared HTTP clients with configurable retries and rate limits
- Typed in-memory caching with expiration
- Optional image downloads with bounded concurrency and progress callbacks
- A common `Post` trait for code that handles more than one provider

## Supported sites

| Site | Client | Tag limit | Credentials |
|------|--------|-----------|-------------|
| [Danbooru](https://danbooru.donmai.us) | `booru_rs::danbooru::Client` | 2 | No |
| [Gelbooru](https://gelbooru.com) | `booru_rs::gelbooru::Client` | Unlimited | Yes |
| [Safebooru](https://safebooru.org) | `booru_rs::safebooru::Client` | Unlimited | No |
| [Rule34](https://rule34.xxx) | `booru_rs::rule34::Client` | Unlimited | Yes |

Gelbooru and Rule34 require API credentials for requests. See the [authentication section](#authentication).

## Installation

Add the crate and a Tokio runtime to your `Cargo.toml`:

```toml
[dependencies]
booru-rs = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The default feature set includes all four providers. Select only the providers you use when you want a smaller dependency tree:

```toml
[dependencies]
booru-rs = { version = "1", default-features = false, features = ["danbooru"] }
```

Available features are `danbooru`, `gelbooru`, `safebooru`, `rule34`, `download`, and `live-tests`. Enable `download` when you use `booru_rs::download`:

```toml
booru-rs = { version = "1", default-features = false, features = ["safebooru", "download"] }
```

## Migrate from 0.x to 1.x

The 1.x release changes the client, query, error, model, and download APIs. Follow the [migration guide](docs/migration.md) before upgrading an application from 0.x.

## Quick start

Each provider has its own client and model types. A search keeps its filters separate from the reusable client:

```rust
use booru_rs::danbooru::{Client, DanbooruRating};
use booru_rs::client::generic::Sort;

#[tokio::main]
async fn main() -> booru_rs::Result<()> {
    let client = Client::new()?;
    let posts = client
        .search()
        .tag("cat_ears")
        .rating(DanbooruRating::General)
        .sort(Sort::Score)
        .limit(20)
        .send()
        .await?;

    for post in posts {
        println!("Post {}: {:?}", post.id, post.file_url);
    }

    Ok(())
}
```

## Common operations

### Reuse a query

`Query` is an owned value. Build it once when an application runs the same search more than once:

```rust
use booru_rs::safebooru::{Client, Query};

let client = Client::new()?;
let query = Query::new().tag("landscape").limit(100);
let first = client.search_with(query.clone()).send().await?;
let second = client.search_with(query).send().await?;
```

Use `blacklist_tag("tag")` for a literal excluded tag and
`exclude_rating(SafebooruRating::Explicit)` for a typed rating exclusion.
Use `raw_query("artist:foo bar")` when a provider expression contains syntax
that the typed query does not model. Raw rating and sort filters cannot be
combined with their typed counterparts.

### Fetch a post

```rust
let post = client.post(12345).await?;
```

A missing post has `BooruError::PostNotFound` as its source error. Provider
operations add context around the source.

### Autocomplete

Autocomplete uses the client endpoint and takes an explicit maximum result count:

```rust
let suggestions = client.autocomplete("cat_", 10).await?;
for suggestion in suggestions {
    println!("{}", suggestion.name);
}
```

### Paginate

Use `page()` when you need continuation metadata:

```rust
let page = client.search().tag("landscape").limit(100).page().await?;
println!("{} posts", page.posts.len());
if let Some(next_search) = page.next {
    let next_page = next_search.page().await?;
    println!("{} posts on the next page", next_page.posts.len());
}
```

Use `pages()` or `posts()` when you want a stream. Both streams stop at an empty page and can enforce a bound:

```rust
let mut posts = client.search().tag("landscape").posts().max_posts(500);
while let Some(post) = posts.next().await {
    println!("{}", post?.id);
}
```

### Handle errors

`BooruError` is non-exhaustive. Match the cases you need and keep a fallback arm:

```rust
use booru_rs::{BooruError, danbooru::Client};

let result = Client::new()?
    .search()
    .tag("a")
    .tag("b")
    .tag("c")
    .send()
    .await;

match result {
    Err(error) => match error.source_error() {
        BooruError::TagLimitExceeded { max, actual, .. } => {
            eprintln!("the query has {actual} tags, but the provider allows {max}");
        }
        _ if error.is_not_found() => eprintln!("post not found"),
        _ => eprintln!("request failed: {error}"),
    }
    Ok(_) => unreachable!(),
}
```

Provider errors include machine-readable provider and operation context. Use
`context()` for that metadata, category helpers such as `is_not_found()` for
common handling, and `source_error()` when matching a concrete error variant.

### Use common post accessors

Provider models keep their provider-specific fields. Implementations of `booru_rs::model::Post` expose the shared fields:

```rust
use booru_rs::model::Post;

fn print_post(post: &impl Post) {
    println!("#{}: {}x{}", post.id(), post.width(), post.height().unwrap_or(0));
    for tag in post.tags_iter() {
        println!("{tag}");
    }
}
```

### Download images

Downloads are optional. Enable the `download` feature before importing the module:

```toml
booru-rs = { version = "1", features = ["safebooru", "download"] }
```

```rust
use booru_rs::download::Downloader;
use std::path::Path;

let downloader = Downloader::new();
let result = downloader.download_post(&post, Path::new("./downloads")).await?;
println!("saved {} bytes to {}", result.size, result.path.display());
```

`download_posts` accepts a concurrency limit and returns one result per input post in input order. Use `download_posts_with_progress` for per post progress updates with the same ordering. Dropping the future or stream cancels in-flight work.

Gelbooru's image servers can redirect downloads to an HTML post page unless the
request includes a `Referer` header. Configure a downloader for Gelbooru like this:

```rust
use booru_rs::download::Downloader;
use reqwest::header::{HeaderMap, HeaderValue, REFERER};

let mut headers = HeaderMap::new();
headers.insert(REFERER, HeaderValue::from_static("https://gelbooru.com"));
let downloader = Downloader::new().with_headers(headers);
```

These headers apply to every request from this downloader, including batch
downloads. They override matching defaults from `Downloader::with_client`.
Calling `with_headers` again replaces the previously configured headers.

Downloads return `BooruError::UnexpectedDownloadContentType` when the response's
`Content-Type` is `text/html` or `application/xhtml+xml`, including after a
redirect. Rejected responses do not create or overwrite destination files.
Other content types and responses without `Content-Type` remain accepted.

## Authentication

Pass credentials to a provider builder. The builder validates the endpoint and returns a client from `build()`:

```rust
use booru_rs::gelbooru::Client;

let client = Client::builder()
    .set_credentials("your_api_key", "your_user_id")
    .build()?;
```

Use `Rule34` in the same way. Keep credentials in application configuration and do not print the client or its errors with secret values.

## Test against live APIs

The live contract tests are ignored by default. Run them when you want to check the provider adapters against their current APIs:

```sh
export BOORU_RS_LIVE_GELBOORU_API_KEY="your_gelbooru_api_key"
export BOORU_RS_LIVE_GELBOORU_USER_ID="your_gelbooru_user_id"
export BOORU_RS_LIVE_RULE34_API_KEY="your_rule34_api_key"
export BOORU_RS_LIVE_RULE34_USER_ID="your_rule34_user_id"
cargo test --features live-tests --test live_contract_tests -- --ignored --nocapture
```

Gelbooru and Rule34 require both credential variables. Danbooru accepts optional credentials. Set `BOORU_RS_LIVE_<PROVIDER>_ENDPOINT` to override a provider endpoint during a check.

## Configure requests

All provider builders accept a `reqwest::Client`, a `RequestPolicy`, a `RetryConfig`, and a `RateLimiter`:

```rust
use booru_rs::safebooru::Client;

let http = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(60))
    .build()?;
let client = Client::builder().http_client(http).build()?;
```

The library does not create a Tokio runtime. Start a runtime in the application that owns the client.

## Minimum supported Rust version

This crate requires Rust 1.92 or later and uses the 2024 edition.

## License

Licensed under the [MIT License](LICENSE-MIT).

[ci-badge]: https://img.shields.io/github/actions/workflow/status/ajiiisai/booru-rs/ci.yml?branch=main
[crates.io link]: https://crates.io/crates/booru-rs
[crates.io version]: https://img.shields.io/crates/v/booru-rs.svg?style=flat-square
[docs.rs]: https://img.shields.io/docsrs/booru-rs
[docs.rs link]: https://docs.rs/booru-rs
