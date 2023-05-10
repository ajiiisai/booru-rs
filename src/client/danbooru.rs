use super::{ClientBuilder, ClientType};
use crate::model::danbooru::*;

use reqwest::{header, header::HeaderMap};

// This is only here because of Danbooru, thanks Danbooru, really cool :)
pub fn get_headers() -> HeaderMap {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("PostmanRuntime/7.30.0"),
    );
    headers
}

/// Client that sends requests to the Danbooru API to retrieve the data.
pub struct DanbooruClient(ClientBuilder);

impl From<ClientBuilder> for DanbooruClient {
    fn from(value: ClientBuilder) -> Self {
        Self(value)
    }
}

impl DanbooruClient {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new(ClientType::Danbooru)
    }

    /// Directly get a post by its unique Id
    pub async fn get_by_id(&self, id: u32) -> Result<DanbooruPost, reqwest::Error> {
        let builder = &self.0;
        let url = builder.url.as_str();
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

    /// Pack the [`DanbooruClientBuilder`] and sent the request to the API to retrieve the posts
    pub async fn get(&self) -> Result<Vec<DanbooruPost>, reqwest::Error> {
        let builder = &self.0;
        let tag_string = builder.tags.join(" ");
        let url = builder.url.as_str();
        let response = builder
            .client
            .get(format!("{url}/posts.json"))
            .headers(get_headers())
            .query(&[
                ("limit", builder.limit.to_string().as_str()),
                ("tags", &tag_string),
            ])
            .send()
            .await?
            .json::<Vec<DanbooruPost>>()
            .await?;

        Ok(response)
    }
}
