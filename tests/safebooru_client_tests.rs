use booru_rs::error::BooruError;
use booru_rs::model::safebooru::SafebooruRating;
use booru_rs::retry::RetryConfig;
use booru_rs::safebooru::{Client, Query};
use std::time::Duration;
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

fn posts_json(ids: &[u32]) -> String {
    let items: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                concat!(
                    "{{\"id\":{id},\"score\":10,\"height\":100,\"width\":100,",
                    "\"hash\":\"hash{id}\",\"tags\":\"tag{id}\",\"image\":\"{id}.jpg\",",
                    "\"directory\":1,\"file_url\":\"https://example.com/{id}.jpg\",",
                    "\"preview_url\":\"https://example.com/p{id}.jpg\",",
                    "\"sample_url\":\"https://example.com/s{id}.jpg\",",
                    "\"source\":\"\",\"change\":1700000000,\"rating\":\"general\"}}"
                ),
                id = id
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

async fn mock_pages(mock_server: &MockServer, pages: &[Vec<u32>]) {
    for (pid, ids) in pages.iter().enumerate() {
        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("pid", pid.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(ids)))
            .mount(mock_server)
            .await;
    }
}

fn test_client(mock_server: &MockServer) -> Client {
    Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap()
}

fn ids(posts: &[booru_rs::model::safebooru::SafebooruPost]) -> Vec<u32> {
    posts.iter().map(|post| post.id).collect()
}

#[tokio::test]
async fn page_returns_continuation_until_empty() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![1, 2], vec![3, 4], vec![]]).await;
    let client = test_client(&mock_server);

    let page = client.search().page().await.expect("page must succeed");
    assert_eq!(ids(&page.posts), vec![1, 2]);

    let page = page
        .next
        .expect("second page must follow")
        .page()
        .await
        .expect("page must succeed");
    assert_eq!(ids(&page.posts), vec![3, 4]);

    let page = page
        .next
        .expect("terminal fetch must follow")
        .page()
        .await
        .expect("page must succeed");
    assert!(page.posts.is_empty());
    assert!(page.next.is_none());
}

#[tokio::test]
async fn last_page_has_no_wrapping_continuation() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("pid", u32::MAX.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_fixture()))
        .mount(&mock_server)
        .await;

    let page = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap()
        .search()
        .start_page(u32::MAX)
        .page()
        .await
        .unwrap();

    assert!(page.next.is_none());
}

#[tokio::test]
async fn posts_stream_preserves_order() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![1, 2], vec![3, 4], vec![]]).await;
    let client = test_client(&mock_server);

    let posts = client
        .search()
        .posts()
        .collect()
        .await
        .expect("stream must succeed");

    assert_eq!(ids(&posts), vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn posts_stream_runs_inside_spawned_task() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![1, 2], vec![]]).await;
    let client = test_client(&mock_server);

    let posts = tokio::spawn(async move { client.search().posts().collect().await })
        .await
        .expect("task must finish")
        .expect("stream must succeed");

    assert_eq!(ids(&posts), vec![1, 2]);
}

#[tokio::test]
async fn empty_first_page_ends_streams() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![]]).await;
    let client = test_client(&mock_server);

    let page = client.search().page().await.expect("page must succeed");
    assert!(page.posts.is_empty());
    assert!(page.next.is_none());

    let posts = client
        .search()
        .posts()
        .collect()
        .await
        .expect("stream must succeed");
    assert!(posts.is_empty());
}

#[tokio::test]
async fn whitespace_tag_rejected_before_request() {
    let mock_server = MockServer::start().await;
    let client = test_client(&mock_server);

    let result = client.search().tag("cat ears").send().await;

    assert!(matches!(result.unwrap_err(), BooruError::InvalidTag { .. }));
}

#[tokio::test]
async fn empty_tag_rejected_before_request() {
    let mock_server = MockServer::start().await;
    let client = test_client(&mock_server);

    let result = client.search().tag("").send().await;

    assert!(matches!(result.unwrap_err(), BooruError::InvalidTag { .. }));
}

#[test]
fn validate_preflight() {
    assert!(Query::new().tag("cat_ears").limit(5).validate().is_ok());
    assert!(Query::new().tag("cat ears").validate().is_err());
    assert!(Query::new().tag("").validate().is_err());
}

