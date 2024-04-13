#[cfg(test)]
mod danbooru {
    use booru_rs::{
        client::{danbooru::DanbooruClient, generic::Sort, Client},
        model::danbooru::DanbooruRating,
    };

    #[tokio::test]
    async fn get_posts_with_tag() {
        let posts = DanbooruClient::builder()
            .tag("kafuu_chino")
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_rating() {
        let posts = DanbooruClient::builder()
            .tag("kafuu_chino")
            .rating(DanbooruRating::General)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_sort() {
        let posts = DanbooruClient::builder()
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
        let posts = DanbooruClient::builder()
            .tag("kafuu_chino")
            .blacklist_tag(DanbooruRating::Explicit)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_limit() {
        let posts = DanbooruClient::builder()
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
        let posts = DanbooruClient::builder()
            .tag("kafuu_chino")
            .tag("1girl")
            .limit(3)
            .build()
            .get()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_random_posts() {
        let posts = DanbooruClient::builder()
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
        let post = DanbooruClient::builder()
            .build()
            .get_by_id(7452417)
            .await;

        assert!(post.is_ok());
        assert_eq!(
            "d796ffc0c83585bb2e836f8d49653675",
            post.unwrap().md5.unwrap()
        );
    }

    #[tokio::test]
    async fn get_posts_from_page() {
        let post_from_first_page = DanbooruClient::builder()
            .build()
            .get()
            .await;

        let post_from_specific_page = DanbooruClient::builder()
            .page(7)
            .build()
            .get()
            .await;

        assert!(post_from_first_page.is_ok());
        assert!(post_from_specific_page.is_ok());

        assert_ne!(
            post_from_first_page.unwrap()[0].id,
            post_from_specific_page.unwrap()[0].id
        );
    }

    #[test]
    fn parse_rating_tags() {
        assert_eq!("explicit", DanbooruRating::Explicit.to_string());
        assert_eq!("questionable", DanbooruRating::Questionable.to_string());
        assert_eq!("sensitive", DanbooruRating::Sensitive.to_string());
        assert_eq!("general", DanbooruRating::General.to_string());
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
