//! Models for Rule34 API responses.
//!
//! This module contains the data structures for deserializing
//! responses from the Rule34 API.

use core::fmt;
use serde::{Deserialize, Serialize};

/// A post from Rule34.
///
/// This struct represents a single image post from Rule34.
/// Rule34 is an NSFW booru site.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Rule34Post {
    /// The ID of the post
    pub id: u32,
    /// Post's score
    pub score: i32,
    /// Post's image width
    pub width: u32,
    /// Post's image height
    pub height: u32,
    /// Post's image file url
    pub file_url: String,
    /// Post's preview/thumbnail url
    pub preview_url: String,
    /// Post's sample (resized) url  
    pub sample_url: String,
    /// Post's tags (space-separated)
    pub tags: String,
    /// Post's rating
    pub rating: Rule34Rating,
    /// Post's source
    #[serde(default)]
    pub source: String,
    /// Whether the post has notes
    #[serde(default)]
    pub has_notes: bool,
    /// Number of comments
    #[serde(default)]
    pub comment_count: u32,
    /// Post owner/uploader
    #[serde(default)]
    pub owner: String,
    /// Parent post ID (0 if none)
    #[serde(default)]
    pub parent_id: u32,
    /// Post status
    #[serde(default)]
    pub status: String,
    /// Change timestamp (Unix time)
    #[serde(default)]
    pub change: u64,
    /// Directory number
    #[serde(default)]
    pub directory: u32,
    /// Image filename
    #[serde(default)]
    pub image: String,
    /// Image hash
    #[serde(default)]
    pub hash: String,
}

/// Post rating classification for Rule34.
///
/// Rule34 is an NSFW site, so most content is explicit or questionable.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Rule34Rating {
    Explicit,
    Questionable,
    Safe,
    General,
    Sensitive,
}

impl From<Rule34Rating> for String {
    fn from(rating: Rule34Rating) -> String {
        rating.to_string()
    }
}

impl fmt::Display for Rule34Rating {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lowercase_tag = format!("{:?}", self).to_lowercase();
        write!(f, "{lowercase_tag}")
    }
}
