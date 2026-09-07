use booru_rs::danbooru::{Client, Query};
use booru_rs::error::BooruError;
use booru_rs::retry::RetryConfig;
use std::time::Duration;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn posts_json(ids: &[u32]) -> String {
    let items: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                concat!(
                    "{{\"id\":{id},\"created_at\":\"2026-01-01T00:00:00.000Z\",",
                    "\"updated_at\":\"2026-01-01T00:00:00.000Z\",\"uploader_id\":1,",
                    "\"tag_string\":\"tag{id}\",\"tag_string_general\":\"tag{id}\",",
                    "\"tag_string_artist\":\"\",\"tag_string_copyright\":\"\",",
                    "\"tag_string_character\":\"\",\"tag_string_meta\":\"\",",
                    "\"source\":\"\",\"file_ext\":\"jpg\",\"file_size\":1,",
                    "\"image_width\":1,\"image_height\":1,\"score\":0,",
                    "\"up_score\":0,\"down_score\":0,\"fav_count\":0,",
                    "\"tag_count_general\":1,\"tag_count_artist\":0,",
                    "\"tag_count_copyright\":0,\"tag_count_character\":0,",
                    "\"tag_count_meta\":0,\"has_large\":false,\"has_children\":false,",
                    "\"has_visible_children\":false,\"has_active_children\":false,",
                    "\"is_banned\":false,\"is_deleted\":false,\"is_flagged\":false,",
                    "\"is_pending\":false,\"bit_flags\":0}}"
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

async fn mock_pages(mock_server: &MockServer, pages: &[Vec<u32>]) {
    for (index, ids) in pages.iter().enumerate() {
        let page = index + 1;
        Mock::given(method("GET"))
            .and(path("/posts.json"))
            .and(query_param("page", page.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(ids)))
            .mount(mock_server)
            .await;
    }
}

fn test_client(mock_server: &MockServer) -> Client {
    Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .set_credentials("test_key", "test_user")
        .build()
        .unwrap()
}

fn ids(posts: &[booru_rs::model::danbooru::DanbooruPost]) -> Vec<u32> {
    posts.iter().map(|post| post.id).collect()
}

#[tokio::test]
async fn search_sends_tags_limit_and_credentials() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
        .and(query_param("limit", "10"))
        .and(query_param("page", "1"))
        .and(query_param("tags", "cat_ears artist:foo bar"))
        .and(query_param("login", "test_user"))
        .and(query_param("api_key", "test_key"))
        .and(header(
            "User-Agent",
            concat!("booru-rs/", env!("CARGO_PKG_VERSION")),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[7654321])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .raw_query("artist:foo bar")
        .limit(10)
        .send()
        .await
        .expect("search must succeed");

    assert_eq!(ids(&posts), vec![7654321]);
}

#[tokio::test]
async fn third_tag_rejected_before_request() {
    let mock_server = MockServer::start().await;
    let client = test_client(&mock_server);

    let result = client
        .search()
        .tag("one")
        .tag("two")
        .tag("three")
        .send()
        .await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::TagLimitExceeded {
            max: 2,
            actual: 3,
            ..
        }
    ));
}

#[test]
fn tag_limit_matches_old_contract() {
    let error = Query::new()
        .tag("tag1")
        .tag("tag2")
        .tag("tag3")
        .validate()
        .unwrap_err();

    assert!(matches!(
        error,
        BooruError::TagLimitExceeded {
            client: "DanbooruClient",
            max: 2,
            actual: 3
        }
    ));
}

