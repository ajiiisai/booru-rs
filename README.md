# booru-rs

![ci-badge][] [![crates.io version]][crates.io link] [![docs.rs]][docs.rs link]

An async Rust client for Danbooru, Gelbooru, Safebooru, Rule34, and Konachan.
Search posts with typed filters, fetch individual posts, and request tag suggestions.
The crate also provides pagination streams, optional image downloads, and common traits for code that uses multiple sites.

[API reference](https://docs.rs/booru-rs) · [Examples](examples) · [Migration guide](docs/migration.md)

## Install

Requires Rust 1.92 or later. Add these dependencies to `Cargo.toml`:

```toml
[dependencies]
booru-rs = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The default feature set includes all five sites. To select specific sites, disable the default features:

```toml
booru-rs = { version = "2", default-features = false, features = ["safebooru"] }
```

Site features are `danbooru`, `gelbooru`, `safebooru`, `rule34`, and `konachan`.
Add `download` for image downloads.

## Search posts

```rust
use booru_rs::danbooru::{Client, DanbooruRating};
use booru_rs::Sort;

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
		if let Some(url) = post.file_url {
			println!("{}: {url}", post.id);
		}
	}

	Ok(())
}
```

Reuse the client across requests. Each site has its own client, post model, and rating type.
Some posts have no media URL.

## Sites and authentication

| Site | Client | API credentials |
| --- | --- | --- |
| [Danbooru](https://danbooru.donmai.us) | `danbooru::Client` | Optional |
| [Gelbooru](https://gelbooru.com) | `gelbooru::Client` | Required for post requests |
| [Safebooru](https://safebooru.org) | `safebooru::Client` | Not required |
| [Rule34](https://rule34.xxx) | `rule34::Client` | Required for post requests |
| [Konachan](https://konachan.com) | `konachan::Client` | Not required |

Client paths are relative to `booru_rs`.
Danbooru queries permit two tags. The sort filter counts toward this limit.
Typed rating filters do not count toward this limit.
Other clients impose no local tag limit. Site limits still apply.

For Gelbooru or Rule34, use the client builder with an API key and user ID:

```rust
use booru_rs::gelbooru::Client;

let client = Client::builder()
	.set_credentials("your_api_key", "your_user_id")
	.build()?;
```

Danbooru accepts an API key and username through the same method.

## Common tasks

These snippets use the client from the search example inside an async function.

Fetch one post or request tag suggestions:

```rust
let post = client.post(12345).await?;
let suggestions = client.autocomplete("cat_", 10).await?;
```

Fetch up to 100 posts across multiple pages:

```rust
let mut posts = client.search().tag("cat_ears").posts().max_posts(100);
while let Some(post) = posts.next().await {
	println!("{}", post?.id);
}
```

For manual pagination, use `page()` and pass its `next` continuation to `client.search_from(next)`.
For reusable searches, create a site-specific `Query` and pass it to `client.search_with(query)`.

Use `blacklist_tag("tag")` to exclude a tag.
Use `raw_query("artist:foo bar")` for site-specific expressions.
Do not combine raw rating or sort filters with their typed equivalents.

For more examples, see [basic queries](examples/basic.rs), [pagination](examples/pagination.rs),
[error handling](examples/error_handling.rs), and [image downloads](examples/download.rs).
Run the download example with `cargo run --example download --features download`.

## Configure requests

Client builders accept a custom `reqwest::Client`, retry configuration, and a shared rate limiter.
Clients use no retries or rate limiter by default.

See the [builder API](https://docs.rs/booru-rs/latest/booru_rs/safebooru/struct.ClientBuilder.html)
for request options and the [cache module](https://docs.rs/booru-rs/latest/booru_rs/cache/index.html)
for a cache that your application manages.

## Contribute

See the [new provider guide](docs/new-provider.md) to add a site.
The `live-tests` feature enables optional tests against site APIs.
See [live_contract_tests.rs](tests/live_contract_tests.rs) for commands and credential variables.

## License

[MIT](LICENSE-MIT).

[ci-badge]: https://img.shields.io/github/actions/workflow/status/ajiiisai/booru-rs/ci.yml?branch=main
[crates.io link]: https://crates.io/crates/booru-rs
[crates.io version]: https://img.shields.io/crates/v/booru-rs.svg?style=flat-square
[docs.rs]: https://img.shields.io/docsrs/booru-rs
[docs.rs link]: https://docs.rs/booru-rs
