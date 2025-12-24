//! Models for Gelbooru API responses.
//!
//! This module contains the data structures for deserializing
//! responses from the Gelbooru API.

use core::fmt;
use serde::{Deserialize, Serialize};

/// A post from Gelbooru.
///
/// This struct represents a single image post from Gelbooru.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GelbooruPost {
    /// The ID of the post
    pub id: u32,
    /// Datestamp of the post's creating date
    pub created_at: String,
    /// Post's score
    pub score: u32,
    /// Post's image width
    pub width: u32,
    /// Post's image height
    pub height: u32,
    /// Post's image md5
    pub md5: String,
    /// Post's image file url
    pub file_url: String,
    /// Post's tags
    pub tags: String,
    /// Post's image name (with extension)
    pub image: String,
    /// Post's image source
    pub source: String,
    /// Post's rating
    pub rating: GelbooruRating,
}

/// Wrapper for Gelbooru's API response containing a list of posts.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GelbooruResponse {
    #[serde(rename = "post")]
    pub posts: Vec<GelbooruPost>,
}

/// Post rating classification for Gelbooru.
///
/// See the [Gelbooru ratings wiki](https://gelbooru.com/index.php?page=help&topic=rating)
/// for detailed information.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GelbooruRating {
    Explicit,
    Questionable,
    Safe,
    Sensitive,
    General,
}

impl From<GelbooruRating> for String {
    fn from(rating: GelbooruRating) -> String {
        rating.to_string()
    }
}

impl fmt::Display for GelbooruRating {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lovercase_tag = format!("{:?}", self).to_lowercase();
        write!(f, "{lovercase_tag}")
    }
}