#[tokio::test]
async fn saved_query_runs_twice() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "cat_ears"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_fixture()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);
    let query = Query::new().tag("cat_ears").limit(2);

    for _ in 0..2 {
        let posts = client
            .search_with(query.clone())
            .send()
            .await
            .expect("search must succeed");
        assert_eq!(posts.len(), 2);
    }

    assert_eq!(
        client.search().tag("cat_ears").query(),
        &Query::new().tag("cat_ears")
    );
}

#[tokio::test]
async fn stream_yields_invalid_once_then_ends() {
    let mock_server = MockServer::start().await;
    let client = test_client(&mock_server);

    let mut stream = client.search().tag("cat ears").posts();

    assert!(matches!(
        stream.next().await.expect("stream must yield"),
        Err(BooruError::InvalidTag { .. })
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn repeated_rating_and_limit_replace_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "rating:general"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .rating(SafebooruRating::Safe)
        .rating(SafebooruRating::General)
        .limit(5)
        .limit(10)
        .send()
        .await
        .expect("search must succeed");
    assert!(posts.is_empty());
}

#[tokio::test]
async fn sort_keeps_provider_prefix_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "sort:score"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .sort(booru_rs::client::generic::Sort::Score)
        .send()
        .await
        .expect("search must succeed");

    assert!(posts.is_empty());
}

#[tokio::test]
async fn blacklist_and_random_append_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "cat_ears -explicit sort:random"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .blacklist_tag(SafebooruRating::Explicit)
        .random()
        .send()
        .await
        .expect("search must succeed");

    assert!(posts.is_empty());
}

#[tokio::test]
async fn page_setter_starts_there() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![], vec![], vec![7]]).await;
    let client = test_client(&mock_server);

    let posts = client
        .search()
        .start_page(2)
        .send()
        .await
        .expect("search must succeed");

    assert_eq!(ids(&posts), vec![7]);
}

#[tokio::test]
async fn posts_max_posts_caps_and_stops() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![1, 2], vec![3, 4], vec![]]).await;
    let client = test_client(&mock_server);

    let posts = client
        .search()
        .posts()
        .max_posts(3)
        .collect()
        .await
        .expect("stream must succeed");

    assert_eq!(ids(&posts), vec![1, 2, 3]);
}

#[tokio::test]
async fn pages_max_pages_caps_fetching() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![1, 2], vec![3, 4], vec![]]).await;
    let client = test_client(&mock_server);

    let mut pages = client.search().pages().max_pages(1);

    let page = pages.next().await.expect("page must follow");
    assert_eq!(ids(&page.expect("page must succeed").posts), vec![1, 2]);
    assert!(pages.next().await.is_none());
}

