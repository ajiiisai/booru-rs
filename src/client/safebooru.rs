use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::Deserialize;

use super::{Client as ClientTrait, ensure_success};
use crate::autocomplete::{Autocomplete, TagSuggestion};
use crate::client::generic::Sort;
use crate::error::{BooruError, Result};
use crate::model::safebooru::{SafebooruPost, SafebooruRating};

/// Client for interacting with the Safebooru API.
///
/// Safebooru is a SFW-only booru with no tag limits.
///
/// # Example
///
/// ```no_run
/// use booru_rs::safebooru::{SafebooruClient, SafebooruRating};
/// use booru_rs::client::Client;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// let posts = SafebooruClient::builder()
///     .tag("cat_ears")?
///     .rating(SafebooruRating::General)
///     .limit(10)
///     .build()
///     .get()
///     .await?;
///
/// println!("Found {} posts", posts.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct SafebooruClient(super::ClientBuilder<Self>);

impl From<super::ClientBuilder<Self>> for SafebooruClient {
    fn from(value: super::ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

impl ClientTrait for SafebooruClient {
    type Post = SafebooruPost;
    type Rating = SafebooruRating;

    const URL: &'static str = "https://safebooru.org";
    const SORT: &'static str = "sort:";
    const MAX_TAGS: Option<usize> = None;

    /// # Errors
    ///
    /// Returns [`BooruError::PostNotFound`] if no post exists with the given ID.
    /// Returns other errors if the request fails or the response cannot be parsed.
    async fn get_by_id(&self, id: u32) -> Result<Self::Post> {
        let builder = &self.0;
        let url = &builder.url;

        let response = builder
            .client
            .get(format!("{url}/index.php"))
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("id", &id.to_string()),
                ("json", "1"),
            ])
            .send()
            .await?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(BooruError::PostNotFound(id));
        }
        let response = ensure_success(response).await?;

        let posts = response.json::<Vec<SafebooruPost>>().await?;

        posts.into_iter().next().ok_or(BooruError::PostNotFound(id))
    }

    async fn get(&self) -> Result<Vec<Self::Post>> {
        let builder = &self.0;
        let url = &builder.url;
        let tags = builder.tags.join(" ");

        let response = builder
            .client
            .get(format!("{url}/index.php"))
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("pid", &builder.page.to_string()),
                ("limit", &builder.limit.to_string()),
                ("tags", &tags),
                ("json", "1"),
            ])
            .send()
            .await?;

        let response = ensure_success(response).await?;

        let posts = response.json::<Vec<SafebooruPost>>().await?;

        Ok(posts)
    }
}

#[derive(Debug, Deserialize)]
struct SafebooruAutocompleteItem {
    value: String,
    label: String,
}

impl Autocomplete for SafebooruClient {
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::safebooru::SafebooruClient;
    /// use booru_rs::autocomplete::Autocomplete;
    /// use booru_rs::client::Client;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let client = SafebooruClient::builder().build();
    /// let suggestions = client.autocomplete("land", 5).await?;
    /// for tag in suggestions {
    ///     println!("{}", tag.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let builder = &self.0;
        let response = builder
            .client
            .get(format!("{}/autocomplete.php", builder.url))
            .query(&[("q", query)])
            .send()
            .await?
            .json::<Vec<SafebooruAutocompleteItem>>()
            .await?;

        // Safebooru folds the post count into the label: "cat_ears (177448)".
        Ok(response
            .into_iter()
            .take(limit as usize)
            .map(|item| {
                let post_count = parse_post_count_from_label(&item.label);
                TagSuggestion {
                    name: item.value,
                    label: item.label,
                    post_count,
                    category: None,
                }
            })
            .collect())
    }
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

// Reusable client with search state kept out of client configuration.

const DEFAULT_ENDPOINT: &str = <SafebooruClient as ClientTrait>::URL;

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: String,
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
        let response = self
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
            .await?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(BooruError::PostNotFound(id));
        }
        let response = ensure_success(response).await?;

        let posts = response.json::<Vec<SafebooruPost>>().await?;
        posts.into_iter().next().ok_or(BooruError::PostNotFound(id))
    }

    pub async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let response = self
            .http
            .get(format!("{}/autocomplete.php", self.endpoint))
            .query(&[("q", query)])
            .send()
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
        self.tags.push("sort:random".to_string());
        self
    }

    pub fn validate(&self) -> Result<()> {
        super::validate_tags(&self.tags)
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

    async fn fetch(&self) -> Result<Vec<SafebooruPost>> {
        self.query.validate()?;
        let mut tags = self.query.tags.clone();
        if let Some(rating) = &self.query.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = &self.query.sort {
            tags.push(sort.clone());
        }
        let tags = tags.join(" ");

        let response = self
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
            .await?;

        let response = ensure_success(response).await?;

        let posts = response.json::<Vec<SafebooruPost>>().await?;
        Ok(posts)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    http: Option<reqwest::Client>,
    endpoint: Option<String>,
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

    pub fn build(self) -> Result<Client> {
        Ok(Client {
            http: self.http.unwrap_or_else(|| super::shared_client().clone()),
            endpoint: self
                .endpoint
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub posts: Vec<SafebooruPost>,
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
