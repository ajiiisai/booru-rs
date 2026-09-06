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
#[serde(from = "Rule34PostWire")]
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
    pub file_url: Option<String>,
    /// Post's preview/thumbnail url
    pub preview_url: Option<String>,
    /// Post's sample (resized) url  
    pub sample_url: Option<String>,
    /// Provider's sample flag, if supplied.
    pub sample: Option<bool>,
    /// Sample height in pixels, if supplied.
    pub sample_height: Option<u32>,
    /// Sample width in pixels, if supplied.
    pub sample_width: Option<u32>,
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
    /// Parent post ID, if available.
    #[serde(default)]
    pub parent_id: Option<u32>,
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

#[derive(Deserialize)]
struct Rule34PostWire {
    id: u32,
    score: i32,
    width: u32,
    height: u32,
    file_url: Option<String>,
    preview_url: Option<String>,
    sample_url: Option<String>,
    sample: Option<bool>,
    sample_height: Option<u32>,
    sample_width: Option<u32>,
    tags: String,
    rating: Rule34Rating,
    #[serde(default)]
    source: String,
    #[serde(default)]
    has_notes: bool,
    #[serde(default)]
    comment_count: u32,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    parent_id: Option<u32>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    change: u64,
    #[serde(default)]
    directory: u32,
    #[serde(default)]
    image: String,
    #[serde(default)]
    hash: String,
}

impl From<Rule34PostWire> for Rule34Post {
    fn from(wire: Rule34PostWire) -> Self {
        Self {
            id: wire.id,
            score: wire.score,
            width: wire.width,
            height: wire.height,
            file_url: wire.file_url.filter(|url| !url.is_empty()),
            preview_url: wire.preview_url.filter(|url| !url.is_empty()),
            sample_url: wire.sample_url.filter(|url| !url.is_empty()),
            sample: wire.sample,
            sample_height: wire.sample_height,
            sample_width: wire.sample_width,
            tags: wire.tags,
            rating: wire.rating,
            source: wire.source,
            has_notes: wire.has_notes,
            comment_count: wire.comment_count,
            owner: wire.owner,
            parent_id: wire.parent_id.filter(|id| *id != 0),
            status: wire.status,
            change: wire.change,
            directory: wire.directory,
            image: wire.image,
            hash: wire.hash,
        }
    }
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
