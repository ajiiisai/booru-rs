//! Models for Safebooru API responses.
//!
//! This module contains the data structures for deserializing
//! responses from the Safebooru API.

use core::fmt;
use serde::{Deserialize, Serialize};

/// A post from Safebooru.
///
/// This struct represents a single image post from Safebooru.
/// Safebooru is a SFW-only booru site.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SafebooruPost {
    pub id: u32,
    pub score: Option<u32>,
    /// Image height in pixels, if available.
    pub height: Option<u32>,
    pub width: u32,
    pub hash: String,
    pub tags: String,
    pub image: String,
    /// Directory number where the image is stored
    pub directory: u32,
    /// Full URL to the image file
    pub file_url: String,
    /// URL to the preview/thumbnail image
    pub preview_url: String,
    /// URL to the sample (resized) image
    pub sample_url: String,
    /// Source URL for the original artwork
    #[serde(default)]
    pub source: String,
    /// This is basically equivalent to `updated_at` in a Danbooru post. Except
    /// that it's provided as a UNIX timestamp. Safebooru provides no `created_at`
    /// field.
    pub change: u32,
    pub rating: SafebooruPostRating,
}

/// A Safebooru response rating, including values unknown to this crate.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum SafebooruPostRating {
    Known(SafebooruRating),
    Unknown(String),
}

impl From<SafebooruRating> for SafebooruPostRating {
    fn from(rating: SafebooruRating) -> Self {
        Self::Known(rating)
    }
}

impl fmt::Display for SafebooruPostRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(rating) => rating.fmt(f),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

/// Supported typed rating filters for Safebooru.
///
/// While Safebooru is primarily a SFW site, the rating field
/// can contain other values for deleted/hidden content.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SafebooruRating {
    Safe,
    General,
    // Yes there are explicit and questionable posts. Though you only need to care
    // about them if you're querying for deleted content.
    Questionable,
    Explicit,
}

impl From<SafebooruRating> for String {
    fn from(rating: SafebooruRating) -> String {
        rating.to_string()
    }
}

impl fmt::Display for SafebooruRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = format!("{:?}", self).to_lowercase();
        write!(f, "{tag}")
    }
}
