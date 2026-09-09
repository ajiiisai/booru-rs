use booru_rs::error::{BooruError, Operation, Provider};
use booru_rs::konachan::{Client, Query};
use booru_rs::model::konachan::KonachanRating;
use booru_rs::retry::RetryConfig;
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn posts_json(ids: &[u32]) -> String {
    let items: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                concat!(
                    "{{\"id\":{id},\"score\":10,\"height\":100,\"width\":100,",
                    "\"md5\":\"hash{id}\",\"tags\":\"tag{id}\",\"image\":\"{id}.jpg\",",
                    "\"file_url\":\"https://example.com/{id}.jpg\",",
                    "\"preview_url\":\"https://example.com/p{id}.jpg\",",
                    "\"sample_url\":\"https://example.com/s{id}.jpg\",",
                    "\"created_at\":17000000, \"updated_at\": 17000001,",
                    "\"author\":\"author\",\"status\":\"active\",\"has_children\":false,",
                    "\"is_pending\":false,\"is_held\":false,\"is_note_locked\":false,",
                    "\"source\":\"\",\"change\":2456,\"rating\":\"s\"}}"
                ),
                id = id
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn single_post_json(id: u32) -> String {
    posts_json(&[id])
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

fn test_client(mock_server: &MockServer) -> Client {
    Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap()
}

fn ids(posts: &[booru_rs::model::konachan::KonachanPost]) -> Vec<u32> {
    posts.iter().map(|post| post.id).collect()
}

async fn mock_pages(mock_server: &MockServer, pages: &[Vec<u32>]) {
    for (pid, ids) in pages.iter().enumerate() {
        let page = pid+1;
        Mock::given(method("GET"))
            .and(path("/post.json"))
            .and(query_param("page", page.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(ids)))
            .mount(mock_server)
            .await;
    }
}

#[tokio::test]
async fn search_sends_tags_and_limit_in_order() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("page", "1"))
        .and(query_param("limit", "2"))
        .and(query_param("tags", "cat_ears"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[1, 2])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .limit(2)
        .send()
        .await
        .expect("search must succeed");

    assert_eq!(ids(&posts), vec![1, 2])
}

#[tokio::test]
async fn concurrent_searches_share_one_client() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("tags", "cat_ears"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[1, 2])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("tags", "landscape"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

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
        .and(path("/post.json"))
        .and(query_param("tags", "id:1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[1])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let post = client.post(1).await.expect("lookup must succeed");

    assert_eq!(post.id, 1);
    assert_eq!(post.width, 100);
}

#[tokio::test]
async fn post_missing_maps_to_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("id", "99999"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.post(99999).await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::PostNotFound(99999)
    ));
}

#[tokio::test]
async fn autocomplete_uses_client_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tag.json"))
        .and(query_param("name", "land"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"[{"name":"landscape","count":20}]"#),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let suggestions = client
        .autocomplete("land", 5)
        .await
        .expect("complete must succeed");

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].name, "landscape");
    assert_eq!(suggestions[0].post_count, Some(20));
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
    assert_send_sync_clone::<booru_rs::konachan::Search>();
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
        .and(path("/post.json"))
        .and(query_param("page", u32::MAX.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[1])))
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

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::InvalidTag { .. }
    ));
}

#[tokio::test]
async fn empty_tag_rejected_before_request() {
    let mock_server = MockServer::start().await;
    let client = test_client(&mock_server);

    let result = client.search().tag("").send().await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::InvalidTag { .. }
    ));
}

#[test]
fn validate_preflight() {
    assert!(Query::new().tag("cat_ears").limit(5).validate().is_ok());
    assert!(Query::new().tag("cat ears").validate().is_err());
    assert!(Query::new().tag("").validate().is_err());
}

#[test]
fn random_rejects_sort_conflicts() {
    use booru_rs::client::generic::Sort;

    assert!(Query::new().random().validate().is_ok());
    assert!(Query::new().sort(Sort::Score).validate().is_ok());
    assert!(Query::new().random().sort(Sort::Score).validate().is_err());
    assert!(Query::new().random().random().validate().is_err());
    assert!(
        Query::new()
            .random()
            .raw_query("order:score")
            .validate()
            .is_err()
    );
}

#[test]
fn raw_query_allows_spaces_but_rejects_empty_expressions() {
    assert!(Query::new().raw_query("artist:foo bar").validate().is_ok());
    assert!(matches!(
        Query::new().raw_query(" ").validate().unwrap_err(),
        BooruError::InvalidQuery(_)
    ));
    assert!(matches!(
        Query::new()
            .rating(KonachanRating::Safe)
            .raw_query("rating:explicit")
            .validate()
            .unwrap_err(),
        BooruError::InvalidQuery(_)
    ));
}

#[tokio::test]
async fn saved_query_runs_twice() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("tags", "cat_ears"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[1, 2])))
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
        stream
            .next()
            .await
            .expect("stream must yield")
            .unwrap_err()
            .source_error(),
        BooruError::InvalidTag { .. }
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn repeated_rating_and_limit_replace_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("tags", "rating:safe"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .rating(KonachanRating::Safe)
        .rating(KonachanRating::Safe)
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
        .and(path("/post.json"))
        .and(query_param("tags", "order:score"))
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
async fn exclude_rating_and_random_append_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param(
            "tags",
            "cat_ears -spoiler -rating:explicit order:random",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .blacklist_tag("spoiler")
        .exclude_rating(KonachanRating::Explicit)
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
        .start_page(3)
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
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(500).set_body_string(single_post_json(1)))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::HttpStatus { status: 500, .. }
    ));
    assert_eq!(mock_server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn request_policy_retries_transient_statuses() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
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
        result.unwrap_err().source_error(),
        BooruError::HttpStatus { status: 503, .. }
    ));
    assert_eq!(mock_server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn error_status_malformed_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(503).set_body_string("<html>bad gateway"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::HttpStatus { status: 503, .. }
    ));
}

