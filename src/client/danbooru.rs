use super::{Client, ClientBuilder, ensure_success, shared_client};
use crate::autocomplete::{Autocomplete, TagSuggestion};
use crate::error::{BooruError, Result};
use crate::model::danbooru::*;

use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

/// Danbooru requires a User-Agent header for requests.
fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static("booru-rs/0.3.0"),
    );
    headers
}

/// Client for interacting with the Danbooru API.
///
/// Danbooru allows 2 tags per query without authentication.
///
/// # Example
///
/// ```no_run
/// use booru_rs::danbooru::{DanbooruClient, DanbooruRating};
/// use booru_rs::client::Client;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// let posts = DanbooruClient::builder()
///     .tag("cat_ears")?
///     .rating(DanbooruRating::General)
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
pub struct DanbooruClient(ClientBuilder<Self>);

impl From<ClientBuilder<Self>> for DanbooruClient {
    fn from(value: ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

impl Client for DanbooruClient {
    type Post = DanbooruPost;
    type Rating = DanbooruRating;

    const URL: &'static str = "https://danbooru.donmai.us";
    const SORT: &'static str = "order:";
    const MAX_TAGS: Option<usize> = Some(2);

    async fn get_by_id(&self, id: u32) -> Result<Self::Post> {
        let builder = &self.0;
        let url = &builder.url;

        let response = builder
            .client
            .get(format!("{url}/posts/{id}.json"))
            .headers(get_headers())
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

    async fn get(&self) -> Result<Vec<Self::Post>> {
        let builder = &self.0;
        let tag_string = builder.tags.join(" ");
        let url = &builder.url;

        let response = builder
            .client
            .get(format!("{url}/posts.json"))
            .headers(get_headers())
            .query(&[
                ("limit", builder.limit.to_string()),
                ("page", builder.page.to_string()),
                ("tags", tag_string),
            ])
            .send()
            .await?;

        let response = ensure_success(response).await?;

        let posts = response.json::<Vec<DanbooruPost>>().await?;

        Ok(posts)
    }
}

#[derive(Debug, Deserialize)]
struct DanbooruAutocompleteItem {
    value: String,
    label: String,
    category: Option<u8>,
    post_count: Option<u32>,
}

impl Autocomplete for DanbooruClient {
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::danbooru::DanbooruClient;
    /// use booru_rs::autocomplete::Autocomplete;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let suggestions = DanbooruClient::autocomplete("cat_", 10).await?;
    /// for tag in suggestions {
    ///     println!("{}: {} posts", tag.name, tag.post_count.unwrap_or(0));
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn autocomplete(query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let response = shared_client()
            .get(format!("{}/autocomplete.json", Self::URL))
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
