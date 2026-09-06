#[cfg(test)]
mod safebooru {
    use booru_rs::{
        client::{Client, generic::Sort, safebooru::SafebooruClient},
        error::BooruError,
        safebooru::SafebooruRating,
    };

    fn result_or_skip<T>(result: booru_rs::error::Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(BooruError::Request(_) | BooruError::Parse(_)) => None,
            Err(error) => panic!("Safebooru request failed: {error}"),
        }
    }

    #[tokio::test]
    async fn get_posts_with_tag() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_rating() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .rating(SafebooruRating::General)
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_sort() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .sort(Sort::Score)
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_blacklist_tag() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .blacklist_tag(SafebooruRating::Explicit)
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_posts_with_limit() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .limit(3)
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert_eq!(posts.len(), 3);
    }

    #[tokio::test]
    async fn get_posts_multiple_tags() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .tag("bangs")
            .unwrap()
            .limit(3)
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_random_posts() {
        let posts = SafebooruClient::builder()
            .tag("kafuu_chino")
            .unwrap()
            .random()
            .build()
            .get()
            .await;

        let Some(posts) = result_or_skip(posts) else {
            return;
        };
        assert!(!posts.is_empty());
    }

    #[tokio::test]
    async fn get_post_by_id() {
        let post = SafebooruClient::builder().build().get_by_id(4348760).await;

        let Some(post) = result_or_skip(post) else {
            return;
        };
        assert_eq!("3e407a7848804119f1064c2aac731545", post.hash);
    }

    #[tokio::test]
    async fn get_posts_from_page() {
        let post_from_first_page = SafebooruClient::builder().build().get().await;

        let post_from_specific_page = SafebooruClient::builder().page(7).build().get().await;

        let Some(post_from_first_page) = result_or_skip(post_from_first_page) else {
            return;
        };
        let Some(post_from_specific_page) = result_or_skip(post_from_specific_page) else {
            return;
        };

        assert_ne!(post_from_first_page[0].id, post_from_specific_page[0].id);
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
