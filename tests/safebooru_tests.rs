#[cfg(test)]
mod safebooru {
    use booru_rs::{
        client::generic::Sort,
        safebooru::{Client, SafebooruRating},
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
            .rating(SafebooruRating::General)
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
            .sort(Sort::Score)
            .send()
            .await;

        assert!(posts.is_ok());
        assert!(!posts.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "contacts a live booru service; run explicitly with --ignored"]
    async fn get_posts_with_blacklist_tag() {
        let posts = client()
            .search()
            .tag("kafuu_chino")
            .blacklist_tag(SafebooruRating::Explicit)
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
            .tag("bangs")
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
        let post = client().post(4348760).await;

        assert!(post.is_ok());
        assert_eq!("3e407a7848804119f1064c2aac731545", post.unwrap().hash);
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
