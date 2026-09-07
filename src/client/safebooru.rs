use serde::Deserialize;

use super::{RequestPolicy, execute_with_policy};
use crate::autocomplete::TagSuggestion;
use crate::client::generic::{QueryCore, Sort};
use crate::error::{BooruError, Operation, Provider, Result, ResultContext};
use crate::model::safebooru::{SafebooruPost, SafebooruRating};
use crate::ratelimit::RateLimiter;
use crate::retry::RetryConfig;

#[derive(Debug, Deserialize)]
struct SafebooruAutocompleteItem {
    value: String,
    label: String,
}

/// Parses post count from a label like "cat_ears (177448)".
fn parse_post_count_from_label(label: &str) -> Option<u32> {
    let start = label.rfind('(')?;
    let end = label.rfind(')')?;
    if start < end {
        label[start + 1..end].parse().ok()
    } else {
        None
    }
}

const DEFAULT_ENDPOINT: &str = "https://safebooru.org";
const SORT_PREFIX: &str = "sort:";

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: String,
    policy: RequestPolicy,
}

impl Client {
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::safebooru::Client;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let client = Client::new()?;
    /// let posts = client.search().tag("cat_ears").limit(10).send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: super::shared_client().clone(),
            endpoint: DEFAULT_ENDPOINT.to_string(),
            policy: RequestPolicy::default(),
        })
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub fn search(&self) -> Search {
        self.search_with(Query::new())
    }

    pub fn search_with(&self, query: Query) -> Search {
        Search {
            client: self.clone(),
            query,
            page: 0,
        }
    }

    pub async fn post(&self, id: u32) -> Result<SafebooruPost> {
        self.post_inner(id)
            .await
            .with_context(Provider::Safebooru, Operation::Post)
    }

    async fn post_inner(&self, id: u32) -> Result<SafebooruPost> {
        let response = match execute_with_policy(&self.policy, || async {
            Ok(self
                .http
                .get(format!("{}/index.php", self.endpoint))
                .query(&[
                    ("page", "dapi"),
                    ("s", "post"),
                    ("q", "index"),
                    ("id", &id.to_string()),
                    ("json", "1"),
                ])
                .send()
                .await?)
        })
        .await
        {
            Ok(response) => response,
            Err(BooruError::HttpStatus { status: 404, .. }) => {
                return Err(BooruError::PostNotFound(id));
            }
            Err(error) => return Err(error),
        };

        let posts = response.json::<Vec<SafebooruPost>>().await?;
        posts.into_iter().next().ok_or(BooruError::PostNotFound(id))
    }

    pub async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        self.autocomplete_inner(query, limit)
            .await
            .with_context(Provider::Safebooru, Operation::Autocomplete)
    }

    async fn autocomplete_inner(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let response = execute_with_policy(&self.policy, || async {
            Ok(self
                .http
                .get(format!("{}/autocomplete.php", self.endpoint))
                .query(&[("q", query)])
                .send()
                .await?)
        })
        .await?;

        let suggestions = response.json::<Vec<SafebooruAutocompleteItem>>().await?;
        Ok(suggestions
            .into_iter()
            .take(limit as usize)
            .map(|item| TagSuggestion {
                name: item.value,
                label: item.label.clone(),
                post_count: parse_post_count_from_label(&item.label),
                category: None,
                tag: None,
                suggestion_type: None,
            })
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    core: QueryCore,
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

impl Query {
    pub fn new() -> Self {
        Self {
            core: QueryCore::new(),
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.core = self.core.tag(tag);
        self
    }

    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.core = self.core.tags(tags);
        self
    }

    /// Adds a provider query expression without literal-tag validation.
    pub fn raw_query(mut self, expression: impl Into<String>) -> Self {
        self.core = self.core.raw_query(expression);
        self
    }

    pub fn raw_queries<I, S>(mut self, expressions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.core = self.core.raw_queries(expressions);
        self
    }

    pub fn rating(mut self, rating: SafebooruRating) -> Self {
        self.core = self.core.rating(rating);
        self
    }

    pub fn sort(mut self, order: Sort) -> Self {
        self.core = self.core.sort(order.to_string());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.core = self.core.limit(limit);
        self
    }

    pub fn blacklist_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.core = self.core.blacklist_tag(tag);
        self
    }

    pub fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.core = self.core.blacklist_tags(tags);
        self
    }

    pub fn exclude_rating(mut self, rating: SafebooruRating) -> Self {
        self.core = self.core.exclude_rating(rating);
        self
    }

    pub fn random(mut self) -> Self {
        self.core = self.core.random(SORT_PREFIX);
        self
    }

    pub fn validate(&self) -> Result<()> {
        self.core.validate_common()
    }
}

#[derive(Debug, Clone)]
pub struct Search {
    client: Client,
    query: Query,
    page: u32,
}