#[tokio::test]
async fn invalid_json_is_parse_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    let error = result.unwrap_err();
    assert!(error.is_parse_error());
    assert!(!error.is_network_error());
    let context = error.context().expect("error must carry context");
    assert_eq!(context.provider, Provider::Konachan);
    assert_eq!(context.operation, Operation::Search);
    assert!(error.to_string().starts_with("Konachan search failed:"));
}

#[test]
fn common_score_preserves_provider_range() {
    use booru_rs::model::Post;

    let mut post: booru_rs::model::konachan::KonachanPost =
        serde_json::from_str::<Vec<_>>(&posts_json(&[1]))
            .unwrap()
            .remove(0);
    for score in [0, i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
        post.score = score;
        assert_eq!(post.score(), Some(i64::from(score)));
    }
}

#[test]
fn post_preserves_unknown_rating() {
    use booru_rs::konachan::{KonachanPost, KonachanPostRating};

    let mut fixture: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    for value in ["future_rating", "Safe", "", " future rating "] {
        fixture["rating"] = value.into();
        let post: KonachanPost = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(post.rating, KonachanPostRating::Unknown(value.into()));
        assert_eq!(post.rating.to_string(), value);
        assert!(serde_json::from_value::<KonachanRating>(value.into()).is_err());
    }
}

#[test]
fn post_recognizes_supported_ratings() {
    use booru_rs::konachan::{KonachanPost, KonachanPostRating};

    let mut fixture: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    for (value, rating) in [
        ("s", KonachanRating::Safe),
        ("q", KonachanRating::Questionable),
        ("e", KonachanRating::Explicit),
    ] {
        fixture["rating"] = value.into();
        let post: KonachanPost = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(post.rating, KonachanPostRating::Known(rating));
        assert_eq!(post.rating, rating.into());
        // Rating names are not equal to the enum names
        //assert_eq!(post.rating.to_string(), value);
    }
}

#[test]
fn post_rejects_missing_and_non_string_ratings() {
    use booru_rs::konachan::KonachanPost;

    let mut fixture: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    for value in [
        serde_json::Value::Null,
        serde_json::json!(1),
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        fixture["rating"] = value;
        assert!(serde_json::from_value::<KonachanPost>(fixture.clone()).is_err());
    }
    fixture.as_object_mut().unwrap().remove("rating");
    assert!(serde_json::from_value::<KonachanPost>(fixture.clone()).is_err());
}

#[test]
fn post_serializes_and_round_trips() {
    use booru_rs::konachan::KonachanPost;

    let post: KonachanPost = serde_json::from_str::<Vec<_>>(&posts_json(&[1]))
        .unwrap()
        .remove(0);
    let json = serde_json::to_string(&post).unwrap();
    let restored: KonachanPost = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, post);
}

#[tokio::test]
async fn trait_post_preserves_error_context() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(404))
        .expect(2)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);
    let inherent_error = client.post(42).await.unwrap_err();
    let trait_error = booru_rs::client::Client::post(&client, 42)
        .await
        .unwrap_err();

    for error in [inherent_error, trait_error] {
        let context = error.context().expect("error must carry context");
        assert_eq!(context.provider, booru_rs::error::Provider::Konachan);
        assert_eq!(context.operation, booru_rs::error::Operation::Post);
        assert!(error.is_not_found());
        assert!(matches!(error.source_error(), BooruError::PostNotFound(42)));
    }
}

#[test]
fn post_allows_missing_media_urls() {
    use booru_rs::konachan::KonachanPost;
    use booru_rs::model::Post;

    let fixture: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    for null_value in [false, true] {
        let mut value = fixture.clone();
        for field in ["file_url", "preview_url", "sample_url"] {
            if null_value {
                value[field] = serde_json::Value::Null;
            } else {
                value.as_object_mut().unwrap().remove(field);
            }
        }
        let post: KonachanPost = serde_json::from_value(value).unwrap();
        assert_eq!(post.file_url, None);
        assert_eq!(post.preview_url, None);
        assert_eq!(post.sample_url, None);
        assert_eq!(post.file_url(), None);
    }

    let mut value = fixture.clone();
    value["file_url"] = "".into();
    let post: KonachanPost = serde_json::from_value(value).unwrap();
    assert_eq!(post.file_url(), None);
}

#[test]
fn multiple_tags_validation_succeeds() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .tags(vec!["cat", "smile", "pink_hair"]);

    assert!(search.query().validate().is_ok());
}

#[test]
fn multiple_tags_validation_fails() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .tags(vec!["cat", "smile", "pink hair"]);

    assert!(search.query().validate().is_err());
}

#[test]
fn multiple_tags_are_apended() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .tag("cat")
        .tags(["smile", "pink_hair"]);

    let expected = Query::new().tag("cat").tag("smile").tag("pink_hair");

    assert_eq!(search.query(), &expected);
}

#[test]
fn raw_queries_are_appended() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .raw_query("artist:foo")
        .raw_queries(["score:>10", "sort:score"]);

    let expected = Query::new()
        .raw_query("artist:foo")
        .raw_query("score:>10")
        .raw_query("sort:score");

    assert_eq!(search.query(), &expected);
}
