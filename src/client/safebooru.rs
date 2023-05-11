use async_trait::async_trait;

use super::{Client, ClientBuilder};
use crate::model::safebooru::SafebooruPost;

pub struct SafebooruClient(ClientBuilder<Self>);

impl From<ClientBuilder<Self>> for SafebooruClient {
    fn from(value: ClientBuilder<Self>) -> Self {
        Self(value)
    }
}

#[async_trait]
impl Client for SafebooruClient {
    type Post = SafebooruPost;

    const URL: &'static str = "https://safebooru.org";
    const SORT: &'static str = "sort:";

    async fn get_by_id(&self, id: u32) -> Result<Self::Post, reqwest::Error> {
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
            .json::<Vec<SafebooruPost>>()
            .await?;
        // FIXME: Assumes there is a post with the given id. Same is true for the
        // Gelbooru client.
        Ok(response
            .into_iter()
            .next()
            .expect("Requested an id that does not exist."))
    }

    async fn get(&self) -> Result<Vec<Self::Post>, reqwest::Error> {
        let builder = &self.0;
        let url = builder.url.as_str();
        let tags = builder.tags.join(" ");
        Ok(builder
            .client
            .get(format!("{url}/index.php"))
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("limit", builder.limit.to_string().as_str()),
                ("tags", &tags),
                ("json", "1"),
            ])
            .send()
            .await?
            .json::<Vec<SafebooruPost>>()
            .await?)
    }
}
