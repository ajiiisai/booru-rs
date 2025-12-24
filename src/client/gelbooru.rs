//! Gelbooru API client implementation.

use super::{Client, ClientBuilder, shared_client};
use crate::autocomplete::{Autocomplete, TagSuggestion};
use crate::error::{BooruError, Result};
use crate::model::gelbooru::*;
use serde::Deserialize;

/// Client for interacting with the Gelbooru API.
///
/// Gelbooru has no tag limit for queries.
///
/// # Authentication
///
/// Gelbooru **requires API credentials** for API access. You can obtain your
/// API key and user ID from your [Gelbooru account settings](https://gelbooru.com/index.php?page=account&s=options).
///
/// Use [`ClientBuilder::set_credentials`] to provide your API key and user ID:
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
/// [`ClientBuilder::set_credentials`]: super::ClientBuilder::set_credentials
/// [`BooruError::Unauthorized`]: crate::error::BooruError::Unauthorized
#[derive(Debug)]
pub struct GelbooruClient(ClientBuilder<Self>);

impl From<ClientBuilder<Self>> for GelbooruClient {
    fn from(value: ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

impl Client for GelbooruClient {
    type Post = GelbooruPost;
    type Rating = GelbooruRating;

    const URL: &'static str = "https://gelbooru.com";
    const SORT: &'static str = "sort:";
    const MAX_TAGS: Option<usize> = None;

    /// Retrieves a single post by its unique ID.
    ///
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

        // Add API credentials if provided
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

        // Check for authentication errors
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        let data = response.json::<GelbooruResponse>().await?;

        data.posts
            .into_iter()
            .next()
            .ok_or(BooruError::PostNotFound(id))
    }

    /// Retrieves posts matching the configured query.
    ///
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

        // Add API credentials if provided
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

        // Check for authentication errors
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Gelbooru requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        let data = response.json::<GelbooruResponse>().await?;

        Ok(data.posts)
    }
}

/// Internal response type for Gelbooru autocomplete.
#[derive(Debug, Deserialize)]
struct GelbooruAutocompleteItem {
    /// The tag name.
    value: String,
    /// Display label (includes post count).
    label: String,
    /// Tag category (optional).
    #[serde(default)]
    category: Option<String>,
    /// Number of posts with this tag (optional).
    #[serde(default)]
    post_count: Option<u32>,
}

impl Autocomplete for GelbooruClient {
    async fn autocomplete(query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let client = shared_client();
        let url = format!("{}/index.php", Self::URL);

        let response = client
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
            .map(|item| {
                // Try to parse post count from label if not provided directly
                let post_count = item
                    .post_count
                    .or_else(|| parse_post_count_from_label(&item.label));

                // Convert category string to numeric ID if present
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

/// Parses category string to numeric ID.
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
