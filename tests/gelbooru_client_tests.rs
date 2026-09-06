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
            r#"[{"value":"cat_ears","label":"cat ears","post_count":"430787","category":"tag"}]"#,
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
