use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::Deserialize;

use super::{RequestPolicy, Secret, execute_with_policy};
use crate::autocomplete::TagSuggestion;
use crate::client::generic::Sort;
use crate::error::{BooruError, Operation, Provider, Result, ResultContext};
use crate::model::gelbooru::*;
use crate::ratelimit::RateLimiter;
use crate::retry::RetryConfig;

#[derive(Debug, Deserialize)]
struct GelbooruAutocompleteItem {
    value: String,
    label: String,
    #[serde(default)]
    category: Option<String>,
    /// The API sends this as a string or a number.
    #[serde(default)]
    post_count: Option<serde_json::Value>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(rename = "type", default)]
    suggestion_type: Option<String>,
}

fn parse_category(cat: &str) -> Option<u8> {
    match cat.to_lowercase().as_str() {
        "general" | "tag" => Some(0),
        "artist" => Some(1),
        "copyright" | "series" => Some(3),
        "character" => Some(4),
        "meta" | "metadata" => Some(5),
        _ => cat.parse().ok(),
    }
}

/// Parses post count from a label like "tag_name (12345)".
fn parse_post_count_from_label(label: &str) -> Option<u32> {
    let start = label.rfind('(')?;
    let end = label.rfind(')')?;
    if start < end {
        label[start + 1..end].parse().ok()
    } else {
        None
    }
}

