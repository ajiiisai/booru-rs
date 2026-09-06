use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use serde::Deserialize;

use super::{Client as ClientTrait, ensure_success};
use crate::autocomplete::{Autocomplete, TagSuggestion};
use crate::client::generic::Sort;
use crate::error::{BooruError, Result};
use crate::model::gelbooru::*;

/// Client for interacting with the Gelbooru API.
///
/// Gelbooru has no tag limit for queries.
///
/// # Authentication
///
/// Gelbooru **requires API credentials** for API access. You can obtain your
/// API key and user ID from your [Gelbooru account settings](https://gelbooru.com/index.php?page=account&s=options).
///
/// ```no_run
/// use booru_rs::gelbooru::{GelbooruClient, GelbooruRating};
/// use booru_rs::client::Client;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// let posts = GelbooruClient::builder()
///     .set_credentials("your_api_key", "your_user_id")
///     .tag("cat_ears")?
///     .rating(GelbooruRating::General)
///     .limit(10)
///     .build()
///     .get()
///     .await?;
///
/// println!("Found {} posts", posts.len());
/// # Ok(())
/// # }
/// ```
///
/// Without credentials, requests will fail with [`BooruError::Unauthorized`].
///
/// [`BooruError::Unauthorized`]: crate::error::BooruError::Unauthorized
#[derive(Debug)]
pub struct GelbooruClient(super::ClientBuilder<Self>);

impl From<super::ClientBuilder<Self>> for GelbooruClient {
    fn from(value: super::ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

impl ClientTrait for GelbooruClient {
    type Post = GelbooruPost;
    type Rating = GelbooruRating;

    const URL: &'static str = "https://gelbooru.com";
    const SORT: &'static str = "sort:";
    const MAX_TAGS: Option<usize> = None;

    /// # Errors
    ///
    /// Returns [`BooruError::PostNotFound`] if no post exists with the given ID.
    /// Returns [`BooruError::Unauthorized`] if API credentials are missing or invalid.
    /// Returns other errors if the request fails or the response cannot be parsed.
    async fn get_by_id(&self, id: u32) -> Result<Self::Post> {
        let builder = &self.0;
        let url = &builder.url;

        let mut query = vec![
            ("page", "dapi".to_string()),
            ("s", "post".to_string()),
            ("q", "index".to_string()),
            ("id", id.to_string()),
            ("json", "1".to_string()),
        ];

        if let (Some(key), Some(user)) = (&builder.key, &builder.user) {
            query.push(("api_key", key.clone()));
            query.push(("user_id", user.clone()));
        }

        let response = builder
            .client
            .get(format!("{url}/index.php"))
            .query(&query)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(BooruError::PostNotFound(id));
        }
        let response = ensure_success(response).await?;

        let data = response.json::<GelbooruResponse>().await?;

        data.posts
            .into_iter()
            .next()
            .ok_or(BooruError::PostNotFound(id))
    }

    /// # Errors
    ///
    /// Returns [`BooruError::Unauthorized`] if API credentials are missing or invalid.
    /// Returns other errors if the request fails or if the response cannot be parsed.
    async fn get(&self) -> Result<Vec<Self::Post>> {
        let builder = &self.0;
        let url = &builder.url;
        let tag_string = builder.tags.join(" ");

        let mut query = vec![
            ("page", "dapi".to_string()),
            ("s", "post".to_string()),
            ("q", "index".to_string()),
            ("pid", builder.page.to_string()),
            ("limit", builder.limit.to_string()),
            ("tags", tag_string),
            ("json", "1".to_string()),
        ];

        if let (Some(key), Some(user)) = (&builder.key, &builder.user) {
            query.push(("api_key", key.clone()));
            query.push(("user_id", user.clone()));
        }

        let response = builder
            .client
            .get(format!("{url}/index.php"))
            .query(&query)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        let response = ensure_success(response).await?;

        let data = response.json::<GelbooruResponse>().await?;

        Ok(data.posts)
    }
}

#[derive(Debug, Deserialize)]
struct GelbooruAutocompleteItem {
    value: String,
    label: String,
    #[serde(default)]
    category: Option<String>,
    /// The API sends this as a string or a number.
    #[serde(default)]
    post_count: Option<serde_json::Value>,
}

impl Autocomplete for GelbooruClient {
    async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let builder = &self.0;
        let url = format!("{}/index.php", builder.url);

        let response = builder
            .client
            .get(&url)
            .query(&[
                ("page", "autocomplete2"),
                ("term", query),
                ("type", "tag_query"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials for some endpoints".into(),
            ));
        }

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
                }
            })
            .collect())
    }
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

const DEFAULT_ENDPOINT: &str = <GelbooruClient as ClientTrait>::URL;

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
        let mut query = vec![
            ("page", "dapi".to_string()),
            ("s", "post".to_string()),
            ("q", "index".to_string()),
            ("id", id.to_string()),
            ("json", "1".to_string()),
        ];
        if let (Some(key), Some(user)) = (&self.key, &self.user) {
            query.push(("api_key", key.clone()));
            query.push(("user_id", user.clone()));
        }

        let response = self
            .http
            .get(format!("{}/index.php", self.endpoint))
            .query(&query)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(BooruError::PostNotFound(id));
        }
        let response = ensure_success(response).await?;

        let data = response.json::<GelbooruResponse>().await?;
        data.posts
            .into_iter()
            .next()
            .ok_or(BooruError::PostNotFound(id))
    }

    pub async fn autocomplete(&self, query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let response = self
            .http
            .get(format!("{}/index.php", self.endpoint))
            .query(&[
                ("page", "autocomplete2"),
                ("term", query),
                ("type", "tag_query"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials for some endpoints".into(),
            ));
        }

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
                }
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
        self.tags
            .push(format!("{}random", <GelbooruClient as ClientTrait>::SORT));
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
    pub async fn send(self) -> Result<Vec<GelbooruPost>> {
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

    async fn fetch(&self) -> Result<Vec<GelbooruPost>> {
        self.query.validate()?;
        let mut tags = self.query.tags.clone();
        if let Some(rating) = &self.query.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = &self.query.sort {
            tags.push(format!("{}{sort}", <GelbooruClient as ClientTrait>::SORT));
        }
        let tags = tags.join(" ");

        let mut query = vec![
            ("page", "dapi".to_string()),
            ("s", "post".to_string()),
            ("q", "index".to_string()),
            ("pid", self.page.to_string()),
            ("limit", self.query.limit.to_string()),
            ("tags", tags),
            ("json", "1".to_string()),
        ];
        if let (Some(key), Some(user)) = (&self.client.key, &self.client.user) {
            query.push(("api_key", key.clone()));
            query.push(("user_id", user.clone()));
        }

        let response = self
            .client
            .http
            .get(format!("{}/index.php", self.client.endpoint))
            .query(&query)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }
        let response = ensure_success(response).await?;

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
