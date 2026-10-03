# Migrate to the current client API

Use the section for your installed version.
For an upgrade from 0.x to 2.x, apply both sections.
See the [README](../README.md) for current usage examples.

## Migrate from 1.x to 2.x

Update the dependency to `booru-rs = "2"`. Version 2 requires Rust 1.92 or later.

### Handle optional media URLs and signed scores

These fields change from `String` to `Option<String>`:

- `GelbooruPost::file_url`
- `SafebooruPost::file_url`, `preview_url`, and `sample_url`

Replace direct string access with an explicit check:

```rust
if let Some(url) = post.file_url.as_deref() {
	println!("{url}");
}
```

`SafebooruPost::score` changes from `Option<u32>` to `Option<i32>`.
Use a signed type to preserve negative scores.
The shared `Post::file_url()` and `Post::md5()` accessors now return `None` for empty strings.

### Construct pages and options through the API

Provider `Page` types now alias `PageResult`.
Keep `page.posts` and `page.next` access. Replace page struct literals with `PageResult::new(posts, next)`.

These structs now use `#[non_exhaustive]`:

- `DownloadOptions`, `DownloadResult`, and `DownloadProgress`
- `CacheConfig` and `RetryConfig`
- `TagValidation` and `TagSuggestion`
- `PageResult` and `ErrorContext`

Use constructors instead of struct literals. For example:

```rust
use booru_rs::download::DownloadOptions;

let options = DownloadOptions::default().overwrite();
```

Use `CacheConfig::short_lived()` or `long_lived()` to enable storage.
Use `RetryConfig::new(retries)` and its `with_*` methods to configure retries.
Use `TagSuggestion::new` or `TagSuggestion::with_count` to create suggestions.
Read result fields directly, or add `..` to patterns.

### Return `Send` futures from custom clients

The shared `Client` trait now requires `Send` futures.
Keep non-`Send` values outside the scope of each `await` in custom implementations.

### Use one sort order per query

Remove combinations of `random()` with `sort()`, raw sort expressions, or another `random()`.
`Query::validate()` now rejects these combinations.

### Handle download error pages

Downloads now reject HTML responses with `BooruError::UnexpectedDownloadContentType`.
For Gelbooru image servers, set a `Referer` header if the server requires one.
See [Downloader::with_headers](https://docs.rs/booru-rs/latest/booru_rs/download/struct.Downloader.html#method.with_headers) for an example.

## Migrate from 0.x to 1.x

The 1.x API separates the client from search filters.
The examples in this section target 1.x. Apply the 2.x changes above for the current release.

### Update imports and features

Replace old provider names such as `DanbooruClient` with `booru_rs::danbooru::Client`.
Replace the generic `booru_rs::client::ClientBuilder` with the provider's `ClientBuilder`.
Enable `download` if the application uses image downloads:

```toml
booru-rs = { version = "1", default-features = false, features = ["safebooru", "download"] }
```

### Separate client configuration from searches

Move the endpoint, HTTP client, credentials, and request policy to the provider builder.
For Gelbooru and Rule34, supply an API key and user ID:

```rust
let client = booru_rs::gelbooru::Client::builder()
	.set_credentials(api_key, user_id)
	.build()?;
```

For sites that permit anonymous requests, use `Client::new()?`.
Move tags, rating filters, sort order, limits, and pagination to `client.search()`.
Remove `?` from search setters. Replace `get()` with `send()`:

```rust
use booru_rs::safebooru::Client;

let client = Client::new()?;
let posts = client
	.search()
	.tag("cat_ears")
	.limit(20)
	.send()
	.await?;
```

`send()` validates the query before the request.
Use a provider-specific `Query` and `query.validate()` for validation before `send()`.
Use `client.search_with(query)` for a query that the application stores or reuses.
Literal tags reject whitespace. Use `raw_query()` for provider expressions with spaces.

### Replace post, autocomplete, and stream calls

| Old operation | Replacement |
| --- | --- |
| `get_by_id(id)` | `client.post(id).await?` |
| Static autocomplete call | `client.autocomplete(prefix, limit).await?` |
| `into_post_stream()` | `client.search().posts()` |

Use `page()` for continuation metadata. Pass `page.next` to `client.search_from(next)` when the continuation is present.
Use `pages()` for a page stream, or `posts()` for a post stream.
Set a bound with `max_pages()` or `max_posts()`.

### Update error handling

Add a fallback arm to matches on the non-exhaustive `BooruError` enum.
Use `error.source_error()` to match the underlying variant.
Use `error.context()` for the provider and operation.
Use `is_not_found()`, `is_network_error()`, or `is_parse_error()` for category checks.

Cache reads and writes return `Result` with `CacheError`.
Propagate serialization errors. Treat `Ok(None)` as a missing or expired entry.
`CacheConfig::default()` disables storage. Select `short_lived()` or `long_lived()` for an active cache.

### Update model access

- Handle `SafebooruPost::height` as `Option<u32>`.
- Handle Rule34 media URLs as `Option<String>`.
- Treat `GelbooruPost::directory` as `Option<String>`, because values can contain paths such as `"49/c6"`.
- Match response rating enums through their `Known` and `Unknown` variants.
- Handle `Rule34Post::parent_id` as `Option<u32>`. The API's zero value maps to `None`.

Use the shared `Post` trait for access to common fields across sites.
Keep the Tokio runtime and application configuration in the application.
