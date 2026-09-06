# Migrate to the current client API

This guide moves an application from the pre-rework API to the current `0.3` API on the `1.0-rework` branch. The current API separates reusable provider clients from per-request search state.

## Update imports and features

Import a provider through its canonical module:

```rust
use booru_rs::danbooru::Client;
use booru_rs::model::danbooru::{DanbooruPost, DanbooruRating};
```

The old `DanbooruClient`, `GelbooruClient`, `SafebooruClient`, and `Rule34Client` names are gone. The old generic `booru_rs::client::ClientBuilder` is gone as well. Use the provider's `Client` and `ClientBuilder` types.

The default feature set still includes all providers. The download API is now optional:

```toml
booru-rs = { version = "0.3", default-features = false, features = ["safebooru", "download"] }
```

Enable `download` before importing `booru_rs::download` or its prelude re-exports.

## Construct a reusable client

Replace a generic builder with the provider builder:

```rust
use booru_rs::safebooru::Client;

let client = Client::builder()
    .http_client(reqwest::Client::new())
    .build()?;
```

For anonymous access, use the fallible default constructor:

```rust
let client = Client::new()?;
```

For Gelbooru and Rule34, set credentials on the provider builder:

```rust
let client = booru_rs::gelbooru::Client::builder()
    .set_credentials(api_key, user_id)
    .build()?;
```

The client owns the endpoint, credentials, HTTP client, and request policy. Clone the client when several tasks need to issue requests concurrently.

## Build and validate searches

The old builder combined client construction and search filters. Build filters on `search()` instead:

```rust
let posts = client
    .search()
    .tag("cat_ears")
    .tag("blue_eyes")
    .limit(20)
    .send()
    .await?;
```

Search setters are fluent and do not return a `Result`. Call `validate()` on an owned `Query` when a form needs feedback before I/O:

```rust
use booru_rs::safebooru::Query;

let query = Query::new().tag("landscape").limit(100);
query.validate()?;
let posts = client.search_with(query).send().await?;
```

`send()` validates the complete query before it sends a request. Repeated `tag()` and `blacklist_tag()` calls append terms. Use the typed `exclude_rating()` method for rating exclusions. Repeated `rating()`, `sort()`, and `limit()` calls replace those settings.

Use the provider's rating type and the shared `Sort` type:

```rust
use booru_rs::client::generic::Sort;
use booru_rs::danbooru::DanbooruRating;

let posts = client
    .search()
    .rating(DanbooruRating::General)
    .sort(Sort::Score)
    .send()
    .await?;
```

## Save and reuse queries

`Query` is owned and cloneable. Store it in application state when the same search runs more than once:

```rust
let query = Query::new().tag("cat_ears").limit(20);
let first = client.search_with(query.clone()).send().await?;
let second = client.search_with(query).send().await?;
```

The fluent search builder and `search_with()` use the same query representation and validation path.

## Fetch posts, autocomplete, and pages

Use `post(id)` instead of the old `get_by_id` operation:

```rust
let post = client.post(12345).await?;
```

Use an instance method for autocomplete. The `limit` argument is the requested maximum:

```rust
let suggestions = client.autocomplete("cat_", 10).await?;
```

Use `page()` when the application needs the continuation search:

```rust
let page = client.search().tag("landscape").page().await?;
for post in page.posts {
    println!("{}", post.id);
}
let next = page.next;
```

Use `pages()` for page results or `posts()` for individual posts. Both streams own a client clone and can move into a spawned task:

```rust
let mut stream = client.search().tag("landscape").posts().max_posts(500);
while let Some(post) = stream.next().await {
    use_post(post?);
}
```

The stream yields one terminal error for an invalid query and does not send a request for that query. Dropping the stream cancels its pending request.

## Match errors by category

`BooruError` is non-exhaustive. Add a wildcard arm when you match variants:

```rust
use booru_rs::{BooruError, Operation, Provider};

match client.post(12345).await {
    Ok(post) => use_post(post),
    Err(error) if matches!(error.source_error(), BooruError::PostNotFound(_)) => {
        eprintln!("the post is missing")
    }
    Err(error) if error.is_network_error() => retry_later(error),
    Err(error) if error.is_parse_error() => report_bad_response(error),
    Err(error) => report_failure(error),
}
```

Provider operations attach machine-readable context. Use `error.context()` to inspect its
`Provider` and `Operation`:

```rust
if let Some(context) = error.context() {
    if context.provider == Provider::Safebooru && context.operation == Operation::Search {
        report_safebooru_search_failure(&error);
    }
}
```

Use `is_not_found()`, `is_network_error()`, and `is_parse_error()` when the application does not need provider-specific detail. These helpers inspect the underlying error through its provider context. Use `source_error()` before matching `HttpStatus`, `Unauthorized`, or another concrete variant. Download failures do not have provider context.

## Update model access

Search methods return owned provider models, such as `DanbooruPost` or `SafebooruPost`. The models remain usable after the client and request are dropped.

Use provider fields for provider-specific behavior:

```rust
let rating = post.rating.clone();
let file_url = post.file_url.as_deref();
```

Use `booru_rs::model::Post` for shared behavior across providers:

```rust
use booru_rs::model::Post;

fn describe(post: &impl Post) {
    println!("#{} has {} tags", post.id(), post.tags_iter().count());
}
```

`tags_iter()` borrows the existing tag string. It does not allocate a second tag list.

## Move downloads behind a feature

Add the feature to the dependency declaration:

```toml
booru-rs = { version = "0.3", features = ["safebooru", "download"] }
```

Then use the downloader with a provider post:

```rust
use booru_rs::download::{DownloadOptions, Downloader};
use std::path::Path;

let downloader = Downloader::new().options(
    DownloadOptions::default().filename("{id}.{ext}"),
);
let result = downloader.download_post(&post, Path::new("downloads")).await?;
```

`download_posts(posts, destination, concurrency)` returns a result for each input post in input order. A zero concurrency value returns `InvalidConcurrency`. A destination collision returns `DestinationConflict` instead of overwriting one result with another.

## Keep runtime and application policy in the application

The crate uses Tokio for asynchronous operations but does not create a runtime. Configure the runtime, logging, environment variables, and task scheduling in the application:

```rust
#[tokio::main]
async fn main() -> booru_rs::Result<()> {
    let client = booru_rs::safebooru::Client::new()?;
    run(client).await
}
```

Configure retry and rate-limit policy on the provider builder. Clone a configured client to share those policies across requests.

## Migration checklist

- Replace old provider aliases with `booru_rs::<provider>::Client`.
- Replace the generic `ClientBuilder` with the provider builder.
- Move tags, ratings, sorting, limits, and pagination settings to `search()`.
- Remove `?` from fluent search setters and call `validate()` when you need preflight feedback.
- Replace `get()` with `send()` and `get_by_id()` with `post()`.
- Replace static autocomplete calls with `client.autocomplete(query, limit)`.
- Replace `into_post_stream()` with `posts()` and use `pages()` for page metadata.
- Add wildcard arms when matching `BooruError`.
- Import downloads only when the `download` feature is enabled.
- Keep Tokio runtime creation and application configuration outside the library.
