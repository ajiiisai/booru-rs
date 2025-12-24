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
// Deprecated types for backwards compatibility
// =============================================================================

#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
use crate::model::danbooru::DanbooruRating;
#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
use crate::model::gelbooru::GelbooruRating;
#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
use crate::model::safebooru::SafebooruRating;

/// Wrapper enum for client-specific ratings.
///
/// # Deprecated
///
/// This enum is deprecated in favor of using the client's associated `Rating` type
/// directly. Each client now has compile-time type safety for its rating type.
///
/// ## Migration
///
/// Before (0.2.x):
/// ```ignore
/// DanbooruClient::builder()
///     .rating(DanbooruRating::General) // Used Rating enum internally
///     .build()
/// ```
///
/// After (0.3.x):
/// ```ignore
/// DanbooruClient::builder()
///     .rating(DanbooruRating::General) // Uses DanbooruRating directly
///     .build()
/// ```
#[deprecated(
    since = "0.3.0",
    note = "Use client-specific rating types directly (e.g., DanbooruRating). \
            The Rating wrapper is no longer needed due to compile-time type safety."
)]
#[derive(Debug, Clone)]
#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
pub enum Rating {
    /// Danbooru-specific rating.
    Danbooru(DanbooruRating),
    /// Gelbooru-specific rating.
    Gelbooru(GelbooruRating),
    /// Safebooru-specific rating.
    Safebooru(SafebooruRating),
}

#[allow(deprecated)]
#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
impl From<DanbooruRating> for Rating {
    fn from(value: DanbooruRating) -> Self {
        Rating::Danbooru(value)
    }
}

#[allow(deprecated)]
#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
impl From<GelbooruRating> for Rating {
    fn from(value: GelbooruRating) -> Self {
        Rating::Gelbooru(value)
    }
}

#[allow(deprecated)]
#[cfg(all(feature = "danbooru", feature = "gelbooru", feature = "safebooru"))]
impl From<SafebooruRating> for Rating {
    fn from(value: SafebooruRating) -> Self {
        Rating::Safebooru(value)
    }
}
