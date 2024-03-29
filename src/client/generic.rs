use crate::model::{
    danbooru::DanbooruRating, gelbooru::GelbooruRating, safebooru::SafebooruRating,
};
use std::fmt;

pub enum Rating {
    Danbooru(DanbooruRating),
    Gelbooru(GelbooruRating),
    Safebooru(SafebooruRating),
}

impl From<DanbooruRating> for Rating {
    fn from(value: DanbooruRating) -> Self {
        Rating::Danbooru(value)
    }
}

impl From<GelbooruRating> for Rating {
    fn from(value: GelbooruRating) -> Self {
        Rating::Gelbooru(value)
    }
}

impl From<SafebooruRating> for Rating {
    fn from(value: SafebooruRating) -> Self {
        Rating::Safebooru(value)
    }
}

#[derive(Debug, Clone)]
pub enum Sort {
    Id,
    Score,
    Rating,
    User,
    Height,
    Width,
    Source,
    Updated,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lowercase_tag = format!("{:?}", self).to_lowercase();
        write!(f, "{lowercase_tag}")
    }
}
