mod gelbooru {
    use booru_rs::{
        client::{gelbooru::GelbooruClient, generic::*},
        model::gelbooru::GelbooruRating,
    };

    #[tokio::test]
    async fn get_posts_with_tag() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_rating() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .rating(GelbooruRating::General)
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_sort() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .sort(Sort::Score)
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_blacklist_tag() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .blacklist_tag(GelbooruRating::Explicit)
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_limit() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .rating(GelbooruRating::General)
            .limit(3)
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().len() == 3);
    }

    #[tokio::test]
    async fn get_posts_multiple_tags() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .tag("table")
            .limit(3)
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_random_posts() {
        let posts = GelbooruClient::builder()
            .tag("kafuu_chino")
            .random()
            .build::<GelbooruClient>()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_post_by_id() {
        let post = GelbooruClient::builder()
            .build::<GelbooruClient>()
            .get_by_id(7898595)
            .await;

        assert!(post.is_ok());
        assert_eq!("e40b797a0e26755b2c0dd7a34d8c95ce", post.unwrap().md5);
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
