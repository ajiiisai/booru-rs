//! Error types for the booru-rs library.

/// A specialized `Result` type for booru-rs operations.
pub type Result<T> = std::result::Result<T, BooruError>;

/// Provider that produced an API operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Provider {
    /// Danbooru.
    Danbooru,
    /// Gelbooru.
    Gelbooru,
    /// Rule34.
    Rule34,
    /// Safebooru.
    Safebooru,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Danbooru => "Danbooru",
            Self::Gelbooru => "Gelbooru",
            Self::Rule34 => "Rule34",
            Self::Safebooru => "Safebooru",
        })
    }
}

/// API operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Operation {
    /// A post search or page fetch.
    Search,
    /// A single-post lookup.
    Post,
    /// A tag autocomplete request.
    Autocomplete,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Search => "search",
            Self::Post => "post lookup",
            Self::Autocomplete => "autocomplete",
        })
    }
}

/// Machine-readable context for a provider API failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorContext {
    /// Provider whose operation failed.
    pub provider: Provider,
    /// Operation that failed.
    pub operation: Operation,
}

/// Errors that can occur when interacting with booru APIs.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking changes.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BooruError {
    /// An error from a provider API operation.
    #[error("{provider} {operation} failed: {source}")]
    Context {
        /// Provider whose operation failed.
        provider: Provider,
        /// Operation that failed.
        operation: Operation,
        /// Underlying classified error.
        #[source]
        source: Box<BooruError>,
    },

    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Request(reqwest::Error),

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

    /// Invalid destination filename.
    #[error("Invalid destination filename: {0}")]
    InvalidFilename(String),

    /// A post does not provide a downloadable media URL.
    #[error("Post {0} has no media URL")]
    MissingMediaUrl(u32),

    /// A download returned HTML instead of media.
    #[error("Download returned HTML instead of media (Content-Type: {0})")]
    UnexpectedDownloadContentType(String),

    /// A spawned download task failed before returning its result.
    #[error("Download task failed: {0}")]
    DownloadTaskFailed(String),

    /// Multiple batch posts resolve to the same destination path.
    #[error("Multiple downloads target the same destination: {0}")]
    DestinationConflict(std::path::PathBuf),

    /// Authentication required or failed.
    ///
    /// Some booru sites (like Gelbooru) require API credentials.
    #[error("Authentication required: {0}")]
    Unauthorized(String),

    /// The API answered with an unsuccessful HTTP status code.
    ///
    /// The body excerpt is capped at 300 characters on one line.
    #[error("Request failed with HTTP status {status}: {message}")]
    HttpStatus { status: u16, message: String },

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

    /// Query configuration is invalid.
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, please wait before making more requests")]
    RateLimited,

    /// Retry configuration is invalid.
    #[error("Invalid retry configuration: {0}")]
    InvalidRetryConfig(String),

    /// Rate limiter configuration is invalid.
    #[error("Invalid rate limiter configuration: {0}")]
    InvalidRateLimitConfig(String),

    /// I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Download concurrency must be greater than zero.
    #[error("Download concurrency must be greater than zero")]
    InvalidConcurrency,
}

impl From<reqwest::Error> for BooruError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error.without_url())
    }
}

impl BooruError {
    #[cfg(any(
        feature = "danbooru",
        feature = "gelbooru",
        feature = "rule34",
        feature = "safebooru"
    ))]
    pub(crate) fn with_context(self, provider: Provider, operation: Operation) -> Self {
        if matches!(self, Self::Context { .. }) {
            self
        } else {
            Self::Context {
                provider,
                operation,
                source: Box::new(self),
            }
        }
    }

    /// Returns the provider and operation associated with this error.
    #[must_use]
    pub fn context(&self) -> Option<ErrorContext> {
        match self {
            Self::Context {
                provider,
                operation,
                ..
            } => Some(ErrorContext {
                provider: *provider,
                operation: *operation,
            }),
            _ => None,
        }
    }

    /// Returns the underlying error without provider context.
    #[must_use]
    pub fn source_error(&self) -> &Self {
        match self {
            Self::Context { source, .. } => source.source_error(),
            _ => self,
        }
    }

    #[cfg(any(
        feature = "danbooru",
        feature = "gelbooru",
        feature = "rule34",
        feature = "safebooru"
    ))]
    pub(crate) fn http_status(status: reqwest::StatusCode, body: &str) -> Self {
        const LIMIT: usize = 300;
        let single_line = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let clipped = single_line.chars().count() > LIMIT;
        let mut message: String = single_line.chars().take(LIMIT).collect();
        if clipped {
            message.push('…');
        }
        Self::HttpStatus {
            status: status.as_u16(),
            message,
        }
    }

    /// Returns `true` if this error occurred while sending an HTTP request.
    ///
    /// Response decoding failures are parse errors, even though reqwest reports
    /// them through its general error type.
    #[must_use]
    pub fn is_network_error(&self) -> bool {
        match self {
            Self::Context { source, .. } => source.is_network_error(),
            Self::Request(error) => !error.is_decode(),
            _ => false,
        }
    }

    /// Returns `true` if this error is a parse/deserialization error.
    #[must_use]
    pub fn is_parse_error(&self) -> bool {
        match self {
            Self::Context { source, .. } => source.is_parse_error(),
            Self::Parse(_) => true,
            Self::Request(error) => error.is_decode(),
            _ => false,
        }
    }

    /// Returns `true` if this error indicates the resource was not found.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        match self {
            Self::Context { source, .. } => source.is_not_found(),
            Self::PostNotFound(_) | Self::EmptyResponse => true,
            _ => false,
        }
    }
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
pub(crate) trait ResultContext<T> {
    fn with_context(self, provider: Provider, operation: Operation) -> Result<T>;
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
impl<T> ResultContext<T> for Result<T> {
    fn with_context(self, provider: Provider, operation: Operation) -> Result<T> {
        self.map_err(|error| error.with_context(provider, operation))
    }
}
