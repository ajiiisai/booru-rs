//! Gelbooru API tests.
//!
//! These tests require API credentials. Set the following environment variables:
//! - `GELBOORU_API_KEY`: Your Gelbooru API key
//! - `GELBOORU_USER_ID`: Your Gelbooru user ID
//!
//! Tests will be skipped if credentials are not available.

mod gelbooru {
    use booru_rs::{client::generic::*, gelbooru::Client, gelbooru::GelbooruRating};

    fn authed_client() -> Option<Client> {
        let api_key = std::env::var("GELBOORU_API_KEY").ok()?;
        let user_id = std::env::var("GELBOORU_USER_ID").ok()?;
        Client::builder()
            .set_credentials(api_key, user_id)
            .build()
            .ok()
    }

    macro_rules! skip_without_credentials {
        () => {
            match authed_client() {
                Some(client) => client,
                None => {
                    eprintln!("Skipping test: GELBOORU_API_KEY and GELBOORU_USER_ID not set");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_tag() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_rating() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .rating(GelbooruRating::General)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_sort() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .sort(Sort::Score)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_blacklist_tag() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .blacklist_tag(GelbooruRating::Explicit)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_limit() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .rating(GelbooruRating::General)
            .limit(3)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().len() == 3);
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_multiple_tags() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .tag("table")
            .limit(3)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_random_posts() {
        let posts = skip_without_credentials!()
            .search()
            .tag("kafuu_chino")
            .random()
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_post_by_id() {
        let post = skip_without_credentials!().post(7898595).await;

        assert!(post.is_ok());
        assert_eq!("e40b797a0e26755b2c0dd7a34d8c95ce", post.unwrap().md5);
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_from_page() {
        let client = skip_without_credentials!();
        let post_from_first_page = client.search().send().await;
        let post_from_specific_page = client.search().start_page(7).send().await;

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
