use reqwest::Client;

use crate::model::gelbooru::*;

#[allow(dead_code)]
pub struct GelbooruClient {
    client: Client,
    key: Option<String>
}

impl Default for GelbooruClient {
    fn default() -> Self {
        GelbooruClient::new(None)
    }
}

/// The Gelbooru client sends requests to the Gelbooru API to retrieve the data.
impl GelbooruClient {
    pub fn new(key: Option<String>) -> Self {
        let client = Client::new();
        GelbooruClient { client, key }
    }

    pub fn set_api_key(&self, key: String) -> Self {
        GelbooruClient { client: self.client.clone(), key: Some(key) }
    }

    /// Gets a post.
    pub async fn get_post_by_id(&self, id: u64) -> Result<GelbooruPost, String> {
        let response = self.client
            .get("https://gelbooru.com/index.php")
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("id", &id.to_string()),
                ("json", "1")])
            .send()
            .await.expect("Error parsing send")
            .json::<GelbooruResponse>()
            .await.expect("Error deserializing");

        let post = response.posts[0].clone();
        Ok(post)
    }

    /// Gets posts with only tags. 
    pub async fn get_posts_by_tag(&self, tags: &str) -> Result<Vec<GelbooruPost>, String> {
        let response = self.client
            .get("https://gelbooru.com/index.php")
            .query(&[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("tags", tags),
                ("json", "1")])
            .send()
            .await.expect("Error sending request")
            .json::<GelbooruResponse>()
            .await.expect("Error parsing response");

        let posts = response.posts;
        Ok(posts)
    }
    
    /// Gets posts with a tag and a rating.
    pub async fn get_posts_by_tag_and_rating(&self, tag: &str, rating: GelbooruRating) -> Result<Vec<GelbooruPost>, String> {
        let new_tag = format!("{} rating:{:?}", tag, rating);
        let posts = self.get_posts_by_tag(new_tag.as_str()).await?;
        Ok(posts)
    }
}