impl Search {
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.query = self.query.tag(tag);
        self
    }

    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query = self.query.tags(tags);
        self
    }

    pub fn raw_query(mut self, expression: impl Into<String>) -> Self {
        self.query = self.query.raw_query(expression);
        self
    }

    pub fn raw_queries<I, S>(mut self, expressions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query = self.query.raw_queries(expressions);
        self
    }

    pub fn rating(mut self, rating: SafebooruRating) -> Self {
        self.query = self.query.rating(rating);
        self
    }

    pub fn sort(mut self, order: Sort) -> Self {
        self.query = self.query.sort(order);
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.query = self.query.limit(limit);
        self
    }

    pub fn blacklist_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.query = self.query.blacklist_tag(tag);
        self
    }

    pub fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.query = self.query.blacklist_tags(tags);
        self
    }

    pub fn exclude_rating(mut self, rating: SafebooruRating) -> Self {
        self.query = self.query.exclude_rating(rating);
        self
    }

    pub fn random(mut self) -> Self {
        self.query = self.query.random();
        self
    }

    pub fn start_page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    /// Fetches the first page of results.
    pub async fn send(self) -> Result<Vec<SafebooruPost>> {
        self.fetch().await
    }

    pub async fn page(self) -> Result<Page> {
        let posts = self.fetch().await?;
        let next = self
            .page
            .checked_add(1)
            .filter(|_| !posts.is_empty())
            .map(|page| {
                let mut next = self.clone();
                next.page = page;
                next
            });
        Ok(Page { posts, next })
    }

    pub fn pages(self) -> PageStream {
        PageStream::new(self.client.clone(), self.query.clone(), Some(self))
    }

    pub fn posts(self) -> PostStream {
        PostStream::new(self.pages())
    }

    async fn fetch(&self) -> Result<Vec<SafebooruPost>> {
        self.fetch_inner()
            .await
            .with_context(Provider::Safebooru, Operation::Search)
    }

    async fn fetch_inner(&self) -> Result<Vec<SafebooruPost>> {
        self.query.validate()?;
        let tags = self.query.core.assemble_tags(SORT_PREFIX);

        let response = execute_with_policy(&self.client.policy, || async {
            Ok(self
                .client
                .http
                .get(format!("{}/index.php", self.client.endpoint))
                .query(&[
                    ("page", "dapi"),
                    ("s", "post"),
                    ("q", "index"),
                    ("pid", &self.page.to_string()),
                    ("limit", &self.query.core.limit.to_string()),
                    ("tags", &tags),
                    ("json", "1"),
                ])
                .send()
                .await?)
        })
        .await?;

        let posts = response.json::<Vec<SafebooruPost>>().await?;
        Ok(posts)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    http: Option<reqwest::Client>,
    endpoint: Option<String>,
    policy: Option<RequestPolicy>,
}

impl ClientBuilder {
    /// # Errors
    ///
    /// Returns [`BooruError::InvalidUrl`] for blank, unparsable, or non-HTTP(S) endpoints.
    pub fn endpoint(mut self, url: impl Into<String>) -> Result<Self> {
        self.endpoint = Some(super::validate_endpoint(&url.into())?);
        Ok(self)
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Sets the request retry and rate-limit policy.
    #[must_use]
    pub fn request_policy(mut self, policy: RequestPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Sets the retry configuration for requests made by this client.
    pub fn retry_config(mut self, config: RetryConfig) -> Result<Self> {
        let policy = self.policy.take().unwrap_or_default();
        self.policy = Some(policy.with_retry_config(config)?);
        Ok(self)
    }

    /// Sets the rate limiter for requests made by this client.
    #[must_use]
    pub fn rate_limiter(mut self, limiter: RateLimiter) -> Self {
        let policy = self.policy.take().unwrap_or_default();
        self.policy = Some(policy.with_rate_limiter(limiter));
        self
    }

    pub fn build(self) -> Result<Client> {
        Ok(Client {
            http: self.http.unwrap_or_else(|| super::shared_client().clone()),
            endpoint: self
                .endpoint
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            policy: self.policy.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub posts: Vec<SafebooruPost>,
    pub next: Option<Search>,
}

impl super::Client for Client {
    type Query = Query;
    type Post = SafebooruPost;
    type Continuation = Search;

    async fn page(
        &self,
        query: Self::Query,
        continuation: Option<Self::Continuation>,
    ) -> Result<super::PageResult<Self::Post, Self::Continuation>> {
        let page = match continuation {
            Some(search) => search.page().await?,
            None => self.search_with(query).page().await?,
        };
        Ok(super::PageResult {
            posts: page.posts,
            next: page.next,
        })
    }

    async fn post(&self, id: u32) -> Result<Self::Post> {
        Client::post(self, id).await
    }
}

/// Stream of result pages for [`Search::pages`].
///
/// Alias for the shared [`super::stream::PageStream`] over this provider. Pages
/// stop at an empty page and can be bounded with `max_pages`.
pub type PageStream = super::stream::PageStream<Client>;

/// Stream of individual posts for [`Search::posts`].
///
/// Alias for the shared [`super::stream::PostStream`] over this provider.
/// Yields posts across pages and can be bounded with `max_posts` or collected
/// with `collect`.
pub type PostStream = super::stream::PostStream<Client>;
