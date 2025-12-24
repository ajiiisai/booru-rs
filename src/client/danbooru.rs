//! Danbooru API client implementation.

use super::{Client, ClientBuilder};
use crate::error::Result;
use crate::model::danbooru::*;

use reqwest::header::{self, HeaderMap, HeaderValue};

/// Returns headers required for Danbooru API requests.
///
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
/// Danbooru has a limit of 2 tags per query for non-authenticated users.
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

    /// Retrieves a single post by its unique ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the response cannot be parsed.
    async fn get_by_id(&self, id: u32) -> Result<Self::Post> {
        let builder = &self.0;
        let url = &builder.url;

        let response = builder
            .client
            .get(format!("{url}/posts/{id}.json"))
            .headers(get_headers())
            .send()
            .await?
            .json::<DanbooruPost>()
            .await?;

        Ok(response)
    }

    /// Retrieves posts matching the configured query.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the response cannot be parsed.
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
            .await?
            .json::<Vec<DanbooruPost>>()
            .await?;

        Ok(response)
    }
}
