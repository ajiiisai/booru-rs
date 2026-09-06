//! Models for Gelbooru API responses.
//!
//! This module contains the data structures for deserializing
//! responses from the Gelbooru API.

use core::fmt;
use serde::de::{Deserializer, Error as DeError};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(u64),
}

fn deserialize_optional_stringish<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<StringOrNumber>::deserialize(deserializer)?;
    Ok(value.map(|value| match value {
        StringOrNumber::String(value) => value,
        StringOrNumber::Number(value) => value.to_string(),
    }))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Boolish {
    Bool(bool),
    Number(u64),
    String(String),
}

fn deserialize_optional_boolish<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Boolish>::deserialize(deserializer)?;
    value
        .map(|value| match value {
            Boolish::Bool(value) => Ok(value),
            Boolish::Number(0) => Ok(false),
            Boolish::Number(1) => Ok(true),
            Boolish::Number(value) => Err(D::Error::custom(format!(
                "expected a boolean or 0/1, got number {value}"
            ))),
            Boolish::String(value) => match value.trim().to_ascii_lowercase().as_str() {
                "false" | "0" => Ok(false),
                "true" | "1" => Ok(true),
                _ => Err(D::Error::custom(format!(
                    "expected a boolean or true/false, got string {value:?}"
                ))),
            },
        })
        .transpose()
}

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
    pub rating: GelbooruPostRating,
    /// Directory path, if supplied.
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    pub directory: Option<String>,
    /// Change timestamp, if supplied.
    #[serde(default)]
    pub change: Option<u64>,
    /// Post owner, if supplied.
    #[serde(default)]
    pub owner: Option<String>,
    /// Creator ID, if supplied.
    #[serde(default)]
    pub creator_id: Option<u32>,
    /// Parent post ID, if supplied.
    #[serde(default)]
    pub parent_id: Option<u32>,
    /// Whether the post has a sample, if supplied.
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    pub sample: Option<bool>,
    /// Preview height in pixels, if supplied.
    #[serde(default)]
    pub preview_height: Option<u32>,
    /// Preview width in pixels, if supplied.
    #[serde(default)]
    pub preview_width: Option<u32>,
    /// Post title, if supplied.
    #[serde(default)]
    pub title: Option<String>,
    /// Whether the post has notes, if supplied.
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    pub has_notes: Option<bool>,
    /// Whether the post has comments, if supplied.
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    pub has_comments: Option<bool>,
    /// Preview URL, if supplied.
    #[serde(default)]
    pub preview_url: Option<String>,
    /// Sample URL, if supplied.
    #[serde(default)]
    pub sample_url: Option<String>,
    /// Sample height in pixels, if supplied.
    #[serde(default)]
    pub sample_height: Option<u32>,
    /// Sample width in pixels, if supplied.
    #[serde(default)]
    pub sample_width: Option<u32>,
    /// Post status, if supplied.
    #[serde(default)]
    pub status: Option<String>,
    /// Whether the post is locked, if supplied.
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    pub post_locked: Option<bool>,
    /// Whether the post has children, if supplied.
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    pub has_children: Option<bool>,
}

/// Wrapper for Gelbooru's API response containing a list of posts.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct GelbooruResponse {
    #[serde(rename = "post")]
    #[serde(default)]
    pub posts: Vec<GelbooruPost>,
}

/// A Gelbooru response rating, including values unknown to this crate.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum GelbooruPostRating {
    Known(GelbooruRating),
    Unknown(String),
}

impl From<GelbooruRating> for GelbooruPostRating {
    fn from(rating: GelbooruRating) -> Self {
        Self::Known(rating)
    }
}

impl fmt::Display for GelbooruPostRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(rating) => rating.fmt(f),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

/// Supported typed rating filters for Gelbooru.
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
