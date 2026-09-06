use booru_rs::error::BooruError;
use booru_rs::retry::RetryConfig;
use booru_rs::rule34::{Client, Query};
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn posts_json(ids: &[u32]) -> String {
    let items: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                concat!(
                    "{{\"id\":{id},\"score\":10,\"width\":100,\"height\":100,",
                    "\"file_url\":\"https://example.com/{id}.png\",",
                    "\"preview_url\":\"https://example.com/p{id}.png\",",
                    "\"sample_url\":\"https://example.com/s{id}.png\",",
                    "\"tags\":\"tag{id}\",\"rating\":\"safe\"}}"
                ),
                id = id
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn posts_fixture() -> &'static str {
    include_str!("fixtures/rule34/posts.json")
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
        .set_credentials("test_key", "test_user")
        .build()
        .unwrap()
}

fn ids(posts: &[booru_rs::model::rule34::Rule34Post]) -> Vec<u32> {
    posts.iter().map(|post| post.id).collect()
}

#[tokio::test]
async fn search_sends_tags_limit_and_credentials() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("page", "dapi"))
        .and(query_param("s", "post"))
        .and(query_param("q", "index"))
        .and(query_param("pid", "0"))
        .and(query_param("limit", "10"))
        .and(query_param("tags", "1girl"))
        .and(query_param("json", "1"))
        .and(query_param("api_key", "test_key"))
        .and(query_param("user_id", "test_user"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[15000000])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("1girl")
        .limit(10)
        .send()
        .await
        .expect("search must succeed");

    assert_eq!(ids(&posts), vec![15000000]);
}

#[tokio::test]
async fn auth_body_reports_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#""Missing authentication. Go to api.rule34.xxx for more information""#,
        ))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let result = client.search().send().await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::Unauthorized(_)
    ));
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
        result.unwrap_err().source_error(),
        BooruError::HttpStatus { status: 503, .. }
    ));
    assert_eq!(mock_server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn post_returns_first_array_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("id", "15000000"))
        .and(query_param("api_key", "test_key"))
        .and(query_param("user_id", "test_user"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[15000000])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let post = client.post(15000000).await.expect("lookup must succeed");

    assert_eq!(post.id, 15000000);
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

    let client = test_client(&mock_server);

    let result = client.post(99999).await;

    assert!(matches!(
        result.unwrap_err().source_error(),
        BooruError::PostNotFound(99999)
    ));
}

#[tokio::test]
async fn autocomplete_uses_instance_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/autocomplete.php"))
        .and(query_param("q", "cat_"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{"value":"cat_ears","label":"cat_ears (1)"},{"value":"cat_girl","label":"cat_girl (2)"},{"value":"cat_tail","label":"cat_tail (3)"}]"#,
        ))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let suggestions = client
        .autocomplete("cat_", 2)
        .await
        .expect("complete must succeed");

    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].name, "cat_ears");
    assert_eq!(suggestions[1].name, "cat_girl");
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
fn validate_preflight() {
    assert!(Query::new().tag("1girl").validate().is_ok());
    assert!(Query::new().tag("bad tag").validate().is_err());
}

#[test]
fn default_client_builds() {
    assert!(Client::new().is_ok());
}

#[test]
fn client_shares_safely_across_tasks() {
    fn assert_send_sync_clone<T: Send + Sync + Clone>() {}

    assert_send_sync_clone::<Client>();
    assert_send_sync_clone::<booru_rs::rule34::Search>();
    assert_send_sync_clone::<booru_rs::rule34::Query>();
}

#[tokio::test]
async fn exclude_rating_and_random_append_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param(
            "tags",
            "1girl -spoiler -rating:explicit sort:random",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("1girl")
        .blacklist_tag("spoiler")
        .exclude_rating(booru_rs::model::rule34::Rule34Rating::Explicit)
        .random()
        .send()
        .await
        .expect("search must succeed");

    assert!(posts.is_empty());
}

