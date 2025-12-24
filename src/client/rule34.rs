//! Rule34 API client implementation.

use super::{Client, ClientBuilder, shared_client};
use crate::autocomplete::{Autocomplete, TagSuggestion};
use crate::error::{BooruError, Result};
use crate::model::rule34::*;
use serde::Deserialize;

/// Client for interacting with the Rule34 API.
///
/// Rule34 has no tag limit for queries.
///
/// # Authentication
///
/// Rule34 **requires API credentials** for API access. You can obtain your
/// API key and user ID from your [Rule34 account settings](https://rule34.xxx/index.php?page=account&s=options).
///
/// Use [`ClientBuilder::set_credentials`] to provide your API key and user ID:
///
/// ```no_run
/// use booru_rs::rule34::{Rule34Client, Rule34Rating};
/// use booru_rs::client::Client;
///
/// # async fn example() -> booru_rs::error::Result<()> {
/// let posts = Rule34Client::builder()
///     .set_credentials("your_api_key", "your_user_id")
///     .tag("cat_ears")?
///     .rating(Rule34Rating::Safe)
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
/// # Content Warning
///
/// Rule34 is an adult (NSFW) image board. Content is not filtered by default.
/// Use rating filters appropriately.
///
/// [`ClientBuilder::set_credentials`]: super::ClientBuilder::set_credentials
/// [`BooruError::Unauthorized`]: crate::error::BooruError::Unauthorized
#[derive(Debug)]
pub struct Rule34Client(ClientBuilder<Self>);

impl From<ClientBuilder<Self>> for Rule34Client {
    fn from(value: ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

impl Client for Rule34Client {
    type Post = Rule34Post;
    type Rating = Rule34Rating;

    const URL: &'static str = "https://api.rule34.xxx";
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

        // Check for authentication errors (some APIs may return 401)
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Rule34 requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        // Rule34 API quirk: returns HTTP 200 OK with error message in body instead of 401
        // Example: "Missing authentication. Go to api.rule34.xxx for more information"
        let text = response.text().await?;
        if text.contains("Missing authentication") {
            return Err(BooruError::Unauthorized(
                "Rule34 requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        let posts: Vec<Rule34Post> = serde_json::from_str(&text)?;
        posts.into_iter().next().ok_or(BooruError::PostNotFound(id))
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

        // Check for authentication errors (some APIs may return 401)
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Rule34 requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        // Rule34 API quirk: returns HTTP 200 OK with error message in body instead of 401
        // Example: "Missing authentication. Go to api.rule34.xxx for more information"
        let text = response.text().await?;
        if text.contains("Missing authentication") {
            return Err(BooruError::Unauthorized(
                "Rule34 requires API credentials. Use set_credentials(api_key, user_id)".into(),
            ));
        }

        // Handle empty response (no results)
        if text.is_empty() || text == "[]" {
            return Ok(Vec::new());
        }

        let posts: Vec<Rule34Post> = serde_json::from_str(&text)?;
        Ok(posts)
    }
}

/// Internal response type for Rule34 autocomplete.
#[derive(Debug, Deserialize)]
struct Rule34AutocompleteItem {
    /// The tag name.
    value: String,
    /// Display label (includes post count).
    label: String,
}

impl Autocomplete for Rule34Client {
    async fn autocomplete(query: &str, _limit: u32) -> Result<Vec<TagSuggestion>> {
        let client = shared_client();
        // Rule34 autocomplete is on api.rule34.xxx, not the main URL
        let url = "https://api.rule34.xxx/autocomplete.php";

        let response = client.get(url).query(&[("q", query)]).send().await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BooruError::Unauthorized(
                "Rule34 autocomplete request failed".into(),
            ));
        }

        let items: Vec<Rule34AutocompleteItem> = response.json().await?;

        Ok(items
            .into_iter()
            .map(|item| TagSuggestion {
                name: item.value,
                label: item.label.clone(),
                post_count: parse_post_count_from_label(&item.label),
                category: None,
            })
            .collect())
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