const DEFAULT_ENDPOINT: &str = "https://gelbooru.com";
const SORT_PREFIX: &str = "sort:";

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
    /// use booru_rs::gelbooru::Client;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let client = Client::builder()
    ///     .set_credentials("your_api_key", "your_user_id")
    ///     .build()?;
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
            page: 0,
        }
    }

    pub async fn post(&self, id: u32) -> Result<GelbooruPost> {
        self.post_inner(id)
            .await
            .with_context(Provider::Gelbooru, Operation::Post)
    }

    async fn post_inner(&self, id: u32) -> Result<GelbooruPost> {
        let response = match execute_with_policy(&self.policy, || async {
            Ok(self
                .http
                .get(super::dapi_url(&self.endpoint))
                .query(&super::dapi_query(
                    &[("id", id.to_string())],
                    super::dapi_credentials(&self.key, &self.user),
                ))
                .send()
                .await?)
        })
        .await
        {
            Ok(response) => response,
            Err(BooruError::HttpStatus { status: 401, .. }) => {
                return Err(BooruError::Unauthorized(
                    "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)"
                        .into(),
                ));
            }
            Err(BooruError::HttpStatus { status: 404, .. }) => {
                return Err(BooruError::PostNotFound(id));
            }
            Err(error) => return Err(error),
        };

        let data = response.json::<GelbooruResponse>().await?;
        data.posts
            .into_iter()
            .next()
            .ok_or(BooruError::PostNotFound(id))
    }

    pub async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        self.autocomplete_inner(query, limit)
            .await
            .with_context(Provider::Gelbooru, Operation::Autocomplete)
    }

    async fn autocomplete_inner(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let response = match execute_with_policy(&self.policy, || async {
            Ok(self
                .http
                .get(format!("{}/index.php", self.endpoint))
                .query(&[
                    ("page", "autocomplete2"),
                    ("term", query),
                    ("type", "tag_query"),
                    ("limit", &limit.to_string()),
                ])
                .send()
                .await?)
        })
        .await
        {
            Ok(response) => response,
            Err(BooruError::HttpStatus { status: 401, .. }) => {
                return Err(BooruError::Unauthorized(
                    "Gelbooru requires API credentials for some endpoints".into(),
                ));
            }
            Err(error) => return Err(error),
        };

        let items: Vec<GelbooruAutocompleteItem> = response.json().await?;

        Ok(items
            .into_iter()
            .take(limit as usize)
            .map(|item| {
                let post_count = item
                    .post_count
                    .and_then(|count| match count {
                        serde_json::Value::Number(number) => {
                            number.as_u64().and_then(|n| u32::try_from(n).ok())
                        }
                        serde_json::Value::String(text) => text.parse().ok(),
                        _ => None,
                    })
                    .or_else(|| parse_post_count_from_label(&item.label));
                let category = item.category.as_deref().and_then(parse_category);

                TagSuggestion {
                    name: item.value,
                    label: item.label,
                    post_count,
                    category,
                    tag: item.tag,
                    suggestion_type: item.suggestion_type,
                }
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

    /// Adds a provider query expression without literal-tag validation.
    pub fn raw_query(mut self, expression: impl Into<String>) -> Self {
        self.raw_queries.push(expression.into());
        self
    }

    pub fn rating(mut self, rating: GelbooruRating) -> Self {
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

    pub fn exclude_rating(mut self, rating: GelbooruRating) -> Self {
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
        )
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

    pub fn raw_query(mut self, expression: impl Into<String>) -> Self {
        self.query = self.query.raw_query(expression);
        self
    }

    pub fn rating(mut self, rating: GelbooruRating) -> Self {
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

    pub fn exclude_rating(mut self, rating: GelbooruRating) -> Self {
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
    pub async fn send(self) -> Result<Vec<GelbooruPost>> {
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
        PageStream {
            search: Some(self),
            pending: None,
            fetched: 0,
            max_pages: None,
        }
    }

    pub fn posts(self) -> PostStream {
        PostStream {
            pages: self.pages(),
            buffer: Vec::new().into_iter(),
            yielded: 0,
            max_posts: None,
        }
    }

    async fn fetch(&self) -> Result<Vec<GelbooruPost>> {
        self.fetch_inner()
            .await
            .with_context(Provider::Gelbooru, Operation::Search)
    }

    async fn fetch_inner(&self) -> Result<Vec<GelbooruPost>> {
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

        let query = super::dapi_query(
            &[
                ("pid", self.page.to_string()),
                ("limit", self.query.limit.to_string()),
                ("tags", tags),
            ],
            super::dapi_credentials(&self.client.key, &self.client.user),
        );

        let response = match execute_with_policy(&self.client.policy, || async {
            Ok(self
                .client
                .http
                .get(super::dapi_url(&self.client.endpoint))
                .query(&query)
                .send()
                .await?)
        })
        .await
        {
            Ok(response) => response,
            Err(BooruError::HttpStatus { status: 401, .. }) => {
                return Err(BooruError::Unauthorized(
                    "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)"
                        .into(),
                ));
            }
            Err(error) => return Err(error),
        };

        let data = response.json::<GelbooruResponse>().await?;
        Ok(data.posts)
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub posts: Vec<GelbooruPost>,
    pub next: Option<Search>,
}

pub struct PageStream {
    search: Option<Search>,
    pending: Option<Pin<Box<dyn Future<Output = Result<Page>> + Send>>>,
    fetched: u32,
    max_pages: Option<u32>,
}

impl PageStream {
    pub async fn next(&mut self) -> Option<Result<Page>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    pub fn max_pages(mut self, max: u32) -> Self {
        self.max_pages = Some(max);
        self
    }
}

impl Stream for PageStream {
    type Item = Result<Page>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(max) = this.max_pages
            && this.fetched >= max
        {
            this.search = None;
            this.pending = None;
            return Poll::Ready(None);
        }
        loop {
            if let Some(mut pending) = this.pending.take() {
                match pending.as_mut().poll(cx) {
                    Poll::Ready(result) => match result {
                        Ok(page) => {
                            this.fetched = this.fetched.saturating_add(1);
                            if page.posts.is_empty() {
                                this.search = None;
                                return Poll::Ready(None);
                            }
                            this.search = page.next.clone();
                            return Poll::Ready(Some(Ok(page)));
                        }
                        Err(error) => {
                            this.search = None;
                            return Poll::Ready(Some(Err(error)));
                        }
                    },
                    Poll::Pending => {
                        this.pending = Some(pending);
                        return Poll::Pending;
                    }
                }
            }

            let Some(search) = this.search.take() else {
                return Poll::Ready(None);
            };
            this.pending = Some(Box::pin(async move { search.page().await }));
        }
    }
}

pub struct PostStream {
    pages: PageStream,
    buffer: std::vec::IntoIter<GelbooruPost>,
    yielded: u32,
    max_posts: Option<u32>,
}

impl PostStream {
    pub async fn next(&mut self) -> Option<Result<GelbooruPost>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    pub fn max_posts(mut self, max: u32) -> Self {
        self.max_posts = Some(max);
        self
    }

    pub async fn collect(mut self) -> Result<Vec<GelbooruPost>> {
        let mut posts = Vec::new();
        while let Some(result) = self.next().await {
            posts.push(result?);
        }
        Ok(posts)
    }
}

impl Stream for PostStream {
    type Item = Result<GelbooruPost>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(max) = this.max_posts
            && this.yielded >= max
        {
            return Poll::Ready(None);
        }
        loop {
            if let Some(post) = this.buffer.next() {
                this.yielded = this.yielded.saturating_add(1);
                return Poll::Ready(Some(Ok(post)));
            }
            match Pin::new(&mut this.pages).poll_next(cx) {
                Poll::Ready(Some(Ok(page))) => {
                    this.buffer = page.posts.into_iter();
                }
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

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
