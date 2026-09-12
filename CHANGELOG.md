# Changelog

All notable changes to this project will be documented in this file.

## [2.0.0](https://github.com/ajiiisai/booru-rs/compare/v1.1.0...v2.0.0) - 2026-09-12

### Added

- expand shared query, post, and pagination APIs ([#57](https://github.com/ajiiisai/booru-rs/pull/57))
- add Konachan ([#55](https://github.com/ajiiisai/booru-rs/pull/55))
- add shared builder trait for generic providers ([#48](https://github.com/ajiiisai/booru-rs/pull/48))

### Fixed

- *(safebooru)* use signed integer for score ([#62](https://github.com/ajiiisai/booru-rs/pull/62))
- harden downloads and release documentation ([#58](https://github.com/ajiiisai/booru-rs/pull/58))
- correct pagination, download verification, and retries ([#56](https://github.com/ajiiisai/booru-rs/pull/56))
- verify downloads against post MD5 ([#47](https://github.com/ajiiisai/booru-rs/pull/47))
- add batch download progress and harden content handling ([#46](https://github.com/ajiiisai/booru-rs/pull/46))
- clarify disabled retry and cache defaults ([#45](https://github.com/ajiiisai/booru-rs/pull/45))
- reject sort conflicts with random() ([#44](https://github.com/ajiiisai/booru-rs/pull/44))
- allow missing file URLs for Gelbooru and Safebooru ([#43](https://github.com/ajiiisai/booru-rs/pull/43))
- use crate version for Danbooru User-Agent ([#42](https://github.com/ajiiisai/booru-rs/pull/42))
- reject HTML downloads and support custom request headers ([#40](https://github.com/ajiiisai/booru-rs/pull/40))

### Other

- update 2.0 dependency examples and provider list
- add 1.x to 2.x migration notes ([#54](https://github.com/ajiiisai/booru-rs/pull/54))
- [**breaking**] mark extensible structs non_exhaustive ([#53](https://github.com/ajiiisai/booru-rs/pull/53))
- add new provider guide, parity tests, and README rewrite ([#52](https://github.com/ajiiisai/booru-rs/pull/52))
- [**breaking**] unify page type and builder core across providers ([#51](https://github.com/ajiiisai/booru-rs/pull/51))
- share query builder core across providers ([#50](https://github.com/ajiiisai/booru-rs/pull/50))
- [**breaking**] share page and post streams across providers ([#49](https://github.com/ajiiisai/booru-rs/pull/49))

## [1.1.0](https://github.com/ajiiisai/booru-rs/compare/v1.0.0...v1.1.0) - 2026-09-07

### Added

- add tags() to Search and Query builders ([#37](https://github.com/ajiiisai/booru-rs/pull/37))

### Fixed

- preserve error context in trait post lookups ([#35](https://github.com/ajiiisai/booru-rs/pull/35))

### Other

- bump dorny/paths-filter from 3 to 4 ([#39](https://github.com/ajiiisai/booru-rs/pull/39))
- bump actions/checkout from 6 to 7 ([#38](https://github.com/ajiiisai/booru-rs/pull/38))
- install cargo-hack for direct dependency checks
- skip CI and release PRs for non-code changes
- Update copyright year in LICENSE-MIT

## [1.0.0](https://github.com/ajiiisai/booru-rs/compare/v0.3.2...v1.0.0) - 2026-09-06

### Added

- [**breaking**] release the 1.0 client rework

## [0.3.2] - 2026-09-05
### Fixed
- Fixed Gelbooru autocomplete failing when `post_count` is returned as a string.
- Gelbooru autocomplete now enforces the requested result limit when the API returns more results.
- Live autocomplete tests now tolerate transient booru API response failures.

## [0.3.1] - 2025-12-24

### Added
- **Tag autocomplete** support for all clients via `Autocomplete` trait
- `TagSuggestion` struct with tag name, label, post count, and category
- Category name mapping (general, artist, copyright, character, meta)

## [0.3.0] - 2025-12-24

### Added
- **Rule34 client** with API key authentication (`Rule34Client`)
- **Download helper** module with progress tracking (`Downloader`)
- **Mock server tests** using `wiremock` for offline testing
- `Sort::Random` variant for explicit random ordering
- Type-safe ratings per client (compile-time checks)
- Rate limiting with `RateLimiter`
- Response caching with `Cache`
- Tag validation with `validate_tag()`
- Async pagination streams with `PostStream`
- Retry logic with exponential backoff
- `Post` trait for generic code across boorus
- Gelbooru API key authentication support
- New error types: `Unauthorized`, `InvalidTag`, `RateLimited`, `Io`
- `ClientBuilder::with_custom_url()` for testing with mock servers

### Changed
- Rust 2024 edition, MSRV 1.92
- `tag()` returns `Result` to check tag limits at build time
- Improved error messages with `thiserror`

### Fixed
- Gelbooru 401 errors now return `BooruError::Unauthorized`
- Safebooru model now includes `file_url`, `preview_url`, `sample_url` fields

## [0.2.0] - Previous Release

### Added
- Initial async client implementation
- Danbooru, Gelbooru, Safebooru support
- Basic builder pattern for queries
