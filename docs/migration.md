# Migrate to the current client API

## Migrate from 1.x to 2.x

Version 2.0 keeps every method name but tightens several types. Most call
sites compile unchanged. Work through the items below only where the compiler
complains.

### Handle optional media URLs

`GelbooruPost::file_url` and the Safebooru `file_url`, `preview_url`, and
`sample_url` fields are now `Option<String>`. Deleted and restricted posts
omit them instead of failing the whole page decode.

Before:

```rust
println!("{}", post.file_url);
```

After:

```rust
println!("{}", post.file_url.as_deref().unwrap_or("(no url)"));
```

Generic code over `booru_rs::model::Post` is unaffected. Its `file_url()`
already returned `Option<&str>` and now maps empty strings to `None` as well.

### Expect `PageResult` from pages and streams

Each provider `Page` is now an alias for the shared `PageResult` over that
provider's post and search types. `Search::page` returns it directly, and page
streams yield it instead of `Page`.

```rust
pub type Page = super::PageResult<DanbooruPost, Search>;
```

Field access is unchanged. `page.posts` and `page.next` keep working in
single fetches, page streams, and generic code over the `Client` trait. Only
struct literals break. Construct results with `PageResult::new` instead:

```rust
let page = PageResult::new(posts, next);
```

### Build option structs with builders and constructors

`DownloadOptions` gained the `verify_md5` flag, and several option and result
structs are now `#[non_exhaustive]` so future fields stay minor:
`DownloadOptions`, `DownloadResult`, `DownloadProgress`, `CacheConfig`,
`RetryConfig`, `TagValidation`, `TagSuggestion`, `PageResult`, and
`ErrorContext`.

Replace struct literals with the matching constructor. For example, before:

```rust
let options = DownloadOptions {
    overwrite: true,
    filename_template: None,
};
```

After:

```rust
let options = DownloadOptions::default().overwrite();
```

Other swaps: `CacheConfig::long_lived` or `short_lived` instead of a literal,
`TagSuggestion::new` and `with_count`, and `PageResult::new`. Read
`ErrorContext` field by field instead of comparing against a literal:

```rust
let context = error.context().expect("error must carry context");
assert_eq!(context.provider, Provider::Danbooru);
```

### Check custom `Client` implementations for `Send` futures

The `Client` trait now declares its futures as `Send` so generic streams stay
usable in spawned tasks. Existing `async fn` implementations satisfy the new
bounds without changes as long as they hold no non-`Send` data across an
await.

### Expect rejected sort and random combinations

`Query::validate` now rejects `random()` combined with `sort()`, raw sort
filters, or a second `random()`. These combinations used to send conflicting
order clauses to the API. Pick one ordering per search.

### New in 2.0 without migration cost

These additions need no changes: `download_posts_with_progress`,
`DownloadOptions::verify_md5` with `BooruError::Md5Mismatch`,
`raw_queries()` on every `Query` and `Search`, the shared `Builder` trait,
`CacheConfig::disabled`, and the new provider guide in `docs/new-provider.md`.

## Migrate from 0.x to 1.x

This guide moves an application from the pre-rework API to the `1.0` API. The current API separates reusable provider clients from per-request search state.

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

Use `raw_query()` for provider expressions that are not represented by typed filters. Raw expressions may contain spaces; empty expressions are rejected.

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

Generic callers can use the operation trait at `booru_rs::client::Client`.
It keeps provider query, post, and continuation types associated with each
adapter while exposing one page operation:

```rust
use booru_rs::client::Client as ProviderClient;

async fn first_page<C: ProviderClient>(client: &C, query: C::Query) -> booru_rs::Result<()> {
    let page = client.page(query, None).await?;
    println!("{} posts", page.posts.len());
    Ok(())
}
```

External adapters can implement this trait with their own configuration; they
do not need access to provider client internals.

## Handle cache errors

Cache writes and reads now return a `Result`. Propagate `CacheError::Serialize`
when a value cannot be encoded and `CacheError::Deserialize` when stored bytes
cannot be decoded as the requested type. A missing or expired entry remains a
successful `Ok(None)` result:

```rust
use booru_rs::cache::{Cache, CacheConfig};

let cache: Cache<String> = Cache::with_config(CacheConfig::default());
cache.insert("search".to_string(), &vec![1, 2, 3]).await?;
let cached: Option<Vec<i32>> = cache.get(&"search".to_string()).await?;
```

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

Several provider fields now preserve missing or unknown wire values:

- `SafebooruPost::height` is `Option<u32>`. Use `unwrap_or(0)` or handle `None` explicitly.
- `GelbooruPost::file_url` and `SafebooruPost::file_url`, `preview_url`, and `sample_url` are `Option<String>`. Check the value before building a download request.
- `Rule34Post::file_url`, `preview_url`, and `sample_url` are `Option<String>`. Check the value before building a download request.
- Danbooru, Gelbooru, and Rule34 post ratings use provider post-rating enums that preserve unknown response values. Match the known variant or use `Display` when the exact classification is not required.
- `Rule34Post::parent_id` maps the provider's `0` sentinel to `None`.

For example, handle a Rule34 URL and a Safebooru dimension before using them:

```rust
if let Some(file_url) = rule34_post.file_url.as_deref() {
    println!("download {file_url}");
}
let height = safebooru_post.height.unwrap_or(0);
```

Gelbooru now exposes `GelbooruPost::directory` as `Option<String>`. Gelbooru sends current directory values as paths such as `"49/c6"`; older numeric values decode to their decimal string form. Treat the value as a path component instead of a number:

```rust
if let Some(directory) = post.directory.as_deref() {
    println!("stored in {directory}");
}
```

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
