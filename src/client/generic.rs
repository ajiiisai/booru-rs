//! Generic types used across booru clients.

use std::fmt;

/// Sort order for post queries.
///
/// These are the common sort options available on most booru sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sort {
    /// Sort by post ID.
    Id,
    /// Sort by score/votes.
    Score,
    /// Sort by rating.
    Rating,
    /// Sort by uploader.
    User,
    /// Sort by image height.
    Height,
    /// Sort by image width.
    Width,
    /// Sort by source URL.
    Source,
    /// Sort by last update time.
    Updated,
    /// Random ordering.
    Random,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Id => "id",
            Self::Score => "score",
            Self::Rating => "rating",
            Self::User => "user",
            Self::Height => "height",
            Self::Width => "width",
            Self::Source => "source",
            Self::Updated => "updated",
            Self::Random => "random",
        };
        write!(f, "{s}")
    }
}

// =============================================================================
