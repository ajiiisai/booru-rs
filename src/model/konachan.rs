//! Models for Konachan API responses.
//!
//! This module contains the data structures for deserializing
//! responses from the Konachan API.

use core::fmt;
use serde::{Deserialize, Serialize};

/// A post from Konachan.
///
/// This struct represents a single image post from Konachan.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct KonachanPost {
    /// The ID of the post
    pub id: u32,
    /// Unix timestamp of the post's creation date
    pub created_at: u32,
    /// Unix timestamp of the post's update date
    pub updated_at: u32,
    // Not sure about this one.
    pub change: u32,
    /// Post's score
    pub score: u32,
    /// Post's image width
    pub width: u32,
    /// Post's image height
    pub height: u32,
    /// Post's image md5
    pub md5: String,
    /// Post's image file url, if available.
    #[serde(default)]
    pub file_url: Option<String>,
    /// File size in bytes, if supplied
    #[serde(default)]
    pub file_size: Option<u32>,
    /// File extension, if supplied
    #[serde(default)]
    pub file_ext: Option<String>,
    /// Preview URL, if supplied.
    #[serde(default)]
    pub preview_url: Option<String>,
    /// Preview width in pixels, if supplied
    #[serde(default)]
    pub preview_width: Option<u32>,
    /// Preview height in pixels, if supplied
    #[serde(default)]
    pub preview_height: Option<u32>,
    /// Sample URL, if supplied.
    #[serde(default)]
    pub sample_url: Option<String>,
    /// Sample width in pixels, if supplied
    #[serde(default)]
    pub sample_width: Option<u32>,
    /// Sample height in pixels, if supplied
    #[serde(default)]
    pub sample_height: Option<u32>,
    /// Sample file size in bytes
    #[serde(default)]
    pub sample_file_size: Option<u32>,
    /// Post's tags
    pub tags: String,
    /// Source URL for the original artwork
    #[serde(default)]
    pub source: String,
    /// Post's rating
    pub rating: KonachanPostRating,
    /// Creator ID, if supplied.
    #[serde(default)]
    pub creator_id: Option<u32>,
    /// Approver ID, if supplied
    #[serde(default)]
    pub approver_id: Option<u32>,
    /// Name of the author
    pub author: String,
    /// Whether the post has children
    pub has_children: bool,
    /// Parent post ID, if supplied.
    #[serde(default)]
    pub parent_id: Option<u32>,
    /// Post status, if supplied.
    #[serde(default)]
    pub status: Option<String>,
    pub is_pending: bool,
    pub is_held: bool,
    pub is_note_locked: bool,
    /// Unix timestamp of the latest note, if supplied
    #[serde(default)]
    pub last_noted_at: Option<u32>,
    /// Unix timestamp of the latest comment, if supplied
    #[serde(default)]
    pub last_commented_at: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum KonachanPostRating {
    Known(KonachanRating),
    Unknown(String),
}

impl From<KonachanRating> for KonachanPostRating {
    fn from(rating: KonachanRating) -> Self {
        Self::Known(rating)
    }
}

impl fmt::Display for KonachanPostRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(rating) => rating.fmt(f),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

/// Supported typed rating filters for Konachan.
///
/// See the [Konachan ratings wiki](https://konachan.com/help/ratings)
/// for detailed information.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum KonachanRating {
    #[serde(rename = "e")]
    Explicit,
    #[serde(rename = "q")]
    Questionable,
    #[serde(rename = "s")]
    Safe,
}

impl From<KonachanRating> for String {
    fn from(rating: KonachanRating) -> String {
        rating.to_string()
    }
}

impl fmt::Display for KonachanRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = format!("{:?}", self).to_lowercase();
        write!(f, "{tag}")
    }
}
