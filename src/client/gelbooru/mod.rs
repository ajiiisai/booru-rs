use async_trait::async_trait;
use reqwest::Client;

use crate::client::generic::*;
use crate::model::gelbooru::*;

/// Client that sends requests to the Gelbooru API to retrieve the data.
pub struct GelbooruClient;

impl BooruClient<GelbooruClientBuilder> for GelbooruClient {
    fn builder() -> GelbooruClientBuilder {
        GelbooruClientBuilder::new()
    }
}

/// Builder for [`GelbooruClient`]
#[derive(Default)]
pub struct GelbooruClientBuilder {
    client: Client,
    key: Option<String>,
    user: Option<String>,
    tags: Vec<String>,
    limit: u32,
    url: String,
}

#[async_trait]
impl BooruBuilder<GelbooruRating, GelbooruSort> for GelbooruClientBuilder {
    fn new() -> Self {
        GelbooruClientBuilder {
            client: Client::new(),
            key: None,
            user: None,
            tags: vec![],
            limit: 100,
            url: "https://gelbooru.com".to_string(),
        }
    }
    /// Set the API key and User for the requests (optional)
    fn set_credentials(mut self, key: String, user: String) -> Self {
        self.key = Some(key);
        self.user = Some(user);
        self
    }

    /// Add a tag to the query
    fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a [`GelbooruRating`] to the query
    fn rating(mut self, rating: GelbooruRating) -> Self {
        self.tags.push(format!("rating:{}", rating));
        self
    }

    /// Set how many posts you want to retrieve (100 is the default and maximum)
    fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Retrieves the posts in a random order
    fn random(mut self, random: bool) -> Self {
        if random {
            self.tags.push("sort:random".to_string());
        }
        self
    }

    /// Add a [`GelbooruSort`] to the query
    fn sort(mut self, order: GelbooruSort) -> Self {
        self.tags.push(format!("sort:{}", order));
        self
    }

    /// Blacklist a tag from the query
    fn blacklist_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(format!("-{}", tag.into()));
        self
    }

    /// Change the default url for the client
    fn default_url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

    /// Directly get a post by its unique Id
    async fn get_by_id(&self, id: u32) -> Result<GelbooruPost, reqwest::Error> {
        let url = self.url.as_str();
        let response = self
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

    /// Pack the [`GelbooruClientBuilder`] and sent the request to the API to retrieve the posts
    async fn get(&self) -> Result<Vec<GelbooruPost>, reqwest::Error> {
        let url = self.url.as_str();
        let tag_string = self.tags.join(" ");
        let response = self
            .client
            .get(format!("{url}/index.php"))
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("limit", self.limit.to_string().as_str()),
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
