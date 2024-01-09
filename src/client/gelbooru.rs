use async_trait::async_trait;

use super::{Client, ClientBuilder};
use crate::model::gelbooru::*;

/// Client that sends requests to the Gelbooru API to retrieve the data.
pub struct GelbooruClient(ClientBuilder<Self>);

impl From<ClientBuilder<Self>> for GelbooruClient {
    fn from(value: ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

#[async_trait]
impl Client for GelbooruClient {
    type Post = GelbooruPost;

    const URL: &'static str = "https://gelbooru.com";
    const SORT: &'static str = "sort:";

    /// Directly get a post by its unique Id
    async fn get_by_id(&self, id: u32) -> Result<GelbooruPost, reqwest::Error> {
        let builder = &self.0;
        let url = builder.url.as_str();
        let response = builder
            .client
            .get(format!("{url}/index.php"))
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("id", id.to_string().as_str()),
                ("json", "1"),
            ])
            .send()
            .await?
            .json::<GelbooruResponse>()
            .await?;

        Ok(response.posts[0].clone())
    }

    /// Pack the [`ClientBuilder`] and sent the request to the API to retrieve the posts
    async fn get(&self) -> Result<Vec<GelbooruPost>, reqwest::Error> {
        let builder = &self.0;
        let url = builder.url.as_str();
        let tag_string = builder.tags.join(" ");
        let response = builder
            .client
            .get(format!("{url}/index.php"))
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("pid", builder.page.to_string().as_str()),
                ("limit", builder.limit.to_string().as_str()),
                ("tags", &tag_string),
                ("json", "1"),
            ])
            .send()
            .await?
            .json::<GelbooruResponse>()
            .await?;

        Ok(response.posts)
    }
}
