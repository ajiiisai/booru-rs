use super::{Client as ClientTrait, ensure_success};
use crate::autocomplete::{Autocomplete, TagSuggestion};
use crate::client::generic::Sort;
use crate::error::{BooruError, Result};
use crate::model::safebooru::{SafebooruPost, SafebooruRating};

use serde::Deserialize;

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
        Search {
            client: self.clone(),
            tags: Vec::new(),
            rating: None,
            sort: None,
            limit: 100,
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

#[derive(Debug, Clone)]
pub struct Search {
    client: Client,
    tags: Vec<String>,
    rating: Option<String>,
    sort: Option<String>,
    limit: u32,
}

impl Search {
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn rating(mut self, rating: SafebooruRating) -> Self {
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

    /// Fetches the first page of results.
    pub async fn send(self) -> Result<Vec<SafebooruPost>> {
        let mut tags = self.tags;
        if let Some(rating) = self.rating {
            tags.push(format!("rating:{rating}"));
        }
        if let Some(sort) = self.sort {
            tags.push(sort);
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
                ("pid", "0"),
                ("limit", &self.limit.to_string()),
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
