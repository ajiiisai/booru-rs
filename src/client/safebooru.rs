use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::Deserialize;

use super::{RequestPolicy, execute_with_policy};
use crate::autocomplete::TagSuggestion;
use crate::client::generic::Sort;
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

    pub fn rating(mut self, rating: SafebooruRating) -> Self {
        self.rating = Some(rating.into());
        self
    }

    pub fn sort(mut self, order: Sort) -> Self {
        self.sort = Some(format!("sort:{order}"));
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

    pub fn exclude_rating(mut self, rating: SafebooruRating) -> Self {
        self.tags.push(format!("-rating:{rating}"));
        self
    }

    pub fn random(mut self) -> Self {
        self.tags.push("sort:random".to_string());
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

    async fn fetch(&self) -> Result<Vec<SafebooruPost>> {
        self.fetch_inner()
            .await
            .with_context(Provider::Safebooru, Operation::Search)
    }

    async fn fetch_inner(&self) -> Result<Vec<SafebooruPost>> {
        self.query.validate()?;
        let mut tags = self.query.tags.clone();
        if let Some(rating) = &self.query.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = &self.query.sort {
            tags.push(sort.clone());
        }
        tags.extend(self.query.raw_queries.iter().cloned());
        let tags = tags.join(" ");

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
                    ("limit", &self.query.limit.to_string()),
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
        self.post_inner(id).await
    }
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
    buffer: std::vec::IntoIter<SafebooruPost>,
    yielded: u32,
    max_posts: Option<u32>,
}

impl PostStream {
    pub async fn next(&mut self) -> Option<Result<SafebooruPost>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    pub fn max_posts(mut self, max: u32) -> Self {
        self.max_posts = Some(max);
        self
    }

    pub async fn collect(mut self) -> Result<Vec<SafebooruPost>> {
        let mut posts = Vec::new();
        while let Some(result) = self.next().await {
            posts.push(result?);
        }
        Ok(posts)
    }
}

impl Stream for PostStream {
    type Item = Result<SafebooruPost>;

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
