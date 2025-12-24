//! Safebooru API client implementation.

use super::{Client, ClientBuilder, shared_client};
use crate::autocomplete::{Autocomplete, TagSuggestion};
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
pub struct SafebooruClient(ClientBuilder<Self>);

impl From<ClientBuilder<Self>> for SafebooruClient {
    fn from(value: ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

impl Client for SafebooruClient {
    type Post = SafebooruPost;
    type Rating = SafebooruRating;

    const URL: &'static str = "https://safebooru.org";
    const SORT: &'static str = "sort:";
    const MAX_TAGS: Option<usize> = None;

    /// Retrieves a single post by its unique ID.
    ///
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
            .await?
            .json::<Vec<SafebooruPost>>()
            .await?;

        response
            .into_iter()
            .next()
            .ok_or(BooruError::PostNotFound(id))
    }

    /// Retrieves posts matching the configured query.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the response cannot be parsed.
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
            .await?
            .json::<Vec<SafebooruPost>>()
            .await?;

        Ok(response)
    }
}

/// Safebooru autocomplete API response item.
#[derive(Debug, Deserialize)]
struct SafebooruAutocompleteItem {
    value: String,
    label: String,
}

impl Autocomplete for SafebooruClient {
    /// Returns tag suggestions from Safebooru's autocomplete API.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::safebooru::SafebooruClient;
    /// use booru_rs::autocomplete::Autocomplete;
    ///
    /// # async fn example() -> booru_rs::error::Result<()> {
    /// let suggestions = SafebooruClient::autocomplete("land", 5).await?;
    /// for tag in suggestions {
    ///     println!("{}", tag.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn autocomplete(query: &str, limit: u32) -> Result<Vec<TagSuggestion>> {
        let response = shared_client()
            .get(format!("{}/autocomplete.php", Self::URL))
            .query(&[("q", query)])
            .send()
            .await?
            .json::<Vec<SafebooruAutocompleteItem>>()
            .await?;

        // Safebooru includes post count in the label like "cat_ears (177448)"
        // Parse it out if present
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