#[test]
fn validate_preflight() {
    assert!(Query::new().tag("cat_ears").validate().is_ok());
    assert!(Query::new().tag("cat ears").validate().is_err());
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

#[tokio::test]
async fn post_decodes_object_and_sends_credentials() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts/7654321.json"))
        .and(query_param("login", "test_user"))
        .and(query_param("api_key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(single_post_json(7654321)))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let post = client.post(7654321).await.expect("lookup must succeed");

    assert_eq!(post.id, 7654321);
}

#[tokio::test]
async fn post_missing_maps_to_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts/99999.json"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
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
async fn request_policy_retries_transient_statuses() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
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
async fn autocomplete_uses_instance_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/autocomplete.json"))
        .and(query_param("search[query]", "cat_"))
        .and(query_param("limit", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{"value":"cat_ears","label":"Cat ears (123)","category":0,"post_count":123,"tag":{"name":"cat_ears","category":0},"type":"tag"}]"#,
        ))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let suggestions = client
        .autocomplete("cat_", 3)
        .await
        .expect("complete must succeed");

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].name, "cat_ears");
    assert_eq!(suggestions[0].post_count, Some(123));
    assert_eq!(suggestions[0].category, Some(0));
    assert_eq!(suggestions[0].tag.as_deref(), Some("cat_ears"));
    assert_eq!(suggestions[0].suggestion_type.as_deref(), Some("tag"));
}

#[tokio::test]
async fn autocomplete_caps_server_results_to_requested_limit() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/autocomplete.json"))
        .and(query_param("limit", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[
                {"value":"cat_ears","label":"Cat ears (123)"},
                {"value":"cat_girl","label":"Cat girl (456)"}
            ]"#,
        ))
        .mount(&mock_server)
        .await;

    let suggestions = test_client(&mock_server)
        .autocomplete("cat_", 1)
        .await
        .unwrap();

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].name, "cat_ears");
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
        .and(path("/posts.json"))
        .and(query_param("page", u32::MAX.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[1])))
        .mount(&mock_server)
        .await;

    let page = test_client(&mock_server)
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

#[test]
fn default_client_builds() {
    assert!(Client::new().is_ok());
}

#[test]
fn client_shares_safely_across_tasks() {
    fn assert_send_sync_clone<T: Send + Sync + Clone>() {}

    assert_send_sync_clone::<Client>();
    assert_send_sync_clone::<booru_rs::danbooru::Search>();
    assert_send_sync_clone::<booru_rs::danbooru::Query>();
}

