use booru_rs::error::BooruError;
use booru_rs::safebooru::Client;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn posts_fixture() -> &'static str {
    include_str!("fixtures/safebooru/posts.json")
}

fn single_post_fixture() -> &'static str {
    include_str!("fixtures/safebooru/post.json")
}

#[tokio::test]
async fn search_sends_tags_and_limit_in_order() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("page", "dapi"))
        .and(query_param("s", "post"))
        .and(query_param("q", "index"))
        .and(query_param("pid", "0"))
        .and(query_param("limit", "2"))
        .and(query_param("tags", "cat_ears"))
        .and(query_param("json", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_fixture()))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let posts = client
        .search()
        .tag("cat_ears")
        .limit(2)
        .send()
        .await
        .expect("search must succeed");

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].id, 12345);
    assert_eq!(posts[1].id, 12346);
}

#[tokio::test]
async fn concurrent_searches_share_one_client() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "cat_ears"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_fixture()))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "landscape"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let (cats, landscapes) = tokio::join!(
        client.search().tag("cat_ears").send(),
        client.search().tag("landscape").send(),
    );

    assert_eq!(cats.expect("search must succeed").len(), 2);
    assert!(landscapes.expect("search must succeed").is_empty());
}

#[tokio::test]
async fn post_returns_single_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("id", "12345"))
        .respond_with(ResponseTemplate::new(200).set_body_string(single_post_fixture()))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let post = client.post(12345).await.expect("lookup must succeed");

    assert_eq!(post.id, 12345);
    assert_eq!(post.width, 1920);
}

#[tokio::test]
async fn post_missing_maps_to_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("id", "99999"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let result = client.post(99999).await;

    assert!(matches!(
        result.unwrap_err(),
        BooruError::PostNotFound(99999)
    ));
}

#[tokio::test]
async fn autocomplete_uses_client_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/autocomplete.php"))
        .and(query_param("q", "land"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"[{"value":"landscape","label":"landscape (123)"}]"#),
        )
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let suggestions = client
        .autocomplete("land", 5)
        .await
        .expect("complete must succeed");

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].name, "landscape");
    assert_eq!(suggestions[0].post_count, Some(123));
}

#[test]
fn default_client_builds() {
    assert!(Client::new().is_ok());
}

#[test]
fn invalid_endpoint_is_rejected() {
    assert!(matches!(
        Client::builder().endpoint("not a url").unwrap_err(),
        BooruError::InvalidUrl(_)
    ));
}

#[test]
fn client_shares_safely_across_tasks() {
    fn assert_send_sync_clone<T: Send + Sync + Clone>() {}

    assert_send_sync_clone::<Client>();
    assert_send_sync_clone::<booru_rs::safebooru::Search>();
}
