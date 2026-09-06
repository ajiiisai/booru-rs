use booru_rs::error::BooruError;
use booru_rs::gelbooru::{Client, Query};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn posts_json(ids: &[u32]) -> String {
    let items: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                concat!(
                    "{{\"id\":{id},\"created_at\":\"2026-01-01 00:00:00\",",
                    "\"score\":10,\"width\":100,\"height\":100,",
                    "\"md5\":\"md5{id}\",\"file_url\":\"https://example.com/{id}.jpg\",",
                    "\"tags\":\"tag{id}\",\"image\":\"{id}.jpg\",\"source\":\"\",",
                    "\"rating\":\"general\"}}"
                ),
                id = id
            )
        })
        .collect();
    format!(
        "{{\"@attributes\":{{\"limit\":100,\"offset\":0,\"count\":{}}},\"post\":[{}]}}",
        items.len(),
        items.join(",")
    )
}

fn single_post_envelope(id: u32) -> String {
    posts_json(&[id])
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

fn ids(posts: &[booru_rs::model::gelbooru::GelbooruPost]) -> Vec<u32> {
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
        .and(query_param("tags", "cat_ears"))
        .and(query_param("json", "1"))
        .and(query_param("api_key", "test_key"))
        .and(query_param("user_id", "test_user"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[9876543])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .limit(10)
        .send()
        .await
        .expect("search must succeed");

    assert_eq!(ids(&posts), vec![9876543]);
}

#[tokio::test]
async fn missing_credentials_report_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .endpoint(mock_server.uri())
        .unwrap()
        .build()
        .unwrap();

    let result = client.search().send().await;

    assert!(matches!(result.unwrap_err(), BooruError::Unauthorized(_)));
}

#[tokio::test]
async fn post_returns_first_envelope_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("id", "9876543"))
        .and(query_param("api_key", "test_key"))
        .and(query_param("user_id", "test_user"))
        .respond_with(ResponseTemplate::new(200).set_body_string(single_post_envelope(9876543)))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let post = client.post(9876543).await.expect("lookup must succeed");

    assert_eq!(post.id, 9876543);
    assert_eq!(post.md5, "md59876543");
}

#[tokio::test]
async fn post_missing_maps_to_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("id", "99999"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let result = client.post(99999).await;

    assert!(matches!(
        result.unwrap_err(),
        BooruError::PostNotFound(99999)
    ));
}

#[tokio::test]
async fn autocomplete_uses_instance_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("page", "autocomplete2"))
        .and(query_param("term", "cat_"))
        .and(query_param("limit", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{"value":"cat_ears","label":"cat ears","post_count":"430787","category":"tag","tag":"cat_ears","type":"tag"}]"#,
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
    assert_eq!(suggestions[0].post_count, Some(430787));
    assert_eq!(suggestions[0].category, Some(0));
    assert_eq!(suggestions[0].tag.as_deref(), Some("cat_ears"));
    assert_eq!(suggestions[0].suggestion_type.as_deref(), Some("tag"));
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
async fn empty_envelope_without_post_key_is_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "zzznonexistenttagzzz"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"@attributes":{"limit":1,"offset":0,"count":0}}"#),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("zzznonexistenttagzzz")
        .send()
        .await
        .expect("empty search must succeed");

    assert!(posts.is_empty());
}

#[test]
fn validate_preflight() {
    assert!(Query::new().tag("cat_ears").validate().is_ok());
    assert!(Query::new().tag("cat ears").validate().is_err());
}

#[test]
fn default_client_builds() {
    assert!(Client::new().is_ok());
}

#[test]
fn client_shares_safely_across_tasks() {
    fn assert_send_sync_clone<T: Send + Sync + Clone>() {}

    assert_send_sync_clone::<Client>();
    assert_send_sync_clone::<booru_rs::gelbooru::Search>();
    assert_send_sync_clone::<booru_rs::gelbooru::Query>();
}

