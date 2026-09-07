use serde::Deserialize;

use super::{RequestPolicy, Secret, execute_with_policy, parse_post_count_from_label};
use crate::autocomplete::TagSuggestion;
use crate::client::generic::{BuilderCore, QueryCore, Sort};
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
            Err(error) => return Err(super::map_post_lookup_error(error, id)),
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
        if limit == 0 {
            return Ok(Vec::new());
        }

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

    pub fn rating(mut self, rating: GelbooruRating) -> Self {
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

    pub fn exclude_rating(mut self, rating: GelbooruRating) -> Self {
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

    pub async fn page(self) -> Result<super::PageResult<GelbooruPost, Search>> {
        let posts = self.fetch().await?;
        let next = super::advance_page(self.page, posts.is_empty()).map(|page| {
            let mut next = self.clone();
            next.page = page;
            next
        });
        Ok(super::PageResult { posts, next })
    }

    pub fn pages(self) -> PageStream {
        PageStream::new(self.client.clone(), self.query.clone(), Some(self))
    }

    pub fn posts(self) -> PostStream {
        PostStream::new(self.pages())
    }

    async fn fetch(&self) -> Result<Vec<GelbooruPost>> {
        self.fetch_inner()
            .await
            .with_context(Provider::Gelbooru, Operation::Search)
    }

    async fn fetch_inner(&self) -> Result<Vec<GelbooruPost>> {
        self.query.validate()?;
        let tags = self.query.core.assemble_tags(SORT_PREFIX);

        let query = super::dapi_query(
            &[
                ("pid", self.page.to_string()),
                ("limit", self.query.core.limit.to_string()),
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

/// One fetched page of results.
///
/// Alias for the shared [`super::PageResult`] over this provider's post and
/// search types. Returned by [`Search::page`] and yielded by [`PageStream`].
pub type Page = super::PageResult<GelbooruPost, Search>;

impl super::Client for Client {
    type Query = Query;
    type Post = GelbooruPost;
    type Continuation = Search;

    async fn page(
        &self,
        query: Self::Query,
        continuation: Option<Self::Continuation>,
    ) -> Result<super::PageResult<Self::Post, Self::Continuation>> {
        match continuation {
            Some(search) => search.page().await,
            None => self.search_with(query).page().await,
        }
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
    core: BuilderCore,
    key: Option<Secret>,
    user: Option<Secret>,
}

impl ClientBuilder {
    /// # Errors
    ///
    /// Returns [`BooruError::InvalidUrl`] for blank, unparsable, or non-HTTP(S) endpoints.
    pub fn endpoint(mut self, url: impl Into<String>) -> Result<Self> {
        self.core = self.core.endpoint(url)?;
        Ok(self)
    }

    pub fn set_credentials(mut self, key: impl Into<String>, user: impl Into<String>) -> Self {
        self.key = Some(Secret::new(key));
        self.user = Some(Secret::new(user));
        self
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.core = self.core.http_client(client);
        self
    }

    /// Sets the request retry and rate-limit policy.
    #[must_use]
    pub fn request_policy(mut self, policy: RequestPolicy) -> Self {
        self.core = self.core.request_policy(policy);
        self
    }

    /// Sets the retry configuration for requests made by this client.
    pub fn retry_config(mut self, config: RetryConfig) -> Result<Self> {
        self.core = self.core.retry_config(config)?;
        Ok(self)
    }

    /// Sets the rate limiter for requests made by this client.
    #[must_use]
    pub fn rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.core = self.core.rate_limiter(limiter);
        self
    }

    pub fn build(self) -> Result<Client> {
        Ok(Client {
            http: self
                .core
                .http
                .unwrap_or_else(|| super::shared_client().clone()),
            endpoint: self
                .core
                .endpoint
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            key: self.key,
            user: self.user,
            policy: self.core.policy.unwrap_or_default(),
        })
    }
}
