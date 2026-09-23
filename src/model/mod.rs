//! Data models for booru API responses.
//!
//! This module contains the post and rating types for each supported booru site,
//! as well as a common [`Post`] trait that provides a unified interface.

#[cfg(feature = "danbooru")]
pub mod danbooru;
#[cfg(feature = "gelbooru")]
pub mod gelbooru;
#[cfg(feature = "konachan")]
pub mod konachan;
#[cfg(feature = "rule34")]
pub mod rule34;
#[cfg(feature = "safebooru")]
pub mod safebooru;

/// A provider's response rating, with unfamiliar values preserved.
///
/// `General` and `Safe` remain separate because providers use both names.
/// An absent rating is represented by `None` in [`Post::rating`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rating<'a> {
    General,
    Safe,
    Sensitive,
    Questionable,
    Explicit,
    Unknown(&'a str),
}

/// Common interface for post types across different booru sites.
///
/// This trait provides access to the fields that are common across all
/// booru post types, allowing for generic code that works with any booru.
///
/// # Example
///
/// ```no_run
/// use booru_rs::model::Post;
/// use booru_rs::prelude::*;
///
/// fn print_post_info(post: &impl Post) {
///     let height = post.height()
///         .map(|height| height.to_string())
///         .unwrap_or_else(|| "unknown".into());
///     println!("Post #{}: {}x{}", post.id(), post.width(), height);
///     if let Some(url) = post.file_url() {
///         println!("  URL: {}", url);
///     }
/// }
/// ```
pub trait Post {
    /// Returns the unique identifier for this post.
    fn id(&self) -> u32;

    /// Returns the width of the image in pixels.
    fn width(&self) -> u32;

    /// Returns the height of the image in pixels, if available.
    fn height(&self) -> Option<u32>;

    /// Returns the URL to the full-size image, if available.
    fn file_url(&self) -> Option<&str>;

    /// Returns the URL to the preview image, if available.
    fn preview_url(&self) -> Option<&str> {
        None
    }

    /// Returns the URL to the sample image, if available.
    fn sample_url(&self) -> Option<&str> {
        None
    }

    /// Returns the parent post ID, if available.
    fn parent_id(&self) -> Option<u32> {
        None
    }

    /// Returns the tags associated with this post as a single string.
    fn tags(&self) -> &str;

    /// Iterates over whitespace-separated tags without allocating.
    fn tags_iter(&self) -> std::str::SplitWhitespace<'_> {
        self.tags().split_whitespace()
    }

    /// Returns the post's score/rating value, if available.
    fn score(&self) -> Option<i64>;

    /// Returns the MD5 hash of the image, if available.
    fn md5(&self) -> Option<&str>;

    /// Returns the source URL for the image, if available.
    fn source(&self) -> Option<&str>;

    /// Returns the response rating, preserving unknown provider values.
    fn rating(&self) -> Option<Rating<'_>> {
        None
    }
}

// Implement Post trait for all post types
#[cfg(feature = "danbooru")]
impl Post for danbooru::DanbooruPost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.image_width
    }

    fn height(&self) -> Option<u32> {
        Some(self.image_height)
    }

    fn file_url(&self) -> Option<&str> {
        self.file_url.as_deref()
    }

    fn preview_url(&self) -> Option<&str> {
        self.preview_file_url
            .as_deref()
            .filter(|url| !url.is_empty())
    }

    fn sample_url(&self) -> Option<&str> {
        self.large_file_url.as_deref().filter(|url| !url.is_empty())
    }

    fn parent_id(&self) -> Option<u32> {
        self.parent_id
    }

    fn tags(&self) -> &str {
        &self.tag_string
    }

    fn score(&self) -> Option<i64> {
        Some(i64::from(self.score))
    }

    fn md5(&self) -> Option<&str> {
        self.md5.as_deref()
    }

    fn source(&self) -> Option<&str> {
        if self.source.is_empty() {
            None
        } else {
            Some(&self.source)
        }
    }

    fn rating(&self) -> Option<Rating<'_>> {
        use danbooru::{DanbooruPostRating as Response, DanbooruRating as Known};
        self.rating.as_ref().map(|rating| match rating {
            Response::Known(Known::General) => Rating::General,
            Response::Known(Known::Sensitive) => Rating::Sensitive,
            Response::Known(Known::Questionable) => Rating::Questionable,
            Response::Known(Known::Explicit) => Rating::Explicit,
            Response::Unknown(value) => Rating::Unknown(value),
        })
    }
}

