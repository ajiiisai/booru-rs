//! Tag autocomplete for booru sites.
//!
//! This module provides tag suggestion/autocomplete functionality,
//! useful for building search UIs and validating tag names.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::prelude::*;
//! use booru_rs::autocomplete::Autocomplete;
//!
//! # async fn example() -> booru_rs::error::Result<()> {
//! // Get tag suggestions starting with "cat_"
//! let suggestions = DanbooruClient::autocomplete("cat_", 10).await?;
//!
//! for tag in suggestions {
//!     println!("{}: {} posts", tag.name, tag.post_count.unwrap_or(0));
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// A tag suggestion from autocomplete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSuggestion {
    /// The tag name (with underscores).
    pub name: String,
    /// Human-readable label (may include post count or spaces).
    pub label: String,
    /// Number of posts with this tag (if available).
    pub post_count: Option<u32>,
    /// Tag category (0=general, 1=artist, 3=copyright, 4=character, 5=meta).
    pub category: Option<u8>,
}

impl TagSuggestion {
    /// Creates a new tag suggestion.
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            post_count: None,
            category: None,
        }
    }

    /// Creates a tag suggestion with post count.
    pub fn with_count(name: impl Into<String>, label: impl Into<String>, post_count: u32) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            post_count: Some(post_count),
            category: None,
        }
    }

    /// Returns the category name based on the category ID.
    #[must_use]
    pub fn category_name(&self) -> Option<&'static str> {
        self.category.map(|c| match c {
            0 => "general",
            1 => "artist",
            3 => "copyright",
            4 => "character",
            5 => "meta",
            _ => "unknown",
        })
    }
}

/// Trait for clients that support tag autocomplete.
///
/// # Example
///
/// ```no_run
/// use booru_rs::prelude::*;
/// use booru_rs::autocomplete::Autocomplete;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// let suggestions = SafebooruClient::autocomplete("land", 5).await?;
/// for tag in suggestions {
///     println!("{}", tag.name);
/// }
/// # Ok(())
/// # }
/// ```
pub trait Autocomplete {
    /// Returns tag suggestions matching the given query prefix.
    ///
    /// # Arguments
    ///
    /// * `query` - The prefix to search for (e.g., "cat_" for tags starting with "cat_")
    /// * `limit` - Maximum number of suggestions to return
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response cannot be parsed.
    fn autocomplete(
        query: &str,
        limit: u32,
    ) -> impl std::future::Future<Output = Result<Vec<TagSuggestion>>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_suggestion_new() {
        let tag = TagSuggestion::new("cat_ears", "cat ears");
        assert_eq!(tag.name, "cat_ears");
        assert_eq!(tag.label, "cat ears");
        assert_eq!(tag.post_count, None);
        assert_eq!(tag.category, None);
    }

    #[test]
    fn test_tag_suggestion_with_count() {
        let tag = TagSuggestion::with_count("cat_ears", "cat ears (12345)", 12345);
        assert_eq!(tag.name, "cat_ears");
        assert_eq!(tag.label, "cat ears (12345)");
        assert_eq!(tag.post_count, Some(12345));
        assert_eq!(tag.category, None);
    }

    #[test]
    fn test_category_name_general() {
        let mut tag = TagSuggestion::new("test", "test");
        tag.category = Some(0);
        assert_eq!(tag.category_name(), Some("general"));
    }

    #[test]
    fn test_category_name_artist() {
        let mut tag = TagSuggestion::new("test", "test");
        tag.category = Some(1);
        assert_eq!(tag.category_name(), Some("artist"));
    }

    #[test]
    fn test_category_name_copyright() {
        let mut tag = TagSuggestion::new("test", "test");
        tag.category = Some(3);
        assert_eq!(tag.category_name(), Some("copyright"));
    }

    #[test]
    fn test_category_name_character() {
        let mut tag = TagSuggestion::new("test", "test");
        tag.category = Some(4);
        assert_eq!(tag.category_name(), Some("character"));
    }

    #[test]
    fn test_category_name_meta() {
        let mut tag = TagSuggestion::new("test", "test");
        tag.category = Some(5);
        assert_eq!(tag.category_name(), Some("meta"));
    }

    #[test]
    fn test_category_name_unknown() {
        let mut tag = TagSuggestion::new("test", "test");
        tag.category = Some(99);
        assert_eq!(tag.category_name(), Some("unknown"));
    }

    #[test]
    fn test_category_name_none() {
        let tag = TagSuggestion::new("test", "test");
        assert_eq!(tag.category_name(), None);
    }

    #[test]
    fn test_tag_suggestion_equality() {
        let tag1 = TagSuggestion::with_count("cat_ears", "cat ears", 100);
        let tag2 = TagSuggestion::with_count("cat_ears", "cat ears", 100);
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn test_tag_suggestion_clone() {
        let tag1 = TagSuggestion::with_count("cat_ears", "cat ears", 100);
        let tag2 = tag1.clone();
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn test_tag_suggestion_serialize() {
        let tag = TagSuggestion::with_count("cat_ears", "cat ears", 100);
        let json = serde_json::to_string(&tag).unwrap();
        assert!(json.contains("cat_ears"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_tag_suggestion_deserialize() {
        let json = r#"{"name":"cat_ears","label":"cat ears","post_count":100,"category":0}"#;
        let tag: TagSuggestion = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "cat_ears");
        assert_eq!(tag.post_count, Some(100));
        assert_eq!(tag.category, Some(0));
    }
}
