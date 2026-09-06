use booru_rs::danbooru::{Client, Query};
use booru_rs::error::BooruError;
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
    for (page, ids) in pages.iter().enumerate() {
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
        .and(query_param("page", "0"))
        .and(query_param("tags", "cat_ears"))
        .and(query_param("login", "test_user"))
        .and(query_param("api_key", "test_key"))
        .and(header("User-Agent", "booru-rs/0.3.0"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[7654321])))
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
        result.unwrap_err(),
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
        result.unwrap_err(),
        BooruError::PostNotFound(99999)
    ));
}

#[tokio::test]
async fn autocomplete_uses_instance_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/autocomplete.json"))
        .and(query_param("search[query]", "cat_"))
        .and(query_param("limit", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{"value":"cat_ears","label":"Cat ears (123)","category":0,"post_count":123}]"#,
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
async fn blacklist_appends_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/posts.json"))
        .and(query_param("tags", "cat_ears -explicit"))
        .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(&[])))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server);

    let posts = client
        .search()
        .tag("cat_ears")
        .blacklist_tag(booru_rs::model::danbooru::DanbooruRating::Explicit)
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