#[cfg(feature = "gelbooru")]
impl Post for gelbooru::GelbooruPost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> Option<u32> {
        Some(self.height)
    }

    fn file_url(&self) -> Option<&str> {
        self.file_url.as_deref().filter(|url| !url.is_empty())
    }

    fn preview_url(&self) -> Option<&str> {
        self.preview_url.as_deref().filter(|url| !url.is_empty())
    }

    fn sample_url(&self) -> Option<&str> {
        self.sample_url.as_deref().filter(|url| !url.is_empty())
    }

    fn parent_id(&self) -> Option<u32> {
        self.parent_id
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i64> {
        Some(i64::from(self.score))
    }

    fn md5(&self) -> Option<&str> {
        Some(&self.md5)
    }

    fn source(&self) -> Option<&str> {
        if self.source.is_empty() {
            None
        } else {
            Some(&self.source)
        }
    }

    fn rating(&self) -> Option<Rating<'_>> {
        use gelbooru::{GelbooruPostRating as Response, GelbooruRating as Known};
        Some(match &self.rating {
            Response::Known(Known::General) => Rating::General,
            Response::Known(Known::Safe) => Rating::Safe,
            Response::Known(Known::Sensitive) => Rating::Sensitive,
            Response::Known(Known::Questionable) => Rating::Questionable,
            Response::Known(Known::Explicit) => Rating::Explicit,
            Response::Unknown(value) => Rating::Unknown(value),
        })
    }
}

#[cfg(feature = "safebooru")]
impl Post for safebooru::SafebooruPost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> Option<u32> {
        self.height
    }

    fn file_url(&self) -> Option<&str> {
        self.file_url.as_deref().filter(|url| !url.is_empty())
    }

    fn preview_url(&self) -> Option<&str> {
        self.preview_url.as_deref().filter(|url| !url.is_empty())
    }

    fn sample_url(&self) -> Option<&str> {
        self.sample_url.as_deref().filter(|url| !url.is_empty())
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i64> {
        self.score.map(i64::from)
    }

    fn md5(&self) -> Option<&str> {
        Some(&self.hash)
    }

    fn source(&self) -> Option<&str> {
        if self.source.is_empty() {
            None
        } else {
            Some(&self.source)
        }
    }

    fn rating(&self) -> Option<Rating<'_>> {
        use safebooru::{SafebooruPostRating as Response, SafebooruRating as Known};
        Some(match &self.rating {
            Response::Known(Known::General) => Rating::General,
            Response::Known(Known::Safe) => Rating::Safe,
            Response::Known(Known::Questionable) => Rating::Questionable,
            Response::Known(Known::Explicit) => Rating::Explicit,
            Response::Unknown(value) => Rating::Unknown(value),
        })
    }
}

#[cfg(feature = "rule34")]
impl Post for rule34::Rule34Post {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> Option<u32> {
        Some(self.height)
    }

    fn file_url(&self) -> Option<&str> {
        self.file_url.as_deref()
    }

    fn preview_url(&self) -> Option<&str> {
        self.preview_url.as_deref().filter(|url| !url.is_empty())
    }

    fn sample_url(&self) -> Option<&str> {
        self.sample_url.as_deref().filter(|url| !url.is_empty())
    }

    fn parent_id(&self) -> Option<u32> {
        self.parent_id
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i64> {
        Some(i64::from(self.score))
    }

    fn md5(&self) -> Option<&str> {
        if self.hash.is_empty() {
            None
        } else {
            Some(&self.hash)
        }
    }

    fn source(&self) -> Option<&str> {
        if self.source.is_empty() {
            None
        } else {
            Some(&self.source)
        }
    }

    fn rating(&self) -> Option<Rating<'_>> {
        use rule34::{Rule34PostRating as Response, Rule34Rating as Known};
        Some(match &self.rating {
            Response::Known(Known::General) => Rating::General,
            Response::Known(Known::Safe) => Rating::Safe,
            Response::Known(Known::Sensitive) => Rating::Sensitive,
            Response::Known(Known::Questionable) => Rating::Questionable,
            Response::Known(Known::Explicit) => Rating::Explicit,
            Response::Unknown(value) => Rating::Unknown(value),
        })
    }
}

#[cfg(feature = "konachan")]
impl Post for konachan::KonachanPost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> Option<u32> {
        Some(self.height)
    }

    fn file_url(&self) -> Option<&str> {
        self.file_url.as_deref().filter(|url| !url.is_empty())
    }

    fn preview_url(&self) -> Option<&str> {
        self.preview_url.as_deref().filter(|url| !url.is_empty())
    }

    fn sample_url(&self) -> Option<&str> {
        self.sample_url.as_deref().filter(|url| !url.is_empty())
    }

    fn parent_id(&self) -> Option<u32> {
        self.parent_id
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i64> {
        Some(i64::from(self.score))
    }

    fn md5(&self) -> Option<&str> {
        if self.md5.is_empty() {
            None
        } else {
            Some(&self.md5)
        }
    }

    fn source(&self) -> Option<&str> {
        if self.source.is_empty() {
            None
        } else {
            Some(&self.source)
        }
    }

    fn rating(&self) -> Option<Rating<'_>> {
        use konachan::{KonachanPostRating as Response, KonachanRating as Known};
        Some(match &self.rating {
            Response::Known(Known::Safe) => Rating::Safe,
            Response::Known(Known::Questionable) => Rating::Questionable,
            Response::Known(Known::Explicit) => Rating::Explicit,
            Response::Unknown(value) => Rating::Unknown(value),
        })
    }
}
