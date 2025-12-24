//! Models for Danbooru API responses.
//!
//! This module contains the data structures for deserializing
//! responses from the Danbooru API.

use core::fmt;
use serde::{Deserialize, Serialize};

/// A post from Danbooru.
///
/// This struct represents a single image post from Danbooru,
/// including all metadata like tags, ratings, and file information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DanbooruPost {
    pub id: u32,
    pub created_at: String,
    pub updated_at: String,
    pub uploader_id: u32,
    pub approver_id: Option<u32>,
    pub tag_string: String,
    pub tag_string_general: String,
    pub tag_string_artist: String,
    pub tag_string_copyright: String,
    pub tag_string_character: String,
    pub tag_string_meta: String,
    pub rating: Option<DanbooruRating>,
    pub parent_id: Option<u32>,
    pub pixiv_id: Option<u32>,
    pub source: String,
    pub md5: Option<String>,
    pub file_url: Option<String>,
    pub large_file_url: Option<String>,
    pub preview_file_url: Option<String>,
    pub file_ext: String,
    pub file_size: u32,
    pub image_width: u32,
    pub image_height: u32,
    pub score: i32,
    pub up_score: i32,
    pub down_score: i32,
    pub fav_count: u32,
    pub tag_count_general: u32,
    pub tag_count_artist: u32,
    pub tag_count_copyright: u32,
    pub tag_count_character: u32,
    pub tag_count_meta: u32,
    pub last_comment_bumped_at: Option<String>,
    pub last_noted_at: Option<String>,
    pub has_large: bool,
    pub has_children: bool,
    pub has_visible_children: bool,
    pub has_active_children: bool,
    pub is_banned: bool,
    pub is_deleted: bool,
    pub is_flagged: bool,
    pub is_pending: bool,
    pub bit_flags: u32,
}

/// Post rating classification.
///
/// Danbooru uses a four-tier rating system. See the
/// [Danbooru ratings wiki](https://danbooru.donmai.us/wiki_pages/howto:rate)
/// for detailed information.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DanbooruRating {
    #[serde(rename = "e")]
    Explicit,
    #[serde(rename = "q")]
    Questionable,
    #[serde(rename = "s")]
    Sensitive,
    #[serde(rename = "g")]
    General,
}

impl From<DanbooruRating> for String {
    fn from(rating: DanbooruRating) -> String {
        rating.to_string()
    }
}

impl fmt::Display for DanbooruRating {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lowercase_tag = format!("{:?}", self).to_lowercase();
        write!(f, "{lowercase_tag}")
    }
}