#[tokio::test]
async fn start_page_starts_there() {
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
async fn sort_keeps_provider_prefix_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "sort:score"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .set_credentials("test_key", "test_user")
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
async fn posts_containing_auth_text_decode() {
    let mock_server = MockServer::start().await;

    let body = r#"[{"id":15000001,"score":1,"width":100,"height":100,"file_url":"https://example.com/a.png","preview_url":"https://example.com/p.png","sample_url":"https://example.com/s.png","tags":"1girl","rating":"safe","source":"artwork about Missing authentication","hash":"abc"}]"#;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "1girl"))
        .and(query_param("api_key", "test_key"))
        .and(query_param("user_id", "test_user"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("1girl")
        .send()
        .await
        .expect("valid posts must decode");

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].id, 15000001);
}

#[tokio::test]
async fn malformed_body_reports_parse_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("api_key", "test_key"))
        .and(query_param("user_id", "test_user"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.search().send().await;

    let error = result.unwrap_err();
    assert!(matches!(error.source_error(), BooruError::Parse(_)));
    assert!(error.is_parse_error());
}

#[tokio::test]
async fn post_trait_methods() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_fixture()))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .set_credentials("test_key", "test_user")
        .build()
        .unwrap();

    let posts = client.search().send().await.unwrap();
    let post = &posts[0];

    use booru_rs::model::Post;
    assert_eq!(post.id(), 15000000);
    assert_eq!(post.width(), 900);
    assert_eq!(post.height(), Some(1200));
    assert_eq!(
        post.file_url(),
        Some("https://example.com/images/3900/rule34hash123.png")
    );
    assert_eq!(post.tags(), "1girl blue_hair");
    assert_eq!(post.score(), Some(75));
    assert_eq!(post.md5(), Some("rule34hash123"));
    assert_eq!(post.source(), Some("https://pixiv.net/artworks/789"));
}
#[test]
fn common_score_preserves_provider_range() {
    use booru_rs::model::Post;

    let mut post: booru_rs::model::rule34::Rule34Post =
        serde_json::from_str::<Vec<_>>(include_str!("fixtures/rule34/posts.json"))
            .unwrap()
            .remove(0);
    for score in [i32::MIN, -1, 0, i32::MAX] {
        post.score = score;
        assert_eq!(post.score(), Some(i64::from(score)));
    }
}
#[test]
fn post_normalizes_absent_parent_and_media() {
    use booru_rs::model::Post;
    use booru_rs::model::rule34::Rule34Post;

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for absent in ["sentinel", "null", "missing"] {
        let mut value = fixture[0].clone();
        for field in ["parent_id", "file_url", "preview_url", "sample_url"] {
            match absent {
                "missing" => {
                    value.as_object_mut().unwrap().remove(field);
                }
                "null" => value[field] = serde_json::Value::Null,
                _ if field == "parent_id" => value[field] = 0.into(),
                _ => value[field] = "".into(),
            }
        }
        let post: Rule34Post = serde_json::from_value(value).unwrap();
        assert_eq!(post.file_url(), None, "{absent}");
        let normalized = serde_json::to_value(&post).unwrap();
        for field in ["parent_id", "file_url", "preview_url", "sample_url"] {
            assert!(normalized[field].is_null(), "{absent}: {field}");
        }
        assert_eq!(
            serde_json::from_value::<Rule34Post>(normalized).unwrap(),
            post
        );
    }
}
#[test]
fn post_normalization_preserves_present_values_and_metadata() {
    use booru_rs::model::rule34::Rule34Post;

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for parent_id in [1, u32::MAX] {
        let mut value = fixture[0].clone();
        value["parent_id"] = parent_id.into();
        let post: Rule34Post = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(post.parent_id, Some(parent_id));
        let normalized = serde_json::to_value(&post).unwrap();
        for (field, actual) in normalized.as_object().unwrap() {
            assert_eq!(actual, &value[field], "{field}");
        }
        assert_eq!(
            serde_json::from_value::<Rule34Post>(normalized).unwrap(),
            post
        );
    }
}

#[test]
fn post_normalization_rejects_invalid_values() {
    use booru_rs::model::rule34::Rule34Post;

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for (field, invalid) in [
        ("parent_id", serde_json::json!(-1)),
        ("parent_id", serde_json::json!(u64::from(u32::MAX) + 1)),
        ("parent_id", serde_json::json!("0")),
        ("file_url", serde_json::json!(123)),
        ("preview_url", serde_json::json!(false)),
        ("sample_url", serde_json::json!([])),
    ] {
        let mut value = fixture[0].clone();
        value[field] = invalid;
        assert!(
            serde_json::from_value::<Rule34Post>(value).is_err(),
            "{field}"
        );
    }
    for field in ["id", "score", "width", "height", "tags", "rating"] {
        let mut value = fixture[0].clone();
        value.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<Rule34Post>(value).is_err(),
            "{field}"
        );
    }
}
#[test]
fn post_preserves_sample_metadata() {
    use booru_rs::model::rule34::Rule34Post;

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    let post: Rule34Post = serde_json::from_value(fixture[0].clone()).unwrap();
    assert_eq!(post.sample, Some(true));
    assert_eq!(post.sample_height, Some(600));
    assert_eq!(post.sample_width, Some(450));
    let serialized = serde_json::to_value(&post).unwrap();
    for field in ["sample", "sample_height", "sample_width"] {
        assert_eq!(serialized[field], fixture[0][field], "{field}");
    }
    assert_eq!(
        serde_json::from_value::<Rule34Post>(serialized).unwrap(),
        post
    );
}