#[tokio::test]
async fn sort_keeps_provider_prefix_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
        .and(query_param("tags", "order:score"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let posts = client
        .search()
        .sort(booru_rs::client::generic::Sort::Score)
        .send()
        .await
        .expect("search must succeed");

    assert!(posts.is_empty());
}

#[tokio::test]
async fn exclude_rating_appends_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
        .and(query_param("tags", "-spoiler -rating:explicit"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .blacklist_tag("spoiler")
        .exclude_rating(booru_rs::model::danbooru::DanbooruRating::Explicit)
        .send()
        .await
        .expect("search must succeed");

    assert!(posts.is_empty());
}

#[tokio::test]
async fn random_requests_random_order() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
        .and(query_param("tags", "order:random"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .random()
        .send()
        .await
        .expect("search must succeed");

    assert!(posts.is_empty());
}

#[tokio::test]
async fn start_page_starts_there() {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, &[vec![], vec![7]]).await;
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
async fn start_page_zero_clamps_to_first_page() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[7])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .start_page(0)
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
#[test]
fn common_score_preserves_provider_range() {
    use booru_rs::model::Post;

    let mut post: booru_rs::model::danbooru::DanbooruPost =
        serde_json::from_str(&single_post_json(1)).unwrap();
    for score in [i32::MIN, -1, 0, i32::MAX] {
        post.score = score;
        assert_eq!(post.score(), Some(i64::from(score)));
    }
}
#[test]
fn post_preserves_unknown_rating() {
    use booru_rs::danbooru::{DanbooruPost, DanbooruPostRating, DanbooruRating};

    let mut value: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    value["rating"] = "x".into();
    let post: DanbooruPost = serde_json::from_value(value).unwrap();
    assert_eq!(post.rating, Some(DanbooruPostRating::Unknown("x".into())));
    assert_eq!(post.rating.as_ref().unwrap().to_string(), "x");
    assert!(serde_json::from_value::<DanbooruRating>("x".into()).is_err());
}
#[test]
fn post_preserves_optional_metadata() {
    use booru_rs::danbooru::DanbooruPost;

    let mut value: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    value["tag_count"] = 42.into();
    value["last_commented_at"] = "2026-01-02T03:04:05.000Z".into();
    value["media_asset"] = serde_json::json!({
        "id": 7,
        "status": "active",
        "nested": {"provider_key": true}
    });
    let post: DanbooruPost = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(post.tag_count, Some(42));
    assert_eq!(
        post.last_commented_at.as_deref(),
        Some("2026-01-02T03:04:05.000Z")
    );
    assert_eq!(post.media_asset, Some(value["media_asset"].clone()));
    let serialized = serde_json::to_value(&post).unwrap();
    assert_eq!(serialized["tag_count"], 42);
    assert_eq!(serialized["last_commented_at"], value["last_commented_at"]);
    assert_eq!(serialized["media_asset"], value["media_asset"]);
}

#[test]
fn post_allows_missing_and_null_optional_metadata() {
    use booru_rs::danbooru::DanbooruPost;

    let fixture: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    for null_value in [false, true] {
        let mut value = fixture.clone();
        for field in ["tag_count", "last_commented_at", "media_asset"] {
            if null_value {
                value[field] = serde_json::Value::Null;
            } else {
                value.as_object_mut().unwrap().remove(field);
            }
        }
        let post: DanbooruPost = serde_json::from_value(value).unwrap();
        assert_eq!(post.tag_count, None);
        assert_eq!(post.last_commented_at, None);
        assert_eq!(post.media_asset, None);
    }
}

#[test]
fn post_rejects_invalid_tag_count_and_timestamp() {
    use booru_rs::danbooru::DanbooruPost;

    let fixture: serde_json::Value = serde_json::from_str(&single_post_json(1)).unwrap();
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(u64::from(u32::MAX) + 1),
        serde_json::json!(1.5),
        serde_json::json!("42"),
        serde_json::json!([]),
    ] {
        let mut value = fixture.clone();
        value["tag_count"] = invalid;
        assert!(serde_json::from_value::<DanbooruPost>(value).is_err());
    }
    for invalid in [
        serde_json::json!(1),
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        let mut value = fixture.clone();
        value["last_commented_at"] = invalid;
        assert!(serde_json::from_value::<DanbooruPost>(value).is_err());
    }
}

#[tokio::test]
async fn trait_post_preserves_error_context() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/posts/42.json"))
        .respond_with(ResponseTemplate::new(404))
        .expect(2)
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();
    let inherent_error = client.post(42).await.unwrap_err();
    let trait_error = booru_rs::client::Client::post(&client, 42)
        .await
        .unwrap_err();

    for error in [inherent_error, trait_error] {
        let context = error.context().expect("error must carry context");
        assert_eq!(context.provider, booru_rs::error::Provider::Danbooru);
        assert_eq!(context.operation, booru_rs::error::Operation::Post);
        assert!(error.is_not_found());
        assert!(matches!(error.source_error(), BooruError::PostNotFound(42)));
    }
}

#[test]
fn multiple_tags_validation_succeeds() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .tags(vec!["cat", "pink_hair"]);

    assert!(search.query().validate().is_ok());
}

#[test]
fn multiple_tags_validation_fails() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .tags(vec!["cat", "pink hair"]);

    assert!(search.query().validate().is_err());
}

#[test]
fn multiple_tags_are_apended() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .tag("cat")
        .tags(["pink_hair"]);

    let expected = Query::new().tag("cat").tag("pink_hair");

    assert_eq!(search.query(), &expected);
}

#[test]
fn raw_queries_are_appended() {
    let search = Client::builder()
        .build()
        .expect("builder must succeed")
        .search()
        .raw_query("artist:foo")
        .raw_queries(["score:>10", "order:score"]);

    let expected = Query::new()
        .raw_query("artist:foo")
        .raw_query("score:>10")
        .raw_query("order:score");

    assert_eq!(search.query(), &expected);
}
