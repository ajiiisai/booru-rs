//! Safebooru API client implementation.

use super::{Client, ClientBuilder};
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
