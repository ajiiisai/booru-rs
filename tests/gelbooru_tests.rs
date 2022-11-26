use booru_rs::{client::gelbooru::GelbooruClient, model::gelbooru::{GelbooruRating, GelbooruSort}};

#[tokio::test]
async fn get_posts_with_tag() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_rating() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .rating(GelbooruRating::General)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_sort() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .sort(GelbooruSort::Score)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_blacklist_tag() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .blacklist_tag(GelbooruRating::Explicit.to_string())
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_limit() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .rating(GelbooruRating::General)
        .limit(3)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() == 3);
}

#[tokio::test]
async fn get_posts_multiple_tags() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .tag("bangs".to_string())
        .limit(3)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_random_posts() {
    let posts = GelbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .random(true)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_post_by_id() {
    let post = GelbooruClient::builder()
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
    assert_eq!("id", GelbooruSort::Id.to_string());
    assert_eq!("score", GelbooruSort::Score.to_string());
    assert_eq!("rating", GelbooruSort::Rating.to_string());
    assert_eq!("user", GelbooruSort::User.to_string());
    assert_eq!("height", GelbooruSort::Height.to_string());
    assert_eq!("width", GelbooruSort::Width.to_string());
    assert_eq!("source", GelbooruSort::Source.to_string());
    assert_eq!("updated", GelbooruSort::Updated.to_string());
}
