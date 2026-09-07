//! Generic types used across booru clients.

use std::fmt;

/// Sort order for post queries.
///
/// These are the common sort options available on most booru sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sort {
    /// Sort by post ID.
    Id,
    /// Sort by score/votes.
    Score,
    /// Sort by rating.
    Rating,
    /// Sort by uploader.
    User,
    /// Sort by image height.
    Height,
    /// Sort by image width.
    Width,
    /// Sort by source URL.
    Source,
    /// Sort by last update time.
    Updated,
    /// Random ordering.
    Random,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Id => "id",
            Self::Score => "score",
            Self::Rating => "rating",
            Self::User => "user",
            Self::Height => "height",
            Self::Width => "width",
            Self::Source => "source",
            Self::Updated => "updated",
            Self::Random => "random",
        };
        write!(f, "{s}")
    }
}

// =============================================================================

/// Shared query state behind provider searches.
///
/// Holds literal tags, raw expressions, rating and sort filters, and the page
/// size. Providers wrap this in their typed `Query` so the builder bodies,
/// validation, and tag assembly live in one place while `rating()` and
/// `sort()` keep provider-specific types.
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueryCore {
    pub(crate) tags: Vec<String>,
    pub(crate) raw_queries: Vec<String>,
    pub(crate) rating: Option<String>,
    pub(crate) sort: Option<String>,
    pub(crate) limit: u32,
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru"
))]
impl QueryCore {
    pub(crate) fn new() -> Self {
        Self {
            tags: Vec::new(),
            raw_queries: Vec::new(),
            rating: None,
            sort: None,
            limit: 100,
        }
    }

    pub(crate) fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub(crate) fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self = self.tag(tag);
        }
        self
    }

    /// Adds a provider query expression without literal-tag validation.
    pub(crate) fn raw_query(mut self, expression: impl Into<String>) -> Self {
        self.raw_queries.push(expression.into());
        self
    }

    pub(crate) fn raw_queries<I, S>(mut self, expressions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for expression in expressions {
            self = self.raw_query(expression);
        }
        self
    }

    pub(crate) fn rating(mut self, rating: impl Into<String>) -> Self {
        self.rating = Some(rating.into());
        self
    }

    pub(crate) fn sort(mut self, order: impl Into<String>) -> Self {
        self.sort = Some(order.into());
        self
    }

    pub(crate) fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub(crate) fn blacklist_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.tags.push(format!("-{}", tag.as_ref()));
        self
    }

    pub(crate) fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for tag in tags {
            self = self.blacklist_tag(tag);
        }
        self
    }

    pub(crate) fn exclude_rating(mut self, rating: impl Into<String>) -> Self {
        self.tags.push(format!("-rating:{}", rating.into()));
        self
    }

    pub(crate) fn random(mut self, prefix: &str) -> Self {
        self.tags.push(format!("{prefix}random"));
        self
    }

    pub(crate) fn validate_common(&self) -> crate::error::Result<()> {
        super::validate_tags(&self.tags)?;
        super::validate_raw_queries(
            &self.raw_queries,
            self.rating.is_some(),
            self.sort.is_some(),
        )?;
        super::validate_random_conflict(&self.tags, &self.raw_queries, self.sort.is_some())
    }

    pub(crate) fn assemble_tags(&self, sort_prefix: &str) -> String {
        let mut tags = self.tags.clone();
        if let Some(rating) = &self.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = &self.sort {
            tags.push(format!("{sort_prefix}{sort}"));
        }
        tags.extend(self.raw_queries.iter().cloned());
        tags.join(" ")
    }
}

// =============================================================================
