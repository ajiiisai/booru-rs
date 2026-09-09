# Adding a provider

This checklist covers everything a new booru provider needs. Copy the
Danbooru or Safebooru implementation as a starting point, then work through
each step. The parity tests in `tests/client_trait_tests.rs` fail to compile
when a surface item is missing, so they double as the definition of done.

## Model

- Add `src/model/<name>.rs` with the post struct and the typed rating enums.
  Follow the existing split between filter ratings such as `DanbooruRating`
  and wire ratings such as `DanbooruPostRating` that preserve unknown values.
- Implement `Post` for the new post type in `src/model/mod.rs`.
- Reuse `Option` with `#[serde(default)]` for any field the API sometimes
  omits, especially media URLs. A required field turns one bad post into a
  failed page.

## Client

- Add `src/client/<name>.rs` reusing the shared pieces in `src/client/mod.rs`
  and `src/client/generic.rs`: `QueryCore`, `BuilderCore`, `stream`,
  `validate_tags`, `validate_raw_queries`, `validate_random_conflict`,
  `map_post_lookup_error`, `advance_page`, and `execute_with_policy`.
- Expose the same surface as the other providers: `Client::new`,
  `Client::builder`, `search`, `search_with`, `post`, and `autocomplete`.
- Expose the same `Query` chain: `tag`, `tags`, `raw_query`, `raw_queries`,
  `rating`, `sort`, `limit`, `blacklist_tag`, `blacklist_tags`,
  `exclude_rating`, `random`, and `validate`.
- Expose the same `Search` chain plus `start_page`, `send`, `page`, `pages`,
  and `posts`. Add `search_from` so a client can resume a client-independent
  continuation. Alias `Page`, `PageStream`, and `PostStream` to the shared
  types instead of reimplementing them.
- Implement the `Client` and `Builder` traits from `src/client/mod.rs` for
  the new types. Keep credential methods provider specific.
- Store only query state and the next page position in `Continuation`; the
  receiving client must supply the endpoint, credentials, HTTP client, and
  request policy.
- Document real API differences where they live: page numbering, sort
  prefixes, tag limits, auth params, and response envelopes.

## Wiring and docs

- Re-export the provider in `src/lib.rs` and `src/prelude.rs`.
- Add a row to the supported sites tables in `README.md` and `src/lib.rs`.
- Add mock tests mirroring an existing provider suite in `tests/`, plus a
  fixture under `tests/fixtures/` when the wire shape is custom.
- Copy the new parity test block in `tests/client_trait_tests.rs` for the
  new provider.
- Run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test --all-features` before opening the PR.