#[test]
fn post_preserves_absent_and_zero_sample_metadata() {
    use booru_rs::model::rule34::Rule34Post;

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for missing in [true, false] {
        let mut value = fixture[0].clone();
        for field in ["sample", "sample_height", "sample_width"] {
            if missing {
                value.as_object_mut().unwrap().remove(field);
            } else {
                value[field] = serde_json::Value::Null;
            }
        }
        let post: Rule34Post = serde_json::from_value(value).unwrap();
        assert_eq!(post.sample, None);
        assert_eq!(post.sample_height, None);
        assert_eq!(post.sample_width, None);
        assert_eq!(
            serde_json::from_value::<Rule34Post>(serde_json::to_value(&post).unwrap()).unwrap(),
            post
        );
    }
    for dimension in [0, u32::MAX] {
        let mut value = fixture[0].clone();
        value["sample"] = false.into();
        value["sample_height"] = dimension.into();
        value["sample_width"] = dimension.into();
        let post: Rule34Post = serde_json::from_value(value).unwrap();
        assert_eq!(post.sample, Some(false));
        assert_eq!(post.sample_height, Some(dimension));
        assert_eq!(post.sample_width, Some(dimension));
    }
}

#[test]
fn post_rejects_invalid_sample_metadata() {
    use booru_rs::model::rule34::Rule34Post;

    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for (field, invalid) in [
        ("sample", serde_json::json!(1)),
        ("sample", serde_json::json!("true")),
        ("sample_height", serde_json::json!(-1)),
        ("sample_height", serde_json::json!(u64::from(u32::MAX) + 1)),
        ("sample_width", serde_json::json!(-1)),
        ("sample_width", serde_json::json!(u64::from(u32::MAX) + 1)),
        ("sample_height", serde_json::json!("600")),
        ("sample_width", serde_json::json!(1.5)),
    ] {
        let mut value = fixture[0].clone();
        value[field] = invalid;
        assert!(
            serde_json::from_value::<Rule34Post>(value).is_err(),
            "{field}"
        );
    }
}
#[test]
fn post_preserves_unknown_rating() {
    use booru_rs::rule34::{Rule34Post, Rule34PostRating, Rule34Rating};

    let mut fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for value in ["future_rating", "Explicit", "", " future rating "] {
        fixture[0]["rating"] = value.into();
        let post: Rule34Post = serde_json::from_value(fixture[0].clone()).unwrap();
        assert_eq!(post.rating, Rule34PostRating::Unknown(value.into()));
        assert_eq!(post.rating.to_string(), value);
        let serialized = serde_json::to_value(&post).unwrap();
        assert_eq!(serialized["rating"], value);
        assert_eq!(
            serde_json::from_value::<Rule34Post>(serialized).unwrap(),
            post
        );
        assert!(serde_json::from_value::<Rule34Rating>(value.into()).is_err());
    }
}

#[test]
fn post_recognizes_supported_ratings() {
    use booru_rs::rule34::{Rule34Post, Rule34PostRating, Rule34Rating};

    let mut fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for (value, rating) in [
        ("explicit", Rule34Rating::Explicit),
        ("questionable", Rule34Rating::Questionable),
        ("safe", Rule34Rating::Safe),
        ("general", Rule34Rating::General),
        ("sensitive", Rule34Rating::Sensitive),
    ] {
        fixture[0]["rating"] = value.into();
        let post: Rule34Post = serde_json::from_value(fixture[0].clone()).unwrap();
        assert_eq!(post.rating, Rule34PostRating::Known(rating));
        assert_eq!(post.rating, rating.into());
        assert_eq!(post.rating.to_string(), value);
        assert_eq!(serde_json::to_value(&post).unwrap()["rating"], value);
    }
}

#[test]
fn post_rejects_non_string_ratings() {
    use booru_rs::rule34::Rule34Post;

    let mut fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rule34/posts.json")).unwrap();
    for value in [
        serde_json::Value::Null,
        serde_json::json!(1),
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        fixture[0]["rating"] = value;
        assert!(serde_json::from_value::<Rule34Post>(fixture[0].clone()).is_err());
    }
}
