use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

use super::ensure_success;
use crate::autocomplete::TagSuggestion;
use crate::client::generic::Sort;
use crate::error::{BooruError, Result};
use crate::model::danbooru::*;

/// Danbooru requires a User-Agent header for requests.
fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static("booru-rs/0.3.0"),
    );
    headers
}

#[derive(Debug, Deserialize)]
struct DanbooruAutocompleteItem {
    value: String,
    label: String,
    category: Option<u8>,
    post_count: Option<u32>,
}

const DEFAULT_ENDPOINT: &str = "https://danbooru.donmai.us";
const MAX_TAGS: usize = 2;
const SORT_PREFIX: &str = "order:";

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: String,
    key: Option<String>,
    user: Option<String>,
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
        let mut query = Vec::new();
        if let (Some(key), Some(user)) = (&self.key, &self.user) {
            query.push(("login", user.clone()));
            query.push(("api_key", key.clone()));
        }

        let response = self
            .http
            .get(format!("{}/posts/{id}.json", self.endpoint))
            .headers(get_headers())
            .query(&query)
            .send()
            .await?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(BooruError::PostNotFound(id));
        }
        let response = ensure_success(response).await?;

        let post = response.json::<DanbooruPost>().await?;
        Ok(post)
    }

    pub async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let response = self
            .http
            .get(format!("{}/autocomplete.json", self.endpoint))
            .headers(get_headers())
            .query(&[
                ("search[query]", query),
                ("search[type]", "tag_query"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?
            .json::<Vec<DanbooruAutocompleteItem>>()
            .await?;

        Ok(response
            .into_iter()
            .map(|item| TagSuggestion {
                name: item.value,
                label: item.label,
                post_count: item.post_count,
                category: item.category,
            })
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    tags: Vec<String>,
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
            rating: None,
            sort: None,
            limit: 100,
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
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

    pub fn blacklist_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(format!("-{}", tag.into()));
        self
    }

    pub fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for tag in tags {
            self = self.blacklist_tag(tag);
        }
        self
    }

    pub fn random(mut self) -> Self {
        self.tags.push(format!("{SORT_PREFIX}random"));
        self
    }

    pub fn validate(&self) -> Result<()> {
        super::validate_tags(&self.tags)?;
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

    pub fn blacklist_tag(mut self, tag: impl Into<String>) -> Self {
        self.query = self.query.blacklist_tag(tag);
        self
    }

    pub fn blacklist_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query = self.query.blacklist_tags(tags);
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
        let next = if posts.is_empty() {
            None
        } else {
            let mut next = self.clone();
            next.page = self.page.saturating_add(1);
            Some(next)
        };
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

    async fn fetch(&self) -> Result<Vec<DanbooruPost>> {
        self.query.validate()?;
        let mut tags = self.query.tags.clone();
        if let Some(rating) = &self.query.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = &self.query.sort {
            tags.push(format!("{SORT_PREFIX}{sort}"));
        }
        let tags = tags.join(" ");

        let mut query = vec![
            ("limit", self.query.limit.to_string()),
            ("page", self.page.to_string()),
            ("tags", tags),
        ];
        if let (Some(key), Some(user)) = (&self.client.key, &self.client.user) {
            query.push(("login", user.clone()));
            query.push(("api_key", key.clone()));
        }

        let response = self
            .client
            .http
            .get(format!("{}/posts.json", self.client.endpoint))
            .headers(get_headers())
            .query(&query)
            .send()
            .await?;

        let response = ensure_success(response).await?;

        let posts = response.json::<Vec<DanbooruPost>>().await?;
        Ok(posts)
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub posts: Vec<DanbooruPost>,
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
    buffer: std::vec::IntoIter<DanbooruPost>,
    yielded: u32,
    max_posts: Option<u32>,
}

impl PostStream {
    pub async fn next(&mut self) -> Option<Result<DanbooruPost>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    pub fn max_posts(mut self, max: u32) -> Self {
        self.max_posts = Some(max);
        self
    }

    pub async fn collect(mut self) -> Result<Vec<DanbooruPost>> {
        let mut posts = Vec::new();
        while let Some(result) = self.next().await {
            posts.push(result?);
        }
        Ok(posts)
    }
}

impl Stream for PostStream {
    type Item = Result<DanbooruPost>;

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
    key: Option<String>,
    user: Option<String>,
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
        self.key = Some(key.into());
        self.user = Some(user.into());
        self
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
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
        })
    }
}
