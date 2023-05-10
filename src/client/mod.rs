use reqwest::Client;

use self::generic::{Rating, Sort};

pub mod danbooru;
pub mod gelbooru;
pub mod generic;

pub struct ClientBuilder {
    client: Client,
    client_type: ClientType,
    key: Option<String>,
    user: Option<String>,
    tags: Vec<String>,
    limit: u32,
    url: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientType {
    Danbooru,
    Gelbooru,
}

impl ClientType {
    fn get_url(&self) -> String {
        match self {
            ClientType::Danbooru => "https://danbooru.donmai.us".to_string(),
            ClientType::Gelbooru => "https://gelbooru.com".to_string(),
        }
    }
}

impl ClientBuilder {
    pub fn new(client_type: ClientType) -> Self {
        Self {
            client: Client::new(),
            client_type,
            key: None,
            user: None,
            tags: vec![],
            limit: 100,
            url: client_type.get_url(),
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
        match self.client_type {
            ClientType::Danbooru if self.tags.len() > 1 => {
                panic!("Danbooru only allows 2 tags per query")
            }
            _ => {}
        }
        self.tags.push(tag.into());
        self
    }

    /// Add a [`DanbooruRating`](crate::model::DanbooruRating) or
    /// [`GelbooruRating`](crate::model::GelbooruRating) to the query
    pub fn rating<R: Into<Rating>>(mut self, rating: R) -> Self {
        let rating_tag = match rating.into() {
            Rating::Danbooru(rating) => {
                assert_eq!(
                    self.client_type,
                    ClientType::Danbooru,
                    "{:?} `ClientBuilder` but tried to apply a Danbooru rating to it.",
                    self.client_type
                );
                format!("rating:{}", rating)
            }
            Rating::Gelbooru(rating) => {
                assert_eq!(
                    self.client_type,
                    ClientType::Gelbooru,
                    "{:?} `ClientBuilder` but tried to apply a Gelbooru rating to it.",
                    self.client_type
                );
                format!("rating:{}", rating)
            }
        };
        self.tags.push(rating_tag);
        self
    }

    /// Set how many posts you want to retrieve (100 is the default and maximum)
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Retrieves the posts in a random order
    pub fn random(mut self) -> Self {
        let random_tag = match self.client_type {
            ClientType::Danbooru => "order:random",
            ClientType::Gelbooru => "sort:random",
        };
        self.tags.push(random_tag.into());
        self
    }

    /// Add a [`Sort`] to the query
    pub fn sort(mut self, order: Sort) -> Self {
        let sort_tag = match self.client_type {
            ClientType::Danbooru => format!("order:{}", order),
            ClientType::Gelbooru => format!("sort:{}", order),
        };
        self.tags.push(sort_tag);
        self
    }

    /// Blacklist a tag from the query
    pub fn blacklist_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(format!("-{}", tag.into()));
        self
    }

    /// Change the default url for the client
    pub fn default_url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

    /// Convert the builder into the necessary client
    pub fn build<T: From<Self>>(self) -> T {
        T::from(self)
    }
}
