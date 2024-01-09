#[cfg(test)]
mod safebooru {
    use booru_rs::{
        client::{generic::Sort, safebooru::SafebooruClient, Client},
        model::safebooru::SafebooruRating,
    };

    #[tokio::test]
    async fn get_posts_with_tag() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_rating() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .rating(SafebooruRating::General)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_sort() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .sort(Sort::Rating)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_blacklist_tag() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .blacklist_tag(SafebooruRating::Explicit)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_limit() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .limit(3)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().len() == 3);
    }

    #[tokio::test]
    async fn get_posts_multiple_tags() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .tag("bangs")
            .limit(3)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_random_posts() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .random()
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_post_by_id() {
        let post = SafebooruClient::builder().build().get_by_id(4348760).await;

        assert!(post.is_ok());
        assert_eq!("3e407a7848804119f1064c2aac731545", post.unwrap().hash);
    }

    #[tokio::test]
    async fn get_posts_from_page() {
        let post_from_first_page = SafebooruClient::builder().build().get().await;

        let post_from_specific_page = SafebooruClient::builder().page(7).build().get().await;

        assert!(post_from_first_page.is_ok());
        assert!(post_from_specific_page.is_ok());

        assert_ne!(
            post_from_first_page.unwrap()[0].id,
            post_from_specific_page.unwrap()[0].id
        );
    }

    #[test]
    fn parse_rating_tags() {
        assert_eq!("safe", SafebooruRating::Safe.to_string());
        assert_eq!("general", SafebooruRating::General.to_string());
        assert_eq!("questionable", SafebooruRating::Questionable.to_string());
        assert_eq!("explicit", SafebooruRating::Explicit.to_string());
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
