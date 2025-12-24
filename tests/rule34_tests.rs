//! Integration tests for the Rule34 client.
//!
//! Note: These tests require API credentials to run.
//! Set RULE34_API_KEY and RULE34_USER_ID environment variables.

use booru_rs::client::Client;
use booru_rs::client::generic::Sort;
use booru_rs::prelude::*;

fn get_credentials() -> Option<(String, String)> {
    let key = std::env::var("RULE34_API_KEY").ok()?;
    let user = std::env::var("RULE34_USER_ID").ok()?;
    Some((key, user))
}

mod rule34 {
    use super::*;

    #[tokio::test]
    async fn get_posts_with_tag() {
        let Some((key, user)) = get_credentials() else {
            eprintln!("Skipping test: RULE34_API_KEY and RULE34_USER_ID not set");
            return;
        };

        let posts = Rule34Client::builder()
            .set_credentials(&key, &user)
            .tag("cat")
            .unwrap()
            .limit(5)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        let posts = posts.unwrap();
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_limit() {
        let Some((key, user)) = get_credentials() else {
            eprintln!("Skipping test: RULE34_API_KEY and RULE34_USER_ID not set");
            return;
        };

        let posts = Rule34Client::builder()
            .set_credentials(&key, &user)
            .limit(3)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        let posts = posts.unwrap();
        assert!(posts.len() <= 3);
    }

    #[tokio::test]
    async fn get_posts_with_rating() {
        let Some((key, user)) = get_credentials() else {
            eprintln!("Skipping test: RULE34_API_KEY and RULE34_USER_ID not set");
            return;
        };

        let posts = Rule34Client::builder()
            .set_credentials(&key, &user)
            .rating(Rule34Rating::Safe)
            .limit(5)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
    }

    #[tokio::test]
    async fn get_posts_with_sort() {
        let Some((key, user)) = get_credentials() else {
            eprintln!("Skipping test: RULE34_API_KEY and RULE34_USER_ID not set");
            return;
        };

        let posts = Rule34Client::builder()
            .set_credentials(&key, &user)
            .sort(Sort::Score)
            .limit(5)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
    }

    #[tokio::test]
    async fn get_posts_from_page() {
        let Some((key, user)) = get_credentials() else {
            eprintln!("Skipping test: RULE34_API_KEY and RULE34_USER_ID not set");
            return;
        };

        let posts = Rule34Client::builder()
            .set_credentials(&key, &user)
            .limit(5)
            .page(2)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
    }

    #[tokio::test]
    async fn unauthorized_without_credentials() {
        let result = Rule34Client::builder()
            .tag("cat")
            .unwrap()
            .limit(1)
            .build()
            .get()
            .await;

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
