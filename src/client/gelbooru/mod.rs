use reqwest::Client;

use crate::model::gelbooru::*;

/// Client that sends requests to the Gelbooru API to retrieve the data.
#[allow(dead_code)]
pub struct GelbooruClient {
    client: Client,
    key: Option<String>
}

impl GelbooruClient {
    pub fn builder() -> GelbooruClientBuilder {
        GelbooruClientBuilder::default()
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
}

impl GelbooruClientBuilder {
    pub fn new() -> GelbooruClientBuilder {
        GelbooruClientBuilder {
            client: Client::new(),
            key: None,
            user: None,
            tags: vec![],
            limit: 100,
        }
    }
    /// Set the API key and User for the requests (optional)
    pub fn set_credentials(mut self, key: String, user: String) -> Self {
        self.key = Some(key);
        self.user = Some(user);
        self
    }

    /// Add a tag to the query
    pub fn tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Add a [`GelbooruRating`] to the query
    pub fn rating(mut self, rating: GelbooruRating) -> Self {
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
            self.tags.push("sort:random".to_string());
        }
        self
    }

    /// Add a [`GelbooruSort`] to the query
    pub fn sort(mut self, order: GelbooruSort) -> Self {
        self.tags.push(format!("sort:{}", order));
        self
    }

    /// Blacklist a tag from the query
    pub fn blacklist_tag(mut self, tag: String) -> Self {
        self.tags.push(format!("-{tag}"));
        self
    }

    /// Directly get a post by its unique Id
    pub async fn get_by_id(&self, id: u32) -> Result<GelbooruPost, String> {
        let response = self.client
            .get("https://gelbooru.com/index.php")
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("id", id.to_string().as_str()),
                ("json", "1")])
            .send()
            .await.expect("Error sending request")
            .json::<GelbooruResponse>()
            .await.expect("Error parsing response");

        let post = response.posts[0].clone();
        Ok(post)
    }

    /// Pack the [`GelbooruClientBuilder`] and sent the request to the API to retrieve the posts
    pub async fn get(&self) -> Result<Vec<GelbooruPost>, String> {
        let tag_string = self.tags.join(" ");
        let response = self.client
            .get("https://gelbooru.com/index.php")
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("limit", self.limit.to_string().as_str()),
                ("tags", &tag_string),
                ("json", "1")])
            .send()
            .await.expect("Error sending request")
            .json::<GelbooruResponse>()
            .await.expect("Error parsing response");

        let posts = response.posts;
        Ok(posts)
    }
}
