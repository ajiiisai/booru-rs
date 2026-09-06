//! Integration tests for the Rule34 client.
//!
//! Note: These tests require API credentials to run.
//! Set RULE34_API_KEY and RULE34_USER_ID environment variables.

mod rule34 {
    use booru_rs::client::generic::Sort;
    use booru_rs::model::rule34::Rule34Rating;
    use booru_rs::rule34::Client;

    fn authed_client() -> Option<Client> {
        let key = std::env::var("RULE34_API_KEY").ok()?;
        let user = std::env::var("RULE34_USER_ID").ok()?;
        Client::builder().set_credentials(key, user).build().ok()
    }

    macro_rules! skip_without_credentials {
        () => {
            match authed_client() {
                Some(client) => client,
                None => {
                    eprintln!("Skipping test: RULE34_API_KEY and RULE34_USER_ID not set");
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
            .tag("cat")
            .limit(5)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_limit() {
        let posts = skip_without_credentials!().search().limit(3).send().await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().len() <= 3);
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_rating() {
        let posts = skip_without_credentials!()
            .search()
            .rating(Rule34Rating::Safe)
            .limit(5)
            .send()
            .await;

        assert!(posts.is_ok());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_sort() {
        let posts = skip_without_credentials!()
            .search()
            .sort(Sort::Score)
            .limit(5)
            .send()
            .await;

        assert!(posts.is_ok());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_from_page() {
        let posts = skip_without_credentials!()
            .search()
            .start_page(2)
            .limit(5)
            .send()
            .await;

        assert!(posts.is_ok());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn unauthorized_without_credentials() {
        let client = Client::new().expect("default client must build");

        let result = client.search().tag("cat").limit(1).send().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, booru_rs::BooruError::Unauthorized(_)));
    }

    #[test]
    fn parse_rating_tags() {
        assert_eq!(Rule34Rating::Explicit.to_string(), "explicit");
        assert_eq!(Rule34Rating::Safe.to_string(), "safe");
        assert_eq!(Rule34Rating::Questionable.to_string(), "questionable");
    }

    #[test]
    fn parse_sort_tags() {
        assert_eq!(Sort::Id.to_string(), "id");
        assert_eq!(Sort::Score.to_string(), "score");
        assert_eq!(Sort::Rating.to_string(), "rating");
        assert_eq!(Sort::Updated.to_string(), "updated");
    }
}
