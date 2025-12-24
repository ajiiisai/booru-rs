//! Error types for the booru-rs library.

/// A specialized `Result` type for booru-rs operations.
pub type Result<T> = std::result::Result<T, BooruError>;

/// Errors that can occur when interacting with booru APIs.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking changes.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BooruError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// Failed to parse the API response.
    #[error("Failed to parse API response: {0}")]
    Parse(#[from] serde_json::Error),

    /// Tag limit exceeded for the client.
    ///
    /// Some booru sites (like Danbooru) limit the number of tags per query.
    #[error("{client} allows a maximum of {max} tags, but {actual} were provided")]
    TagLimitExceeded {
        /// The client type that has the limit.
        client: &'static str,
        /// Maximum allowed tags.
        max: usize,
        /// Actual number of tags attempted.
        actual: usize,
    },

    /// Post with the given ID was not found.
    #[error("Post not found with ID: {0}")]
    PostNotFound(u32),

    /// The API returned an empty response when data was expected.
    #[error("Empty response from API")]
    EmptyResponse,

    /// Invalid URL provided.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Authentication required or failed.
    ///
    /// Some booru sites (like Gelbooru) require API credentials.
    #[error("Authentication required: {0}")]
    Unauthorized(String),

    /// Tag validation failed.
    ///
    /// The tag is invalid or contains problematic characters.
    #[error("Invalid tag '{tag}': {reason}")]
    InvalidTag {
        /// The invalid tag.
        tag: String,
        /// Reason the tag is invalid.
        reason: String,
    },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, please wait before making more requests")]
    RateLimited,

    /// I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl BooruError {
    /// Returns `true` if this error is a network-related error.
    #[must_use]
    pub fn is_network_error(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    /// Returns `true` if this error is a parse/deserialization error.
    #[must_use]
    pub fn is_parse_error(&self) -> bool {
        matches!(self, Self::Parse(_))
    }

    /// Returns `true` if this error indicates the resource was not found.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::PostNotFound(_) | Self::EmptyResponse)
    }
}
