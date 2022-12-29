use reqwest::Client;

use crate::model::danbooru::*;
use crate::utils::utils::get_headers;

/// Client that sends requests to the Danbooru API to retrieve the data.
pub struct DanbooruClient;

impl DanbooruClient {
    pub fn builder() -> DanbooruClientBuilder {
        DanbooruClientBuilder::new()
    }
}

/// Builder for [`DanbooruClient`]
#[derive(Default)]
pub struct DanbooruClientBuilder {
    client: Client,
    key: Option<String>,
    user: Option<String>,
    tags: Vec<String>,
    limit: u32,
    url: String,
}

impl DanbooruClientBuilder {
    pub fn new() -> DanbooruClientBuilder {
        DanbooruClientBuilder {
            client: Client::new(),
            key: None,
            user: None,
            tags: vec![],
            limit: 100,
            url: "https://danbooru.donmai.us".to_string(),
        }
    }
    /// Set the API key and User for the requests (optional)
    pub fn set_credentials(mut self, key: String, user: String) -> Self {
        self.key = Some(key);
        self.user = Some(user);
        self
    }

    /// Add a tag to the query
    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        if self.tags.len() > 1 {
            panic!("Danbooru only allows 2 tags per query");
        }
        self.tags.push(tag.into());
        self
    }

    /// Add a [`DanbooruRating`] to the query
    pub fn rating(mut self, rating: DanbooruRating) -> Self {
        self.tags.push(format!("rating:{}", rating));
        self
    }

    /// Set how many posts you want to retrieve (100 is the default and maximum)
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Retrieves the posts in a random order
    pub fn random(mut self, random: bool) -> Self {
        if random {
            self.tags.push("order:random".into());
        }
        self
    }

    /// Add a [`DanbooruSort`] to the query
    pub fn sort(mut self, order: DanbooruSort) -> Self {
        self.tags.push(format!("order:{}", order));
        self
    }

    /// Blacklist a tag from the query
    pub fn blacklist_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(format!("-{}", tag.into()));
        self
    }

    pub fn change_default_url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

    /// Directly get a post by its unique Id
    pub async fn get_by_id(&self, id: u32) -> Result<DanbooruPost, reqwest::Error> {
        let url = self.url.as_str();
        let response = self
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
        let tag_string = self.tags.join(" ");
        let response = self
            .client
            .get(format!("https://testbooru.donmai.us/posts.json"))
            .headers(get_headers())
            .query(&[
                ("limit", self.limit.to_string().as_str()),
                ("tags", &tag_string),
            ])
            .send()
            .await?
            .json::<Vec<DanbooruPost>>()
            .await?;

        Ok(response)
    }
}
