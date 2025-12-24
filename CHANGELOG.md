# Changelog

All notable changes to this project will be documented in this file.

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
