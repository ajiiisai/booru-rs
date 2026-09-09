![ci-badge][] [![crates.io version]][crates.io link] [![docs.rs]][docs.rs link]

# booru-rs

An async Rust client for Danbooru, Gelbooru, Safebooru, Rule34, and Konachan.

## Features

- Provider clients with typed ratings and provider-specific models
- Fluent searches with preflight validation
- Reusable owned queries, single post lookups, autocomplete, and async streams
- Shared HTTP clients with opt-in retries and rate limits
- Opt-in in-memory caching with expiration
- Optional image downloads with concurrency limits, progress, and MD5 checks
- `Post`, `Client`, and `Builder` traits for code that spans providers

## Supported sites

| Site | Client | Tag limit | Credentials |
|------|--------|-----------|-------------|
| [Danbooru](https://danbooru.donmai.us) | `booru_rs::danbooru::Client` | 2 | No |
| [Gelbooru](https://gelbooru.com) | `booru_rs::gelbooru::Client` | Unlimited | Yes |
| [Safebooru](https://safebooru.org) | `booru_rs::safebooru::Client` | Unlimited | No |
| [Rule34](https://rule34.xxx) | `booru_rs::rule34::Client` | Unlimited | Yes |
| [Konachan](https://konachan.com) | `booru_rs::konachan::Client` | Unlimited | No |

Gelbooru and Rule34 require API credentials for requests. See the [authentication section](#authentication).

Adding a new provider? See the [new provider guide](docs/new-provider.md).

## Installation

Add the crate and a Tokio runtime to your `Cargo.toml`:

```toml
[dependencies]
booru-rs = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The default feature set includes all five providers. Pick providers to trim dependencies:

```toml
[dependencies]
booru-rs = { version = "1", default-features = false, features = ["danbooru"] }
```

Available features are `danbooru`, `gelbooru`, `safebooru`, `rule34`, `konachan`, `download`, and `live-tests`. For downloads:

```toml
booru-rs = { version = "1", default-features = false, features = ["safebooru", "download"] }
```

## Migrate between major versions

The 2.x release tightens several types while keeping method names. Follow the [1.x to 2.x notes](docs/migration.md#migrate-from-1x-to-2x) when upgrading from 1.x. Coming from 0.x, read the [0.x to 1.x guide](docs/migration.md#migrate-from-0x-to-1x) first.

## Quick start

Each provider has its own client and models. Filters live on the search, not the client:

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

`Query` is owned. Build it once, run it many times:

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

A missing post surfaces as `BooruError::PostNotFound` through `source_error()`.

### Autocomplete

Autocomplete takes an explicit result limit:

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
if let Some(next) = page.next {
    let next_page = client.search_from(next).page().await?;
    println!("{} posts on the next page", next_page.posts.len());
}
```

Use `pages()` or `posts()` for streams. Both stop at an empty page and accept a bound:

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

Use `context()` for provider and operation metadata, `is_not_found()` style helpers for common cases, and `source_error()` to match concrete variants.

### Use common post accessors

Models keep provider-specific fields. `Post` exposes the shared ones:

```rust
use booru_rs::model::Post;

fn print_post(post: &impl Post) {
    println!("#{}: {}x{}", post.id(), post.width(), post.height().unwrap_or(0));
    for tag in post.tags_iter() {
        println!("{tag}");
    }
}
```

### Write generic code

`Client` and `Builder` cover every provider, so one function can configure and query any of them:

```rust
use booru_rs::client::Builder;

fn build<B: Builder>(builder: B) -> booru_rs::Result<B::Client> {
    builder.build()
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

`download_posts` takes a concurrency limit and returns one result per post in input order. `download_posts_with_progress` adds per post progress. Dropping either future cancels in-flight work.

Gelbooru image servers may redirect to an HTML post page without a `Referer` header:

```rust
use booru_rs::download::Downloader;
use reqwest::header::{HeaderMap, HeaderValue, REFERER};

let mut headers = HeaderMap::new();
headers.insert(REFERER, HeaderValue::from_static("https://gelbooru.com"));
let downloader = Downloader::new().with_headers(headers);
```

These headers apply to every request from this downloader and override matching client defaults. Calling `with_headers` again replaces them.

Error pages are rejected as `UnexpectedDownloadContentType` without creating files. Responses without a header are accepted.

Pass `DownloadOptions::default().verify_md5()` to check bytes against the post hash. Mismatches are removed and reported as `Md5Mismatch`.

## Authentication

Pass credentials to a provider builder:

```rust
use booru_rs::gelbooru::Client;

let client = Client::builder()
    .set_credentials("your_api_key", "your_user_id")
    .build()?;
```

Use `Rule34` in the same way. Keep credentials in application configuration and do not print the client or its errors with secret values.

## Test against live APIs

Live contract tests are ignored by default. Run them to check the adapters against current APIs:

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
