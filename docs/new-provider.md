# Add a provider

Use [Safebooru](../src/client/safebooru.rs) as a template for a DAPI site,
or [Danbooru](../src/client/danbooru.rs) for a Danbooru-style API.
Replace the endpoint, request parameters, response types, and site-specific limits.

## Register the feature

- Add `<name> = []` to `[features]` in [Cargo.toml](../Cargo.toml).
- Add the feature to `default` if the provider belongs in the default set.
- Declare feature-gated modules in [src/client/mod.rs](../src/client/mod.rs) and [src/model/mod.rs](../src/model/mod.rs).
- Add feature-gated exports to [src/lib.rs](../src/lib.rs) and [src/prelude.rs](../src/prelude.rs).
- Extend shared helper `cfg(any(...))` lists to include the new feature wherever the provider needs those helpers.
  Check the client, model, stream, and error modules with only the new feature enabled.
- Set `required-features` for provider-specific tests and examples in `Cargo.toml`.

## Add the model

- Create `src/model/<name>.rs` with the post struct and rating types.
- Separate query ratings from response ratings. Preserve unknown response values, as `SafebooruPostRating::Unknown(String)` does.
- Use `Option<T>` for nullable or missing fields. Use `#[serde(default)]` for non-optional fields only when a default is valid.
- Implement `Post` in `src/model/mod.rs`. Return `None` for absent or empty media URLs and hashes.
- Map known response ratings to the shared `Rating` enum. Preserve unknown ratings with `Rating::Unknown`.

## Add the client

- Create `src/client/<name>.rs`. Reuse `QueryCore` and `BuilderCore` from [src/client/generic.rs](../src/client/generic.rs).
- Match the public methods in the template: `Client`, `ClientBuilder`, `Query`, and `Search`.
  Keep typed rating filters on the provider's `Query` and `Search`.
- Use `QueryCore::validate_common()` before requests. Add the site's tag limits and other query rules.
- Use `execute_with_policy` for search, post, and autocomplete requests.
  Reuse `shared_client()` when the builder supplies no HTTP client.
- Set the initial page and sort prefix to match the API.
  Use `advance_page` only for APIs that end pagination with an empty page.
- Alias `Continuation`, `Page`, `PageStream`, and `PostStream` to the shared types.
  Keep only the query and page position in `Continuation`.

- Implement `search_from` with the receiver's endpoint, credentials, HTTP client, and request policy.
- Implement the shared `Client` and `Builder` traits.
  Register the query with `impl_query!(<name>, "<name>")` in `src/client/mod.rs`.
- Add autocomplete with an explicit result limit if the API supports tag suggestions.
  Register it with `impl_autocomplete!(<name>, "<name>")`. `Autocomplete` is separate from `Client`.
- Keep credential methods specific to the provider. Use the shared `Secret` type for stored credentials.
- Add the provider to `Provider` and its `Display` implementation in [src/error.rs](../src/error.rs).
  Attach provider and operation context to request errors with `ResultContext::with_context`.
- Map missing posts to `BooruError::PostNotFound`. Reuse `map_post_lookup_error` for HTTP 404 responses.

## Add tests and docs

- Copy the closest provider's mock tests. Add response fixtures under `tests/fixtures/` as needed.
- Cover query validation, request parameters, credentials, response decoding, missing posts, and pagination.
  Include nullable media fields and unknown ratings.
- Add the provider to the trait checks and `surface_tests` in [tests/client_trait_tests.rs](../tests/client_trait_tests.rs).
  These tests check selected methods. Use mock tests to check request behavior.
- Add autocomplete tests if the provider supports autocomplete.
- Add an optional live contract test in [tests/live_contract_tests.rs](../tests/live_contract_tests.rs).
- Update the site table and feature list in [README.md](../README.md), plus the site table in `src/lib.rs`.
- Document API differences beside the relevant code: pagination, sort syntax, tag limits, credentials, and response formats.

## Check the change

Replace `<name>` with the feature name. Use `nix develop -c` for repository commands:

```sh
nix develop -c cargo fmt --all --check
nix develop -c cargo clippy --all-targets --all-features --locked -- -D warnings
nix develop -c cargo test --all-features --locked
nix develop -c cargo check --all-targets --no-default-features --features <name> --locked
nix develop -c cargo check --all-targets --no-default-features --locked
```

Run the live contract test separately with the required credentials.
The default test commands do not run ignored live tests.
See [CI](../.github/workflows/ci.yml) for the full feature and platform checks.