#[tokio::test]
async fn blacklist_and_random_append_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "cat_ears -explicit sort:random"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .blacklist_tag(booru_rs::model::gelbooru::GelbooruRating::Explicit)
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
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
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
async fn present_empty_post_list_is_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.php"))
        .and(query_param("tags", "zzznonexistenttagzzz"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"@attributes":{"limit":1,"offset":0,"count":0},"post":[]}"#),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("zzznonexistenttagzzz")
        .send()
        .await
        .expect("empty search must succeed");

    assert!(posts.is_empty());
}
#[test]
fn common_score_preserves_provider_range() {
    use booru_rs::model::Post;

    let mut post: booru_rs::model::gelbooru::GelbooruPost = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(&posts_json(&[1])).unwrap()["post"][0].clone(),
    )
    .unwrap();
    for score in [0, i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
        post.score = score;
        assert_eq!(post.score(), Some(i64::from(score)));
    }
}
#[test]
fn post_preserves_unknown_rating() {
    use booru_rs::gelbooru::{GelbooruPost, GelbooruRating};

    let mut fixture: serde_json::Value =
        serde_json::from_str::<serde_json::Value>(&posts_json(&[1])).unwrap()["post"][0].clone();
    fixture["rating"] = "future_rating".into();
    let post: GelbooruPost = serde_json::from_value(fixture).unwrap();
    assert_eq!(post.rating.to_string(), "future_rating");
    assert!(serde_json::from_value::<GelbooruRating>("future_rating".into()).is_err());
}
#[test]
fn post_preserves_optional_metadata() {
    use booru_rs::gelbooru::GelbooruPost;

    let mut value: serde_json::Value =
        serde_json::from_str::<serde_json::Value>(&posts_json(&[1])).unwrap()["post"][0].clone();
    value["directory"] = 42.into();
    value["change"] = 1700000000u64.into();
    value["owner"] = "owner".into();
    value["creator_id"] = 7.into();
    value["parent_id"] = 8.into();
    value["sample"] = true.into();
    value["preview_height"] = 90.into();
    value["preview_width"] = 120.into();
    value["title"] = "title".into();
    value["has_notes"] = false.into();
    value["has_comments"] = true.into();
    value["preview_url"] = "https://example.com/preview.jpg".into();
    value["sample_url"] = "https://example.com/sample.jpg".into();
    value["sample_height"] = 600.into();
    value["sample_width"] = 450.into();
    value["status"] = "active".into();
    value["post_locked"] = false.into();
    value["has_children"] = true.into();

    let post: GelbooruPost = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(post.directory, Some(42));
    assert_eq!(post.change, Some(1700000000));
    assert_eq!(post.owner.as_deref(), Some("owner"));
    assert_eq!(post.creator_id, Some(7));
    assert_eq!(post.parent_id, Some(8));
    assert_eq!(post.sample, Some(true));
    assert_eq!(post.preview_height, Some(90));
    assert_eq!(post.preview_width, Some(120));
    assert_eq!(post.title.as_deref(), Some("title"));
    assert_eq!(post.has_notes, Some(false));
    assert_eq!(post.has_comments, Some(true));
    assert_eq!(
        post.preview_url.as_deref(),
        Some("https://example.com/preview.jpg")
    );
    assert_eq!(
        post.sample_url.as_deref(),
        Some("https://example.com/sample.jpg")
    );
    assert_eq!(post.sample_height, Some(600));
    assert_eq!(post.sample_width, Some(450));
    assert_eq!(post.status.as_deref(), Some("active"));
    assert_eq!(post.post_locked, Some(false));
    assert_eq!(post.has_children, Some(true));
    assert_eq!(serde_json::to_value(&post).unwrap(), value);
}

#[test]
fn post_allows_missing_and_null_optional_metadata() {
    use booru_rs::gelbooru::GelbooruPost;

    let fixture: serde_json::Value =
        serde_json::from_str::<serde_json::Value>(&posts_json(&[1])).unwrap()["post"][0].clone();
    for null_value in [false, true] {
        let mut value = fixture.clone();
        for field in [
            "directory",
            "change",
            "owner",
            "creator_id",
            "parent_id",
            "sample",
            "preview_height",
            "preview_width",
            "title",
            "has_notes",
            "has_comments",
            "preview_url",
            "sample_url",
            "sample_height",
            "sample_width",
            "status",
            "post_locked",
            "has_children",
        ] {
            if null_value {
                value[field] = serde_json::Value::Null;
            } else {
                value.as_object_mut().unwrap().remove(field);
            }
        }
        let post: GelbooruPost = serde_json::from_value(value).unwrap();
        assert_eq!(post.directory, None);
        assert_eq!(post.change, None);
        assert_eq!(post.owner, None);
        assert_eq!(post.creator_id, None);
        assert_eq!(post.parent_id, None);
        assert_eq!(post.sample, None);
        assert_eq!(post.preview_height, None);
        assert_eq!(post.preview_width, None);
        assert_eq!(post.title, None);
        assert_eq!(post.has_notes, None);
        assert_eq!(post.has_comments, None);
        assert_eq!(post.preview_url, None);
        assert_eq!(post.sample_url, None);
        assert_eq!(post.sample_height, None);
        assert_eq!(post.sample_width, None);
        assert_eq!(post.status, None);
        assert_eq!(post.post_locked, None);
        assert_eq!(post.has_children, None);
    }
}
