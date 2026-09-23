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
    feature = "safebooru",
    feature = "konachan"
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
    feature = "safebooru",
    feature = "konachan"
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

#[cfg(all(
    test,
    any(
        feature = "danbooru",
        feature = "gelbooru",
        feature = "rule34",
        feature = "safebooru",
        feature = "konachan"
    )
))]
mod tests {
    use super::QueryCore;
    use crate::error::BooruError;

    #[test]
    fn blacklist_tags_reject_empty_values() {
        for query in [
            QueryCore::new().blacklist_tag(""),
            QueryCore::new().blacklist_tags([""]),
        ] {
            assert!(matches!(
                query.validate_common(),
                Err(BooruError::InvalidTag { .. })
            ));
        }
    }
}

/// Shared builder state behind provider clients.
///
/// Holds the HTTP client, endpoint, and request policy. Providers wrap this in
/// their `ClientBuilder` so the configuration methods live in one place while
/// `build()` and credential methods keep provider-specific types.
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru",
    feature = "konachan"
))]
#[derive(Debug, Clone, Default)]
pub(crate) struct BuilderCore {
    pub(crate) http: Option<reqwest::Client>,
    pub(crate) endpoint: Option<String>,
    pub(crate) policy: Option<super::RequestPolicy>,
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru",
    feature = "konachan"
))]
impl BuilderCore {
    pub(crate) fn endpoint(mut self, url: impl Into<String>) -> crate::error::Result<Self> {
        self.endpoint = Some(super::validate_endpoint(&url.into())?);
        Ok(self)
    }

    pub(crate) fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Sets the request retry and rate-limit policy.
    pub(crate) fn request_policy(mut self, policy: super::RequestPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Sets the retry configuration for requests made by this client.
    pub(crate) fn retry_config(
        mut self,
        config: crate::retry::RetryConfig,
    ) -> crate::error::Result<Self> {
        let policy = self.policy.take().unwrap_or_default();
        self.policy = Some(policy.with_retry_config(config)?);
        Ok(self)
    }

    /// Sets the rate limiter for requests made by this client.
    pub(crate) fn rate_limiter(mut self, limiter: crate::ratelimit::RateLimiter) -> Self {
        let policy = self.policy.take().unwrap_or_default();
        self.policy = Some(policy.with_rate_limiter(limiter));
        self
    }
}
