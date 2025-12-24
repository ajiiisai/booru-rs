//! Data models for booru API responses.
//!
//! This module contains the post and rating types for each supported booru site,
//! as well as a common [`Post`] trait that provides a unified interface.

#[cfg(feature = "danbooru")]
pub mod danbooru;
#[cfg(feature = "gelbooru")]
pub mod gelbooru;
#[cfg(feature = "rule34")]
pub mod rule34;
#[cfg(feature = "safebooru")]
pub mod safebooru;

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
///     println!("Post #{}: {}x{}", post.id(), post.width(), post.height());
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

    /// Returns the height of the image in pixels.
    fn height(&self) -> u32;

    /// Returns the URL to the full-size image, if available.
    fn file_url(&self) -> Option<&str>;

    /// Returns the tags associated with this post as a single string.
    fn tags(&self) -> &str;

    /// Returns the post's score/rating value, if available.
    fn score(&self) -> Option<i32>;

    /// Returns the MD5 hash of the image, if available.
    fn md5(&self) -> Option<&str>;

    /// Returns the source URL for the image, if available.
    fn source(&self) -> Option<&str>;
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

    fn height(&self) -> u32 {
        self.image_height
    }

    fn file_url(&self) -> Option<&str> {
        self.file_url.as_deref()
    }

    fn tags(&self) -> &str {
        &self.tag_string
    }

    fn score(&self) -> Option<i32> {
        Some(self.score)
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
}

#[cfg(feature = "gelbooru")]
impl Post for gelbooru::GelbooruPost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn file_url(&self) -> Option<&str> {
        Some(&self.file_url)
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i32> {
        Some(self.score as i32)
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
}

#[cfg(feature = "safebooru")]
impl Post for safebooru::SafebooruPost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn file_url(&self) -> Option<&str> {
        Some(&self.file_url)
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i32> {
        self.score.map(|s| s as i32)
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
}

#[cfg(feature = "rule34")]
impl Post for rule34::Rule34Post {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn file_url(&self) -> Option<&str> {
        Some(&self.file_url)
    }

    fn tags(&self) -> &str {
        &self.tags
    }

    fn score(&self) -> Option<i32> {
        Some(self.score)
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
}
