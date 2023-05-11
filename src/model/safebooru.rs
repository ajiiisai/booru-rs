use core::fmt;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SafebooruPost {
    pub id: u32,
    pub score: u32,
    pub height: u32,
    pub width: u32,
    pub hash: String,
    pub tags: String,
    pub image: String,
    /// This is basically equivalent to `updated_at` in a Danbooru post. Except
    /// that it's provided as a UNIX timestamp. Safebooru provides no `created_at`
    /// field.
    pub change: u32,
    pub rating: SafebooruRating,
}

#[derive(Deserialize, Debug, Clone)]
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
