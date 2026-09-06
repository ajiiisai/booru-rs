#[cfg(test)]
mod danbooru {
    use booru_rs::{
        client::generic::Sort,
        danbooru::{Client, DanbooruRating},
    };

    fn client() -> Client {
        Client::new().expect("default client must build")
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_tag() {
        let posts = client().search().tag("kafuu_chino").send().await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_rating() {
        let posts = client()
            .search()
            .tag("kafuu_chino")
            .rating(DanbooruRating::General)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_sort() {
        let posts = client()
            .search()
            .tag("kafuu_chino")
            .sort(Sort::Rating)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_excluded_rating() {
        let posts = client()
            .search()
            .tag("kafuu_chino")
            .exclude_rating(DanbooruRating::Explicit)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_limit() {
        let posts = client().search().tag("kafuu_chino").limit(3).send().await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().len() == 3);
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_multiple_tags() {
        let posts = client()
            .search()
            .tag("kafuu_chino")
            .tag("1girl")
            .limit(3)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_random_posts() {
        let posts = client().search().tag("kafuu_chino").random().send().await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_post_by_id() {
        let post = client().post(7452417).await;

        assert!(post.is_ok());
        assert_eq!(
            "d796ffc0c83585bb2e836f8d49653675",
            post.unwrap().md5.unwrap()
        );
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_from_page() {
        let post_from_first_page = client().search().send().await;

        let post_from_specific_page = client().search().start_page(7).send().await;

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
