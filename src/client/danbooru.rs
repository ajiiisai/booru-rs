use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

use super::{RequestPolicy, Secret, execute_with_policy};
use crate::autocomplete::TagSuggestion;
use crate::client::generic::Sort;
use crate::error::{BooruError, Operation, Provider, Result, ResultContext};
use crate::model::danbooru::*;
use crate::ratelimit::RateLimiter;
use crate::retry::RetryConfig;

/// Danbooru requires a User-Agent header for requests.
fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static(concat!("booru-rs/", env!("CARGO_PKG_VERSION"))),
    );
    headers
}

#[derive(Debug, Deserialize)]
struct DanbooruAutocompleteItem {
    value: String,
    label: String,
    category: Option<u8>,
    post_count: Option<u32>,
    #[serde(default)]
    tag: Option<DanbooruAutocompleteTag>,
    #[serde(rename = "type", default)]
    suggestion_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DanbooruAutocompleteTag {
    Object { name: String },
    Name(String),
}

impl DanbooruAutocompleteTag {
    fn into_name(self) -> String {
        match self {
            Self::Object { name } | Self::Name(name) => name,
        }
    }
}

const DEFAULT_ENDPOINT: &str = "https://danbooru.donmai.us";
const MAX_TAGS: usize = 2;
const SORT_PREFIX: &str = "order:";

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: String,
    key: Option<Secret>,
    user: Option<Secret>,
    policy: RequestPolicy,
}

impl Client {
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::danbooru::Client;
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
            key: None,
            user: None,
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
            page: 1,
        }
    }

    pub async fn post(&self, id: u32) -> Result<DanbooruPost> {
        self.post_inner(id)
            .await
            .with_context(Provider::Danbooru, Operation::Post)
    }

    async fn post_inner(&self, id: u32) -> Result<DanbooruPost> {
        let mut query = Vec::new();
        if let (Some(key), Some(user)) = (&self.key, &self.user) {
            query.push(("login", user.expose().to_string()));
            query.push(("api_key", key.expose().to_string()));
        }

        let response = match execute_with_policy(&self.policy, || async {
            Ok(self
                .http
                .get(format!("{}/posts/{id}.json", self.endpoint))
                .headers(get_headers())
                .query(&query)
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

        let post = response.json::<DanbooruPost>().await?;
        Ok(post)
    }

    pub async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        self.autocomplete_inner(query, limit)
            .await
            .with_context(Provider::Danbooru, Operation::Autocomplete)
    }

    async fn autocomplete_inner(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let response = execute_with_policy(&self.policy, || async {
            Ok(self
                .http
                .get(format!("{}/autocomplete.json", self.endpoint))
                .headers(get_headers())
                .query(&[
                    ("search[query]", query),
                    ("search[type]", "tag_query"),
                    ("limit", &limit.to_string()),
                ])
                .send()
                .await?)
        })
        .await?
        .json::<Vec<DanbooruAutocompleteItem>>()
        .await?;

        Ok(response
            .into_iter()
            .take(limit as usize)
            .map(|item| TagSuggestion {
                name: item.value,
                label: item.label,
                post_count: item.post_count,
                category: item.category,
                tag: item.tag.map(DanbooruAutocompleteTag::into_name),
                suggestion_type: item.suggestion_type,
            })
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    tags: Vec<String>,
    raw_queries: Vec<String>,
    rating: Option<String>,
    sort: Option<String>,
    limit: u32,
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

impl Query {
    pub fn new() -> Self {
        Self {
            tags: Vec::new(),
            raw_queries: Vec::new(),
            rating: None,
            sort: None,
            limit: 100,
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn tags<I, S>(mut self, tags: I) -> Self
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
    pub fn raw_query(mut self, expression: impl Into<String>) -> Self {
        self.raw_queries.push(expression.into());
        self
    }

    pub fn rating(mut self, rating: DanbooruRating) -> Self {
        self.rating = Some(rating.into());
        self
    }

    pub fn sort(mut self, order: Sort) -> Self {
        self.sort = Some(order.to_string());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn blacklist_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.tags.push(format!("-{}", tag.as_ref()));
        self
    }

    pub fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for tag in tags {
            self = self.blacklist_tag(tag);
        }
        self
    }

    pub fn exclude_rating(mut self, rating: DanbooruRating) -> Self {
        self.tags.push(format!("-rating:{rating}"));
        self
    }

    pub fn random(mut self) -> Self {
        self.tags.push(format!("{SORT_PREFIX}random"));
        self
    }

    pub fn validate(&self) -> Result<()> {
        super::validate_tags(&self.tags)?;
        super::validate_raw_queries(
            &self.raw_queries,
            self.rating.is_some(),
            self.sort.is_some(),
        )?;
        super::validate_random_conflict(&self.tags, &self.raw_queries, self.sort.is_some())?;
        if self.tags.len() > MAX_TAGS {
            return Err(BooruError::TagLimitExceeded {
                client: "DanbooruClient",
                max: MAX_TAGS,
                actual: self.tags.len(),
            });
        }
        Ok(())
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

    pub fn rating(mut self, rating: DanbooruRating) -> Self {
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

    pub fn exclude_rating(mut self, rating: DanbooruRating) -> Self {
        self.query = self.query.exclude_rating(rating);
        self
    }

    pub fn random(mut self) -> Self {
        self.query = self.query.random();
        self
    }

    pub fn start_page(mut self, page: u32) -> Self {
        self.page = page.max(1);
        self
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    /// Fetches the first page of results.
    pub async fn send(self) -> Result<Vec<DanbooruPost>> {
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

    async fn fetch(&self) -> Result<Vec<DanbooruPost>> {
        self.fetch_inner()
            .await
            .with_context(Provider::Danbooru, Operation::Search)
    }

    async fn fetch_inner(&self) -> Result<Vec<DanbooruPost>> {
        self.query.validate()?;
        let mut tags = self.query.tags.clone();
        if let Some(rating) = &self.query.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = &self.query.sort {
            tags.push(format!("{SORT_PREFIX}{sort}"));
        }
        tags.extend(self.query.raw_queries.iter().cloned());
        let tags = tags.join(" ");

        let mut query = vec![
            ("limit", self.query.limit.to_string()),
            ("page", self.page.to_string()),
            ("tags", tags),
        ];
        if let (Some(key), Some(user)) = (&self.client.key, &self.client.user) {
            query.push(("login", user.expose().to_string()));
            query.push(("api_key", key.expose().to_string()));
        }

        let response = execute_with_policy(&self.client.policy, || async {
            Ok(self
                .client
                .http
                .get(format!("{}/posts.json", self.client.endpoint))
                .headers(get_headers())
                .query(&query)
                .send()
                .await?)
        })
        .await?;

        let posts = response.json::<Vec<DanbooruPost>>().await?;
        Ok(posts)
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub posts: Vec<DanbooruPost>,
    pub next: Option<Search>,
}

impl super::Client for Client {
    type Query = Query;
    type Post = DanbooruPost;
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

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    http: Option<reqwest::Client>,
    endpoint: Option<String>,
    key: Option<Secret>,
    user: Option<Secret>,
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

    pub fn set_credentials(mut self, key: impl Into<String>, user: impl Into<String>) -> Self {
        self.key = Some(Secret::new(key));
        self.user = Some(Secret::new(user));
        self
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
            key: self.key,
            user: self.user,
            policy: self.policy.unwrap_or_default(),
        })
    }
}
