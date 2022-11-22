use booru_rs::{client::gelbooru::GelbooruClient, model::gelbooru::GelbooruRating};

#[tokio::test]
async fn get_post_by_id() {
    let client = GelbooruClient::new(None);

    let post = client.get_post_by_id(7948920).await;

    assert!(post.is_ok());
}

#[tokio::test]
async fn get_posts_by_tag() {
    let client = GelbooruClient::new(None);

    let posts = client.get_posts_by_tag("kafuu_chino rating:general").await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}

#[tokio::test]
async fn get_posts_by_tag_and_rating() {
    let client = GelbooruClient::new(None);

    let posts = client.get_posts_by_tag_and_rating("kafuu_chino", GelbooruRating::General).await;

    assert!(posts.is_ok());
    assert_eq!(true, posts.unwrap().len() > 0);
}
