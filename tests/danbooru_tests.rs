use booru_rs::{client::danbooru::DanbooruClient, model::danbooru::{DanbooruRating, DanbooruSort}};

#[tokio::test]
async fn get_posts_with_tag() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_rating() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .rating(DanbooruRating::G)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_sort() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .sort(DanbooruSort::Score)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_blacklist_tag() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .blacklist_tag(DanbooruRating::E.to_string())
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_with_limit() {
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .rating(DanbooruRating::G)
        .limit(3)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() == 3);
}

#[tokio::test]
async fn get_posts_multiple_tags() {
    let posts = DanbooruClient::builder()
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
    let posts = DanbooruClient::builder()
        .tag("kafuu_chino".to_string())
        .random(true)
        .get()
        .await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_post_by_id() {
    let post = DanbooruClient::builder()
        .get_by_id(5817461)
        .await;

    assert!(post.is_ok());
    assert_eq!("569b8df8a16d7b42b4c244bfa0b6a838", post.unwrap().md5.unwrap());
}

#[test]
fn parse_rating_tags() {
    assert_eq!("e", DanbooruRating::E.to_string());
    assert_eq!("q", DanbooruRating::Q.to_string());
    assert_eq!("s", DanbooruRating::S.to_string());
    assert_eq!("g", DanbooruRating::G.to_string());
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
