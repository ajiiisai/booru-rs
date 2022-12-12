use booru_rs::{
    client::danbooru::DanbooruClient,
    model::danbooru::{DanbooruRating, DanbooruSort},
};

#[tokio::test]
async fn get_posts_with_tag() {
    let posts = DanbooruClient::builder().tag("kafuu_chino").get().await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_rating() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino")
        .rating(DanbooruRating::General)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_sort() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino")
        .sort(DanbooruSort::Score)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_blacklist_tag() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino")
        .blacklist_tag(DanbooruRating::Explicit)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_limit() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino")
        .rating(DanbooruRating::General)
        .limit(3)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() == 3);
}

#[tokio::test]
async fn get_posts_multiple_tags() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino")
        .tag("bangs")
        .limit(3)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_random_posts() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino")
        .random(true)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_post_by_id() {
    let post = DanbooruClient::builder().get_by_id(5817461).await;

    assert!(post.is_ok());
    assert_eq!(
        "569b8df8a16d7b42b4c244bfa0b6a838",
        post.unwrap().md5.unwrap()
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
    assert_eq!("id", DanbooruSort::Id.to_string());
    assert_eq!("score", DanbooruSort::Score.to_string());
    assert_eq!("rating", DanbooruSort::Rating.to_string());
    assert_eq!("user", DanbooruSort::User.to_string());
    assert_eq!("height", DanbooruSort::Height.to_string());
    assert_eq!("width", DanbooruSort::Width.to_string());
    assert_eq!("source", DanbooruSort::Source.to_string());
    assert_eq!("updated", DanbooruSort::Updated.to_string());
}
