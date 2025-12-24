//! Gelbooru API tests.
//!
//! These tests require API credentials. Set the following environment variables:
//! - `GELBOORU_API_KEY`: Your Gelbooru API key
//! - `GELBOORU_USER_ID`: Your Gelbooru user ID
//!
//! Tests will be skipped if credentials are not available.

mod gelbooru {
    use booru_rs::{
        client::{Client, ClientBuilder, gelbooru::GelbooruClient, generic::*},
        gelbooru::GelbooruRating,
    };

    /// Returns a builder with credentials if available, or None to skip the test.
    fn builder_with_credentials() -> Option<ClientBuilder<GelbooruClient>> {
        let api_key = std::env::var("GELBOORU_API_KEY").ok()?;
        let user_id = std::env::var("GELBOORU_USER_ID").ok()?;
        Some(GelbooruClient::builder().set_credentials(api_key, user_id))
    }

    macro_rules! skip_without_credentials {
        () => {
            match builder_with_credentials() {
                Some(builder) => builder,
                None => {
                    eprintln!("Skipping test: GELBOORU_API_KEY and GELBOORU_USER_ID not set");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn get_posts_with_tag() {
        let builder = skip_without_credentials!();
        let posts = builder.tag("kafuu_chino").unwrap().build().get().await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_rating() {
        let builder = skip_without_credentials!();
        let posts = builder
            .tag("kafuu_chino")
            .unwrap()
            .rating(GelbooruRating::General)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_sort() {
        let builder = skip_without_credentials!();
        let posts = builder
            .tag("kafuu_chino")
            .unwrap()
            .sort(Sort::Score)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_blacklist_tag() {
        let builder = skip_without_credentials!();
        let posts = builder
            .tag("kafuu_chino")
            .unwrap()
            .blacklist_tag(GelbooruRating::Explicit)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_limit() {
        let builder = skip_without_credentials!();
        let posts = builder
            .tag("kafuu_chino")
            .unwrap()
            .rating(GelbooruRating::General)
            .limit(3)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().len() == 3);
    }

    #[tokio::test]
    async fn get_posts_multiple_tags() {
        let builder = skip_without_credentials!();
        let posts = builder
            .tag("kafuu_chino")
            .unwrap()
            .tag("table")
            .unwrap()
            .limit(3)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_random_posts() {
        let builder = skip_without_credentials!();
        let posts = builder
            .tag("kafuu_chino")
            .unwrap()
            .random()
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_post_by_id() {
        let builder = skip_without_credentials!();
        let post = builder.build().get_by_id(7898595).await;

        assert!(post.is_ok());
        assert_eq!("e40b797a0e26755b2c0dd7a34d8c95ce", post.unwrap().md5);
    }

    #[tokio::test]
    async fn get_posts_from_page() {
        let builder = skip_without_credentials!();
        let builder2 = builder_with_credentials().unwrap();

        let post_from_first_page = builder.build().get().await;
        let post_from_specific_page = builder2.page(7).build().get().await;

        assert!(post_from_first_page.is_ok());
        assert!(post_from_specific_page.is_ok());

        assert_ne!(
            post_from_first_page.unwrap()[0].id,
            post_from_specific_page.unwrap()[0].id
        );
    }

    #[test]
    fn parse_rating_tags() {
        assert_eq!("explicit", GelbooruRating::Explicit.to_string());
        assert_eq!("questionable", GelbooruRating::Questionable.to_string());
        assert_eq!("safe", GelbooruRating::Safe.to_string());
        assert_eq!("sensitive", GelbooruRating::Sensitive.to_string());
        assert_eq!("general", GelbooruRating::General.to_string());
    }

    #[test]
    fn parse_sort_tags() {
        assert_eq!("id", Sort::Id.to_string());
        assert_eq!("score", Sort::Score.to_string());
        assert_eq!("rating", Sort::Rating.to_string());
        assert_eq!("user", Sort::User.to_string());
        assert_eq!("height", Sort::Height.to_string());
        assert_eq!("width", Sort::Width.to_string());
        assert_eq!("source", Sort::Source.to_string());
        assert_eq!("updated", Sort::Updated.to_string());
    }
}