#[tokio::test]
async fn error_status_despite_decodable_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(500).set_body_string(posts_fixture()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    assert!(matches!(
        result.unwrap_err(),
        BooruError::HttpStatus { status: 500, .. }
    ));
    assert_eq!(mock_server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn request_policy_retries_transient_statuses() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(503).set_body_string("temporary outage"))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .retry_config(
            RetryConfig::new(1)
                .with_initial_delay(Duration::ZERO)
                .with_max_delay(Duration::ZERO),
        )
        .unwrap()
        .build()
        .unwrap();

    let result = client.search().send().await;

    assert!(matches!(
        result,
        Err(BooruError::HttpStatus { status: 503, .. })
    ));
    assert_eq!(mock_server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn error_status_malformed_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(503).set_body_string("<html>bad gateway"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    assert!(matches!(
        result.unwrap_err(),
        BooruError::HttpStatus { status: 503, .. }
    ));
}

#[tokio::test]
async fn invalid_json_is_parse_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    let error = result.unwrap_err();
    assert!(error.is_parse_error());
    assert!(!error.is_network_error());
}
#[test]
fn common_score_preserves_provider_range() {
    use booru_rs::model::Post;

    let mut post: booru_rs::model::safebooru::SafebooruPost =
        serde_json::from_str::<Vec<_>>(include_str!("fixtures/safebooru/post.json"))
            .unwrap()
            .remove(0);
    for score in [0, i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
        post.score = Some(score);
        assert_eq!(post.score(), Some(i64::from(score)));
    }
    post.score = None;
    assert_eq!(post.score(), None);
}
#[test]
fn post_decodes_null_height() {
    use booru_rs::model::Post;

    let mut value: serde_json::Value = serde_json::from_str(single_post_fixture()).unwrap();
    value[0]["height"] = serde_json::Value::Null;
    let posts = serde_json::from_value::<Vec<booru_rs::safebooru::SafebooruPost>>(value).unwrap();
    assert_eq!(posts[0].height, None);
    assert_eq!(posts[0].height(), None);
}

#[test]
fn post_height_preserves_missing_and_numeric_values() {
    use booru_rs::model::Post;

    let fixture: serde_json::Value = serde_json::from_str(single_post_fixture()).unwrap();
    for height in [None, Some(0), Some(1080), Some(u32::MAX)] {
        let mut value = fixture[0].clone();
        if let Some(height) = height {
            value["height"] = height.into();
        } else {
            value.as_object_mut().unwrap().remove("height");
        }
        let post: booru_rs::safebooru::SafebooruPost = serde_json::from_value(value).unwrap();
        assert_eq!(post.height, height);
        assert_eq!(post.height(), height);
    }
}

#[test]
fn post_rejects_invalid_heights_and_missing_identity() {
    let fixture: serde_json::Value = serde_json::from_str(single_post_fixture()).unwrap();
    for height in [
        serde_json::json!(-1),
        serde_json::json!(u64::from(u32::MAX) + 1),
        serde_json::json!("1080"),
        serde_json::json!(1.5),
    ] {
        let mut value = fixture[0].clone();
        value["height"] = height;
        assert!(serde_json::from_value::<booru_rs::safebooru::SafebooruPost>(value).is_err());
    }
    let mut value = fixture[0].clone();
    value.as_object_mut().unwrap().remove("id");
    assert!(serde_json::from_value::<booru_rs::safebooru::SafebooruPost>(value).is_err());
}
#[test]
fn post_preserves_unknown_rating() {
    use booru_rs::safebooru::{SafebooruPost, SafebooruPostRating, SafebooruRating};

    let mut fixture: serde_json::Value = serde_json::from_str(single_post_fixture()).unwrap();
    for value in ["future_rating", "Safe", "", " future rating "] {
        fixture[0]["rating"] = value.into();
        let post: SafebooruPost = serde_json::from_value(fixture[0].clone()).unwrap();
        assert_eq!(post.rating, SafebooruPostRating::Unknown(value.into()));
        assert_eq!(post.rating.to_string(), value);
        assert!(serde_json::from_value::<SafebooruRating>(value.into()).is_err());
    }
}

#[test]
fn post_recognizes_supported_ratings() {
    use booru_rs::safebooru::{SafebooruPost, SafebooruPostRating, SafebooruRating};

    let mut fixture: serde_json::Value = serde_json::from_str(single_post_fixture()).unwrap();
    for (value, rating) in [
        ("safe", SafebooruRating::Safe),
        ("general", SafebooruRating::General),
        ("questionable", SafebooruRating::Questionable),
        ("explicit", SafebooruRating::Explicit),
    ] {
        fixture[0]["rating"] = value.into();
        let post: SafebooruPost = serde_json::from_value(fixture[0].clone()).unwrap();
        assert_eq!(post.rating, SafebooruPostRating::Known(rating));
        assert_eq!(post.rating, rating.into());
        assert_eq!(post.rating.to_string(), value);
    }
}

#[test]
fn post_rejects_missing_and_non_string_ratings() {
    use booru_rs::safebooru::SafebooruPost;

    let mut fixture: serde_json::Value = serde_json::from_str(single_post_fixture()).unwrap();
    for value in [
        serde_json::Value::Null,
        serde_json::json!(1),
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        fixture[0]["rating"] = value;
        assert!(serde_json::from_value::<SafebooruPost>(fixture[0].clone()).is_err());
    }
    fixture[0].as_object_mut().unwrap().remove("rating");
    assert!(serde_json::from_value::<SafebooruPost>(fixture[0].clone()).is_err());
}
#[test]
fn post_serializes_and_round_trips() {
    use booru_rs::safebooru::SafebooruPost;

    let post: SafebooruPost = serde_json::from_str::<Vec<_>>(single_post_fixture())
        .unwrap()
        .remove(0);
    let json = serde_json::to_string(&post).unwrap();
    let restored: SafebooruPost = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, post);
}
